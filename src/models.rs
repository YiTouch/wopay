// WoPay MVP 数据模型定义
// 包含商户、支付订单、区块链交易等核心数据结构

mod merchant;
mod payment;
mod transaction;
mod webhook;

// 重新导出核心类型
pub use merchant::*;
pub use payment::*;
pub use transaction::*;
pub use webhook::*;

use serde::Serialize;

/// 标准API响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// 响应状态码
    pub code: i32,
    /// 响应消息
    pub message: String,
    /// 响应数据
    pub data: Option<T>,
    /// 响应时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "Success".to_string(),
            data: Some(data),
            timestamp: chrono::Utc::now(),
        }
    }

    /// 创建成功响应（无数据）
    pub fn success_no_data() -> ApiResponse<()> {
        ApiResponse {
            code: 200,
            message: "Success".to_string(),
            data: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// 创建错误响应
    pub fn error(code: i32, message: String) -> ApiResponse<()> {
        ApiResponse {
            code,
            message,
            data: None,
            timestamp: chrono::Utc::now(),
        }
    }
}