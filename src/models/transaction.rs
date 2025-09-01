// 区块链交易数据模型
// 定义区块链交易相关的数据结构

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 区块链交易记录模型
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BlockchainTransaction {
    /// 交易记录唯一标识符
    pub id: Uuid,
    /// 关联的支付订单ID
    pub payment_id: Uuid,
    /// 区块链网络名称
    pub blockchain: String,
    /// 区块链交易哈希
    pub transaction_hash: String,
    /// 发送方地址
    pub from_address: String,
    /// 接收方地址
    pub to_address: String,
    /// 交易金额
    pub amount: Decimal,
    /// Gas费用
    pub gas_fee: Option<Decimal>,
    /// 区块号
    pub block_number: Option<i64>,
    /// 区块确认数
    pub confirmations: i32,
    /// 交易状态
    pub status: TransactionStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 区块链交易状态枚举
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "varchar")]
pub enum TransactionStatus {
    /// 待确认状态 (交易已提交但未确认)
    #[sqlx(rename = "pending")]
    Pending,
    /// 已确认状态 (交易已被区块链确认)
    #[sqlx(rename = "confirmed")]
    Confirmed,
    /// 失败状态 (交易执行失败)
    #[sqlx(rename = "failed")]
    Failed,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus::Pending
    }
}

/// 创建区块链交易记录请求
#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    /// 关联的支付订单ID
    pub payment_id: Uuid,
    /// 区块链网络名称
    pub blockchain: String,
    /// 区块链交易哈希
    pub transaction_hash: String,
    /// 发送方地址
    pub from_address: String,
    /// 接收方地址
    pub to_address: String,
    /// 交易金额
    pub amount: Decimal,
    /// Gas费用 (可选)
    pub gas_fee: Option<Decimal>,
    /// 区块号 (可选)
    pub block_number: Option<i64>,
}

/// 交易详情响应
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    /// 交易记录ID
    pub id: Uuid,
    /// 支付订单ID
    pub payment_id: Uuid,
    /// 区块链网络
    pub blockchain: String,
    /// 交易哈希
    pub transaction_hash: String,
    /// 发送方地址
    pub from_address: String,
    /// 接收方地址
    pub to_address: String,
    /// 交易金额
    pub amount: Decimal,
    /// Gas费用
    pub gas_fee: Option<Decimal>,
    /// 区块号
    pub block_number: Option<i64>,
    /// 确认数
    pub confirmations: i32,
    /// 交易状态
    pub status: TransactionStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 区块链浏览器链接
    pub explorer_url: String,
}

impl BlockchainTransaction {
    /// 检查交易是否已确认
    pub fn is_confirmed(&self) -> bool {
        self.status == TransactionStatus::Confirmed
    }

    /// 检查交易是否失败
    pub fn is_failed(&self) -> bool {
        self.status == TransactionStatus::Failed
    }

    /// 检查是否需要更多确认
    pub fn needs_more_confirmations(&self, required: i32) -> bool {
        self.confirmations < required && !self.is_failed()
    }

    /// 转换为API响应格式
    pub fn to_response(&self) -> TransactionResponse {
        TransactionResponse {
            id: self.id,
            payment_id: self.payment_id,
            blockchain: self.blockchain.clone(),
            transaction_hash: self.transaction_hash.clone(),
            from_address: self.from_address.clone(),
            to_address: self.to_address.clone(),
            amount: self.amount,
            gas_fee: self.gas_fee,
            block_number: self.block_number,
            confirmations: self.confirmations,
            status: self.status.clone(),
            created_at: self.created_at,
            explorer_url: self.generate_explorer_url(),
        }
    }

    /// 生成区块链浏览器链接
    fn generate_explorer_url(&self) -> String {
        match self.blockchain.as_str() {
            "ethereum" => format!("https://etherscan.io/tx/{}", self.transaction_hash),
            "bsc" => format!("https://bscscan.com/tx/{}", self.transaction_hash),
            "solana" => format!("https://explorer.solana.com/tx/{}", self.transaction_hash),
            _ => format!("https://etherscan.io/tx/{}", self.transaction_hash), // 默认使用以太坊
        }
    }
}

/// 交易监听事件
#[derive(Debug, Clone)]
pub struct TransactionEvent {
    /// 事件类型
    pub event_type: TransactionEventType,
    /// 交易哈希
    pub transaction_hash: String,
    /// 相关的支付订单ID (如果有)
    pub payment_id: Option<Uuid>,
    /// 区块链网络
    pub blockchain: String,
    /// 事件数据
    pub data: TransactionEventData,
    /// 事件时间
    pub timestamp: DateTime<Utc>,
}

/// 交易事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionEventType {
    /// 新交易检测到
    NewTransaction,
    /// 交易确认数更新
    ConfirmationUpdate,
    /// 交易完成
    TransactionCompleted,
    /// 交易失败
    TransactionFailed,
}

/// 交易事件数据
#[derive(Debug, Clone)]
pub struct TransactionEventData {
    /// 发送方地址
    pub from_address: String,
    /// 接收方地址
    pub to_address: String,
    /// 交易金额
    pub amount: Decimal,
    /// Gas费用
    pub gas_fee: Option<Decimal>,
    /// 区块号
    pub block_number: Option<i64>,
    /// 确认数
    pub confirmations: i32,
}
