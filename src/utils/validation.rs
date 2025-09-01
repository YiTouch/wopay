// 数据验证工具函数
// 提供输入数据验证和格式检查功能

use regex::Regex;
use rust_decimal::Decimal;
use anyhow::{Result, Context};
use std::collections::HashMap;

/// 验证以太坊地址格式
/// 
/// # Arguments
/// * `address` - 以太坊地址字符串
/// 
/// # Returns
/// * 地址是否有效
pub fn validate_ethereum_address(address: &str) -> bool {
    // 以太坊地址格式: 0x + 40个十六进制字符
    if address.len() != 42 {
        return false;
    }
    
    if !address.starts_with("0x") {
        return false;
    }
    
    // 验证是否为有效的十六进制字符
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// 验证交易哈希格式
/// 
/// # Arguments
/// * `hash` - 交易哈希字符串
/// 
/// # Returns
/// * 哈希是否有效
pub fn validate_transaction_hash(hash: &str) -> bool {
    // 以太坊交易哈希格式: 0x + 64个十六进制字符
    if hash.len() != 66 {
        return false;
    }
    
    if !hash.starts_with("0x") {
        return false;
    }
    
    hash[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// 验证邮箱地址格式
/// 
/// # Arguments
/// * `email` - 邮箱地址字符串
/// 
/// # Returns
/// * 邮箱是否有效
pub fn validate_email(email: &str) -> bool {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    email_regex.is_match(email)
}

/// 验证URL格式
/// 
/// # Arguments
/// * `url` - URL字符串
/// 
/// # Returns
/// * URL是否有效
pub fn validate_url(url: &str) -> bool {
    let url_regex = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
    url_regex.is_match(url)
}

/// 验证支付金额
/// 
/// # Arguments
/// * `amount` - 支付金额
/// * `currency` - 币种
/// 
/// # Returns
/// * 金额是否有效
pub fn validate_payment_amount(amount: &Decimal, currency: &str) -> Result<()> {
    // 检查金额是否为正数
    if *amount <= Decimal::ZERO {
        anyhow::bail!("Payment amount must be positive");
    }

    // 检查金额精度
    let scale = amount.scale();
    let max_scale = match currency {
        "ETH" => 18,
        "USDT" => 6,
        _ => 18, // 默认18位精度
    };

    if scale > max_scale {
        anyhow::bail!("Amount precision too high for currency {}", currency);
    }

    // 检查最小金额限制
    let min_amount = match currency {
        "ETH" => Decimal::new(1, 4), // 0.0001 ETH
        "USDT" => Decimal::new(1, 2), // 0.01 USDT
        _ => Decimal::new(1, 4),
    };

    if *amount < min_amount {
        anyhow::bail!("Amount too small for currency {}", currency);
    }

    // 检查最大金额限制 (防止意外的大额交易)
    let max_amount = match currency {
        "ETH" => Decimal::new(1000, 0), // 1000 ETH
        "USDT" => Decimal::new(1000000, 0), // 1,000,000 USDT
        _ => Decimal::new(1000, 0),
    };

    if *amount > max_amount {
        anyhow::bail!("Amount too large for currency {}", currency);
    }

    Ok(())
}

/// 验证订单ID格式
/// 
/// # Arguments
/// * `order_id` - 订单ID字符串
/// 
/// # Returns
/// * 订单ID是否有效
pub fn validate_order_id(order_id: &str) -> Result<()> {
    // 检查长度
    if order_id.is_empty() {
        anyhow::bail!("Order ID cannot be empty");
    }

    if order_id.len() > 255 {
        anyhow::bail!("Order ID too long (max 255 characters)");
    }

    // 检查字符集 (只允许字母、数字、下划线、连字符)
    let valid_chars = order_id.chars().all(|c| {
        c.is_alphanumeric() || c == '_' || c == '-'
    });

    if !valid_chars {
        anyhow::bail!("Order ID contains invalid characters");
    }

    Ok(())
}

/// 验证商户名称
/// 
/// # Arguments
/// * `name` - 商户名称
/// 
/// # Returns
/// * 名称是否有效
pub fn validate_merchant_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("Merchant name cannot be empty");
    }

    if name.len() > 255 {
        anyhow::bail!("Merchant name too long (max 255 characters)");
    }

    // 检查是否包含有害字符
    let forbidden_chars = ['<', '>', '"', '\'', '&'];
    if name.chars().any(|c| forbidden_chars.contains(&c)) {
        anyhow::bail!("Merchant name contains forbidden characters");
    }

    Ok(())
}

/// 验证API密钥格式
/// 
/// # Arguments
/// * `api_key` - API密钥
/// 
/// # Returns
/// * 密钥是否有效
pub fn validate_api_key(api_key: &str) -> Result<()> {
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    if api_key.len() < 16 {
        anyhow::bail!("API key too short (minimum 16 characters)");
    }

    if api_key.len() > 128 {
        anyhow::bail!("API key too long (maximum 128 characters)");
    }

    // 检查字符集 (只允许字母和数字)
    if !api_key.chars().all(|c| c.is_alphanumeric()) {
        anyhow::bail!("API key contains invalid characters");
    }

    Ok(())
}

/// 通用输入验证器
pub struct InputValidator {
    errors: HashMap<String, Vec<String>>,
}

impl InputValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    /// 添加字段验证错误
    pub fn add_error(&mut self, field: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(message.to_string());
    }

    /// 验证必填字段
    pub fn validate_required(&mut self, field: &str, value: &str) {
        if value.trim().is_empty() {
            self.add_error(field, "This field is required");
        }
    }

    /// 验证字符串长度
    pub fn validate_length(&mut self, field: &str, value: &str, min: usize, max: usize) {
        let len = value.len();
        if len < min {
            self.add_error(field, &format!("Must be at least {} characters", min));
        }
        if len > max {
            self.add_error(field, &format!("Must be at most {} characters", max));
        }
    }

    /// 验证邮箱格式
    pub fn validate_email_field(&mut self, field: &str, email: &str) {
        if !validate_email(email) {
            self.add_error(field, "Invalid email format");
        }
    }

    /// 验证URL格式
    pub fn validate_url_field(&mut self, field: &str, url: &str) {
        if !url.is_empty() && !validate_url(url) {
            self.add_error(field, "Invalid URL format");
        }
    }

    /// 验证以太坊地址
    pub fn validate_ethereum_address_field(&mut self, field: &str, address: &str) {
        if !validate_ethereum_address(address) {
            self.add_error(field, "Invalid Ethereum address format");
        }
    }

    /// 检查是否有验证错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 获取验证错误
    pub fn get_errors(&self) -> &HashMap<String, Vec<String>> {
        &self.errors
    }

    /// 获取第一个错误消息
    pub fn first_error(&self) -> Option<String> {
        self.errors.values().next()?.first().cloned()
    }

    /// 转换为错误结果
    pub fn into_result(self) -> Result<()> {
        if self.has_errors() {
            let error_msg = self.errors
                .iter()
                .map(|(field, messages)| {
                    format!("{}: {}", field, messages.join(", "))
                })
                .collect::<Vec<_>>()
                .join("; ");
            
            anyhow::bail!("Validation failed: {}", error_msg);
        }
        
        Ok(())
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ethereum_address() {
        // 有效地址
        assert!(validate_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2"));
        assert!(validate_ethereum_address("0x0000000000000000000000000000000000000000"));
        
        // 无效地址
        assert!(!validate_ethereum_address("742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2")); // 缺少0x
        assert!(!validate_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b")); // 太短
        assert!(!validate_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2G")); // 包含非十六进制字符
    }

    #[test]
    fn test_validate_transaction_hash() {
        // 有效哈希
        assert!(validate_transaction_hash("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"));
        
        // 无效哈希
        assert!(!validate_transaction_hash("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")); // 缺少0x
        assert!(!validate_transaction_hash("0x1234567890abcdef")); // 太短
    }

    #[test]
    fn test_validate_email() {
        // 有效邮箱
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name+tag@domain.co.uk"));
        
        // 无效邮箱
        assert!(!validate_email("invalid-email"));
        assert!(!validate_email("@domain.com"));
        assert!(!validate_email("user@"));
    }

    #[test]
    fn test_validate_payment_amount() {
        // 有效金额
        assert!(validate_payment_amount(&Decimal::new(1, 0), "ETH").is_ok()); // 1 ETH
        assert!(validate_payment_amount(&Decimal::new(100, 2), "USDT").is_ok()); // 1.00 USDT
        
        // 无效金额
        assert!(validate_payment_amount(&Decimal::ZERO, "ETH").is_err()); // 零金额
        assert!(validate_payment_amount(&Decimal::new(-1, 0), "ETH").is_err()); // 负金额
        assert!(validate_payment_amount(&Decimal::new(10000, 0), "ETH").is_err()); // 金额过大
    }

    #[test]
    fn test_input_validator() {
        let mut validator = InputValidator::new();
        
        validator.validate_required("name", "");
        validator.validate_email_field("email", "invalid-email");
        validator.validate_length("description", "short", 10, 100);
        
        assert!(validator.has_errors());
        assert_eq!(validator.get_errors().len(), 3);
        assert!(validator.into_result().is_err());
    }
}
