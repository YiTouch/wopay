// 资金归集服务
// 负责自动扫描和归集分散在各个支付地址的资金

use sqlx::PgPool;
use anyhow::{Result, Context};
use tokio::time::{sleep, Duration};
use crate::services::WalletManager;
use std::sync::Arc;

/// 资金归集服务
pub struct CollectionService {
    wallet_manager: Arc<WalletManager>,
    pool: PgPool,
    collection_interval: Duration,
}

impl CollectionService {
    /// 创建新的归集服务
    pub fn new(
        wallet_manager: Arc<WalletManager>,
        pool: PgPool,
        interval_minutes: u64,
    ) -> Self {
        Self {
            wallet_manager,
            pool,
            collection_interval: Duration::from_secs(interval_minutes * 60),
        }
    }

    /// 启动自动归集任务
    pub async fn start_auto_collection(&self) -> Result<()> {
        log::info!("Starting automatic fund collection service");

        loop {
            if let Err(e) = self.run_collection_cycle().await {
                log::error!("Collection cycle failed: {}", e);
            }

            sleep(self.collection_interval).await;
        }
    }

    /// 执行一次归集周期
    async fn run_collection_cycle(&self) -> Result<()> {
        // 检查是否启用自动归集
        let config = self.get_collection_config().await?;
        if !config.auto_collection_enabled {
            log::debug!("Auto collection is disabled, skipping cycle");
            return Ok(());
        }

        log::info!("Starting collection cycle");

        // 执行资金归集
        let collected_txs = self.wallet_manager.collect_funds().await?;

        if !collected_txs.is_empty() {
            log::info!("Collection cycle completed, {} transactions created", collected_txs.len());
            
            // 记录归集统计
            self.record_collection_stats(collected_txs.len() as i32).await?;
        } else {
            log::debug!("No funds to collect in this cycle");
        }

        Ok(())
    }

    /// 手动触发归集
    pub async fn manual_collection(&self) -> Result<Vec<String>> {
        log::info!("Manual collection triggered");
        let collected_txs = self.wallet_manager.collect_funds().await?;
        
        if !collected_txs.is_empty() {
            self.record_collection_stats(collected_txs.len() as i32).await?;
        }

        Ok(collected_txs)
    }

    /// 获取归集配置
    async fn get_collection_config(&self) -> Result<CollectionConfig> {
        let config = sqlx::query!(
            r#"
            SELECT auto_collection_enabled, collection_threshold, collection_interval_minutes
            FROM wallet_config 
            ORDER BY created_at DESC 
            LIMIT 1
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get collection config")?;

        Ok(CollectionConfig {
            auto_collection_enabled: config.auto_collection_enabled.unwrap_or(true),
            collection_threshold: config.collection_threshold.unwrap_or(rust_decimal::Decimal::new(1, 1)), // 0.1 ETH
            collection_interval_minutes: config.collection_interval_minutes.unwrap_or(60),
        })
    }

    /// 记录归集统计
    async fn record_collection_stats(&self, transaction_count: i32) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO collection_stats (
                id, collection_date, transaction_count, created_at
            )
            VALUES ($1, CURRENT_DATE, $2, NOW())
            ON CONFLICT (collection_date) 
            DO UPDATE SET 
                transaction_count = collection_stats.transaction_count + $2,
                updated_at = NOW()
            "#,
            uuid::Uuid::new_v4(),
            transaction_count
        )
        .execute(&self.pool)
        .await
        .context("Failed to record collection stats")?;

        Ok(())
    }

    /// 获取归集统计信息
    pub async fn get_collection_stats(&self, days: i32) -> Result<Vec<CollectionStat>> {
        let stats = sqlx::query!(
            r#"
            SELECT collection_date, transaction_count
            FROM collection_stats 
            WHERE collection_date >= CURRENT_DATE - INTERVAL '%d days'
            ORDER BY collection_date DESC
            "#,
            days
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get collection stats")?;

        Ok(stats.into_iter().map(|row| CollectionStat {
            date: row.collection_date.unwrap(),
            transaction_count: row.transaction_count.unwrap_or(0),
        }).collect())
    }

    /// 更新归集配置
    pub async fn update_collection_config(&self, config: UpdateCollectionConfig) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE wallet_config SET
                auto_collection_enabled = COALESCE($1, auto_collection_enabled),
                collection_threshold = COALESCE($2, collection_threshold),
                collection_interval_minutes = COALESCE($3, collection_interval_minutes),
                updated_at = NOW()
            "#,
            config.auto_collection_enabled,
            config.collection_threshold,
            config.collection_interval_minutes
        )
        .execute(&self.pool)
        .await
        .context("Failed to update collection config")?;

        log::info!("Collection configuration updated");
        Ok(())
    }
}

/// 归集配置
#[derive(Debug)]
struct CollectionConfig {
    auto_collection_enabled: bool,
    collection_threshold: rust_decimal::Decimal,
    collection_interval_minutes: i32,
}

/// 归集统计
#[derive(Debug, serde::Serialize)]
pub struct CollectionStat {
    pub date: chrono::NaiveDate,
    pub transaction_count: i32,
}

/// 更新归集配置请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateCollectionConfig {
    pub auto_collection_enabled: Option<bool>,
    pub collection_threshold: Option<rust_decimal::Decimal>,
    pub collection_interval_minutes: Option<i32>,
}
