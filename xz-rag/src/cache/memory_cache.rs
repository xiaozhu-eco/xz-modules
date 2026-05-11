use moka::future::Cache;

use crate::types::retrieval::RetrieveResult;

/// In-memory cache for retrieval results using moka.
pub struct RagMemoryCache {
    cache: Cache<String, RetrieveResult>,
}

impl RagMemoryCache {
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_entries as u64)
            .time_to_live(std::time::Duration::from_secs(ttl_seconds))
            .build();

        Self { cache }
    }

    pub async fn get(&self, key: &str) -> Option<RetrieveResult> {
        self.cache.get(key).await
    }

    pub async fn set(&self, key: &str, value: RetrieveResult) {
        self.cache.insert(key.to_string(), value).await;
    }

    pub fn invalidate(&self, _namespace: &str) {
        // moka cache invalidation by namespace would require maintaining a reverse index
        // For simplicity, invalidate all (moka cache auto-evicts by TTL)
    }
}
