// 配置管理模块
// 负责加载和管理应用程序配置

use serde::{Deserialize, Serialize};
use std::env;
use anyhow::{Result, Context};

/// 应用程序配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 服务器配置
    pub server: ServerConfig,
    /// 数据库配置
    pub database: DatabaseConfig,
    /// 区块链配置
    pub blockchain: BlockchainConfig,
    /// 安全配置
    pub security: SecurityConfig,
    /// Webhook配置
    pub webhook: WebhookConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 服务器监听地址
    pub host: String,
    /// 服务器监听端口
    pub port: u16,
    /// 工作线程数
    pub workers: Option<usize>,
    /// 请求超时时间 (秒)
    pub timeout: u64,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库连接URL
    pub url: String,
    /// 最大连接数
    pub max_connections: u32,
    /// 最小空闲连接数
    pub min_connections: u32,
    /// 连接超时时间 (秒)
    pub connect_timeout: u64,
    /// 空闲超时时间 (秒)
    pub idle_timeout: u64,
}

/// 区块链配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// Ethereum配置
    pub ethereum: EthereumConfig,
    /// 默认确认数要求
    pub default_confirmations: i32,
    /// 交易监听间隔 (秒)
    pub listener_interval: u64,
}

/// Ethereum网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    /// RPC节点URL
    pub rpc_url: String,
    /// WebSocket URL
    pub ws_url: Option<String>,
    /// 链ID
    pub chain_id: u64,
    /// 私钥 (用于签名交易)
    pub private_key: String,
    /// Gas价格限制 (Gwei)
    pub max_gas_price: u64,
    /// Gas限制
    pub gas_limit: u64,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT密钥
    pub jwt_secret: String,
    /// API密钥长度
    pub api_key_length: usize,
    /// HMAC密钥长度
    pub hmac_key_length: usize,
    /// 请求限流配置
    pub rate_limit: RateLimitConfig,
}

/// 请求限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每分钟最大请求数
    pub requests_per_minute: u32,
    /// 突发请求允许数
    pub burst_size: u32,
}

/// Webhook配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔 (秒)
    pub retry_interval: u64,
    /// 请求超时时间 (秒)
    pub timeout: u64,
    /// 并发发送数量
    pub concurrent_sends: usize,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // 加载.env文件，忽略错误

        Ok(Config {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .context("Invalid SERVER_PORT")?,
                workers: env::var("SERVER_WORKERS")
                    .ok()
                    .and_then(|s| s.parse().ok()),
                timeout: env::var("SERVER_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .context("Invalid SERVER_TIMEOUT")?,
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .context("DATABASE_URL environment variable is required")?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .context("Invalid DB_MAX_CONNECTIONS")?,
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .context("Invalid DB_MIN_CONNECTIONS")?,
                connect_timeout: env::var("DB_CONNECT_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .context("Invalid DB_CONNECT_TIMEOUT")?,
                idle_timeout: env::var("DB_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()
                    .context("Invalid DB_IDLE_TIMEOUT")?,
            },
            blockchain: BlockchainConfig {
                ethereum: EthereumConfig {
                    rpc_url: env::var("ETHEREUM_RPC_URL")
                        .context("ETHEREUM_RPC_URL environment variable is required")?,
                    ws_url: env::var("ETHEREUM_WS_URL").ok(),
                    chain_id: env::var("ETHEREUM_CHAIN_ID")
                        .unwrap_or_else(|_| "1".to_string())
                        .parse()
                        .context("Invalid ETHEREUM_CHAIN_ID")?,
                    private_key: env::var("ETHEREUM_PRIVATE_KEY")
                        .context("ETHEREUM_PRIVATE_KEY environment variable is required")?,
                    max_gas_price: env::var("ETHEREUM_MAX_GAS_PRICE")
                        .unwrap_or_else(|_| "100".to_string())
                        .parse()
                        .context("Invalid ETHEREUM_MAX_GAS_PRICE")?,
                    gas_limit: env::var("ETHEREUM_GAS_LIMIT")
                        .unwrap_or_else(|_| "21000".to_string())
                        .parse()
                        .context("Invalid ETHEREUM_GAS_LIMIT")?,
                },
                default_confirmations: env::var("DEFAULT_CONFIRMATIONS")
                    .unwrap_or_else(|_| "12".to_string())
                    .parse()
                    .context("Invalid DEFAULT_CONFIRMATIONS")?,
                listener_interval: env::var("LISTENER_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .context("Invalid LISTENER_INTERVAL")?,
            },
            security: SecurityConfig {
                jwt_secret: env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "default-jwt-secret-change-in-production".to_string()),
                api_key_length: env::var("API_KEY_LENGTH")
                    .unwrap_or_else(|_| "32".to_string())
                    .parse()
                    .context("Invalid API_KEY_LENGTH")?,
                hmac_key_length: env::var("HMAC_KEY_LENGTH")
                    .unwrap_or_else(|_| "64".to_string())
                    .parse()
                    .context("Invalid HMAC_KEY_LENGTH")?,
                rate_limit: RateLimitConfig {
                    requests_per_minute: env::var("RATE_LIMIT_RPM")
                        .unwrap_or_else(|_| "100".to_string())
                        .parse()
                        .context("Invalid RATE_LIMIT_RPM")?,
                    burst_size: env::var("RATE_LIMIT_BURST")
                        .unwrap_or_else(|_| "10".to_string())
                        .parse()
                        .context("Invalid RATE_LIMIT_BURST")?,
                },
            },
            webhook: WebhookConfig {
                max_retries: env::var("WEBHOOK_MAX_RETRIES")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .context("Invalid WEBHOOK_MAX_RETRIES")?,
                retry_interval: env::var("WEBHOOK_RETRY_INTERVAL")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .context("Invalid WEBHOOK_RETRY_INTERVAL")?,
                timeout: env::var("WEBHOOK_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .context("Invalid WEBHOOK_TIMEOUT")?,
                concurrent_sends: env::var("WEBHOOK_CONCURRENT_SENDS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .context("Invalid WEBHOOK_CONCURRENT_SENDS")?,
            },
        })
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证服务器配置
        if self.server.port == 0 {
            anyhow::bail!("Server port cannot be 0");
        }

        // 验证数据库配置
        if self.database.url.is_empty() {
            anyhow::bail!("Database URL cannot be empty");
        }

        // 验证区块链配置
        if self.blockchain.ethereum.rpc_url.is_empty() {
            anyhow::bail!("Ethereum RPC URL cannot be empty");
        }

        if self.blockchain.ethereum.private_key.is_empty() {
            anyhow::bail!("Ethereum private key cannot be empty");
        }

        // 验证安全配置
        if self.security.jwt_secret.len() < 32 {
            anyhow::bail!("JWT secret must be at least 32 characters");
        }

        if self.security.api_key_length < 16 {
            anyhow::bail!("API key length must be at least 16");
        }

        Ok(())
    }

    /// 获取服务器绑定地址
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: None,
                timeout: 30,
            },
            database: DatabaseConfig {
                url: "postgres://wopay:password@localhost/wopay_mvp".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout: 30,
                idle_timeout: 600,
            },
            blockchain: BlockchainConfig {
                ethereum: EthereumConfig {
                    rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
                    ws_url: None,
                    chain_id: 1,
                    private_key: "".to_string(),
                    max_gas_price: 100,
                    gas_limit: 21000,
                },
                default_confirmations: 12,
                listener_interval: 30,
            },
            security: SecurityConfig {
                jwt_secret: "default-jwt-secret-change-in-production".to_string(),
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
        }
    }
}
