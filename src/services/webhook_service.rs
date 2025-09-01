// Webhook通知服务
// 负责向商户发送支付状态变更通知，包含重试机制和签名验证

use sqlx::PgPool;
use uuid::Uuid;
use anyhow::{Result, Context};
use reqwest::{Client, header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT}};
use serde_json::json;
use tokio::time::{sleep, Duration};
use crate::models::{
    WebhookLog, WebhookEventType, WebhookStatus, PaymentWebhookPayload,
    MerchantWebhookPayload, WebhookRequest, WebhookResponse
};
use crate::utils::{generate_webhook_signature, verify_webhook_signature};

/// Webhook服务
pub struct WebhookService {
    pool: PgPool,
    client: Client,
    max_retries: u32,
    retry_delays: Vec<u64>, // 重试延迟时间 (秒)
}

impl WebhookService {
    /// 创建新的Webhook服务实例
    /// 
    /// # Arguments
    /// * `pool` - 数据库连接池
    /// * `max_retries` - 最大重试次数
    /// 
    /// # Returns
    /// * Webhook服务实例
    pub fn new(pool: PgPool, max_retries: u32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("WoPay-Webhook/1.0")
            .build()
            .expect("Failed to create HTTP client");

        // 指数退避重试策略: 5s, 15s, 45s, 135s, 405s
        let retry_delays = vec![5, 15, 45, 135, 405];

        Self {
            pool,
            client,
            max_retries,
            retry_delays,
        }
    }

    /// 发送支付状态变更通知
    /// 
    /// # Arguments
    /// * `payment_id` - 支付订单ID
    /// * `merchant_id` - 商户ID
    /// * `webhook_url` - Webhook URL
    /// * `api_secret` - 商户API密钥 (用于签名)
    /// * `payload` - 通知载荷
    /// 
    /// # Returns
    /// * 发送结果
    pub async fn send_payment_notification(
        &self,
        payment_id: Uuid,
        merchant_id: Uuid,
        webhook_url: &str,
        api_secret: &str,
        payload: PaymentWebhookPayload,
    ) -> Result<()> {
        let webhook_id = Uuid::new_v4();
        let event_type = WebhookEventType::PaymentStatusChanged;

        // 记录Webhook日志
        self.create_webhook_log(
            webhook_id,
            merchant_id,
            Some(payment_id),
            event_type,
            webhook_url,
            &payload,
        ).await?;

        // 发送通知
        let request = WebhookRequest {
            event_type,
            timestamp: chrono::Utc::now(),
            data: json!(payload),
        };

        self.send_webhook_with_retry(
            webhook_id,
            webhook_url,
            api_secret,
            &request,
        ).await
    }

    /// 发送商户状态变更通知
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// * `webhook_url` - Webhook URL
    /// * `api_secret` - 商户API密钥
    /// * `payload` - 通知载荷
    /// 
    /// # Returns
    /// * 发送结果
    pub async fn send_merchant_notification(
        &self,
        merchant_id: Uuid,
        webhook_url: &str,
        api_secret: &str,
        payload: MerchantWebhookPayload,
    ) -> Result<()> {
        let webhook_id = Uuid::new_v4();
        let event_type = WebhookEventType::MerchantStatusChanged;

        // 记录Webhook日志
        self.create_webhook_log(
            webhook_id,
            merchant_id,
            None,
            event_type,
            webhook_url,
            &payload,
        ).await?;

        // 发送通知
        let request = WebhookRequest {
            event_type,
            timestamp: chrono::Utc::now(),
            data: json!(payload),
        };

        self.send_webhook_with_retry(
            webhook_id,
            webhook_url,
            api_secret,
            &request,
        ).await
    }

    /// 带重试机制的Webhook发送
    async fn send_webhook_with_retry(
        &self,
        webhook_id: Uuid,
        url: &str,
        api_secret: &str,
        request: &WebhookRequest,
    ) -> Result<()> {
        let payload = serde_json::to_string(request)
            .context("Failed to serialize webhook request")?;

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match self.send_webhook_attempt(webhook_id, url, api_secret, &payload).await {
                Ok(response) => {
                    // 更新成功状态
                    self.update_webhook_status(
                        webhook_id,
                        WebhookStatus::Success,
                        Some(&response),
                        attempt,
                    ).await?;

                    log::info!("Webhook {} sent successfully after {} attempts", webhook_id, attempt + 1);
                    return Ok(());
                },
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.max_retries {
                        // 获取重试延迟时间
                        let delay = self.retry_delays.get(attempt as usize)
                            .copied()
                            .unwrap_or(300); // 默认5分钟

                        log::warn!("Webhook {} attempt {} failed, retrying in {}s", 
                            webhook_id, attempt + 1, delay);

                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        // 所有重试都失败
        let error_msg = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        self.update_webhook_status(
            webhook_id,
            WebhookStatus::Failed,
            Some(&WebhookResponse {
                status_code: 0,
                headers: std::collections::HashMap::new(),
                body: error_msg.clone(),
                duration_ms: 0,
            }),
            self.max_retries,
        ).await?;

        log::error!("Webhook {} failed after {} attempts: {}", 
            webhook_id, self.max_retries + 1, error_msg);

        anyhow::bail!("Webhook delivery failed after all retries: {}", error_msg)
    }

    /// 单次Webhook发送尝试
    async fn send_webhook_attempt(
        &self,
        webhook_id: Uuid,
        url: &str,
        api_secret: &str,
        payload: &str,
    ) -> Result<WebhookResponse> {
        let start_time = std::time::Instant::now();

        // 生成签名
        let signature = generate_webhook_signature(api_secret, payload)?;

        // 构建请求头
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("WoPay-Webhook/1.0"));
        headers.insert("X-WoPay-Signature", HeaderValue::from_str(&signature)?);
        headers.insert("X-WoPay-Webhook-Id", HeaderValue::from_str(&webhook_id.to_string())?);

        // 发送请求
        let response = self.client
            .post(url)
            .headers(headers)
            .body(payload.to_string())
            .send()
            .await
            .context("Failed to send webhook request")?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let status_code = response.status().as_u16();

        // 收集响应头
        let mut response_headers = std::collections::HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(name.to_string(), value_str.to_string());
            }
        }

        // 读取响应体
        let body = response.text().await
            .context("Failed to read response body")?;

        let webhook_response = WebhookResponse {
            status_code,
            headers: response_headers,
            body,
            duration_ms,
        };

        // 检查响应状态
        if status_code >= 200 && status_code < 300 {
            Ok(webhook_response)
        } else {
            anyhow::bail!("Webhook request failed with status {}: {}", 
                status_code, webhook_response.body)
        }
    }

    /// 创建Webhook日志记录
    async fn create_webhook_log<T: serde::Serialize>(
        &self,
        webhook_id: Uuid,
        merchant_id: Uuid,
        payment_id: Option<Uuid>,
        event_type: WebhookEventType,
        url: &str,
        payload: &T,
    ) -> Result<()> {
        let payload_json = serde_json::to_value(payload)
            .context("Failed to serialize webhook payload")?;

        sqlx::query!(
            r#"
            INSERT INTO webhook_logs (
                id, merchant_id, payment_id, event_type, url, 
                payload, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', NOW(), NOW())
            "#,
            webhook_id,
            merchant_id,
            payment_id,
            event_type as WebhookEventType,
            url,
            payload_json
        )
        .execute(&self.pool)
        .await
        .context("Failed to create webhook log")?;

        Ok(())
    }

    /// 更新Webhook状态
    async fn update_webhook_status(
        &self,
        webhook_id: Uuid,
        status: WebhookStatus,
        response: Option<&WebhookResponse>,
        attempts: u32,
    ) -> Result<()> {
        let response_json = response.map(|r| {
            serde_json::to_value(r).unwrap_or_default()
        });

        sqlx::query!(
            r#"
            UPDATE webhook_logs 
            SET status = $1, response = $2, attempts = $3, updated_at = NOW()
            WHERE id = $4
            "#,
            status as WebhookStatus,
            response_json,
            attempts as i32,
            webhook_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to update webhook status")?;

        Ok(())
    }

    /// 获取失败的Webhook日志 (用于重试)
    /// 
    /// # Arguments
    /// * `limit` - 返回数量限制
    /// 
    /// # Returns
    /// * 失败的Webhook日志列表
    pub async fn get_failed_webhooks(&self, limit: u32) -> Result<Vec<WebhookLog>> {
        let webhooks = sqlx::query_as!(
            WebhookLog,
            r#"
            SELECT id, merchant_id, payment_id, 
                   event_type as "event_type: _", url, payload,
                   status as "status: _", response, attempts,
                   created_at, updated_at
            FROM webhook_logs 
            WHERE status = 'failed' AND attempts < $1
            ORDER BY created_at ASC
            LIMIT $2
            "#,
            self.max_retries as i32,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch failed webhooks")?;

        Ok(webhooks)
    }

    /// 重试失败的Webhook
    /// 
    /// # Arguments
    /// * `webhook_log` - Webhook日志记录
    /// * `api_secret` - 商户API密钥
    /// 
    /// # Returns
    /// * 重试结果
    pub async fn retry_webhook(&self, webhook_log: &WebhookLog, api_secret: &str) -> Result<()> {
        let payload = serde_json::to_string(&webhook_log.payload)
            .context("Failed to serialize webhook payload")?;

        let request = WebhookRequest {
            event_type: webhook_log.event_type,
            timestamp: chrono::Utc::now(),
            data: webhook_log.payload.clone(),
        };

        let request_payload = serde_json::to_string(&request)
            .context("Failed to serialize webhook request")?;

        match self.send_webhook_attempt(webhook_log.id, &webhook_log.url, api_secret, &request_payload).await {
            Ok(response) => {
                self.update_webhook_status(
                    webhook_log.id,
                    WebhookStatus::Success,
                    Some(&response),
                    webhook_log.attempts as u32 + 1,
                ).await?;

                log::info!("Webhook {} retry succeeded", webhook_log.id);
                Ok(())
            },
            Err(e) => {
                let error_response = WebhookResponse {
                    status_code: 0,
                    headers: std::collections::HashMap::new(),
                    body: e.to_string(),
                    duration_ms: 0,
                };

                let new_attempts = webhook_log.attempts as u32 + 1;
                let new_status = if new_attempts >= self.max_retries {
                    WebhookStatus::Failed
                } else {
                    WebhookStatus::Pending
                };

                self.update_webhook_status(
                    webhook_log.id,
                    new_status,
                    Some(&error_response),
                    new_attempts,
                ).await?;

                log::warn!("Webhook {} retry failed (attempt {}): {}", 
                    webhook_log.id, new_attempts, e);

                Err(e)
            }
        }
    }

    /// 批量处理失败的Webhook
    /// 
    /// # Returns
    /// * 处理的Webhook数量
    pub async fn process_failed_webhooks(&self) -> Result<u32> {
        let failed_webhooks = self.get_failed_webhooks(50).await?;
        let mut processed_count = 0;

        for webhook in failed_webhooks {
            // 获取商户API密钥
            let api_secret = match self.get_merchant_api_secret(webhook.merchant_id).await {
                Ok(secret) => secret,
                Err(e) => {
                    log::error!("Failed to get API secret for merchant {}: {}", 
                        webhook.merchant_id, e);
                    continue;
                }
            };

            // 计算重试延迟
            let delay_index = (webhook.attempts as usize).min(self.retry_delays.len() - 1);
            let delay = self.retry_delays[delay_index];

            // 检查是否到了重试时间
            let should_retry = webhook.updated_at + chrono::Duration::seconds(delay as i64) <= chrono::Utc::now();
            
            if !should_retry {
                continue;
            }

            // 执行重试
            if let Err(e) = self.retry_webhook(&webhook, &api_secret).await {
                log::error!("Failed to retry webhook {}: {}", webhook.id, e);
            }

            processed_count += 1;
        }

        if processed_count > 0 {
            log::info!("Processed {} failed webhooks", processed_count);
        }

        Ok(processed_count)
    }

    /// 获取商户API密钥
    async fn get_merchant_api_secret(&self, merchant_id: Uuid) -> Result<String> {
        let api_secret = sqlx::query_scalar!(
            "SELECT api_secret FROM merchants WHERE id = $1 AND status = 'active'",
            merchant_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch merchant API secret")?
        .ok_or_else(|| anyhow::anyhow!("Merchant not found or inactive"))?;

        Ok(api_secret)
    }

    /// 验证Webhook签名
    /// 
    /// # Arguments
    /// * `signature` - 请求中的签名
    /// * `payload` - 请求载荷
    /// * `api_secret` - 商户API密钥
    /// 
    /// # Returns
    /// * 验证结果
    pub fn verify_signature(&self, signature: &str, payload: &str, api_secret: &str) -> Result<bool> {
        verify_webhook_signature(api_secret, payload, signature)
    }

    /// 获取Webhook统计信息
    /// 
    /// # Arguments
    /// * `merchant_id` - 商户ID
    /// * `days` - 统计天数
    /// 
    /// # Returns
    /// * Webhook统计数据
    pub async fn get_webhook_stats(&self, merchant_id: Uuid, days: u32) -> Result<WebhookStats> {
        let start_date = chrono::Utc::now() - chrono::Duration::days(days as i64);

        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_webhooks,
                COUNT(*) FILTER (WHERE status = 'success') as successful_webhooks,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_webhooks,
                COUNT(*) FILTER (WHERE status = 'pending') as pending_webhooks,
                AVG(attempts) as avg_attempts
            FROM webhook_logs 
            WHERE merchant_id = $1 AND created_at >= $2
            "#,
            merchant_id,
            start_date
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch webhook stats")?;

        let total = stats.total_webhooks.unwrap_or(0) as u64;
        let success_rate = if total > 0 {
            (stats.successful_webhooks.unwrap_or(0) as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(WebhookStats {
            total_webhooks: total,
            successful_webhooks: stats.successful_webhooks.unwrap_or(0) as u64,
            failed_webhooks: stats.failed_webhooks.unwrap_or(0) as u64,
            pending_webhooks: stats.pending_webhooks.unwrap_or(0) as u64,
            success_rate,
            average_attempts: stats.avg_attempts.unwrap_or(0.0),
        })
    }

    /// 清理过期的Webhook日志
    /// 
    /// # Arguments
    /// * `days` - 保留天数
    /// 
    /// # Returns
    /// * 清理的记录数
    pub async fn cleanup_old_webhooks(&self, days: u32) -> Result<u64> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days as i64);

        let rows_affected = sqlx::query!(
            "DELETE FROM webhook_logs WHERE created_at < $1",
            cutoff_date
        )
        .execute(&self.pool)
        .await
        .context("Failed to cleanup old webhooks")?
        .rows_affected();

        if rows_affected > 0 {
            log::info!("Cleaned up {} old webhook logs", rows_affected);
        }

        Ok(rows_affected)
    }
}

/// Webhook统计信息
#[derive(Debug, serde::Serialize)]
pub struct WebhookStats {
    /// 总Webhook数量
    pub total_webhooks: u64,
    /// 成功Webhook数量
    pub successful_webhooks: u64,
    /// 失败Webhook数量
    pub failed_webhooks: u64,
    /// 待处理Webhook数量
    pub pending_webhooks: u64,
    /// 成功率 (百分比)
    pub success_rate: f64,
    /// 平均尝试次数
    pub average_attempts: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PaymentStatus, Currency};

    async fn setup_test_service() -> WebhookService {
        let pool = PgPool::connect("postgres://test:test@localhost/wopay_test")
            .await
            .expect("Failed to connect to test database");

        WebhookService::new(pool, 3)
    }

    #[tokio::test]
    async fn test_webhook_service_creation() {
        let service = setup_test_service().await;
        assert_eq!(service.max_retries, 3);
        assert_eq!(service.retry_delays.len(), 5);
    }

    #[tokio::test]
    async fn test_verify_signature() {
        let service = setup_test_service().await;
        let api_secret = "test_secret";
        let payload = r#"{"test": "data"}"#;

        // 生成签名
        let signature = generate_webhook_signature(api_secret, payload).unwrap();

        // 验证签名
        let is_valid = service.verify_signature(&signature, payload, api_secret).unwrap();
        assert!(is_valid);

        // 验证错误签名
        let wrong_signature = "sha256=wrong_signature";
        let is_invalid = service.verify_signature(wrong_signature, payload, api_secret).unwrap();
        assert!(!is_invalid);
    }

    #[tokio::test]
    async fn test_webhook_payload_serialization() {
        let payload = PaymentWebhookPayload {
            payment_id: Uuid::new_v4(),
            order_id: "TEST_ORDER".to_string(),
            status: PaymentStatus::Completed,
            amount: rust_decimal::Decimal::new(100, 2),
            currency: Currency::ETH,
            transaction_hash: Some("0x123...".to_string()),
            confirmations: Some(12),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("payment_id"));
        assert!(json.contains("order_id"));
        assert!(json.contains("status"));
    }
}
