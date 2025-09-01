// 健康检查和系统状态API处理器
// 提供系统健康状态、版本信息、区块链网络状态等查询接口

use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::Serialize;
use crate::models::ApiResponse;
use crate::services::EthereumService;
use crate::state::AppState;

/// 系统健康检查响应
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// 服务状态
    pub status: String,
    /// 版本信息
    pub version: String,
    /// 数据库连接状态
    pub database: String,
    /// 区块链连接状态
    pub blockchain: String,
    /// 当前时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 区块链网络状态响应
#[derive(Debug, Serialize)]
pub struct NetworkStatusResponse {
    /// 以太坊网络状态
    pub ethereum: crate::services::ethereum_service::NetworkStatus,
}

/// 基础健康检查
/// 
/// GET /health
/// 
/// 无需认证
/// 响应: HealthResponse
pub async fn health_check(
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let mut health = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "unknown".to_string(),
        blockchain: "unknown".to_string(),
        timestamp: chrono::Utc::now(),
    };

    // 检查数据库连接
    match sqlx::query("SELECT 1").fetch_one(&data.db_pool).await {
        Ok(_) => {
            health.database = "connected".to_string();
        },
        Err(e) => {
            log::error!("Database health check failed: {}", e);
            health.database = "disconnected".to_string();
            health.status = "unhealthy".to_string();
        }
    }

    // 检查区块链连接
    match EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await {
        Ok(ethereum_service) => {
            match ethereum_service.get_network_status().await {
                Ok(_) => {
                    health.blockchain = "connected".to_string();
                },
                Err(e) => {
                    log::error!("Blockchain health check failed: {}", e);
                    health.blockchain = "disconnected".to_string();
                    health.status = "degraded".to_string();
                }
            }
        },
        Err(e) => {
            log::error!("Failed to create Ethereum service for health check: {}", e);
            health.blockchain = "unavailable".to_string();
            health.status = "degraded".to_string();
        }
    }

    let status_code = match health.status.as_str() {
        "healthy" => 200,
        "degraded" => 200, // 部分功能可用
        _ => 503, // 服务不可用
    };

    Ok(HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
        .json(health))
}

/// 详细系统状态检查
/// 
/// GET /api/v1/status
/// 
/// 无需认证
/// 响应: 详细的系统状态信息
pub async fn system_status(
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    // 获取区块链网络状态
    let ethereum_status = match EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await {
        Ok(service) => {
            match service.get_network_status().await {
                Ok(status) => Some(status),
                Err(e) => {
                    log::error!("Failed to get Ethereum network status: {}", e);
                    None
                }
            }
        },
        Err(e) => {
            log::error!("Failed to create Ethereum service: {}", e);
            None
        }
    };

    // 获取数据库统计
    let db_stats = get_database_stats(&data.db_pool).await;

    let response = serde_json::json!({
        "service": {
            "name": "WoPay",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": get_uptime(),
            "timestamp": chrono::Utc::now()
        },
        "database": db_stats,
        "blockchain": {
            "ethereum": ethereum_status
        }
    });

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// 获取区块链网络状态
/// 
/// GET /api/v1/network/status
/// 
/// 无需认证
/// 响应: NetworkStatusResponse
pub async fn network_status(
    data: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    match EthereumService::new_with_config(
        data.config.blockchain.ethereum_rpc_url.clone(),
        data.config.blockchain.ethereum_ws_url.clone(),
        data.config.blockchain.chain_id,
    ).await {
        Ok(ethereum_service) => {
            match ethereum_service.get_network_status().await {
                Ok(ethereum_status) => {
                    let response = NetworkStatusResponse {
                        ethereum: ethereum_status,
                    };
                    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
                },
                Err(e) => {
                    log::error!("Failed to get network status: {}", e);
                    Ok(HttpResponse::ServiceUnavailable().json(
                        ApiResponse::<()>::error("Blockchain network unavailable")
                    ))
                }
            }
        },
        Err(e) => {
            log::error!("Failed to create Ethereum service: {}", e);
            Ok(HttpResponse::ServiceUnavailable().json(
                ApiResponse::<()>::error("Blockchain service unavailable")
            ))
        }
    }
}

/// 获取数据库统计信息
async fn get_database_stats(pool: &sqlx::PgPool) -> serde_json::Value {
    let stats = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM merchants) as total_merchants,
            (SELECT COUNT(*) FROM merchants WHERE status = 'active') as active_merchants,
            (SELECT COUNT(*) FROM payments) as total_payments,
            (SELECT COUNT(*) FROM payments WHERE status = 'completed') as completed_payments,
            (SELECT COUNT(*) FROM webhook_logs) as total_webhooks,
            (SELECT COUNT(*) FROM webhook_logs WHERE status = 'success') as successful_webhooks
        "#
    )
    .fetch_one(pool)
    .await;

    match stats {
        Ok(row) => serde_json::json!({
            "status": "connected",
            "merchants": {
                "total": row.total_merchants.unwrap_or(0),
                "active": row.active_merchants.unwrap_or(0)
            },
            "payments": {
                "total": row.total_payments.unwrap_or(0),
                "completed": row.completed_payments.unwrap_or(0)
            },
            "webhooks": {
                "total": row.total_webhooks.unwrap_or(0),
                "successful": row.successful_webhooks.unwrap_or(0)
            }
        }),
        Err(e) => {
            log::error!("Failed to get database stats: {}", e);
            serde_json::json!({
                "status": "error",
                "error": e.to_string()
            })
        }
    }
}

/// 获取服务运行时间
fn get_uptime() -> String {
    // 简单实现，实际应用中可以记录启动时间
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    format!("{} seconds", now % 86400) // 简化显示
}

/// 系统版本信息
/// 
/// GET /api/v1/version
/// 
/// 无需认证
/// 响应: 版本信息
pub async fn version_info() -> ActixResult<HttpResponse> {
    let version_info = serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "authors": env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<&str>>(),
        "build_time": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION")
    });

    Ok(HttpResponse::Ok().json(ApiResponse::success(version_info)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::state::AppState;

    #[actix_web::test]
    async fn test_health_check() {
        let app_state = AppState::new_for_test().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/health", web::get().to(health_check))
        ).await;

        let req = test::TestRequest::get()
            .uri("/health")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success() || resp.status().is_server_error());
    }

    #[actix_web::test]
    async fn test_version_info() {
        let app = test::init_service(
            App::new()
                .route("/version", web::get().to(version_info))
        ).await;

        let req = test::TestRequest::get()
            .uri("/version")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
