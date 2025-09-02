// WoPay Web3支付系统主程序
// 初始化服务器、数据库连接、中间件和路由配置

mod config;
mod handlers;
mod models;
mod routes;
mod state;
mod services;
mod utils;
mod middleware;

use crate::config::Config;
use crate::routes::{api_v1_routes, public_routes};
use crate::state::AppState;
use crate::middleware::{RequestLogging, create_cors};
use actix_web::{App, HttpServer, middleware::Logger};
use sqlx::postgres::PgPoolOptions;
use anyhow::{Result, Context};

#[actix_web::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    log::info!("Starting WoPay Web3 Payment System...");

    // 加载配置
    let config = Config::from_env()
        .context("Failed to load configuration")?;

    log::info!("Configuration loaded successfully");

    // 验证配置
    config.validate()
        .context("Configuration validation failed")?;

    // 创建数据库连接池
    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect(&config.database.url)
        .await
        .context("Failed to create database connection pool")?;

    log::info!("Database connection pool created");

    // 运行数据库迁移
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .context("Failed to run database migrations")?;

    log::info!("Database migrations completed");

    // 创建应用状态
    let app_state = actix_web::web::Data::new(AppState::new(db_pool, config.clone()));

    // 启动后台任务
    start_background_tasks(app_state.clone()).await?;

    let server_host = config.server.host.clone();
    let server_port = config.server.port;
    let workers = config.server.workers;

    log::info!("Starting HTTP server on {}:{}", server_host, server_port);

    // 启动HTTP服务器
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // 添加中间件
            .wrap(Logger::default())
            .wrap(RequestLogging)
            .wrap(create_cors())
            // 添加路由
            .service(public_routes())
            .service(api_v1_routes())
    })
    .workers(workers)
    .bind(format!("{}:{}", server_host, server_port))
    .context("Failed to bind server address")?
    .run()
    .await
    .context("Server execution failed")?;

    Ok(())
}

/// 启动后台任务
async fn start_background_tasks(app_state: actix_web::web::Data<AppState>) -> Result<()> {
    let pool = app_state.db_pool.clone();
    let config = app_state.config.clone();

    // 启动支付监听任务
    tokio::spawn(async move {
        if let Err(e) = payment_monitoring_task(pool.clone(), config.clone()).await {
            log::error!("Payment monitoring task failed: {}", e);
        }
    });

    // 启动Webhook重试任务
    let pool_clone = app_state.db_pool.clone();
    tokio::spawn(async move {
        if let Err(e) = webhook_retry_task(pool_clone).await {
            log::error!("Webhook retry task failed: {}", e);
        }
    });

    // 启动过期支付清理任务
    let pool_clone = app_state.db_pool.clone();
    tokio::spawn(async move {
        if let Err(e) = expired_payment_cleanup_task(pool_clone).await {
            log::error!("Expired payment cleanup task failed: {}", e);
        }
    });

    log::info!("Background tasks started successfully");
    Ok(())
}

/// 支付监听后台任务
async fn payment_monitoring_task(pool: sqlx::PgPool, config: Config) -> Result<()> {
    use crate::services::{PaymentService, EthereumService};
    use tokio::time::{sleep, Duration};

    let ethereum_service = EthereumService::new_with_config(
        config.blockchain.ethereum_rpc_url.clone(),
        config.blockchain.ethereum_ws_url.clone(),
        config.blockchain.chain_id,
    ).await?;

    let payment_service = PaymentService::new(pool.clone(), ethereum_service.clone());

    loop {
        // 更新确认数
        if let Err(e) = ethereum_service.update_confirmations(&pool).await {
            log::error!("Failed to update confirmations: {}", e);
        }

        // 标记过期支付
        if let Err(e) = payment_service.mark_expired_payments().await {
            log::error!("Failed to mark expired payments: {}", e);
        }

        sleep(Duration::from_secs(30)).await; // 每30秒检查一次
    }
}

/// Webhook重试后台任务
async fn webhook_retry_task(pool: sqlx::PgPool) -> Result<()> {
    use crate::services::WebhookService;
    use tokio::time::{sleep, Duration};

    let webhook_service = WebhookService::new(pool, 5);

    loop {
        if let Err(e) = webhook_service.process_failed_webhooks().await {
            log::error!("Failed to process failed webhooks: {}", e);
        }

        sleep(Duration::from_secs(60)).await; // 每分钟检查一次
    }
}

/// 过期支付清理后台任务
async fn expired_payment_cleanup_task(pool: sqlx::PgPool) -> Result<()> {
    use crate::services::WebhookService;
    use tokio::time::{sleep, Duration};

    let webhook_service = WebhookService::new(pool, 5);

    loop {
        // 清理30天前的Webhook日志
        if let Err(e) = webhook_service.cleanup_old_webhooks(30).await {
            log::error!("Failed to cleanup old webhooks: {}", e);
        }

        sleep(Duration::from_secs(86400)).await; // 每天清理一次
    }
}
