// 商户管理服务
// 负责商户注册、认证、API密钥管理等业务逻辑

use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::models::{
    Merchant, MerchantStatus, CreateMerchantRequest, CreateMerchantResponse,
    UpdateMerchantRequest, RegenerateApiKeyResponse
};
use crate::utils::{generate_api_key_pair, validate_merchant_name, validate_email, validate_url, InputValidator};

/// 商户管理服务
pub struct MerchantService {
    pool: PgPool,
}

impl MerchantService {
    /// 创建新的商户服务实例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 注册新商户
    /// 
    /// # Arguments
    /// * `request` - 商户注册请求
    /// 
    /// # Returns
    /// * 商户注册响应，包含API密钥信息
    pub async fn create_merchant(&self, request: CreateMerchantRequest) -> Result<CreateMerchantResponse> {
        // 输入验证
        self.validate_create_request(&request)?;

        // 检查邮箱是否已存在
        self.check_email_exists(&request.email).await?;

        // 生成API密钥对
        let (api_key, api_secret) = generate_api_key_pair(32, 64);

        // 插入数据库
        let merchant_id = Uuid::new_v4();
        let created_at = chrono::Utc::now();

        sqlx::query!(
            r#"
            INSERT INTO merchants (id, name, email, api_key, api_secret, webhook_url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            "#,
            merchant_id,
            request.name,
            request.email,
            api_key,
            api_secret,
            request.webhook_url,
            created_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create merchant")?;

        log::info!("Created new merchant: {} ({})", request.name, merchant_id);

        Ok(CreateMerchantResponse {
            merchant_id,
            name: request.name,
            email: request.email,
            api_key,
            api_secret,
            created_at,
        })
    }

    /// 根据ID获取商户信息
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// 
    /// # Returns
    /// * 商户信息 (如果存在)
    pub async fn get_merchant(&self, merchant_id: Uuid) -> Result<Option<Merchant>> {
        let merchant = sqlx::query_as!(
            Merchant,
            r#"
            SELECT id, name, email, api_key, api_secret, webhook_url,
                   status as "status: _", created_at, updated_at
            FROM merchants 
            WHERE id = $1
            "#,
            merchant_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch merchant")?;

        Ok(merchant)
    }

    /// 根据API密钥获取商户信息
    /// 
    /// # Arguments
    /// * `api_key` - API密钥
    /// 
    /// # Returns
    /// * 商户信息 (如果存在且活跃)
    pub async fn get_merchant_by_api_key(&self, api_key: &str) -> Result<Option<Merchant>> {
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
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch merchant by API key")?;

        Ok(merchant)
    }

    /// 更新商户信息
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// * `request` - 更新请求
    /// 
    /// # Returns
    /// * 更新后的商户信息
    pub async fn update_merchant(
        &self, 
        merchant_id: Uuid, 
        request: UpdateMerchantRequest
    ) -> Result<Merchant> {
        // 输入验证
        self.validate_update_request(&request)?;

        // 检查商户是否存在
        let existing_merchant = self.get_merchant(merchant_id).await?
            .ok_or_else(|| anyhow::anyhow!("Merchant not found"))?;

        // 构建更新查询
        let name = request.name.unwrap_or(existing_merchant.name);
        let webhook_url = request.webhook_url.or(existing_merchant.webhook_url);
        let status = request.status.unwrap_or(existing_merchant.status);

        sqlx::query!(
            r#"
            UPDATE merchants 
            SET name = $1, webhook_url = $2, status = $3, updated_at = NOW()
            WHERE id = $4
            "#,
            name,
            webhook_url,
            status as MerchantStatus,
            merchant_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to update merchant")?;

        log::info!("Updated merchant: {}", merchant_id);

        // 返回更新后的商户信息
        self.get_merchant(merchant_id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch updated merchant"))
    }

    /// 重新生成API密钥
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// 
    /// # Returns
    /// * 新的API密钥信息
    pub async fn regenerate_api_keys(&self, merchant_id: Uuid) -> Result<RegenerateApiKeyResponse> {
        // 检查商户是否存在
        self.get_merchant(merchant_id).await?
            .ok_or_else(|| anyhow::anyhow!("Merchant not found"))?;

        // 生成新的API密钥对
        let (api_key, api_secret) = generate_api_key_pair(32, 64);
        let generated_at = chrono::Utc::now();

        // 更新数据库
        sqlx::query!(
            r#"
            UPDATE merchants 
            SET api_key = $1, api_secret = $2, updated_at = $3
            WHERE id = $4
            "#,
            api_key,
            api_secret,
            generated_at,
            merchant_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to regenerate API keys")?;

        log::info!("Regenerated API keys for merchant: {}", merchant_id);

        Ok(RegenerateApiKeyResponse {
            api_key,
            api_secret,
            generated_at,
        })
    }

    /// 停用商户
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// 
    /// # Returns
    /// * 操作结果
    pub async fn deactivate_merchant(&self, merchant_id: Uuid) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE merchants 
            SET status = 'inactive', updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            "#,
            merchant_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to deactivate merchant")?
        .rows_affected();

        if rows_affected == 0 {
            anyhow::bail!("Merchant not found or already inactive");
        }

        log::info!("Deactivated merchant: {}", merchant_id);
        Ok(())
    }

    /// 激活商户
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// 
    /// # Returns
    /// * 操作结果
    pub async fn activate_merchant(&self, merchant_id: Uuid) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE merchants 
            SET status = 'active', updated_at = NOW()
            WHERE id = $1 AND status != 'active'
            "#,
            merchant_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to activate merchant")?
        .rows_affected();

        if rows_affected == 0 {
            anyhow::bail!("Merchant not found or already active");
        }

        log::info!("Activated merchant: {}", merchant_id);
        Ok(())
    }

    /// 获取商户统计信息
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// 
    /// # Returns
    /// * 商户统计数据
    pub async fn get_merchant_stats(&self, merchant_id: Uuid) -> Result<MerchantStats> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_payments,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_payments,
                COUNT(*) FILTER (WHERE status = 'pending') as pending_payments,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_payments,
                COALESCE(SUM(amount) FILTER (WHERE status = 'completed'), 0) as total_volume
            FROM payments 
            WHERE merchant_id = $1
            "#,
            merchant_id
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch merchant stats")?;

        Ok(MerchantStats {
            total_payments: stats.total_payments.unwrap_or(0) as u64,
            completed_payments: stats.completed_payments.unwrap_or(0) as u64,
            pending_payments: stats.pending_payments.unwrap_or(0) as u64,
            failed_payments: stats.failed_payments.unwrap_or(0) as u64,
            total_volume: stats.total_volume.unwrap_or(rust_decimal::Decimal::ZERO),
            success_rate: if stats.total_payments.unwrap_or(0) > 0 {
                (stats.completed_payments.unwrap_or(0) as f64 / stats.total_payments.unwrap_or(1) as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// 验证创建商户请求
    fn validate_create_request(&self, request: &CreateMerchantRequest) -> Result<()> {
        let mut validator = InputValidator::new();

        validator.validate_required("name", &request.name);
        validator.validate_length("name", &request.name, 1, 255);
        
        validator.validate_required("email", &request.email);
        validator.validate_email_field("email", &request.email);

        if let Some(webhook_url) = &request.webhook_url {
            if !webhook_url.is_empty() {
                validator.validate_url_field("webhook_url", webhook_url);
            }
        }

        validator.into_result()
    }

    /// 验证更新商户请求
    fn validate_update_request(&self, request: &UpdateMerchantRequest) -> Result<()> {
        let mut validator = InputValidator::new();

        if let Some(name) = &request.name {
            validator.validate_required("name", name);
            validator.validate_length("name", name, 1, 255);
        }

        if let Some(webhook_url) = &request.webhook_url {
            if !webhook_url.is_empty() {
                validator.validate_url_field("webhook_url", webhook_url);
            }
        }

        validator.into_result()
    }

    /// 检查邮箱是否已存在
    async fn check_email_exists(&self, email: &str) -> Result<()> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM merchants WHERE email = $1",
            email
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to check email existence")?;

        if count.unwrap_or(0) > 0 {
            anyhow::bail!("Email already exists");
        }

        Ok(())
    }
}

/// 商户统计信息
#[derive(Debug, serde::Serialize)]
pub struct MerchantStats {
    /// 总支付订单数
    pub total_payments: u64,
    /// 已完成支付数
    pub completed_payments: u64,
    /// 待支付订单数
    pub pending_payments: u64,
    /// 失败支付数
    pub failed_payments: u64,
    /// 总交易金额
    pub total_volume: rust_decimal::Decimal,
    /// 成功率 (百分比)
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        // 注意: 这里需要配置测试数据库
        PgPool::connect("postgres://test:test@localhost/wopay_test")
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_create_merchant() {
        let pool = setup_test_db().await;
        let service = MerchantService::new(pool);

        let request = CreateMerchantRequest {
            name: "Test Merchant".to_string(),
            email: "test@example.com".to_string(),
            webhook_url: Some("https://example.com/webhook".to_string()),
        };

        let response = service.create_merchant(request).await.unwrap();
        
        assert!(!response.api_key.is_empty());
        assert!(!response.api_secret.is_empty());
        assert_eq!(response.name, "Test Merchant");
        assert_eq!(response.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_merchant_by_api_key() {
        let pool = setup_test_db().await;
        let service = MerchantService::new(pool);

        // 首先创建一个商户
        let create_request = CreateMerchantRequest {
            name: "Test Merchant".to_string(),
            email: "test2@example.com".to_string(),
            webhook_url: None,
        };

        let create_response = service.create_merchant(create_request).await.unwrap();
        
        // 然后通过API密钥查询
        let merchant = service.get_merchant_by_api_key(&create_response.api_key)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(merchant.id, create_response.merchant_id);
        assert_eq!(merchant.name, "Test Merchant");
    }
}
