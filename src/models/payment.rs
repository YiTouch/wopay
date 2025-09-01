// 支付订单数据模型
// 定义支付相关的数据结构和业务逻辑

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 支付订单模型
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Payment {
    /// 支付订单唯一标识符
    pub id: Uuid,
    /// 商户ID
    pub merchant_id: Uuid,
    /// 商户订单号
    pub order_id: String,
    /// 支付金额
    pub amount: Decimal,
    /// 支付币种
    pub currency: Currency,
    /// 收款地址
    pub payment_address: String,
    /// 支付状态
    pub status: PaymentStatus,
    /// 区块链交易哈希
    pub transaction_hash: Option<String>,
    /// 区块确认数
    pub confirmations: i32,
    /// 订单过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 支付状态枚举
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "varchar")]
pub enum PaymentStatus {
    /// 待支付状态
    #[sqlx(rename = "pending")]
    Pending,
    /// 已确认状态 (收到交易但确认数不足)
    #[sqlx(rename = "confirmed")]
    Confirmed,
    /// 已完成状态 (确认数足够)
    #[sqlx(rename = "completed")]
    Completed,
    /// 已过期状态
    #[sqlx(rename = "expired")]
    Expired,
    /// 失败状态
    #[sqlx(rename = "failed")]
    Failed,
}

impl Default for PaymentStatus {
    fn default() -> Self {
        PaymentStatus::Pending
    }
}

/// 支持的币种枚举
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "varchar")]
pub enum Currency {
    /// 以太坊原生代币
    #[sqlx(rename = "ETH")]
    ETH,
    /// USDT稳定币
    #[sqlx(rename = "USDT")]
    USDT,
}

impl Currency {
    /// 获取代币合约地址 (如果是ERC20代币)
    pub fn contract_address(&self) -> Option<&'static str> {
        match self {
            Currency::ETH => None, // ETH是原生代币，没有合约地址
            Currency::USDT => Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"), // USDT合约地址
        }
    }

    /// 获取代币精度 (小数位数)
    pub fn decimals(&self) -> u8 {
        match self {
            Currency::ETH => 18,
            Currency::USDT => 6,
        }
    }

    /// 检查是否为原生代币
    pub fn is_native(&self) -> bool {
        matches!(self, Currency::ETH)
    }
}

/// 创建支付订单请求
#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    /// 商户订单号 (商户系统中的唯一标识)
    pub order_id: String,
    /// 支付金额
    pub amount: Decimal,
    /// 支付币种
    pub currency: Currency,
    /// 回调地址 (可选，覆盖商户默认配置)
    pub callback_url: Option<String>,
    /// 过期时间 (秒，可选，默认1小时)
    pub expires_in: Option<i64>,
}

/// 创建支付订单响应
#[derive(Debug, Serialize)]
pub struct CreatePaymentResponse {
    /// 支付订单ID
    pub payment_id: Uuid,
    /// 收款地址
    pub payment_address: String,
    /// 支付金额
    pub amount: Decimal,
    /// 支付币种
    pub currency: Currency,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 支付二维码 (Base64编码的PNG图片)
    pub qr_code: String,
    /// 支付链接 (用于钱包应用直接调用)
    pub payment_url: String,
}

/// 支付订单查询响应
#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    /// 支付订单ID
    pub payment_id: Uuid,
    /// 商户订单号
    pub order_id: String,
    /// 支付状态
    pub status: PaymentStatus,
    /// 支付金额
    pub amount: Decimal,
    /// 支付币种
    pub currency: Currency,
    /// 收款地址
    pub payment_address: String,
    /// 区块链交易哈希
    pub transaction_hash: Option<String>,
    /// 区块确认数
    pub confirmations: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 完成时间 (如果已完成)
    pub completed_at: Option<DateTime<Utc>>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
}

impl Payment {
    /// 检查支付订单是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// 检查支付订单是否可以被取消
    pub fn can_be_cancelled(&self) -> bool {
        matches!(self.status, PaymentStatus::Pending | PaymentStatus::Confirmed)
    }

    /// 检查支付订单是否已完成
    pub fn is_completed(&self) -> bool {
        self.status == PaymentStatus::Completed
    }

    /// 检查支付订单是否需要更多确认
    pub fn needs_more_confirmations(&self, required_confirmations: i32) -> bool {
        self.status == PaymentStatus::Confirmed && self.confirmations < required_confirmations
    }

    /// 转换为API响应格式
    pub fn to_response(&self) -> PaymentResponse {
        PaymentResponse {
            payment_id: self.id,
            order_id: self.order_id.clone(),
            status: self.status.clone(),
            amount: self.amount,
            currency: self.currency.clone(),
            payment_address: self.payment_address.clone(),
            transaction_hash: self.transaction_hash.clone(),
            confirmations: self.confirmations,
            created_at: self.created_at,
            completed_at: if self.is_completed() { 
                Some(self.updated_at) 
            } else { 
                None 
            },
            expires_at: self.expires_at,
        }
    }

    /// 生成支付URL (用于钱包应用)
    pub fn generate_payment_url(&self) -> String {
        match self.currency {
            Currency::ETH => {
                format!("ethereum:{}?value={}", 
                    self.payment_address, 
                    self.amount_in_wei()
                )
            },
            Currency::USDT => {
                format!("ethereum:{}@1/transfer?address={}&uint256={}",
                    self.currency.contract_address().unwrap(),
                    self.payment_address,
                    self.amount_in_smallest_unit()
                )
            }
        }
    }

    /// 获取以Wei为单位的金额 (ETH)
    fn amount_in_wei(&self) -> String {
        let wei_amount = self.amount * Decimal::from(10_u64.pow(18));
        format!("{}", wei_amount.trunc())
    }

    /// 获取以最小单位的金额
    fn amount_in_smallest_unit(&self) -> String {
        let smallest_unit = self.amount * Decimal::from(10_u64.pow(self.currency.decimals() as u32));
        format!("{}", smallest_unit.trunc())
    }
}

/// 支付订单列表查询参数
#[derive(Debug, Deserialize)]
pub struct PaymentListQuery {
    /// 页码 (从1开始)
    pub page: Option<u32>,
    /// 每页数量 (默认20，最大100)
    pub limit: Option<u32>,
    /// 状态过滤
    pub status: Option<PaymentStatus>,
    /// 币种过滤
    pub currency: Option<Currency>,
    /// 开始时间
    pub start_date: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_date: Option<DateTime<Utc>>,
}

impl PaymentListQuery {
    /// 获取分页偏移量
    pub fn offset(&self) -> u32 {
        let page = self.page.unwrap_or(1);
        let limit = self.limit();
        (page - 1) * limit
    }

    /// 获取每页限制数量
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(20).min(100).max(1)
    }
}

/// 支付订单列表响应
#[derive(Debug, Serialize)]
pub struct PaymentListResponse {
    /// 支付订单列表
    pub payments: Vec<PaymentResponse>,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 分页信息
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    /// 当前页码
    pub page: u32,
    /// 每页数量
    pub limit: u32,
    /// 总记录数
    pub total: u64,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

impl PaginationInfo {
    /// 创建分页信息
    pub fn new(page: u32, limit: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
        
        Self {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}
