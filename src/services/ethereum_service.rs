// 以太坊区块链服务
// 负责与以太坊网络交互，包括地址生成、交易监听、余额查询等

use ethers::{
    prelude::*,
    providers::{Provider, Ws, Http},
    types::{Address, U256, H256, Filter, Log, TransactionRequest, Bytes},
    utils::parse_ether,
};
use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::models::{PaymentStatus, Currency, BlockchainTransaction, TransactionStatus};

/// 以太坊服务
#[derive(Clone)]
pub struct EthereumService {
    http_provider: Arc<Provider<Http>>,
    ws_provider: Option<Arc<Provider<Ws>>>,
    chain_id: u64,
    confirmation_blocks: u64,
}

impl EthereumService {
    /// 创建新的以太坊服务实例
    /// 
    /// # Arguments
    /// * `rpc_url` - HTTP RPC节点URL
    /// * `ws_url` - WebSocket节点URL (可选)
    /// * `chain_id` - 链ID (1=主网, 5=Goerli测试网)
    /// 
    /// # Returns
    /// * 以太坊服务实例
    pub async fn new_with_config(
        rpc_url: String,
        ws_url: Option<String>,
        chain_id: u64,
    ) -> Result<Self> {
        // 创建HTTP Provider
        let http_provider = Provider::<Http>::try_from(&rpc_url)
            .context("Failed to create HTTP provider")?;

        // 创建WebSocket Provider (如果提供)
        let ws_provider = if let Some(ws_url) = ws_url {
            let ws = Ws::connect(&ws_url).await
                .context("Failed to connect to WebSocket provider")?;
            Some(Arc::new(Provider::new(ws)))
        } else {
            None
        };

        // 验证连接
        let network = http_provider.get_chainid().await
            .context("Failed to get chain ID from provider")?;
        
        if network.as_u64() != chain_id {
            anyhow::bail!("Chain ID mismatch: expected {}, got {}", chain_id, network);
        }

        let confirmation_blocks = match chain_id {
            1 => 12,  // 主网需要更多确认
            5 => 6,   // Goerli测试网
            _ => 6,   // 默认6个确认
        };

        log::info!("Connected to Ethereum network (chain_id: {})", chain_id);

        Ok(Self {
            http_provider: Arc::new(http_provider),
            ws_provider,
            chain_id,
            confirmation_blocks,
        })
    }

    /// 生成支付地址
    /// 
    /// # Returns
    /// * 新生成的以太坊地址
    pub async fn generate_payment_address(&self) -> Result<String> {
        // 在实际应用中，这里应该从HD钱包派生地址
        // 为了演示，我们生成一个随机地址
        let wallet = LocalWallet::new(&mut rand::thread_rng());
        let address = wallet.address();
        
        log::debug!("Generated payment address: {:?}", address);
        Ok(format!("{:?}", address))
    }

    /// 获取地址余额
    /// 
    /// # Arguments
    /// * `address` - 以太坊地址
    /// * `currency` - 币种类型
    /// 
    /// # Returns
    /// * 余额 (以最小单位计算)
    pub async fn get_balance(&self, address: &str, currency: &Currency) -> Result<U256> {
        let address: Address = address.parse()
            .context("Invalid Ethereum address")?;

        match currency {
            Currency::ETH => {
                let balance = self.http_provider.get_balance(address, None).await
                    .context("Failed to get ETH balance")?;
                Ok(balance)
            },
            Currency::USDT => {
                // USDT是ERC20代币，需要调用合约
                let contract_address = currency.contract_address()
                    .ok_or_else(|| anyhow::anyhow!("No contract address for currency"))?;
                
                self.get_erc20_balance(address, &contract_address).await
            }
        }
    }

    /// 获取ERC20代币余额
    async fn get_erc20_balance(&self, address: Address, contract_address: &str) -> Result<U256> {
        let contract_addr: Address = contract_address.parse()
            .context("Invalid contract address")?;

        // ERC20 balanceOf函数调用
        let call = self.http_provider.call(
            &TransactionRequest::new()
                .to(contract_addr)
                .data(
                    // balanceOf(address) function selector + address parameter
                    format!("0x70a08231{:0>64}", format!("{:x}", address))
                        .parse::<Bytes>()
                        .context("Failed to encode function call")?
                ),
            None
        ).await
        .context("Failed to call balanceOf")?;

        // 解析返回值
        let balance = U256::from_big_endian(&call);
        Ok(balance)
    }

    /// 监听支付地址的交易
    /// 
    /// # Arguments
    /// * `payment_id` - 支付订单ID
    /// * `payment_address` - 支付地址
    /// * `pool` - 数据库连接池
    /// 
    /// # Returns
    /// * 监听结果
    pub async fn monitor_payment(
        &self,
        payment_id: Uuid,
        payment_address: &str,
        pool: PgPool,
    ) -> Result<()> {
        let address: Address = payment_address.parse()
            .context("Invalid payment address")?;

        log::info!("Starting payment monitoring for address: {:?}", address);

        // 获取当前区块号
        let current_block = self.http_provider.get_block_number().await
            .context("Failed to get current block number")?;

        // 创建过滤器监听转入交易
        let filter = Filter::new()
            .address(address)
            .from_block(current_block);

        // 如果有WebSocket连接，使用实时监听
        if let Some(ws_provider) = &self.ws_provider {
            self.monitor_with_websocket(payment_id, address, pool, ws_provider.clone()).await
        } else {
            // 否则使用轮询方式
            self.monitor_with_polling(payment_id, address, pool, current_block).await
        }
    }

    /// 使用WebSocket实时监听
    async fn monitor_with_websocket(
        &self,
        payment_id: Uuid,
        address: Address,
        pool: PgPool,
        ws_provider: Arc<Provider<Ws>>,
    ) -> Result<()> {
        let mut stream = ws_provider.subscribe_logs(
            &Filter::new().address(address)
        ).await
        .context("Failed to subscribe to logs")?;

        // 设置超时时间 (1小时)
        let timeout = tokio::time::timeout(Duration::from_secs(3600), async {
            while let Some(log) = stream.next().await {
                if let Err(e) = self.process_transaction_log(payment_id, log, &pool).await {
                    log::error!("Failed to process transaction log: {}", e);
                }
            }
        });

        match timeout.await {
            Ok(_) => log::info!("WebSocket monitoring completed for payment: {}", payment_id),
            Err(_) => log::warn!("WebSocket monitoring timed out for payment: {}", payment_id),
        }

        Ok(())
    }

    /// 使用轮询方式监听
    async fn monitor_with_polling(
        &self,
        payment_id: Uuid,
        address: Address,
        pool: PgPool,
        start_block: U64,
    ) -> Result<()> {
        let mut last_checked_block = start_block;
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 720; // 1小时 (每5秒检查一次)

        while attempts < MAX_ATTEMPTS {
            sleep(Duration::from_secs(5)).await;
            attempts += 1;

            // 获取最新区块
            let latest_block = match self.http_provider.get_block_number().await {
                Ok(block) => block,
                Err(e) => {
                    log::warn!("Failed to get latest block: {}", e);
                    continue;
                }
            };

            if latest_block <= last_checked_block {
                continue;
            }

            // 查询新区块中的交易
            let filter = Filter::new()
                .address(address)
                .from_block(last_checked_block + 1)
                .to_block(latest_block);

            match self.http_provider.get_logs(&filter).await {
                Ok(logs) => {
                    for log in logs {
                        if let Err(e) = self.process_transaction_log(payment_id, log, &pool).await {
                            log::error!("Failed to process transaction log: {}", e);
                        }
                    }
                },
                Err(e) => {
                    log::warn!("Failed to get logs: {}", e);
                }
            }

            last_checked_block = latest_block;

            // 检查支付状态，如果已完成则停止监听
            if let Ok(Some(payment)) = self.get_payment_from_db(payment_id, &pool).await {
                if payment.status == PaymentStatus::Completed || payment.status == PaymentStatus::Failed {
                    log::info!("Payment {} completed, stopping monitoring", payment_id);
                    break;
                }
            }
        }

        log::info!("Polling monitoring completed for payment: {}", payment_id);
        Ok(())
    }

    /// 处理交易日志
    async fn process_transaction_log(
        &self,
        payment_id: Uuid,
        log: Log,
        pool: &PgPool,
    ) -> Result<()> {
        let tx_hash = log.transaction_hash
            .ok_or_else(|| anyhow::anyhow!("No transaction hash in log"))?;

        // 获取交易详情
        let tx = self.http_provider.get_transaction(tx_hash).await
            .context("Failed to get transaction")?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;

        // 获取交易回执
        let receipt = self.http_provider.get_transaction_receipt(tx_hash).await
            .context("Failed to get transaction receipt")?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        // 检查交易是否成功
        let status = if receipt.status == Some(U64::from(1)) {
            TransactionStatus::Success
        } else {
            TransactionStatus::Failed
        };

        // 记录区块链交易
        let blockchain_tx_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO blockchain_transactions (
                id, payment_id, tx_hash, from_address, to_address,
                amount, gas_used, gas_price, block_number, block_hash,
                status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
            ON CONFLICT (tx_hash) DO NOTHING
            "#,
            blockchain_tx_id,
            payment_id,
            format!("{:?}", tx_hash),
            format!("{:?}", tx.from),
            format!("{:?}", tx.to.unwrap_or_default()),
            tx.value.to_string(),
            receipt.gas_used.map(|g| g.as_u64() as i64),
            tx.gas_price.map(|g| g.to_string()),
            receipt.block_number.map(|b| b.as_u64() as i64),
            receipt.block_hash.map(|h| format!("{:?}", h)),
            status as TransactionStatus
        )
        .execute(pool)
        .await
        .context("Failed to insert blockchain transaction")?;

        // 更新支付状态
        if status == TransactionStatus::Success {
            // 检查确认数
            let current_block = self.http_provider.get_block_number().await?;
            let confirmations = if let Some(tx_block) = receipt.block_number {
                (current_block.as_u64() - tx_block.as_u64()) as i32
            } else {
                0
            };

            let payment_status = if confirmations >= self.confirmation_blocks as i32 {
                PaymentStatus::Completed
            } else {
                PaymentStatus::Confirmed
            };

            sqlx::query!(
                r#"
                UPDATE payments 
                SET status = $1, transaction_hash = $2, confirmations = $3, updated_at = NOW()
                WHERE id = $4
                "#,
                payment_status as PaymentStatus,
                format!("{:?}", tx_hash),
                confirmations,
                payment_id
            )
            .execute(pool)
            .await
            .context("Failed to update payment status")?;

            log::info!("Payment {} updated to {:?} with {} confirmations", 
                payment_id, payment_status, confirmations);
        } else {
            // 交易失败
            sqlx::query!(
                r#"
                UPDATE payments 
                SET status = 'failed', transaction_hash = $1, updated_at = NOW()
                WHERE id = $2
                "#,
                format!("{:?}", tx_hash),
                payment_id
            )
            .execute(pool)
            .await
            .context("Failed to update payment status to failed")?;

            log::warn!("Payment {} marked as failed due to transaction failure", payment_id);
        }

        Ok(())
    }

    /// 验证交易确认数
    /// 
    /// # Arguments
    /// * `tx_hash` - 交易哈希
    /// 
    /// # Returns
    /// * 确认数
    pub async fn get_transaction_confirmations(&self, tx_hash: &str) -> Result<u64> {
        let hash: H256 = tx_hash.parse()
            .context("Invalid transaction hash")?;

        let receipt = self.http_provider.get_transaction_receipt(hash).await
            .context("Failed to get transaction receipt")?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;

        let current_block = self.http_provider.get_block_number().await
            .context("Failed to get current block number")?;

        let confirmations = if let Some(tx_block) = receipt.block_number {
            current_block.as_u64() - tx_block.as_u64()
        } else {
            0
        };

        Ok(confirmations)
    }

    /// 估算Gas费用
    /// 
    /// # Arguments
    /// * `to` - 接收地址
    /// * `value` - 转账金额
    /// 
    /// # Returns
    /// * Gas费用估算 (wei)
    pub async fn estimate_gas_fee(&self, to: &str, value: U256) -> Result<U256> {
        let to_address: Address = to.parse()
            .context("Invalid to address")?;

        let tx = TransactionRequest::new()
            .to(to_address)
            .value(value);

        let gas_estimate = self.http_provider.estimate_gas(&tx, None).await
            .context("Failed to estimate gas")?;

        let gas_price = self.http_provider.get_gas_price().await
            .context("Failed to get gas price")?;

        let total_fee = gas_estimate * gas_price;
        Ok(total_fee)
    }

    /// 获取当前Gas价格
    /// 
    /// # Returns
    /// * Gas价格 (wei)
    pub async fn get_gas_price(&self) -> Result<U256> {
        self.http_provider.get_gas_price().await
            .context("Failed to get gas price")
    }

    /// 检查地址是否为合约地址
    /// 
    /// # Arguments
    /// * `address` - 以太坊地址
    /// 
    /// # Returns
    /// * 是否为合约地址
    pub async fn is_contract(&self, address: &str) -> Result<bool> {
        let addr: Address = address.parse()
            .context("Invalid Ethereum address")?;

        let code = self.http_provider.get_code(addr, None).await
            .context("Failed to get contract code")?;

        Ok(!code.is_empty())
    }

    /// 验证交易哈希格式
    /// 
    /// # Arguments
    /// * `tx_hash` - 交易哈希
    /// 
    /// # Returns
    /// * 验证结果
    pub fn validate_transaction_hash(&self, tx_hash: &str) -> Result<()> {
        if tx_hash.len() != 66 || !tx_hash.starts_with("0x") {
            anyhow::bail!("Invalid transaction hash format");
        }

        tx_hash.parse::<H256>()
            .context("Invalid transaction hash")?;

        Ok(())
    }

    /// 从数据库获取支付信息
    async fn get_payment_from_db(&self, payment_id: Uuid, pool: &PgPool) -> Result<Option<crate::models::Payment>> {
        let payment = sqlx::query_as!(
            crate::models::Payment,
            r#"
            SELECT id, merchant_id, order_id, amount, 
                   currency as "currency: _", payment_address,
                   status as "status: _", transaction_hash, confirmations,
                   expires_at, created_at, updated_at
            FROM payments 
            WHERE id = $1
            "#,
            payment_id
        )
        .fetch_optional(pool)
        .await
        .context("Failed to fetch payment from database")?;

        Ok(payment)
    }

    /// 批量检查确认数并更新状态
    /// 
    /// # Arguments
    /// * `pool` - 数据库连接池
    /// 
    /// # Returns
    /// * 更新的支付订单数量
    pub async fn update_confirmations(&self, pool: &PgPool) -> Result<u64> {
        // 获取所有已确认但未完成的支付
        let payments = sqlx::query!(
            r#"
            SELECT id, transaction_hash, confirmations
            FROM payments 
            WHERE status = 'confirmed' AND transaction_hash IS NOT NULL
            "#
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch confirmed payments")?;

        let mut updated_count = 0;

        for payment in payments {
            if let Some(tx_hash) = payment.transaction_hash {
                match self.get_transaction_confirmations(&tx_hash).await {
                    Ok(confirmations) => {
                        let confirmations_i32 = confirmations as i32;
                        
                        // 如果确认数达到要求，标记为完成
                        if confirmations >= self.confirmation_blocks {
                            sqlx::query!(
                                r#"
                                UPDATE payments 
                                SET status = 'completed', confirmations = $1, updated_at = NOW()
                                WHERE id = $2
                                "#,
                                confirmations_i32,
                                payment.id
                            )
                            .execute(pool)
                            .await
                            .context("Failed to update payment to completed")?;

                            log::info!("Payment {} completed with {} confirmations", 
                                payment.id, confirmations);
                            updated_count += 1;
                        } else if confirmations_i32 != payment.confirmations.unwrap_or(0) {
                            // 更新确认数
                            sqlx::query!(
                                r#"
                                UPDATE payments 
                                SET confirmations = $1, updated_at = NOW()
                                WHERE id = $2
                                "#,
                                confirmations_i32,
                                payment.id
                            )
                            .execute(pool)
                            .await
                            .context("Failed to update payment confirmations")?;
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to get confirmations for payment {}: {}", payment.id, e);
                    }
                }
            }
        }

        Ok(updated_count)
    }

    /// 获取网络状态
    /// 
    /// # Returns
    /// * 网络状态信息
    pub async fn get_network_status(&self) -> Result<NetworkStatus> {
        let block_number = self.http_provider.get_block_number().await
            .context("Failed to get block number")?;

        let gas_price = self.http_provider.get_gas_price().await
            .context("Failed to get gas price")?;

        let syncing = self.http_provider.syncing().await
            .context("Failed to get sync status")?;

        Ok(NetworkStatus {
            chain_id: self.chain_id,
            block_number: block_number.as_u64(),
            gas_price: gas_price.as_u64(),
            is_syncing: syncing.is_some(),
            confirmation_blocks: self.confirmation_blocks,
        })
    }
}

/// 网络状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkStatus {
    /// 链ID
    pub chain_id: u64,
    /// 当前区块号
    pub block_number: u64,
    /// 当前Gas价格 (wei)
    pub gas_price: u64,
    /// 是否正在同步
    pub is_syncing: bool,
    /// 需要的确认区块数
    pub confirmation_blocks: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ethereum_service_creation() {
        let service = EthereumService::new_with_config(
            "https://eth-goerli.alchemyapi.io/v2/demo".to_string(),
            None,
            5, // Goerli testnet
        ).await;

        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_generate_payment_address() {
        let service = EthereumService::new_with_config(
            "https://eth-goerli.alchemyapi.io/v2/demo".to_string(),
            None,
            5,
        ).await.unwrap();

        let address = service.generate_payment_address().await.unwrap();
        
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
    }

    #[tokio::test]
    async fn test_validate_transaction_hash() {
        let service = EthereumService::new_with_config(
            "https://eth-goerli.alchemyapi.io/v2/demo".to_string(),
            None,
            5,
        ).await.unwrap();

        // 有效的交易哈希
        let valid_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert!(service.validate_transaction_hash(valid_hash).is_ok());

        // 无效的交易哈希
        let invalid_hash = "invalid_hash";
        assert!(service.validate_transaction_hash(invalid_hash).is_err());
    }
}
