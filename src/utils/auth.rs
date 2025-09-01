// 认证工具函数
// 提供API密钥验证、JWT处理等认证功能

use actix_web::{HttpRequest, Result as ActixResult, error::ErrorUnauthorized};
use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::models::Merchant;

/// 从HTTP请求中提取API密钥
/// 
/// # Arguments
/// * `req` - HTTP请求对象
/// 
/// # Returns
/// * API密钥字符串
pub fn extract_api_key(req: &HttpRequest) -> ActixResult<String> {
    // 从Authorization头部提取Bearer token
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Ok(auth_str[7..].to_string());
            }
        }
    }
    
    // 从X-API-Key头部提取
    if let Some(api_key_header) = req.headers().get("X-API-Key") {
        if let Ok(api_key) = api_key_header.to_str() {
            return Ok(api_key.to_string());
        }
    }
    
    Err(ErrorUnauthorized("Missing or invalid API key"))
}

/// 验证API密钥并返回商户信息
/// 
/// # Arguments
/// * `pool` - 数据库连接池
/// * `api_key` - API密钥
/// 
/// # Returns
/// * 商户信息
pub async fn verify_api_key(pool: &PgPool, api_key: &str) -> Result<Merchant> {
    let merchant = sqlx::query_as!(
        Merchant,
        r#"
        SELECT id, name, email, api_key, api_secret, webhook_url,
               status as "status: _", created_at, updated_at
        FROM merchants 
        WHERE api_key = $1 AND status = 'active'
        "#,
        api_key
    )
    .fetch_optional(pool)
    .await
    .context("Failed to query merchant")?;

    merchant.ok_or_else(|| anyhow::anyhow!("Invalid or inactive API key"))
}

/// 验证商户是否有权限访问指定的支付订单
/// 
/// # Arguments
/// * `pool` - 数据库连接池
/// * `payment_id` - 支付订单ID
/// * `merchant_id` - 商户ID
/// 
/// # Returns
/// * 是否有权限
pub async fn verify_payment_access(
    pool: &PgPool, 
    payment_id: Uuid, 
    merchant_id: Uuid
) -> Result<bool> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM payments WHERE id = $1 AND merchant_id = $2",
        payment_id,
        merchant_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to verify payment access")?;

    Ok(count.unwrap_or(0) > 0)
}

/// 生成JWT令牌 (用于管理后台)
/// 
/// # Arguments
/// * `merchant_id` - 商户ID
/// * `secret` - JWT密钥
/// 
/// # Returns
/// * JWT令牌字符串
pub fn generate_jwt_token(merchant_id: Uuid, secret: &str) -> Result<String> {
    use jsonwebtoken::{encode, Header, EncodingKey};
    use serde::{Serialize};
    use chrono::{Utc, Duration};

    #[derive(Debug, Serialize)]
    struct Claims {
        sub: String, // 商户ID
        exp: i64,    // 过期时间
        iat: i64,    // 签发时间
    }

    let now = Utc::now();
    let claims = Claims {
        sub: merchant_id.to_string(),
        exp: (now + Duration::hours(24)).timestamp(), // 24小时过期
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .context("Failed to generate JWT token")?;

    Ok(token)
}

/// 验证JWT令牌
/// 
/// # Arguments
/// * `token` - JWT令牌
/// * `secret` - JWT密钥
/// 
/// # Returns
/// * 商户ID
pub fn verify_jwt_token(token: &str, secret: &str) -> Result<Uuid> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Claims {
        sub: String,
        exp: i64,
    }

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .context("Invalid JWT token")?;

    let merchant_id = Uuid::parse_str(&token_data.claims.sub)
        .context("Invalid merchant ID in token")?;

    Ok(merchant_id)
}

/// 验证请求签名 (用于Webhook验证)
/// 
/// # Arguments
/// * `payload` - 请求载荷
/// * `signature` - 请求签名
/// * `secret` - 签名密钥
/// 
/// # Returns
/// * 签名是否有效
pub fn verify_request_signature(payload: &str, signature: &str, secret: &str) -> Result<bool> {
    crate::utils::crypto::verify_hmac_signature(payload, signature, secret)
}

/// 生成Webhook签名
/// 
/// # Arguments
/// * `payload` - Webhook载荷JSON字符串
/// * `secret` - 商户API密钥
/// 
/// # Returns
/// * HMAC签名
pub fn generate_webhook_signature(payload: &str, secret: &str) -> Result<String> {
    crate::utils::crypto::generate_hmac_signature(payload, secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key1 = generate_api_key(32);
        let key2 = generate_api_key(32);
        
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2); // 应该生成不同的密钥
    }

    #[test]
    fn test_generate_api_key_pair() {
        let (api_key, api_secret) = generate_api_key_pair(32, 64);
        
        assert_eq!(api_key.len(), 32);
        assert_eq!(api_secret.len(), 64);
        assert_ne!(api_key, api_secret);
    }

    #[test]
    fn test_jwt_token() {
        let merchant_id = Uuid::new_v4();
        let secret = "test_jwt_secret";
        
        let token = generate_jwt_token(merchant_id, secret).unwrap();
        assert!(!token.is_empty());
        
        let verified_id = verify_jwt_token(&token, secret).unwrap();
        assert_eq!(merchant_id, verified_id);
    }

    #[test]
    fn test_webhook_signature() {
        let payload = r#"{"event":"payment.completed","payment_id":"123"}"#;
        let secret = "webhook_secret";
        
        let signature = generate_webhook_signature(payload, secret).unwrap();
        let is_valid = verify_request_signature(payload, &signature, secret).unwrap();
        assert!(is_valid);
        
        let is_invalid = verify_request_signature(payload, "invalid_signature", secret).unwrap();
        assert!(!is_invalid);
    }
}
