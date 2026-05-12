#[cfg(feature = "caching")]
pub mod memory_cache;

#[cfg(not(feature = "caching"))]
pub mod memory_cache {
    use crate::types::retrieval::RetrieveResult;

    /// No-op cache when caching feature is disabled.
    pub struct NoopCache;

    impl NoopCache {
        pub fn new(_max_entries: usize, _ttl_seconds: u64) -> Self {
            Self
        }

        pub async fn get(&self, _key: &str) -> Option<RetrieveResult> {
            None
        }

        pub async fn set(&self, _key: &str, _value: RetrieveResult) {}

        pub fn invalidate(&self, _namespace: &str) {}
    }
}
