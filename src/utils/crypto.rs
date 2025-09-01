// 加密工具函数
// 提供API密钥生成、HMAC签名验证等安全功能

use hmac::{Hmac, Mac};
use sha2::Sha256;
use rand::{distributions::Alphanumeric, Rng};
use hex;
use anyhow::{Result, Context};

type HmacSha256 = Hmac<Sha256>;

/// 生成随机API密钥
/// 
/// # Arguments
/// * `length` - 密钥长度
/// 
/// # Returns
/// * 随机生成的API密钥字符串
pub fn generate_api_key(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// 生成随机API密钥对
/// 
/// # Arguments
/// * `key_length` - API密钥长度
/// * `secret_length` - API密钥长度
/// 
/// # Returns
/// * (api_key, api_secret) 元组
pub fn generate_api_key_pair(key_length: usize, secret_length: usize) -> (String, String) {
    let api_key = generate_api_key(key_length);
    let api_secret = generate_api_key(secret_length);
    (api_key, api_secret)
}

/// 生成HMAC-SHA256签名
/// 
/// # Arguments
/// * `message` - 要签名的消息
/// * `secret` - 签名密钥
/// 
/// # Returns
/// * 十六进制格式的签名字符串
pub fn generate_hmac_signature(message: &str, secret: &str) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .context("Invalid HMAC key")?;
    
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    
    Ok(signature)
}

/// 验证HMAC-SHA256签名
/// 
/// # Arguments
/// * `message` - 原始消息
/// * `signature` - 要验证的签名
/// * `secret` - 签名密钥
/// 
/// # Returns
/// * 签名是否有效
pub fn verify_hmac_signature(message: &str, signature: &str, secret: &str) -> Result<bool> {
    let expected_signature = generate_hmac_signature(message, secret)?;
    Ok(constant_time_eq(&expected_signature, signature))
}

/// 常量时间字符串比较 (防止时序攻击)
/// 
/// # Arguments
/// * `a` - 字符串A
/// * `b` - 字符串B
/// 
/// # Returns
/// * 字符串是否相等
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }
    
    result == 0
}

/// 为Webhook载荷生成签名
/// 
/// # Arguments
/// * `payload` - JSON载荷字符串
/// * `secret` - 商户API密钥
/// 
/// # Returns
/// * HMAC签名
pub fn sign_webhook_payload(payload: &str, secret: &str) -> Result<String> {
    generate_hmac_signature(payload, secret)
}

/// 验证Webhook载荷签名
/// 
/// # Arguments
/// * `payload` - JSON载荷字符串
/// * `signature` - 收到的签名
/// * `secret` - 商户API密钥
/// 
/// # Returns
/// * 签名是否有效
pub fn verify_webhook_signature(payload: &str, signature: &str, secret: &str) -> Result<bool> {
    verify_hmac_signature(payload, signature, secret)
}

/// 生成安全的随机字符串
/// 
/// # Arguments
/// * `length` - 字符串长度
/// 
/// # Returns
/// * 随机字符串
pub fn generate_secure_random_string(length: usize) -> String {
    use rand::distributions::Uniform;
    
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let range = Uniform::from(0..CHARSET.len());
    
    (0..length)
        .map(|_| {
            let idx = rand::thread_rng().sample(range);
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key(32);
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_api_key_pair() {
        let (api_key, api_secret) = generate_api_key_pair(32, 64);
        assert_eq!(api_key.len(), 32);
        assert_eq!(api_secret.len(), 64);
        assert_ne!(api_key, api_secret);
    }

    #[test]
    fn test_hmac_signature() {
        let message = "test message";
        let secret = "test secret";
        
        let signature = generate_hmac_signature(message, secret).unwrap();
        assert!(!signature.is_empty());
        
        let is_valid = verify_hmac_signature(message, &signature, secret).unwrap();
        assert!(is_valid);
        
        let is_invalid = verify_hmac_signature(message, "invalid_signature", secret).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("hello", "hello world"));
    }

    #[test]
    fn test_webhook_signature() {
        let payload = r#"{"event":"payment.completed","payment_id":"123"}"#;
        let secret = "webhook_secret";
        
        let signature = sign_webhook_payload(payload, secret).unwrap();
        let is_valid = verify_webhook_signature(payload, &signature, secret).unwrap();
        assert!(is_valid);
    }
}
