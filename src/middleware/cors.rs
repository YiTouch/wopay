// CORS中间件配置
// 处理跨域请求，支持前端应用访问API

use actix_cors::Cors;
use actix_web::http::header;

/// 创建CORS中间件
/// 
/// 配置允许的源、方法、头部等
pub fn create_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|origin, _req_head| {
            // 在生产环境中，应该限制为特定域名
            // 开发环境允许所有源
            origin.as_bytes().starts_with(b"http://localhost") ||
            origin.as_bytes().starts_with(b"https://localhost") ||
            origin.as_bytes().starts_with(b"http://127.0.0.1") ||
            origin.as_bytes().starts_with(b"https://127.0.0.1")
        })
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-api-key"),
            header::HeaderName::from_static("x-wopay-signature"),
        ])
        .expose_headers(vec![
            header::HeaderName::from_static("x-total-count"),
            header::HeaderName::from_static("x-page-count"),
        ])
        .max_age(3600)
}

/// 创建生产环境CORS配置
/// 
/// # Arguments
/// * `allowed_origins` - 允许的源列表
/// 
/// # Returns
/// * 配置好的CORS中间件
pub fn create_production_cors(allowed_origins: Vec<&str>) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-api-key"),
            header::HeaderName::from_static("x-wopay-signature"),
        ])
        .expose_headers(vec![
            header::HeaderName::from_static("x-total-count"),
            header::HeaderName::from_static("x-page-count"),
        ])
        .max_age(3600);

    // 添加允许的源
    for origin in allowed_origins {
        cors = cors.allowed_origin(origin);
    }

    cors
}
