// 应用状态管理
// 包含数据库连接池、配置信息等全局状态

use sqlx::PgPool;
use actix_web::web;
use crate::config::Config;

/// 应用全局状态
pub struct AppState {
    /// 数据库连接池
    pub db_pool: PgPool,
    /// 应用配置
    pub config: Config,
}

impl AppState {
    /// 创建新的应用状态实例
    /// 
    /// # Arguments
    /// * `db_pool` - 数据库连接池
    /// * `config` - 应用配置
    /// 
    /// # Returns
    /// * 应用状态实例
    pub fn new(db_pool: PgPool, config: Config) -> Self {
        Self {
            db_pool,
            config,
        }
    }

    /// 创建测试用的应用状态
    #[cfg(test)]
    pub async fn new_for_test() -> Self {
        use crate::config::{Config, ServerConfig, DatabaseConfig, BlockchainConfig, EthereumConfig, SecurityConfig, RateLimitConfig, WebhookConfig};
        
        // 创建测试数据库连接
        let db_pool = PgPool::connect("postgres://test:test@localhost/wopay_test")
            .await
            .expect("Failed to connect to test database");

        // 创建测试配置
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: Some(1),
                timeout: 30,
            },
            database: DatabaseConfig {
                url: "postgres://test:test@localhost/wopay_test".to_string(),
                max_connections: 5,
                min_connections: 1,
                connection_timeout: 30,
                idle_timeout: 600,
            },
            blockchain: BlockchainConfig {
                ethereum: EthereumConfig {
                    rpc_url: "https://eth-goerli.alchemyapi.io/v2/demo".to_string(),
                    ws_url: None,
                    chain_id: 5,
                    private_key: "test_private_key".to_string(),
                    max_gas_price: 100,
                    gas_limit: 21000,
                },
                default_confirmations: 6,
                listener_interval: 30,
            },
            security: SecurityConfig {
                jwt_secret: "test_jwt_secret".to_string(),
                api_key_length: 32,
                hmac_key_length: 64,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 100,
                    burst_size: 10,
                },
            },
            webhook: WebhookConfig {
                max_retries: 3,
                retry_interval: 5,
                timeout: 30,
                concurrent_sends: 10,
            },
        };

        Self::new(db_pool, config)
    }
}

/// 应用状态数据类型别名
pub type AppStateData = web::Data<AppState>;