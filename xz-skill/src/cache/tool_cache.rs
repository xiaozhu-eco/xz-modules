use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Tool result cache with TTL — reduces redundant HTTP/WASM calls.
#[derive(Debug)]
pub struct ToolResultCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: serde_json::Value,
    created_at: Instant,
}

impl ToolResultCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Generate a cache key from tool name + hashed args.
    pub fn cache_key(tool_name: &str, args: &serde_json::Value) -> String {
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        args.to_string().hash(&mut hasher);
        format!("tool:{}:{:x}", tool_name, hasher.finish())
    }

    /// Get a cached tool result if not expired.
    pub fn get(&self, tool_name: &str, args: &serde_json::Value) -> Option<serde_json::Value> {
        let key = Self::cache_key(tool_name, args);
        let entries = self.entries.read().unwrap();
        if let Some(entry) = entries.get(&key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Store a tool result in the cache.
    pub fn set(&self, tool_name: &str, args: &serde_json::Value, value: &serde_json::Value) {
        let key = Self::cache_key(tool_name, args);
        let mut entries = self.entries.write().unwrap();
        entries.insert(
            key,
            CacheEntry {
                value: value.clone(),
                created_at: Instant::now(),
            },
        );
    }

    /// Remove all expired entries.
    pub fn evict_expired(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.retain(|_, e| e.created_at.elapsed() < self.ttl);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
