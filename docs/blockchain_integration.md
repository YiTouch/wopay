# WoPay 区块链集成架构设计

## 1. 多链支持架构

### 1.1 区块链适配器模式

```rust
// 通用区块链接口
pub trait BlockchainAdapter {
    async fn create_wallet(&self) -> Result<WalletInfo, BlockchainError>;
    async fn get_balance(&self, address: &str, token: Option<&str>) -> Result<Balance, BlockchainError>;
    async fn send_transaction(&self, tx: TransactionRequest) -> Result<TransactionHash, BlockchainError>;
    async fn get_transaction(&self, hash: &str) -> Result<Transaction, BlockchainError>;
    async fn listen_for_payments(&self, address: &str) -> Result<PaymentStream, BlockchainError>;
    async fn estimate_fee(&self, tx: &TransactionRequest) -> Result<Fee, BlockchainError>;
}

// 以太坊适配器实现
pub struct EthereumAdapter {
    client: Provider<Http>,
    chain_id: u64,
}

// Solana适配器实现
pub struct SolanaAdapter {
    client: RpcClient,
    commitment: CommitmentConfig,
}

// BSC适配器实现
pub struct BscAdapter {
    client: Provider<Http>,
    chain_id: u64,
}
```

### 1.2 支持的区块链网络

#### Ethereum生态
- **主网**: Ethereum Mainnet
- **测试网**: Goerli, Sepolia
- **Layer 2**: Polygon, Arbitrum, Optimism
- **支持代币**: ETH, USDT, USDC, DAI等ERC20代币

#### Solana生态
- **主网**: Solana Mainnet
- **测试网**: Devnet, Testnet
- **支持代币**: SOL, USDT, USDC等SPL代币

#### BSC生态
- **主网**: Binance Smart Chain
- **测试网**: BSC Testnet
- **支持代币**: BNB, BUSD, USDT等BEP20代币

## 2. 智能合约设计

### 2.1 WoPay网关合约 (Ethereum/BSC)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract WoPayGateway is ReentrancyGuard, Ownable {
    struct Payment {
        address merchant;
        address token;
        uint256 amount;
        uint256 fee;
        bool completed;
        uint256 timestamp;
    }
    
    mapping(bytes32 => Payment) public payments;
    mapping(address => bool) public authorizedMerchants;
    
    uint256 public feeRate = 250; // 2.5% = 250/10000
    address public feeRecipient;
    
    event PaymentCreated(
        bytes32 indexed paymentId,
        address indexed merchant,
        address indexed token,
        uint256 amount,
        uint256 fee
    );
    
    event PaymentCompleted(
        bytes32 indexed paymentId,
        address indexed payer,
        uint256 timestamp
    );
    
    constructor(address _feeRecipient) {
        feeRecipient = _feeRecipient;
    }
    
    function createPayment(
        bytes32 paymentId,
        address token,
        uint256 amount
    ) external {
        require(authorizedMerchants[msg.sender], "Unauthorized merchant");
        require(payments[paymentId].merchant == address(0), "Payment already exists");
        
        uint256 fee = (amount * feeRate) / 10000;
        
        payments[paymentId] = Payment({
            merchant: msg.sender,
            token: token,
            amount: amount,
            fee: fee,
            completed: false,
            timestamp: block.timestamp
        });
        
        emit PaymentCreated(paymentId, msg.sender, token, amount, fee);
    }
    
    function completePayment(bytes32 paymentId) external payable nonReentrant {
        Payment storage payment = payments[paymentId];
        require(payment.merchant != address(0), "Payment not found");
        require(!payment.completed, "Payment already completed");
        
        if (payment.token == address(0)) {
            // ETH payment
            require(msg.value >= payment.amount + payment.fee, "Insufficient payment");
            
            payable(payment.merchant).transfer(payment.amount);
            payable(feeRecipient).transfer(payment.fee);
            
            // Refund excess
            if (msg.value > payment.amount + payment.fee) {
                payable(msg.sender).transfer(msg.value - payment.amount - payment.fee);
            }
        } else {
            // ERC20 token payment
            IERC20 token = IERC20(payment.token);
            require(
                token.transferFrom(msg.sender, payment.merchant, payment.amount),
                "Merchant transfer failed"
            );
            require(
                token.transferFrom(msg.sender, feeRecipient, payment.fee),
                "Fee transfer failed"
            );
        }
        
        payment.completed = true;
        emit PaymentCompleted(paymentId, msg.sender, block.timestamp);
    }
    
    function authorizeMerchant(address merchant) external onlyOwner {
        authorizedMerchants[merchant] = true;
    }
    
    function revokeMerchant(address merchant) external onlyOwner {
        authorizedMerchants[merchant] = false;
    }
    
    function updateFeeRate(uint256 newFeeRate) external onlyOwner {
        require(newFeeRate <= 1000, "Fee rate too high"); // Max 10%
        feeRate = newFeeRate;
    }
}
```

### 2.2 Solana程序设计

```rust
// Solana WoPay程序
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("WoPay11111111111111111111111111111111111");

#[program]
pub mod wopay_solana {
    use super::*;
    
    pub fn create_payment(
        ctx: Context<CreatePayment>,
        payment_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        let payment = &mut ctx.accounts.payment;
        payment.merchant = ctx.accounts.merchant.key();
        payment.mint = ctx.accounts.mint.key();
        payment.amount = amount;
        payment.fee = amount * 250 / 10000; // 2.5%
        payment.completed = false;
        payment.bump = *ctx.bumps.get("payment").unwrap();
        
        emit!(PaymentCreated {
            payment_id,
            merchant: payment.merchant,
            mint: payment.mint,
            amount,
            fee: payment.fee,
        });
        
        Ok(())
    }
    
    pub fn complete_payment(ctx: Context<CompletePayment>) -> Result<()> {
        let payment = &mut ctx.accounts.payment;
        require!(!payment.completed, ErrorCode::PaymentAlreadyCompleted);
        
        // Transfer tokens to merchant
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_token_account.to_account_info(),
            to: ctx.accounts.merchant_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, payment.amount)?;
        
        // Transfer fee
        let fee_cpi_accounts = Transfer {
            from: ctx.accounts.payer_token_account.to_account_info(),
            to: ctx.accounts.fee_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let fee_cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), fee_cpi_accounts);
        token::transfer(fee_cpi_ctx, payment.fee)?;
        
        payment.completed = true;
        
        emit!(PaymentCompleted {
            payment_id: payment.key().to_bytes(),
            payer: ctx.accounts.payer.key(),
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(payment_id: [u8; 32])]
pub struct CreatePayment<'info> {
    #[account(
        init,
        payer = merchant,
        space = 8 + Payment::INIT_SPACE,
        seeds = [b"payment", payment_id.as_ref()],
        bump
    )]
    pub payment: Account<'info, Payment>,
    
    #[account(mut)]
    pub merchant: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Payment {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub completed: bool,
    pub bump: u8,
}

#[event]
pub struct PaymentCreated {
    pub payment_id: [u8; 32],
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct PaymentCompleted {
    pub payment_id: [u8; 32],
    pub payer: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Payment already completed")]
    PaymentAlreadyCompleted,
}
```

## 3. 交易监听与确认机制

### 3.1 区块链监听器架构

```rust
// 交易监听器接口
#[async_trait]
pub trait TransactionListener {
    async fn start_listening(&self) -> Result<(), ListenerError>;
    async fn stop_listening(&self) -> Result<(), ListenerError>;
    async fn subscribe_to_address(&self, address: &str) -> Result<(), ListenerError>;
    async fn unsubscribe_from_address(&self, address: &str) -> Result<(), ListenerError>;
}

// 以太坊监听器实现
pub struct EthereumListener {
    provider: Provider<Ws>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionStream<Log>>>>,
    event_sender: mpsc::UnboundedSender<BlockchainEvent>,
}

impl EthereumListener {
    pub async fn new(ws_url: &str, event_sender: mpsc::UnboundedSender<BlockchainEvent>) -> Result<Self, ListenerError> {
        let provider = Provider::<Ws>::connect(ws_url).await?;
        
        Ok(Self {
            provider,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        })
    }
    
    async fn process_log(&self, log: Log) -> Result<(), ListenerError> {
        // 解析日志并发送事件
        if let Ok(event) = self.parse_payment_event(log).await {
            self.event_sender.send(event)?;
        }
        Ok(())
    }
}

// Solana监听器实现
pub struct SolanaListener {
    client: RpcClient,
    subscriptions: Arc<RwLock<HashMap<String, PubsubClient>>>,
    event_sender: mpsc::UnboundedSender<BlockchainEvent>,
}

impl SolanaListener {
    pub async fn new(rpc_url: &str, ws_url: &str, event_sender: mpsc::UnboundedSender<BlockchainEvent>) -> Result<Self, ListenerError> {
        let client = RpcClient::new(rpc_url);
        
        Ok(Self {
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        })
    }
    
    async fn process_account_change(&self, account_info: UiAccount) -> Result<(), ListenerError> {
        // 处理账户变化并发送事件
        if let Ok(event) = self.parse_account_change(account_info).await {
            self.event_sender.send(event)?;
        }
        Ok(())
    }
}
```

### 3.2 确认机制设计

```rust
#[derive(Debug, Clone)]
pub struct ConfirmationConfig {
    pub required_confirmations: u32,
    pub max_wait_time: Duration,
    pub check_interval: Duration,
}

impl Default for ConfirmationConfig {
    fn default() -> Self {
        Self {
            required_confirmations: 12, // Ethereum默认12个确认
            max_wait_time: Duration::from_secs(3600), // 1小时超时
            check_interval: Duration::from_secs(30), // 30秒检查一次
        }
    }
}

pub struct ConfirmationTracker {
    blockchain_service: Arc<dyn BlockchainAdapter>,
    config: ConfirmationConfig,
    pending_transactions: Arc<RwLock<HashMap<String, PendingTransaction>>>,
}

impl ConfirmationTracker {
    pub async fn track_transaction(&self, tx_hash: &str, payment_id: Uuid) -> Result<(), ConfirmationError> {
        let pending_tx = PendingTransaction {
            payment_id,
            tx_hash: tx_hash.to_string(),
            confirmations: 0,
            start_time: Instant::now(),
        };
        
        self.pending_transactions.write().await.insert(tx_hash.to_string(), pending_tx);
        
        // 启动确认检查任务
        let tracker = self.clone();
        let tx_hash = tx_hash.to_string();
        tokio::spawn(async move {
            tracker.check_confirmations_loop(tx_hash).await;
        });
        
        Ok(())
    }
    
    async fn check_confirmations_loop(&self, tx_hash: String) {
        let mut interval = tokio::time::interval(self.config.check_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_single_transaction(&tx_hash).await {
                error!("Error checking transaction {}: {}", tx_hash, e);
                continue;
            }
            
            // 检查是否应该停止跟踪
            let should_stop = {
                let pending = self.pending_transactions.read().await;
                if let Some(tx) = pending.get(&tx_hash) {
                    tx.confirmations >= self.config.required_confirmations ||
                    tx.start_time.elapsed() > self.config.max_wait_time
                } else {
                    true
                }
            };
            
            if should_stop {
                self.pending_transactions.write().await.remove(&tx_hash);
                break;
            }
        }
    }
}
```

## 4. 钱包管理系统

### 4.1 分层确定性钱包 (HD Wallet)

```rust
use bip39::{Mnemonic, Language, Seed};
use bip32::{ExtendedPrivateKey, DerivationPath};
use secp256k1::SecretKey;

pub struct HdWalletManager {
    master_seed: Seed,
    ethereum_master_key: ExtendedPrivateKey,
    solana_master_key: ExtendedPrivateKey,
}

impl HdWalletManager {
    pub fn new(mnemonic: &str) -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
        let seed = Seed::new(&mnemonic, "");
        
        let ethereum_master_key = ExtendedPrivateKey::new(&seed)?;
        let solana_master_key = ExtendedPrivateKey::new(&seed)?;
        
        Ok(Self {
            master_seed: seed,
            ethereum_master_key,
            solana_master_key,
        })
    }
    
    pub fn derive_ethereum_wallet(&self, account_index: u32) -> Result<EthereumWallet, WalletError> {
        // m/44'/60'/0'/0/{account_index}
        let path = DerivationPath::from_str(&format!("m/44'/60'/0'/0/{}", account_index))?;
        let private_key = self.ethereum_master_key.derive_priv(&path)?;
        
        Ok(EthereumWallet::from_private_key(private_key.private_key()))
    }
    
    pub fn derive_solana_wallet(&self, account_index: u32) -> Result<SolanaWallet, WalletError> {
        // m/44'/501'/0'/{account_index}
        let path = DerivationPath::from_str(&format!("m/44'/501'/0'/{}", account_index))?;
        let private_key = self.ethereum_master_key.derive_priv(&path)?;
        
        Ok(SolanaWallet::from_private_key(private_key.private_key()))
    }
}
```

### 4.2 冷热钱包分离

```rust
pub enum WalletType {
    Hot,    // 热钱包：在线签名，小额资金
    Warm,   // 温钱包：半离线，中等资金
    Cold,   // 冷钱包：离线签名，大额资金
}

pub struct WalletTier {
    pub wallet_type: WalletType,
    pub max_balance: u64,
    pub daily_limit: u64,
    pub requires_approval: bool,
}

pub struct TieredWalletManager {
    hot_wallet: HotWallet,
    warm_wallet: WarmWallet,
    cold_wallet: ColdWallet,
    tiers: Vec<WalletTier>,
}

impl TieredWalletManager {
    pub async fn process_withdrawal(&self, amount: u64, destination: &str) -> Result<TransactionHash, WalletError> {
        let tier = self.determine_wallet_tier(amount)?;
        
        match tier.wallet_type {
            WalletType::Hot => {
                self.hot_wallet.send_transaction(amount, destination).await
            },
            WalletType::Warm => {
                if tier.requires_approval {
                    self.request_approval(amount, destination).await?;
                }
                self.warm_wallet.send_transaction(amount, destination).await
            },
            WalletType::Cold => {
                self.initiate_cold_wallet_transaction(amount, destination).await
            }
        }
    }
}
```

## 5. 多签名安全机制

### 5.1 多签名钱包合约

```solidity
contract MultiSigWallet {
    mapping(address => bool) public isOwner;
    address[] public owners;
    uint256 public required;
    
    struct Transaction {
        address to;
        uint256 value;
        bytes data;
        bool executed;
        uint256 confirmations;
    }
    
    Transaction[] public transactions;
    mapping(uint256 => mapping(address => bool)) public confirmations;
    
    modifier onlyOwner() {
        require(isOwner[msg.sender], "Not an owner");
        _;
    }
    
    modifier notExecuted(uint256 transactionId) {
        require(!transactions[transactionId].executed, "Transaction already executed");
        _;
    }
    
    constructor(address[] memory _owners, uint256 _required) {
        require(_owners.length > 0, "Owners required");
        require(_required > 0 && _required <= _owners.length, "Invalid required confirmations");
        
        for (uint256 i = 0; i < _owners.length; i++) {
            address owner = _owners[i];
            require(owner != address(0), "Invalid owner");
            require(!isOwner[owner], "Owner not unique");
            
            isOwner[owner] = true;
            owners.push(owner);
        }
        
        required = _required;
    }
    
    function submitTransaction(address to, uint256 value, bytes memory data) 
        public 
        onlyOwner 
        returns (uint256) 
    {
        uint256 transactionId = transactions.length;
        transactions.push(Transaction({
            to: to,
            value: value,
            data: data,
            executed: false,
            confirmations: 0
        }));
        
        confirmTransaction(transactionId);
        return transactionId;
    }
    
    function confirmTransaction(uint256 transactionId) 
        public 
        onlyOwner 
        notExecuted(transactionId) 
    {
        require(!confirmations[transactionId][msg.sender], "Transaction already confirmed");
        
        confirmations[transactionId][msg.sender] = true;
        transactions[transactionId].confirmations++;
        
        if (transactions[transactionId].confirmations >= required) {
            executeTransaction(transactionId);
        }
    }
    
    function executeTransaction(uint256 transactionId) 
        public 
        notExecuted(transactionId) 
    {
        require(transactions[transactionId].confirmations >= required, "Not enough confirmations");
        
        Transaction storage transaction = transactions[transactionId];
        transaction.executed = true;
        
        (bool success, ) = transaction.to.call{value: transaction.value}(transaction.data);
        require(success, "Transaction execution failed");
    }
}
```

## 6. 性能优化策略

### 6.1 批量交易处理

```rust
pub struct BatchProcessor {
    pending_transactions: Vec<TransactionRequest>,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchProcessor {
    pub async fn add_transaction(&mut self, tx: TransactionRequest) -> Result<(), BatchError> {
        self.pending_transactions.push(tx);
        
        if self.pending_transactions.len() >= self.batch_size {
            self.process_batch().await?;
        }
        
        Ok(())
    }
    
    async fn process_batch(&mut self) -> Result<(), BatchError> {
        if self.pending_transactions.is_empty() {
            return Ok(());
        }
        
        let batch = std::mem::take(&mut self.pending_transactions);
        
        // 并行处理批量交易
        let futures: Vec<_> = batch.into_iter()
            .map(|tx| self.process_single_transaction(tx))
            .collect();
        
        let results = futures::future::join_all(futures).await;
        
        // 处理结果
        for result in results {
            match result {
                Ok(tx_hash) => info!("Transaction successful: {}", tx_hash),
                Err(e) => error!("Transaction failed: {}", e),
            }
        }
        
        Ok(())
    }
}
```

### 6.2 连接池管理

```rust
pub struct BlockchainConnectionPool {
    ethereum_pool: Pool<EthereumConnection>,
    solana_pool: Pool<SolanaConnection>,
    bsc_pool: Pool<BscConnection>,
}

impl BlockchainConnectionPool {
    pub async fn new(config: PoolConfig) -> Result<Self, PoolError> {
        let ethereum_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(config.min_idle)
            .build(EthereumConnectionManager::new(&config.ethereum_rpc_url))
            .await?;
        
        let solana_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(config.min_idle)
            .build(SolanaConnectionManager::new(&config.solana_rpc_url))
            .await?;
        
        let bsc_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(config.min_idle)
            .build(BscConnectionManager::new(&config.bsc_rpc_url))
            .await?;
        
        Ok(Self {
            ethereum_pool,
            solana_pool,
            bsc_pool,
        })
    }
    
    pub async fn get_ethereum_connection(&self) -> Result<PooledConnection<EthereumConnection>, PoolError> {
        self.ethereum_pool.get().await
    }
    
    pub async fn get_solana_connection(&self) -> Result<PooledConnection<SolanaConnection>, PoolError> {
        self.solana_pool.get().await
    }
    
    pub async fn get_bsc_connection(&self) -> Result<PooledConnection<BscConnection>, PoolError> {
        self.bsc_pool.get().await
    }
}
```

这个区块链集成架构提供了完整的多链支持方案，包括智能合约、交易监听、钱包管理和安全机制，为WoPay系统提供了强大的区块链基础设施。
