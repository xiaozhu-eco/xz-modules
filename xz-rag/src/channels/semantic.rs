use async_trait::async_trait;
use std::sync::Arc;

use crate::error::RagError;
use crate::pipeline::channel::{ChannelConfig, ChannelType};
use crate::types::chunk::ChunkMetadata;
use crate::types::retrieval::{RetrievedChunk, StructuredFilter};

/// Trait for semantic vector search.
/// Returns: Vec of (chunk_id, score, metadata, content, document_id)
#[async_trait]
pub trait SemanticSearch: Send + Sync {
    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        namespace: Option<&str>,
    ) -> Result<Vec<(String, f32, ChunkMetadata, String, String)>, RagError>;
}

/// Trait for text embedding.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &[String]) -> Result<Vec<Vec<f32>>, RagError>;
    fn dimensions(&self) -> usize;
}

/// Semantic channel executor using vector similarity search.
pub struct SemanticChannelExecutor {
    embedder: Arc<dyn Embedder>,
    store: Arc<dyn SemanticSearch>,
}

impl SemanticChannelExecutor {
    pub fn new(embedder: Arc<dyn Embedder>, store: Arc<dyn SemanticSearch>) -> Self {
        Self { embedder, store }
    }

    pub async fn execute(
        &self,
        query: &str,
        config: &ChannelConfig,
        _global_filters: &[StructuredFilter],
        namespace: Option<&str>,
    ) -> Result<Vec<RetrievedChunk>, RagError> {
        let embeddings = self
            .embedder
            .embed(&[query.to_string()])
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        let query_embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| RagError::Embedding("no embedding returned".into()))?;

        let results = self
            .store
            .search(&query_embedding, config.top_k, namespace)
            .await
            .map_err(|e| RagError::Store(e.to_string()))?;

        let hits: Vec<RetrievedChunk> = results
            .into_iter()
            .filter(|(_, score, _, _, _)| {
                if let Some(min_score) = config.min_score {
                    *score >= min_score
                } else {
                    true
                }
            })
            .map(|(id, score, metadata, content, document_id)| RetrievedChunk {
                chunk_id: id.clone(),
                document_id,
                content,
                score,
                channel: ChannelType::Semantic.as_str().to_string(),
                channel_score: score,
                metadata,
                embedding: None,
            })
            .collect();

        Ok(hits)
    }
}
