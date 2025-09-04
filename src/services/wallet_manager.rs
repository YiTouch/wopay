// HD钱包管理服务
// 负责地址生成、私钥管理、资金归集等功能

use ethers::{
    prelude::*,
    providers::{Provider, Http},
    types::{Address, U256, TransactionRequest, Bytes},
    utils::parse_ether,
    signers::{LocalWallet, Signer},
};
use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::models::{PaymentAddress, WalletInfo};

/// HD钱包管理器
pub struct WalletManager {
    /// 主钱包（用于签名交易）
    master_wallet: LocalWallet,
    /// 以太坊提供者
    provider: Arc<Provider<Http>>,
    /// 地址索引计数器
    address_index: Arc<RwLock<u32>>,
    /// 地址到私钥的映射缓存
    address_cache: Arc<RwLock<HashMap<Address, LocalWallet>>>,
    /// 数据库连接池
    pool: PgPool,
    /// 归集阈值（ETH）
    collection_threshold: U256,
    /// 主归集地址
    master_address: Address,
}

impl WalletManager {
    /// 创建新的钱包管理器
    pub fn new(
        master_private_key: &str,
        provider: Arc<Provider<Http>>,
        pool: PgPool,
        collection_threshold_eth: f64,
    ) -> Result<Self> {
        let master_wallet: LocalWallet = master_private_key.parse()
            .context("Invalid master private key")?;
        
        let master_address = master_wallet.address();
        let collection_threshold = parse_ether(collection_threshold_eth)?;

        Ok(Self {
            master_wallet,
            provider,
            address_index: Arc::new(RwLock::new(0)),
            address_cache: Arc::new(RwLock::new(HashMap::new())),
            pool,
            collection_threshold,
            master_address,
        })
    }

    /// 生成新的支付地址
    /// 
    /// 使用HD钱包派生路径: m/44'/60'/0'/0/{index}
    pub async fn generate_payment_address(&self, payment_id: Uuid) -> Result<String> {
        let mut index_guard = self.address_index.write().await;
        let current_index = *index_guard;
        *index_guard += 1;
        drop(index_guard);

        // 在实际应用中，这里应该使用HD钱包派生
        // 为了演示，我们使用确定性方法生成地址
        let derived_key = self.derive_private_key(current_index)?;
        let wallet = LocalWallet::from(derived_key);
        let address = wallet.address();

        // 缓存地址和私钥
        {
            let mut cache = self.address_cache.write().await;
            cache.insert(address, wallet);
        }

        // 保存地址信息到数据库
        sqlx::query!(
            r#"
            INSERT INTO payment_addresses (
                id, payment_id, address_index, address, 
                private_key_encrypted, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            "#,
            Uuid::new_v4(),
            payment_id,
            current_index as i32,
            format!("{:?}", address),
            self.encrypt_private_key(&derived_key)?, // 加密存储私钥
        )
        .execute(&self.pool)
        .await
        .context("Failed to save payment address")?;

        log::info!("Generated payment address {} for payment {}", address, payment_id);
        Ok(format!("{:?}", address))
    }

    /// 检查并执行资金归集
    /// 
    /// 扫描所有有余额的地址，如果余额超过阈值则归集到主地址
    pub async fn collect_funds(&self) -> Result<Vec<String>> {
        let mut collected_txs = Vec::new();

        // 获取所有有余额的地址
        let addresses = self.get_funded_addresses().await?;

        for address_info in addresses {
            let address: Address = address_info.address.parse()
                .context("Invalid address format")?;

            // 检查余额
            let balance = self.provider.get_balance(address, None).await
                .context("Failed to get balance")?;

            if balance > self.collection_threshold {
                match self.collect_from_address(address, balance).await {
                    Ok(tx_hash) => {
                        collected_txs.push(tx_hash);
                        log::info!("Collected {} ETH from {} to master address", 
                            ethers::utils::format_ether(balance), address);
                    },
                    Err(e) => {
                        log::error!("Failed to collect from {}: {}", address, e);
                    }
                }
            }
        }

        Ok(collected_txs)
    }

    /// 从指定地址归集资金到主地址
    async fn collect_from_address(&self, from_address: Address, balance: U256) -> Result<String> {
        // 从缓存获取私钥
        let wallet = {
            let cache = self.address_cache.read().await;
            cache.get(&from_address).cloned()
                .ok_or_else(|| anyhow::anyhow!("Private key not found for address"))?
        };

        // 估算gas费用
        let gas_price = self.provider.get_gas_price().await?;
        let gas_limit = U256::from(21000); // 标准ETH转账gas限制
        let gas_cost = gas_price * gas_limit;

        // 确保余额足够支付gas费用
        if balance <= gas_cost {
            return Err(anyhow::anyhow!("Insufficient balance to cover gas fees"));
        }

        let amount_to_send = balance - gas_cost;

        // 构建交易
        let tx = TransactionRequest::new()
            .from(from_address)
            .to(self.master_address)
            .value(amount_to_send)
            .gas(gas_limit)
            .gas_price(gas_price);

        // 签名并发送交易
        let signed_tx = wallet.sign_transaction(&tx).await?;
        let tx_hash = self.provider.send_raw_transaction(signed_tx).await?;

        // 记录归集交易
        self.record_collection_transaction(from_address, amount_to_send, tx_hash).await?;

        Ok(format!("{:?}", tx_hash))
    }

    /// 获取有资金的地址列表
    async fn get_funded_addresses(&self) -> Result<Vec<PaymentAddressInfo>> {
        let addresses = sqlx::query_as!(
            PaymentAddressInfo,
            r#"
            SELECT address, address_index, created_at
            FROM payment_addresses 
            WHERE is_collected = false
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch payment addresses")?;

        Ok(addresses)
    }

    /// 记录归集交易
    async fn record_collection_transaction(
        &self,
        from_address: Address,
        amount: U256,
        tx_hash: H256,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO collection_transactions (
                id, from_address, to_address, amount, tx_hash, created_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
            Uuid::new_v4(),
            format!("{:?}", from_address),
            format!("{:?}", self.master_address),
            amount.to_string(),
            format!("{:?}", tx_hash),
        )
        .execute(&self.pool)
        .await
        .context("Failed to record collection transaction")?;

        // 标记地址为已归集
        sqlx::query!(
            r#"
            UPDATE payment_addresses 
            SET is_collected = true, updated_at = NOW()
            WHERE address = $1
            "#,
            format!("{:?}", from_address)
        )
        .execute(&self.pool)
        .await
        .context("Failed to update address collection status")?;

        Ok(())
    }

    /// 派生私钥（简化版本，实际应使用BIP32）
    fn derive_private_key(&self, index: u32) -> Result<k256::SecretKey> {
        // 这里应该使用真正的BIP32 HD钱包派生
        // 为了演示，使用简化的确定性生成
        use k256::elliptic_curve::rand_core::{RngCore, SeedableRng};
        use rand_chacha::ChaCha20Rng;
        
        let master_key = self.master_wallet.signer().to_bytes();
        let mut seed = [0u8; 32];
        seed[..master_key.len()].copy_from_slice(&master_key);
        
        // 使用index作为额外熵
        seed[28..32].copy_from_slice(&index.to_be_bytes());
        
        let mut rng = ChaCha20Rng::from_seed(seed);
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);
        
        k256::SecretKey::from_bytes(&key_bytes.into())
            .map_err(|e| anyhow::anyhow!("Failed to create secret key: {}", e))
    }

    /// 加密私钥存储
    fn encrypt_private_key(&self, private_key: &k256::SecretKey) -> Result<String> {
        // 实际应用中应使用AES加密
        // 这里为了演示使用简单的hex编码
        Ok(hex::encode(private_key.to_bytes()))
    }

    /// 获取钱包统计信息
    pub async fn get_wallet_stats(&self) -> Result<WalletStats> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_addresses,
                COUNT(CASE WHEN is_collected = true THEN 1 END) as collected_addresses,
                COUNT(CASE WHEN is_collected = false THEN 1 END) as active_addresses
            FROM payment_addresses
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get wallet stats")?;

        // 获取主地址余额
        let master_balance = self.provider.get_balance(self.master_address, None).await?;

        Ok(WalletStats {
            total_addresses: stats.total_addresses.unwrap_or(0) as u32,
            collected_addresses: stats.collected_addresses.unwrap_or(0) as u32,
            active_addresses: stats.active_addresses.unwrap_or(0) as u32,
            master_balance: ethers::utils::format_ether(master_balance),
            master_address: format!("{:?}", self.master_address),
        })
    }
}

/// 支付地址信息
#[derive(Debug)]
struct PaymentAddressInfo {
    address: String,
    address_index: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 钱包统计信息
#[derive(Debug, serde::Serialize)]
pub struct WalletStats {
    pub total_addresses: u32,
    pub collected_addresses: u32,
    pub active_addresses: u32,
    pub master_balance: String,
    pub master_address: String,
}
