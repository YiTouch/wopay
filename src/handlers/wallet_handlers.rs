// 钱包管理API处理器
// 提供地址管理、资金归集、统计查询等功能

use actix_web::{web, HttpResponse, Result as ActixResult};
use crate::models::ApiResponse;
use crate::services::{WalletManager, CollectionService};
use crate::state::AppState;
use crate::utils::extract_api_key;
use serde::{Deserialize, Serialize};

/// 获取钱包统计信息
/// 
/// GET /api/v1/wallet/stats
/// 
/// 需要管理员权限
pub async fn get_wallet_stats(
    data: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 验证管理员权限
    let _api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    // TODO: 验证是否为管理员权限

    let wallet_manager = &data.wallet_manager;
    match wallet_manager.get_wallet_stats().await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        },
        Err(e) => {
            log::error!("Failed to get wallet stats: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to get wallet statistics")
            ))
        }
    }
}

/// 手动触发资金归集
/// 
/// POST /api/v1/wallet/collect
/// 
/// 需要管理员权限
pub async fn manual_collection(
    data: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 验证管理员权限
    let _api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let collection_service = &data.collection_service;
    match collection_service.manual_collection().await {
        Ok(tx_hashes) => {
            let response = ManualCollectionResponse {
                transaction_count: tx_hashes.len(),
                transaction_hashes: tx_hashes,
                message: format!("Successfully initiated {} collection transactions", tx_hashes.len()),
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        Err(e) => {
            log::error!("Manual collection failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Collection failed")
            ))
        }
    }
}

/// 获取归集统计
/// 
/// GET /api/v1/wallet/collection-stats?days=30
/// 
/// 需要管理员权限
pub async fn get_collection_stats(
    data: web::Data<AppState>,
    query: web::Query<CollectionStatsQuery>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 验证管理员权限
    let _api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let days = query.days.unwrap_or(30).min(365); // 最多查询1年
    let collection_service = &data.collection_service;

    match collection_service.get_collection_stats(days).await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        },
        Err(e) => {
            log::error!("Failed to get collection stats: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to get collection statistics")
            ))
        }
    }
}

/// 更新归集配置
/// 
/// PUT /api/v1/wallet/collection-config
/// 
/// 需要管理员权限
pub async fn update_collection_config(
    data: web::Data<AppState>,
    request: web::Json<UpdateCollectionConfigRequest>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 验证管理员权限
    let _api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let collection_service = &data.collection_service;
    let config = crate::services::collection_service::UpdateCollectionConfig {
        auto_collection_enabled: request.auto_collection_enabled,
        collection_threshold: request.collection_threshold,
        collection_interval_minutes: request.collection_interval_minutes,
    };

    match collection_service.update_collection_config(config).await {
        Ok(()) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success_no_data()))
        },
        Err(e) => {
            log::error!("Failed to update collection config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to update configuration")
            ))
        }
    }
}

/// 获取活跃地址列表
/// 
/// GET /api/v1/wallet/addresses?page=1&limit=50
/// 
/// 需要管理员权限
pub async fn get_active_addresses(
    data: web::Data<AppState>,
    query: web::Query<AddressListQuery>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    // 验证管理员权限
    let _api_key = match extract_api_key(&req) {
        Ok(key) => key,
        Err(e) => {
            return Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(&e.to_string())));
        }
    };

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).min(200); // 最多200条
    let offset = (page - 1) * limit;

    // 查询活跃地址
    let addresses = match sqlx::query!(
        r#"
        SELECT 
            pa.address,
            pa.address_index,
            pa.created_at,
            p.order_id,
            p.amount,
            p.currency,
            p.status as payment_status
        FROM payment_addresses pa
        JOIN payments p ON pa.payment_id = p.id
        WHERE pa.is_collected = false
        ORDER BY pa.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit as i64,
        offset as i64
    )
    .fetch_all(&data.db_pool)
    .await {
        Ok(addresses) => addresses,
        Err(e) => {
            log::error!("Failed to fetch active addresses: {}", e);
            return Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to fetch addresses")
            ));
        }
    };

    // 获取总数
    let total = match sqlx::query!(
        "SELECT COUNT(*) as count FROM payment_addresses WHERE is_collected = false"
    )
    .fetch_one(&data.db_pool)
    .await {
        Ok(row) => row.count.unwrap_or(0) as u32,
        Err(_) => 0,
    };

    let response = AddressListResponse {
        addresses: addresses.into_iter().map(|row| AddressInfo {
            address: row.address,
            address_index: row.address_index,
            order_id: row.order_id,
            amount: row.amount.to_string(),
            currency: row.currency,
            payment_status: row.payment_status,
            created_at: row.created_at,
        }).collect(),
        pagination: PaginationInfo {
            page,
            limit,
            total,
            total_pages: (total + limit - 1) / limit,
        },
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// 请求和响应结构体

#[derive(Debug, Deserialize)]
pub struct CollectionStatsQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollectionConfigRequest {
    pub auto_collection_enabled: Option<bool>,
    pub collection_threshold: Option<rust_decimal::Decimal>,
    pub collection_interval_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AddressListQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ManualCollectionResponse {
    pub transaction_count: usize,
    pub transaction_hashes: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AddressListResponse {
    pub addresses: Vec<AddressInfo>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct AddressInfo {
    pub address: String,
    pub address_index: i32,
    pub order_id: String,
    pub amount: String,
    pub currency: String,
    pub payment_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub total_pages: u32,
}
