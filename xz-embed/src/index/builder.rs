use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::error::StoreError;
use crate::traits::VectorStore;
use crate::types::StoreStats;

/// 索引构建 throttle 配置
#[derive(Debug, Clone)]
pub struct IndexThrottleConfig {
    /// 重建间隔（冷却期）
    pub cooldown: Duration,
    /// 触发重建的新增条目阈值
    pub count_threshold: usize,
}

impl Default for IndexThrottleConfig {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(60),
            count_threshold: 1000,
        }
    }
}

/// 索引构建器
///
/// 管理向量存储的索引生命周期。
#[derive(Debug)]
pub struct IndexBuilder {
    store: Arc<dyn VectorStore>,
    config: IndexThrottleConfig,
}

impl IndexBuilder {
    pub fn new(store: Arc<dyn VectorStore>, config: IndexThrottleConfig) -> Self {
        Self { store, config }
    }

    /// 重建索引
    pub async fn rebuild(&self) -> Result<(), StoreError> {
        info!(target: "xz_embed", "starting index rebuild");
        self.store.rebuild_index().await?;
        info!(target: "xz_embed", "index rebuild completed");
        Ok(())
    }

    /// 条件重建（仅当条目超过阈值时）
    pub async fn rebuild_if_needed(&self) -> Result<(), StoreError> {
        let count = self.store.count().await?;
        if count >= self.config.count_threshold {
            self.rebuild().await?;
        } else {
            debug!(target: "xz_embed", count, threshold = self.config.count_threshold, "skipping rebuild");
        }
        Ok(())
    }

    /// 后台定期重建任务
    pub async fn start_background_rebuild(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.config.cooldown).await;
                match self.rebuild_if_needed().await {
                    Ok(_) => {}
                    Err(e) => warn!(target: "xz_embed", error = %e, "background rebuild failed"),
                }
            }
        });
    }
}
