use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::types::RerankResult;

/// Rerank 结果缓存
#[async_trait]
pub trait RerankCache: Send + Sync + Debug {
    /// 获取缓存结果
    async fn get(&self, query: &str, candidate_ids: &[String]) -> Option<RerankResult>;

    /// 存储缓存结果
    async fn set(
        &self,
        query: &str,
        candidate_ids: &[String],
        result: &RerankResult,
        ttl: Duration,
    );
}

/// LRU 内存缓存实现
#[derive(Debug)]
pub struct MemoryRerankCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    result: RerankResult,
    expires_at: Instant,
    last_accessed: Instant,
}

impl MemoryRerankCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    fn make_key(query: &str, candidate_ids: &[String]) -> String {
        let ids_hash = candidate_ids.join(",");
        format!("rerank:{query}:{ids_hash}")
    }

    async fn evict_if_needed(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.retain(|_, e| e.expires_at > now);

        if entries.len() >= self.max_entries {
            let lru_key = entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone());

            if let Some(key) = lru_key {
                entries.remove(&key);
            }
        }
    }
}

#[async_trait]
impl RerankCache for MemoryRerankCache {
    async fn get(&self, query: &str, candidate_ids: &[String]) -> Option<RerankResult> {
        self.evict_if_needed().await;
        let key = Self::make_key(query, candidate_ids);
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&key) {
            if entry.expires_at > Instant::now() {
                entry.last_accessed = Instant::now();
                return Some(entry.result.clone());
            }
        }
        None
    }

    async fn set(
        &self,
        query: &str,
        candidate_ids: &[String],
        result: &RerankResult,
        ttl: Duration,
    ) {
        self.evict_if_needed().await;
        let key = Self::make_key(query, candidate_ids);
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        entries.insert(
            key,
            CacheEntry {
                result: result.clone(),
                expires_at: now + ttl,
                last_accessed: now,
            },
        );
    }
}
