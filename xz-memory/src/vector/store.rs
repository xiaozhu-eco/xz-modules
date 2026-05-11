//! Vector store wrapper (feature-gated behind `vector-memory`).

#[cfg(feature = "vector-memory")]
use std::sync::Arc;

#[cfg(feature = "vector-memory")]
use crate::error::MemoryError;

/// Vector store configuration and lifecycle.
#[cfg(feature = "vector-memory")]
pub struct VectorStore {
    inner: Arc<dyn xz_embed::traits::VectorStore>,
}

#[cfg(feature = "vector-memory")]
impl VectorStore {
    /// Wrap an existing vector store.
    pub fn new(inner: Arc<dyn xz_embed::traits::VectorStore>) -> Self {
        Self { inner }
    }

    /// Store a vector entry.
    pub async fn store(&self, entry: crate::types::vector::VectorEntry) -> Result<(), MemoryError> {
        self.inner
            .insert(entry)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))
    }

    /// Search vectors by similarity.
    pub async fn search(
        &self,
        query: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<crate::types::vector::SearchResult>, MemoryError> {
        let mut results = self
            .inner
            .search(query, limit)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        results.retain(|r| r.score >= threshold);
        Ok(results)
    }

    /// Delete a vector entry.
    pub async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        self.inner
            .delete(&[id.to_string()])
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;
        Ok(())
    }
}
