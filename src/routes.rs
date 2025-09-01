// API路由配置
// 定义所有HTTP接口的路由规则

use actix_web::{web, Scope};
use crate::handlers::*;

/// API v1路由配置
pub fn api_v1_routes() -> Scope {
    web::scope("/api/v1")
        // 商户管理路由
        .service(merchant_routes())
        // 支付订单路由
        .service(payment_routes())
        // Webhook路由
        .service(webhook_routes())
        // 系统状态路由
        .route("/status", web::get().to(system_status))
        .route("/version", web::get().to(version_info))
        .route("/network/status", web::get().to(network_status))
}

/// 商户管理路由
fn merchant_routes() -> Scope {
    web::scope("/merchants")
        .route("", web::post().to(create_merchant))
        .route("/{merchant_id}", web::get().to(get_merchant))
        .route("/{merchant_id}", web::put().to(update_merchant))
        .route("/{merchant_id}", web::delete().to(deactivate_merchant))
        .route("/{merchant_id}/regenerate-keys", web::post().to(regenerate_api_keys))
        .route("/{merchant_id}/stats", web::get().to(get_merchant_stats))
}

/// 支付订单路由
fn payment_routes() -> Scope {
    web::scope("/payments")
        .route("", web::post().to(create_payment))
        .route("", web::get().to(list_payments))
        .route("/{payment_id}", web::get().to(get_payment))
        .route("/{payment_id}/qrcode", web::get().to(get_payment_qrcode))
}

/// Webhook路由
fn webhook_routes() -> Scope {
    web::scope("/webhooks")
        .route("/test", web::post().to(test_webhook))
        .route("/stats", web::get().to(get_webhook_stats))
}


/// 公共路由 (无需认证)
pub fn public_routes() -> Scope {
    web::scope("")
        .route("/health", web::get().to(health_check))
}