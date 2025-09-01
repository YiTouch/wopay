// 商户管理API处理器
// 处理商户注册、查询、更新、API密钥管理等HTTP请求

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;
use crate::models::{
    CreateMerchantRequest, UpdateMerchantRequest, 
    MerchantResponse, ApiResponse
};
use crate::services::{MerchantService, merchant_service::MerchantStats};
use crate::state::AppState;
use crate::utils::extract_api_key;

/// 注册新商户
/// 
/// POST /api/v1/merchants
/// 
/// 请求体: CreateMerchantRequest
/// 响应: CreateMerchantResponse
pub async fn create_merchant(
    data: web::Data<AppState>,
    request: web::Json<CreateMerchantRequest>,
) -> ActixResult<HttpResponse> {
    let merchant_service = MerchantService::new(data.db_pool.clone());

    match merchant_service.create_merchant(request.into_inner()).await {
        Ok(response) => {
            log::info!("Successfully created merchant: {}", response.merchant_id);
            Ok(HttpResponse::Created().json(ApiResponse::success(response)))
        },
        Err(e) => {
            log::error!("Failed to create merchant: {}", e);
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// 获取商户信息
/// 
/// GET /api/v1/merchants/{merchant_id}
/// 
/// 需要API密钥认证
/// 响应: MerchantResponse
pub async fn get_merchant(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let merchant_id = path.into_inner();

    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let merchant_service = MerchantService::new(data.db_pool.clone());

    // 验证API密钥并获取商户信息
    match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(auth_merchant)) => {
            // 检查权限：只能查询自己的信息
            if auth_merchant.id != merchant_id {
                return Ok(HttpResponse::Forbidden().json(
                    ApiResponse::<()>::error("Access denied")
                ));
            }

            let response = MerchantResponse {
                id: auth_merchant.id,
                name: auth_merchant.name,
                email: auth_merchant.email,
                webhook_url: auth_merchant.webhook_url,
                status: auth_merchant.status,
                created_at: auth_merchant.created_at,
                updated_at: auth_merchant.updated_at,
            };

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        Ok(None) => {
            Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")))
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 更新商户信息
/// 
/// PUT /api/v1/merchants/{merchant_id}
/// 
/// 需要API密钥认证
/// 请求体: UpdateMerchantRequest
/// 响应: MerchantResponse
pub async fn update_merchant(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    request: web::Json<UpdateMerchantRequest>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let merchant_id = path.into_inner();

    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let merchant_service = MerchantService::new(data.db_pool.clone());

    // 验证API密钥
    match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(auth_merchant)) => {
            // 检查权限
            if auth_merchant.id != merchant_id {
                return Ok(HttpResponse::Forbidden().json(
                    ApiResponse::<()>::error("Access denied")
                ));
            }

            // 执行更新
            match merchant_service.update_merchant(merchant_id, request.into_inner()).await {
                Ok(updated_merchant) => {
                    let response = MerchantResponse {
                        id: updated_merchant.id,
                        name: updated_merchant.name,
                        email: updated_merchant.email,
                        webhook_url: updated_merchant.webhook_url,
                        status: updated_merchant.status,
                        created_at: updated_merchant.created_at,
                        updated_at: updated_merchant.updated_at,
                    };

                    log::info!("Successfully updated merchant: {}", merchant_id);
                    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
                },
                Err(e) => {
                    log::error!("Failed to update merchant {}: {}", merchant_id, e);
                    Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
                }
            }
        },
        Ok(None) => {
            Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")))
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 重新生成API密钥
/// 
/// POST /api/v1/merchants/{merchant_id}/regenerate-keys
/// 
/// 需要API密钥认证
/// 响应: RegenerateApiKeyResponse
pub async fn regenerate_api_keys(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let merchant_id = path.into_inner();

    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let merchant_service = MerchantService::new(data.db_pool.clone());

    // 验证API密钥
    match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(auth_merchant)) => {
            // 检查权限
            if auth_merchant.id != merchant_id {
                return Ok(HttpResponse::Forbidden().json(
                    ApiResponse::<()>::error("Access denied")
                ));
            }

            // 重新生成密钥
            match merchant_service.regenerate_api_keys(merchant_id).await {
                Ok(response) => {
                    log::info!("Successfully regenerated API keys for merchant: {}", merchant_id);
                    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
                },
                Err(e) => {
                    log::error!("Failed to regenerate API keys for merchant {}: {}", merchant_id, e);
                    Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
                }
            }
        },
        Ok(None) => {
            Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")))
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 获取商户统计信息
/// 
/// GET /api/v1/merchants/{merchant_id}/stats
/// 
/// 需要API密钥认证
/// 响应: MerchantStats
pub async fn get_merchant_stats(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let merchant_id = path.into_inner();

    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let merchant_service = MerchantService::new(data.db_pool.clone());

    // 验证API密钥
    match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(auth_merchant)) => {
            // 检查权限
            if auth_merchant.id != merchant_id {
                return Ok(HttpResponse::Forbidden().json(
                    ApiResponse::<()>::error("Access denied")
                ));
            }

            // 获取统计信息
            match merchant_service.get_merchant_stats(merchant_id).await {
                Ok(stats) => {
                    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
                },
                Err(e) => {
                    log::error!("Failed to get merchant stats for {}: {}", merchant_id, e);
                    Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
                }
            }
        },
        Ok(None) => {
            Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")))
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

/// 停用商户账户
/// 
/// DELETE /api/v1/merchants/{merchant_id}
/// 
/// 需要API密钥认证
/// 响应: 成功消息
pub async fn deactivate_merchant(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let merchant_id = path.into_inner();

    // 提取并验证API密钥
    let api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let merchant_service = MerchantService::new(data.db_pool.clone());

    // 验证API密钥
    match merchant_service.get_merchant_by_api_key(&api_key).await {
        Ok(Some(auth_merchant)) => {
            // 检查权限
            if auth_merchant.id != merchant_id {
                return Ok(HttpResponse::Forbidden().json(
                    ApiResponse::<()>::error("Access denied")
                ));
            }

            // 停用商户
            match merchant_service.deactivate_merchant(merchant_id).await {
                Ok(_) => {
                    log::info!("Successfully deactivated merchant: {}", merchant_id);
                    Ok(HttpResponse::Ok().json(ApiResponse::success("Merchant deactivated successfully")))
                },
                Err(e) => {
                    log::error!("Failed to deactivate merchant {}: {}", merchant_id, e);
                    Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
                }
            }
        },
        Ok(None) => {
            Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Invalid API key")))
        },
        Err(e) => {
            log::error!("Failed to authenticate merchant: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::state::AppState;

    #[actix_web::test]
    async fn test_create_merchant_handler() {
        let app_state = AppState::new_for_test().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/merchants", web::post().to(create_merchant))
        ).await;

        let request_body = serde_json::json!({
            "name": "Test Merchant",
            "email": "test@example.com",
            "webhook_url": "https://example.com/webhook"
        });

        let req = test::TestRequest::post()
            .uri("/merchants")
            .set_json(&request_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }
}
