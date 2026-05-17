use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::RwLock;

use crate::error::StoreError;
use crate::traits::{StoreLifecycle, VectorStore};
use crate::types::{MetadataFilter, SearchResult, StoreStats, VectorEntry};

/// 内存向量存储（测试用）
#[derive(Debug)]
pub struct InMemoryVectorStore {
    entries: RwLock<Vec<VectorEntry>>,
    dimensions: usize,
    closed: RwLock<bool>,
}

impl InMemoryVectorStore {
    pub fn new(dimensions: usize) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            dimensions,
            closed: RwLock::new(false),
        }
    }

    fn check_closed(&self) -> Result<(), StoreError> {
        if *self.closed.read().map_err(|_| StoreError::Database("closed lock poisoned".into()))? {
            return Err(StoreError::Closed);
        }
        Ok(())
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    fn matches_filter(entry: &VectorEntry, filter: &MetadataFilter) -> bool {
        match filter {
            MetadataFilter::Eq { key, value } => {
                entry.metadata.get(key).map(|v| v == value).unwrap_or(false)
            }
            MetadataFilter::Ne { key, value } => {
                entry.metadata.get(key).map(|v| v != value).unwrap_or(true)
            }
            MetadataFilter::In { key, values } => {
                entry.metadata.get(key).map(|v| values.contains(v)).unwrap_or(false)
            }
            MetadataFilter::NotIn { key, values } => {
                entry.metadata.get(key).map(|v| !values.contains(v)).unwrap_or(true)
            }
            MetadataFilter::Exists { key } => entry.metadata.contains_key(key),
            MetadataFilter::Contains { key, value } => {
                entry.metadata.get(key).map(|v| v.contains(value)).unwrap_or(false)
            }
            MetadataFilter::Range { key, min, max } => {
                if let Some(v) = entry.metadata.get(key) {
                    if let Ok(num) = v.parse::<f64>() {
                        return min.map_or(true, |m| num >= m) && max.map_or(true, |m| num <= m);
                    }
                }
                false
            }
            MetadataFilter::And(filters) => filters.iter().all(|f| Self::matches_filter(entry, f)),
            MetadataFilter::Or(filters) => filters.iter().any(|f| Self::matches_filter(entry, f)),
            MetadataFilter::Not(filter) => !Self::matches_filter(entry, filter),
        }
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn insert(&self, entry: VectorEntry) -> Result<(), StoreError> {
        self.insert_batch(vec![entry]).await
    }

    async fn insert_batch(&self, entries: Vec<VectorEntry>) -> Result<(), StoreError> {
        self.check_closed()?;
        for entry in &entries {
            if entry.vector.len() != self.dimensions {
                return Err(StoreError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: entry.vector.len(),
                });
            }
        }
        self.entries.write().unwrap().extend(entries);
        Ok(())
    }

    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, StoreError> {
        self.check_closed()?;
        if query.len() != self.dimensions {
            return Err(StoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let entries = self.entries.read().unwrap();
        let mut scored: Vec<(SearchResult, f32)> = entries
            .iter()
            .map(|entry| {
                let similarity = Self::cosine_similarity(query, &entry.vector);
                (
                    SearchResult {
                        id: entry.id.clone(),
                        score: similarity,
                        metadata: entry.metadata.clone(),
                        content: entry.content.clone(),
                        channel: entry.channel.clone(),
                    },
                    similarity,
                )
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(r, _)| r).collect())
    }

    async fn search_with_filter(
        &self,
        query: &[f32],
        filter: &MetadataFilter,
        limit: usize,
    ) -> Result<Vec<SearchResult>, StoreError> {
        self.check_closed()?;
        if query.len() != self.dimensions {
            return Err(StoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let entries = self.entries.read().unwrap();
        let mut scored: Vec<(SearchResult, f32)> = entries
            .iter()
            .filter(|entry| Self::matches_filter(entry, filter))
            .map(|entry| {
                let similarity = Self::cosine_similarity(query, &entry.vector);
                (
                    SearchResult {
                        id: entry.id.clone(),
                        score: similarity,
                        metadata: entry.metadata.clone(),
                        content: entry.content.clone(),
                        channel: entry.channel.clone(),
                    },
                    similarity,
                )
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(r, _)| r).collect())
    }

    async fn delete(&self, ids: &[String]) -> Result<usize, StoreError> {
        self.check_closed()?;
        let mut entries = self.entries.write().unwrap();
        let before = entries.len();
        entries.retain(|e| !ids.contains(&e.id));
        Ok(before - entries.len())
    }

    async fn delete_by_filter(&self, filter: &MetadataFilter) -> Result<usize, StoreError> {
        self.check_closed()?;
        let mut entries = self.entries.write().unwrap();
        let before = entries.len();
        entries.retain(|e| !Self::matches_filter(e, filter));
        Ok(before - entries.len())
    }

    async fn clear(&self) -> Result<(), StoreError> {
        self.check_closed()?;
        self.entries.write().unwrap().clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize, StoreError> {
        self.check_closed()?;
        Ok(self.entries.read().unwrap().len())
    }

    async fn rebuild_index(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn stats(&self) -> Result<StoreStats, StoreError> {
        self.check_closed()?;
        let count = self.entries.read().unwrap().len();
        Ok(StoreStats {
            total_vectors: count,
            total_dimensions: self.dimensions,
            index_size_bytes: 0,
            data_size_bytes: 0,
            last_indexed_at: None,
        })
    }
}

#[async_trait]
impl StoreLifecycle for InMemoryVectorStore {
    async fn initialize(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn close(&self) -> Result<(), StoreError> {
        let mut closed = self.closed.write().unwrap();
        *closed = true;
        Ok(())
    }

    async fn checkpoint(&self) -> Result<(), StoreError> {
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, StoreError> {
        Ok(!*self.closed.read().unwrap())
    }
}
