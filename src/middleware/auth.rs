// API认证中间件
// 负责验证API密钥、JWT令牌等认证机制

use actix_web::{
    dev::{ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    web, Result as ActixResult,
};
use futures_util::future::{ok, Ready};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use crate::models::ApiResponse;
use crate::services::MerchantService;
use crate::utils::extract_api_key;

/// API密钥认证中间件
pub struct ApiKeyAuth;

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ApiKeyAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ApiKeyAuthMiddleware { service })
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = &self.service;

        // 检查是否需要认证
        if should_skip_auth(req.path()) {
            let fut = service.call(req);
            return Box::pin(async move { fut.await });
        }

        // 提取API密钥
        let api_key = match extract_api_key(req.request()) {
            Ok(key) => key,
            Err(e) => {
                let response = HttpResponse::Unauthorized()
                    .json(ApiResponse::<()>::error(&e.to_string()));
                return Box::pin(async move {
                    Ok(req.into_response(response))
                });
            }
        };

        // 获取数据库连接池
        let pool = match req.app_data::<web::Data<crate::state::AppState>>() {
            Some(data) => data.db_pool.clone(),
            None => {
                let response = HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("Database unavailable"));
                return Box::pin(async move {
                    Ok(req.into_response(response))
                });
            }
        };

        let fut = service.call(req);

        Box::pin(async move {
            // 验证API密钥
            let merchant_service = MerchantService::new(pool);
            match merchant_service.get_merchant_by_api_key(&api_key).await {
                Ok(Some(merchant)) => {
                    // 将商户信息添加到请求扩展中
                    let (req, _) = fut.await?.into_parts();
                    req.extensions_mut().insert(merchant);
                    
                    // 继续处理请求
                    Ok(ServiceResponse::new(req.request().clone(), HttpResponse::Ok().finish()))
                },
                Ok(None) => {
                    let response = HttpResponse::Unauthorized()
                        .json(ApiResponse::<()>::error("Invalid API key"));
                    Ok(ServiceResponse::new(fut.await?.request().clone(), response))
                },
                Err(e) => {
                    log::error!("Failed to validate API key: {}", e);
                    let response = HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("Authentication service error"));
                    Ok(ServiceResponse::new(fut.await?.request().clone(), response))
                }
            }
        })
    }
}

/// 检查路径是否需要跳过认证
fn should_skip_auth(path: &str) -> bool {
    let public_paths = [
        "/health",
        "/api/v1/version",
        "/api/v1/status",
        "/api/v1/network/status",
        "/api/v1/merchants", // 商户注册接口
    ];

    public_paths.iter().any(|&public_path| path == public_path)
}

/// 从请求扩展中获取已认证的商户信息
pub fn get_authenticated_merchant(req: &actix_web::HttpRequest) -> Option<&crate::models::Merchant> {
    req.extensions().get::<crate::models::Merchant>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_auth() {
        assert!(should_skip_auth("/health"));
        assert!(should_skip_auth("/api/v1/version"));
        assert!(should_skip_auth("/api/v1/merchants"));
        assert!(!should_skip_auth("/api/v1/payments"));
        assert!(!should_skip_auth("/api/v1/merchants/123"));
    }
}
