// Webhook处理器
// 处理Webhook相关的HTTP请求，包括测试和统计查询

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;
use serde::Deserialize;
use crate::models::{ApiResponse, PaymentWebhookPayload, PaymentStatus, Currency};
use crate::services::{WebhookService, webhook_service::WebhookStats};
use crate::state::AppState;
use crate::utils::extract_api_key;

/// Webhook测试请求
#[derive(Debug, Deserialize)]
pub struct TestWebhookRequest {
    /// 测试载荷类型
    pub event_type: String,
    /// 测试数据
    pub test_data: Option<serde_json::Value>,
}

/// 测试Webhook端点
/// 
/// POST /api/v1/webhooks/test
/// 
/// 需要API密钥认证
/// 请求体: TestWebhookRequest
/// 响应: 测试结果
pub async fn test_webhook(
    data: web::Data<AppState>,
    request: web::Json<TestWebhookRequest>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    // 验证商户身份
    let merchant_service = crate::services::MerchantService::new(data.db_pool.clone());
    let merchant = match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(merchant)) => merchant,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")));
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")));
        }
    };

    // 检查商户是否配置了Webhook URL
    let webhook_url = match &merchant.webhook_url {
        Some(url) => url,
        None => {
            return Ok(HttpResponse::BadRequest().json(
                ApiResponse::<()>::error("Webhook URL not configured")
            ));
        }
    };

    // 创建测试载荷
    let test_payload = PaymentWebhookPayload {
        payment_id: Uuid::new_v4(),
        order_id: "TEST_ORDER_WEBHOOK".to_string(),
        status: PaymentStatus::Completed,
        amount: rust_decimal::Decimal::new(100, 2),
        currency: Currency::ETH,
        transaction_hash: Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
        confirmations: Some(12),
    };

    // 发送测试Webhook
    let webhook_service = WebhookService::new(data.db_pool.clone(), 1); // 测试时只重试1次

    match webhook_service.send_payment_notification(
        test_payload.payment_id,
        merchant.id,
        webhook_url,
        &merchant.api_secret,
        test_payload,
    ).await {
        Ok(_) => {
            log::info!("Test webhook sent successfully for merchant: {}", merchant.id);
            Ok(HttpResponse::Ok().json(ApiResponse::success("Webhook test successful")))
        },
        Err(e) => {
            log::warn!("Test webhook failed for merchant {}: {}", merchant.id, e);
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Webhook test failed: {}", e))))
        }
    }
}

/// 获取Webhook统计信息
/// 
/// GET /api/v1/webhooks/stats
/// 
/// 需要API密钥认证
/// 查询参数: days (可选，默认7天)
/// 响应: WebhookStats
pub async fn get_webhook_stats(
    data: web::Data<AppState>,
    query: web::Query<WebhookStatsQuery>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    // 验证商户身份
    let merchant_service = crate::services::MerchantService::new(data.db_pool.clone());
    let merchant = match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(merchant)) => merchant,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")));
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")));
        }
    };

    // 获取Webhook统计
    let webhook_service = WebhookService::new(data.db_pool.clone(), 5);
    let days = query.days.unwrap_or(7);

    match webhook_service.get_webhook_stats(merchant.id, days).await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        },
        Err(e) => {
            log::error!("Failed to get webhook stats for merchant {}: {}", merchant.id, e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// Webhook统计查询参数
#[derive(Debug, Deserialize)]
pub struct WebhookStatsQuery {
    /// 统计天数
    pub days: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::state::AppState;

    #[actix_web::test]
    async fn test_webhook_stats_handler() {
        let app_state = AppState::new_for_test().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/webhooks/stats", web::get().to(get_webhook_stats))
        ).await;

        let req = test::TestRequest::get()
            .uri("/webhooks/stats?days=30")
            .insert_header(("X-API-Key", "test_api_key"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // 注意: 这个测试需要有效的API密钥
        assert!(resp.status().is_client_error() || resp.status().is_success());
    }
}
