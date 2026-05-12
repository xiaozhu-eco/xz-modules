use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::traits::{CacheStats, SearchCache};
use crate::types::{SearchItem, SearchResult};

#[derive(Debug, Clone)]
struct CacheEntry {
    result: SearchResult,
    expires_at: Instant,
    last_accessed: Instant,
}

#[derive(Debug)]
pub struct MemorySearchCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
    hits: RwLock<u64>,
    misses: RwLock<u64>,
}

impl MemorySearchCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            hits: RwLock::new(0),
            misses: RwLock::new(0),
        }
    }

    async fn evict_expired(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.retain(|_, v| v.expires_at > now);
    }

    async fn evict_lru(&self) {
        let mut entries = self.entries.write().await;
        if entries.len() < self.max_entries {
            return;
        }

        let now = Instant::now();
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = now;

        for (key, entry) in entries.iter() {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            entries.remove(&key);
        }
    }
}

#[async_trait]
impl SearchCache for MemorySearchCache {
    async fn get(&self, key: &str) -> Option<SearchResult> {
        self.evict_expired().await;

        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key) {
            if entry.expires_at > Instant::now() {
                entry.last_accessed = Instant::now();
                *self.hits.write().await += 1;
                let mut result = entry.result.clone();
                result.cached = true;
                return Some(result);
            }
        }

        *self.misses.write().await += 1;
        None
    }

    async fn set(&self, key: &str, result: &SearchResult, ttl: Duration) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;

        if entries.contains_key(key) {
            entries.insert(
                key.to_string(),
                CacheEntry {
                    result: result.clone(),
                    expires_at: now + ttl,
                    last_accessed: now,
                },
            );
            return;
        }

        if entries.len() >= self.max_entries {
            drop(entries);
            self.evict_lru().await;
            entries = self.entries.write().await;
        }

        entries.insert(
            key.to_string(),
            CacheEntry {
                result: result.clone(),
                expires_at: now + ttl,
                last_accessed: now,
            },
        );
    }

    async fn invalidate(&self, key: &str) {
        self.entries.write().await.remove(key);
    }

    fn stats(&self) -> CacheStats {
        let (size_bytes, entry_count) = match self.entries.try_read() {
            Ok(entries) => {
                let size_bytes: usize = entries
                    .values()
                    .map(|e| {
                        e.result.query.len()
                            + e.result
                                .items
                                .iter()
                                .map(|i: &SearchItem| i.title.len() + i.url.len() + i.snippet.len())
                                .sum::<usize>()
                    })
                    .sum();
                (size_bytes as u64, entries.len())
            }
            Err(_) => (0, 0),
        };

        CacheStats {
            hits: self.hits.try_read().map(|hits| *hits).unwrap_or(0),
            misses: self.misses.try_read().map(|misses| *misses).unwrap_or(0),
            size_bytes,
            entry_count,
        }
    }
}
