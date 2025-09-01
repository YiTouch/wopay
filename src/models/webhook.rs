// Webhook通知数据模型
// 定义Webhook回调相关的数据结构

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::payment::PaymentStatus;
use rust_decimal::Decimal;

/// Webhook日志记录模型
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct WebhookLog {
    /// 日志记录唯一标识符
    pub id: Uuid,
    /// 关联的支付订单ID
    pub payment_id: Uuid,
    /// Webhook回调地址
    pub webhook_url: String,
    /// 发送的载荷数据
    pub payload: serde_json::Value,
    /// HTTP响应状态码
    pub response_status: Option<i32>,
    /// HTTP响应内容
    pub response_body: Option<String>,
    /// 重试次数
    pub retry_count: i32,
    /// 是否成功
    pub success: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// Webhook事件类型
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum WebhookEventType {
    /// 支付创建事件
    #[serde(rename = "payment.created")]
    PaymentCreated,
    /// 支付确认事件
    #[serde(rename = "payment.confirmed")]
    PaymentConfirmed,
    /// 支付完成事件
    #[serde(rename = "payment.completed")]
    PaymentCompleted,
    /// 支付过期事件
    #[serde(rename = "payment.expired")]
    PaymentExpired,
    /// 支付失败事件
    #[serde(rename = "payment.failed")]
    PaymentFailed,
}

impl From<PaymentStatus> for WebhookEventType {
    fn from(status: PaymentStatus) -> Self {
        match status {
            PaymentStatus::Pending => WebhookEventType::PaymentCreated,
            PaymentStatus::Confirmed => WebhookEventType::PaymentConfirmed,
            PaymentStatus::Completed => WebhookEventType::PaymentCompleted,
            PaymentStatus::Expired => WebhookEventType::PaymentExpired,
            PaymentStatus::Failed => WebhookEventType::PaymentFailed,
        }
    }
}

/// Webhook载荷数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookPayload {
    /// 事件类型
    pub event: WebhookEventType,
    /// 支付订单ID
    pub payment_id: Uuid,
    /// 商户订单号
    pub order_id: String,
    /// 支付状态
    pub status: PaymentStatus,
    /// 支付金额
    pub amount: Decimal,
    /// 支付币种
    pub currency: String,
    /// 区块链交易哈希 (如果有)
    pub transaction_hash: Option<String>,
    /// 区块确认数
    pub confirmations: i32,
    /// 事件时间戳
    pub timestamp: DateTime<Utc>,
    /// HMAC签名 (用于验证载荷完整性)
    pub signature: String,
}

/// Webhook发送请求
#[derive(Debug, Clone)]
pub struct WebhookRequest {
    /// 目标URL
    pub url: String,
    /// 载荷数据
    pub payload: WebhookPayload,
    /// 商户API密钥 (用于生成签名)
    pub api_secret: String,
    /// 重试次数
    pub retry_count: i32,
    /// 最大重试次数
    pub max_retries: i32,
}

impl WebhookRequest {
    /// 创建新的Webhook请求
    pub fn new(
        url: String, 
        payload: WebhookPayload, 
        api_secret: String
    ) -> Self {
        Self {
            url,
            payload,
            api_secret,
            retry_count: 0,
            max_retries: 3, // 默认最多重试3次
        }
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// 获取下次重试的延迟时间 (指数退避)
    pub fn next_retry_delay(&self) -> std::time::Duration {
        let base_delay = 5; // 基础延迟5秒
        let delay_seconds = base_delay * (2_u64.pow(self.retry_count as u32));
        std::time::Duration::from_secs(delay_seconds.min(300)) // 最大延迟5分钟
    }
}

/// Webhook发送响应
#[derive(Debug, Clone)]
pub struct WebhookResponse {
    /// HTTP状态码
    pub status_code: u16,
    /// 响应内容
    pub body: String,
    /// 是否成功 (状态码200-299视为成功)
    pub success: bool,
    /// 响应时间 (毫秒)
    pub response_time_ms: u64,
}

impl WebhookResponse {
    /// 创建成功响应
    pub fn success(status_code: u16, body: String, response_time_ms: u64) -> Self {
        Self {
            status_code,
            body,
            success: (200..300).contains(&status_code),
            response_time_ms,
        }
    }

    /// 创建失败响应
    pub fn failure(status_code: u16, body: String, response_time_ms: u64) -> Self {
        Self {
            status_code,
            body,
            success: false,
            response_time_ms,
        }
    }

    /// 检查是否为临时错误 (可以重试)
    pub fn is_retryable_error(&self) -> bool {
        match self.status_code {
            // 5xx服务器错误通常可以重试
            500..=599 => true,
            // 429限流错误可以重试
            429 => true,
            // 408请求超时可以重试
            408 => true,
            // 其他错误不重试
            _ => false,
        }
    }
}

/// 区块链网络配置
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    /// 网络名称
    pub name: String,
    /// RPC节点URL
    pub rpc_url: String,
    /// WebSocket URL (用于实时监听)
    pub ws_url: Option<String>,
    /// 链ID
    pub chain_id: u64,
    /// 所需确认数
    pub required_confirmations: i32,
    /// 区块时间 (秒)
    pub block_time: u64,
    /// 是否为测试网
    pub is_testnet: bool,
}

impl BlockchainConfig {
    /// 创建Ethereum主网配置
    pub fn ethereum_mainnet() -> Self {
        Self {
            name: "ethereum".to_string(),
            rpc_url: "https://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string(),
            ws_url: Some("wss://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string()),
            chain_id: 1,
            required_confirmations: 12,
            block_time: 12,
            is_testnet: false,
        }
    }

    /// 创建Ethereum测试网配置
    pub fn ethereum_goerli() -> Self {
        Self {
            name: "ethereum_goerli".to_string(),
            rpc_url: "https://eth-goerli.alchemyapi.io/v2/your-api-key".to_string(),
            ws_url: Some("wss://eth-goerli.alchemyapi.io/v2/your-api-key".to_string()),
            chain_id: 5,
            required_confirmations: 6,
            block_time: 12,
            is_testnet: true,
        }
    }

    /// 创建BSC主网配置
    pub fn bsc_mainnet() -> Self {
        Self {
            name: "bsc".to_string(),
            rpc_url: "https://bsc-dataseed1.binance.org".to_string(),
            ws_url: Some("wss://bsc-ws-node.nariox.org:443".to_string()),
            chain_id: 56,
            required_confirmations: 15,
            block_time: 3,
            is_testnet: false,
        }
    }
}

/// 交易监听配置
#[derive(Debug, Clone)]
pub struct TransactionListenerConfig {
    /// 监听的地址列表
    pub addresses: Vec<String>,
    /// 检查间隔 (秒)
    pub check_interval: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 超时时间 (秒)
    pub timeout: u64,
}

impl Default for TransactionListenerConfig {
    fn default() -> Self {
        Self {
            addresses: Vec::new(),
            check_interval: 30, // 30秒检查一次
            max_retries: 3,
            timeout: 60, // 60秒超时
        }
    }
}
