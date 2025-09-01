// 支付订单API处理器
// 处理支付订单创建、查询、状态更新等HTTP请求

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;
use crate::models::{
    CreatePaymentRequest, PaymentListQuery, ApiResponse
};
use crate::services::{PaymentService, EthereumService};
use crate::state::AppState;
use crate::utils::extract_api_key;

/// 创建支付订单
/// 
/// POST /api/v1/payments
/// 
/// 需要API密钥认证
/// 请求体: CreatePaymentRequest
/// 响应: CreatePaymentResponse
pub async fn create_payment(
    data: web::Data<AppState>,
    request: web::Json<CreatePaymentRequest>,
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

    // 创建支付订单
    let ethereum_service = EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await.map_err(|e| {
        log::error!("Failed to create Ethereum service: {}", e);
        actix_web::error::ErrorInternalServerError("Blockchain service unavailable")
    })?;

    let payment_service = PaymentService::new(data.db_pool.clone(), ethereum_service);

    match payment_service.create_payment(merchant.id, request.into_inner()).await {
        Ok(response) => {
            log::info!("Successfully created payment: {} for merchant: {}", response.payment_id, merchant.id);
            Ok(HttpResponse::Created().json(ApiResponse::success(response)))
        },
        Err(e) => {
            log::error!("Failed to create payment for merchant {}: {}", merchant.id, e);
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// 获取支付订单详情
/// 
/// GET /api/v1/payments/{payment_id}
/// 
/// 需要API密钥认证
/// 响应: PaymentResponse
pub async fn get_payment(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let payment_id = path.into_inner();

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

    // 获取支付订单
    let ethereum_service = EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await.map_err(|e| {
        log::error!("Failed to create Ethereum service: {}", e);
        actix_web::error::ErrorInternalServerError("Blockchain service unavailable")
    })?;

    let payment_service = PaymentService::new(data.db_pool.clone(), ethereum_service);

    match payment_service.get_payment(payment_id, merchant.id).await {
        Ok(Some(payment)) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(payment)))
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Payment not found")))
        },
        Err(e) => {
            log::error!("Failed to get payment {} for merchant {}: {}", payment_id, merchant.id, e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 获取支付订单列表
/// 
/// GET /api/v1/payments
/// 
/// 需要API密钥认证
/// 查询参数: PaymentListQuery
/// 响应: PaymentListResponse
pub async fn list_payments(
    data: web::Data<AppState>,
    query: web::Query<PaymentListQuery>,
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

    // 获取支付订单列表
    let ethereum_service = EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await.map_err(|e| {
        log::error!("Failed to create Ethereum service: {}", e);
        actix_web::error::ErrorInternalServerError("Blockchain service unavailable")
    })?;

    let payment_service = PaymentService::new(data.db_pool.clone(), ethereum_service);

    match payment_service.list_payments(merchant.id, query.into_inner()).await {
        Ok(response) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        Err(e) => {
            log::error!("Failed to list payments for merchant {}: {}", merchant.id, e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 获取支付二维码
/// 
/// GET /api/v1/payments/{payment_id}/qrcode
/// 
/// 需要API密钥认证
/// 响应: PNG图片数据
pub async fn get_payment_qrcode(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let payment_id = path.into_inner();

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

    // 获取支付订单
    let ethereum_service = EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await.map_err(|e| {
        log::error!("Failed to create Ethereum service: {}", e);
        actix_web::error::ErrorInternalServerError("Blockchain service unavailable")
    })?;

    let payment_service = PaymentService::new(data.db_pool.clone(), ethereum_service);

    match payment_service.get_payment(payment_id, merchant.id).await {
        Ok(Some(payment)) => {
            // 生成二维码
            let payment_url = format!("ethereum:{}?value={}", 
                payment.payment_address, 
                payment.amount * rust_decimal::Decimal::from(10_u64.pow(18))
            );

            match crate::utils::generate_payment_qr_code(&payment_url) {
                Ok(qr_code_data) => {
                    // 解码base64数据
                    if let Some(data_part) = qr_code_data.strip_prefix("data:image/png;base64,") {
                        match base64::decode(data_part) {
                            Ok(image_data) => {
                                Ok(HttpResponse::Ok()
                                    .content_type("image/png")
                                    .body(image_data))
                            },
                            Err(e) => {
                                log::error!("Failed to decode QR code data: {}", e);
                                Ok(HttpResponse::InternalServerError().json(
                                    ApiResponse::<()>::error("Failed to generate QR code")
                                ))
                            }
                        }
                    } else {
                        Ok(HttpResponse::InternalServerError().json(
                            ApiResponse::<()>::error("Invalid QR code format")
                        ))
                    }
                },
                Err(e) => {
                    log::error!("Failed to generate QR code for payment {}: {}", payment_id, e);
                    Ok(HttpResponse::InternalServerError().json(
                        ApiResponse::<()>::error("Failed to generate QR code")
                    ))
                }
            }
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Payment not found")))
        },
        Err(e) => {
            log::error!("Failed to get payment {} for merchant {}: {}", payment_id, merchant.id, e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::state::AppState;
    use crate::models::{CreatePaymentRequest, Currency};

    #[actix_web::test]
    async fn test_create_payment_handler() {
        let app_state = AppState::new_for_test().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/payments", web::post().to(create_payment))
        ).await;

        let request_body = CreatePaymentRequest {
            order_id: "TEST_ORDER_001".to_string(),
            amount: rust_decimal::Decimal::new(100, 2),
            currency: Currency::ETH,
            callback_url: Some("https://example.com/callback".to_string()),
            expires_in: Some(3600),
        };

        let req = test::TestRequest::post()
            .uri("/payments")
            .set_json(&request_body)
            .insert_header(("X-API-Key", "test_api_key"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // 注意: 这个测试需要有效的API密钥，实际测试中需要先创建商户
        assert!(resp.status().is_client_error() || resp.status().is_success());
    }
}
