// 支付服务
// 负责支付订单创建、状态管理、查询等核心业务逻辑

use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use crate::models::{
    Payment, PaymentStatus, Currency, CreatePaymentRequest, CreatePaymentResponse,
    PaymentResponse, PaymentListQuery, PaymentListResponse, PaginationInfo
};
use crate::utils::{validate_order_id, validate_payment_amount, generate_payment_qr_code};
use crate::services::EthereumService;

/// 支付服务
pub struct PaymentService {
    pool: PgPool,
    ethereum_service: EthereumService,
}

impl PaymentService {
    /// 创建新的支付服务实例
    pub fn new(pool: PgPool, ethereum_service: EthereumService) -> Self {
        Self { pool, ethereum_service }
    }

    /// 创建支付订单
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// * `request` - 支付创建请求
    /// 
    /// # Returns
    /// * 支付订单创建响应
    pub async fn create_payment(
        &self,
        merchant_id: Uuid,
        request: CreatePaymentRequest,
    ) -> Result<CreatePaymentResponse> {
        // 输入验证
        self.validate_create_request(&request)?;

        // 检查订单ID是否已存在
        self.check_order_id_exists(merchant_id, &request.order_id).await?;

        // 生成支付地址
        let payment_address = self.ethereum_service.generate_payment_address().await?;

        // 计算过期时间
        let expires_at = request.expires_in.map(|seconds| {
            Utc::now() + Duration::seconds(seconds)
        }).unwrap_or_else(|| {
            Utc::now() + Duration::hours(1) // 默认1小时过期
        });

        // 创建支付订单
        let payment_id = Uuid::new_v4();
        let created_at = Utc::now();

        sqlx::query!(
            r#"
            INSERT INTO payments (
                id, merchant_id, order_id, amount, currency, 
                payment_address, expires_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            "#,
            payment_id,
            merchant_id,
            request.order_id,
            request.amount,
            request.currency as Currency,
            payment_address,
            expires_at,
            created_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create payment")?;

        // 生成支付URL和二维码
        let payment_url = self.generate_payment_url(&request.currency, &payment_address, &request.amount);
        let qr_code = generate_payment_qr_code(&payment_url)
            .context("Failed to generate QR code")?;

        // 启动交易监听
        let pool_clone = self.pool.clone();
        let ethereum_service_clone = self.ethereum_service.clone();
        tokio::spawn(async move {
            if let Err(e) = ethereum_service_clone.monitor_payment(payment_id, &payment_address, pool_clone).await {
                log::error!("Failed to monitor payment {}: {}", payment_id, e);
            }
        });

        log::info!("Created payment order: {} for merchant: {}", payment_id, merchant_id);

        Ok(CreatePaymentResponse {
            payment_id,
            payment_address,
            amount: request.amount,
            currency: request.currency,
            expires_at: Some(expires_at),
            qr_code,
            payment_url,
        })
    }

    /// 根据ID获取支付订单
    /// 
    /// # Arguments
    /// * `payment_id` - 支付订单ID
    /// * `merchant_id` - 商户ID (用于权限验证)
    /// 
    /// # Returns
    /// * 支付订单信息
    pub async fn get_payment(
        &self,
        payment_id: Uuid,
        merchant_id: Uuid,
    ) -> Result<Option<PaymentResponse>> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            SELECT id, merchant_id, order_id, amount, 
                   currency as "currency: _", payment_address,
                   status as "status: _", transaction_hash, confirmations,
                   expires_at, created_at, updated_at
            FROM payments 
            WHERE id = $1 AND merchant_id = $2
            "#,
            payment_id,
            merchant_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch payment")?;

        Ok(payment.map(|p| p.to_response()))
    }

    /// 获取商户的支付订单列表
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// * `query` - 查询参数
    /// 
    /// # Returns
    /// * 支付订单列表响应
    pub async fn list_payments(
        &self,
        merchant_id: Uuid,
        query: PaymentListQuery,
    ) -> Result<PaymentListResponse> {
        let limit = query.limit() as i64;
        let offset = query.offset() as i64;

        // 构建查询条件
        let mut where_conditions = vec!["merchant_id = $1".to_string()];
        let mut param_index = 2;

        if let Some(status) = &query.status {
            where_conditions.push(format!("status = ${}", param_index));
            param_index += 1;
        }

        if let Some(currency) = &query.currency {
            where_conditions.push(format!("currency = ${}", param_index));
            param_index += 1;
        }

        if query.start_date.is_some() {
            where_conditions.push(format!("created_at >= ${}", param_index));
            param_index += 1;
        }

        if query.end_date.is_some() {
            where_conditions.push(format!("created_at <= ${}", param_index));
        }

        let where_clause = where_conditions.join(" AND ");

        // 查询总数
        let total_count = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM payments WHERE {}",
            where_clause
        ))
        .bind(merchant_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count payments")?
        .unwrap_or(0) as u64;

        // 查询支付订单
        let payments = sqlx::query_as!(
            Payment,
            &format!(
                r#"
                SELECT id, merchant_id, order_id, amount, 
                       currency as "currency: _", payment_address,
                       status as "status: _", transaction_hash, confirmations,
                       expires_at, created_at, updated_at
                FROM payments 
                WHERE {}
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                where_clause
            ),
            merchant_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch payments")?;

        let payment_responses: Vec<PaymentResponse> = payments
            .into_iter()
            .map(|p| p.to_response())
            .collect();

        let pagination = PaginationInfo::new(
            query.page.unwrap_or(1),
            query.limit(),
            total_count,
        );

        Ok(PaymentListResponse {
            payments: payment_responses,
            pagination,
        })
    }

    /// 更新支付订单状态
    /// 
    /// # Arguments
    /// * `payment_id` - 支付订单ID
    /// * `status` - 新状态
    /// * `transaction_hash` - 交易哈希 (可选)
    /// * `confirmations` - 确认数 (可选)
    /// 
    /// # Returns
    /// * 操作结果
    pub async fn update_payment_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
        transaction_hash: Option<String>,
        confirmations: Option<i32>,
    ) -> Result<()> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "UPDATE payments SET status = "
        );
        
        query_builder.push_bind(status as PaymentStatus);
        
        if let Some(hash) = transaction_hash {
            query_builder.push(", transaction_hash = ");
            query_builder.push_bind(hash);
        }
        
        if let Some(conf) = confirmations {
            query_builder.push(", confirmations = ");
            query_builder.push_bind(conf);
        }
        
        query_builder.push(", updated_at = NOW() WHERE id = ");
        query_builder.push_bind(payment_id);

        let rows_affected = query_builder
            .build()
            .execute(&self.pool)
            .await
            .context("Failed to update payment status")?
            .rows_affected();

        if rows_affected == 0 {
            anyhow::bail!("Payment not found");
        }

        log::info!("Updated payment {} status to {:?}", payment_id, status);
        Ok(())
    }

    /// 标记过期的支付订单
    /// 
    /// # Returns
    /// * 标记的订单数量
    pub async fn mark_expired_payments(&self) -> Result<u64> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE payments 
            SET status = 'expired', updated_at = NOW()
            WHERE status = 'pending' AND expires_at < NOW()
            "#
        )
        .execute(&self.pool)
        .await
        .context("Failed to mark expired payments")?
        .rows_affected();

        if rows_affected > 0 {
            log::info!("Marked {} payments as expired", rows_affected);
        }

        Ok(rows_affected)
    }

    /// 获取待处理的支付订单 (用于监听服务)
    /// 
    /// # Returns
    /// * 待处理的支付订单列表
    pub async fn get_pending_payments(&self) -> Result<Vec<Payment>> {
        let payments = sqlx::query_as!(
            Payment,
            r#"
            SELECT id, merchant_id, order_id, amount, 
                   currency as "currency: _", payment_address,
                   status as "status: _", transaction_hash, confirmations,
                   expires_at, created_at, updated_at
            FROM payments 
            WHERE status IN ('pending', 'confirmed') 
            AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch pending payments")?;

        Ok(payments)
    }

    /// 生成支付URL
    fn generate_payment_url(&self, currency: &Currency, address: &str, amount: &Decimal) -> String {
        match currency {
            Currency::ETH => {
                let wei_amount = amount * Decimal::from(10_u64.pow(18));
                format!("ethereum:{}?value={}", address, wei_amount.trunc())
            },
            Currency::USDT => {
                let usdt_amount = amount * Decimal::from(10_u64.pow(6));
                format!("ethereum:{}@1/transfer?address={}&uint256={}",
                    currency.contract_address().unwrap(),
                    address,
                    usdt_amount.trunc()
                )
            }
        }
    }

    /// 验证创建支付请求
    fn validate_create_request(&self, request: &CreatePaymentRequest) -> Result<()> {
        // 验证订单ID
        validate_order_id(&request.order_id)?;

        // 验证支付金额
        validate_payment_amount(&request.amount, &format!("{:?}", request.currency))?;

        // 验证过期时间
        if let Some(expires_in) = request.expires_in {
            if expires_in <= 0 {
                anyhow::bail!("Expiration time must be positive");
            }
            if expires_in > 86400 * 7 { // 最大7天
                anyhow::bail!("Expiration time too long (max 7 days)");
            }
        }

        // 验证回调URL
        if let Some(callback_url) = &request.callback_url {
            if !callback_url.is_empty() && !crate::utils::validate_url(callback_url) {
                anyhow::bail!("Invalid callback URL format");
            }
        }

        Ok(())
    }

    /// 检查订单ID是否已存在
    async fn check_order_id_exists(&self, merchant_id: Uuid, order_id: &str) -> Result<()> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM payments WHERE merchant_id = $1 AND order_id = $2",
            merchant_id,
            order_id
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to check order ID existence")?;

        if count.unwrap_or(0) > 0 {
            anyhow::bail!("Order ID already exists for this merchant");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::EthereumService;

    async fn setup_test_service() -> PaymentService {
        let pool = PgPool::connect("postgres://test:test@localhost/wopay_test")
            .await
            .expect("Failed to connect to test database");
        
        let ethereum_service = EthereumService::new_with_config(
            "https://eth-goerli.alchemyapi.io/v2/demo".to_string(),
            None,
            5, // Goerli testnet
        ).await.expect("Failed to create Ethereum service");

        PaymentService::new(pool, ethereum_service)
    }

    #[tokio::test]
    async fn test_create_payment() {
        let service = setup_test_service().await;
        let merchant_id = Uuid::new_v4();

        let request = CreatePaymentRequest {
            order_id: "TEST_ORDER_001".to_string(),
            amount: Decimal::new(100, 2), // 1.00
            currency: Currency::USDT,
            callback_url: Some("https://example.com/webhook".to_string()),
            expires_in: Some(3600), // 1小时
        };

        let response = service.create_payment(merchant_id, request).await.unwrap();
        
        assert!(!response.payment_address.is_empty());
        assert!(response.payment_address.starts_with("0x"));
        assert_eq!(response.amount, Decimal::new(100, 2));
        assert_eq!(response.currency, Currency::USDT);
        assert!(response.expires_at.is_some());
        assert!(response.qr_code.starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn test_validate_create_request() {
        let service = setup_test_service().await;

        // 有效请求
        let valid_request = CreatePaymentRequest {
            order_id: "VALID_ORDER_123".to_string(),
            amount: Decimal::new(100, 2),
            currency: Currency::ETH,
            callback_url: None,
            expires_in: Some(3600),
        };
        assert!(service.validate_create_request(&valid_request).is_ok());

        // 无效金额
        let invalid_amount_request = CreatePaymentRequest {
            order_id: "ORDER_123".to_string(),
            amount: Decimal::ZERO,
            currency: Currency::ETH,
            callback_url: None,
            expires_in: Some(3600),
        };
        assert!(service.validate_create_request(&invalid_amount_request).is_err());

        // 无效过期时间
        let invalid_expiry_request = CreatePaymentRequest {
            order_id: "ORDER_123".to_string(),
            amount: Decimal::new(100, 2),
            currency: Currency::ETH,
            callback_url: None,
            expires_in: Some(-1),
        };
        assert!(service.validate_create_request(&invalid_expiry_request).is_err());
    }
}
