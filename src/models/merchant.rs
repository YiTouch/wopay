// 商户数据模型
// 定义商户相关的数据结构和业务逻辑

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 商户信息模型
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Merchant {
    /// 商户唯一标识符
    pub id: Uuid,
    /// 商户名称
    pub name: String,
    /// 商户邮箱地址
    pub email: String,
    /// API访问密钥
    pub api_key: String,
    /// API签名密钥 (不在API响应中返回)
    #[serde(skip_serializing)]
    pub api_secret: String,
    /// Webhook回调地址
    pub webhook_url: Option<String>,
    /// 商户状态
    pub status: MerchantStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 商户状态枚举
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "varchar")]
pub enum MerchantStatus {
    /// 活跃状态
    #[sqlx(rename = "active")]
    Active,
    /// 非活跃状态
    #[sqlx(rename = "inactive")]
    Inactive,
    /// 暂停状态
    #[sqlx(rename = "suspended")]
    Suspended,
}

impl Default for MerchantStatus {
    fn default() -> Self {
        MerchantStatus::Active
    }
}

/// 商户注册请求
#[derive(Debug, Deserialize)]
pub struct CreateMerchantRequest {
    /// 商户名称
    pub name: String,
    /// 商户邮箱
    pub email: String,
    /// Webhook回调地址 (可选)
    pub webhook_url: Option<String>,
}

/// 商户注册响应
#[derive(Debug, Serialize)]
pub struct CreateMerchantResponse {
    /// 商户ID
    pub merchant_id: Uuid,
    /// 商户名称
    pub name: String,
    /// 商户邮箱
    pub email: String,
    /// API访问密钥
    pub api_key: String,
    /// API签名密钥
    pub api_secret: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 商户更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateMerchantRequest {
    /// 商户名称 (可选)
    pub name: Option<String>,
    /// Webhook回调地址 (可选)
    pub webhook_url: Option<String>,
    /// 商户状态 (可选)
    pub status: Option<MerchantStatus>,
}

/// API密钥重新生成响应
#[derive(Debug, Serialize)]
pub struct RegenerateApiKeyResponse {
    /// 新的API访问密钥
    pub api_key: String,
    /// 新的API签名密钥
    pub api_secret: String,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
}

impl Merchant {
    /// 检查商户是否处于活跃状态
    pub fn is_active(&self) -> bool {
        self.status == MerchantStatus::Active
    }

    /// 验证API密钥是否匹配
    pub fn verify_api_key(&self, api_key: &str) -> bool {
        self.api_key == api_key
    }

    /// 获取商户的公开信息 (不包含敏感信息)
    pub fn to_public(&self) -> MerchantPublic {
        MerchantPublic {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
        }
    }
}

/// 商户公开信息 (不包含API密钥等敏感信息)
#[derive(Debug, Serialize)]
pub struct MerchantPublic {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub status: MerchantStatus,
    pub created_at: DateTime<Utc>,
}
