use async_trait::async_trait;
use std::sync::Arc;

use crate::error::RagError;
use crate::pipeline::channel::{ChannelConfig, ChannelType};
use crate::types::chunk::ChunkMetadata;
use crate::types::retrieval::{RetrievedChunk, StructuredFilter};

/// Trait for knowledge-graph based search.
/// Returns: Vec of (chunk_id, score, metadata, content, document_id)
#[async_trait]
pub trait KnowledgeGraphSearch: Send + Sync {
    async fn search(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<(String, f32, ChunkMetadata, String, String)>, RagError>;
}

/// Graph channel executor using a knowledge graph search backend.
pub struct GraphChannelExecutor {
    store: Arc<dyn KnowledgeGraphSearch>,
}

impl GraphChannelExecutor {
    pub fn new(store: Arc<dyn KnowledgeGraphSearch>) -> Self {
        Self { store }
    }

    pub async fn execute(
        &self,
        query: &str,
        config: &ChannelConfig,
        _global_filters: &[StructuredFilter],
        _namespace: Option<&str>,
    ) -> Result<Vec<RetrievedChunk>, RagError> {
        let results = self
            .store
            .search(query, config.top_k)
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
                chunk_id: id,
                document_id,
                content,
                score,
                channel: ChannelType::Graph.as_str().to_string(),
                channel_score: score,
                metadata,
                embedding: None,
            })
            .collect();

        Ok(hits)
    }
}
