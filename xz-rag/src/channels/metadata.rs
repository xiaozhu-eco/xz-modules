use async_trait::async_trait;
use std::sync::Arc;

use crate::error::RagError;
use crate::pipeline::channel::{ChannelConfig, ChannelType};
use crate::types::chunk::ChunkMetadata;
use crate::types::retrieval::{RetrievedChunk, StructuredFilter};

/// Trait for metadata-based search.
/// Returns: Vec of (chunk_id, score, metadata, content, document_id)
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn search_by_metadata(
        &self,
        query: &str,
        filters: &[StructuredFilter],
        top_k: usize,
        namespace: Option<&str>,
    ) -> Result<Vec<(String, f32, ChunkMetadata, String, String)>, RagError>;
}

/// Metadata channel executor using structured filters.
pub struct MetadataChannelExecutor {
    store: Arc<dyn MetadataStore>,
}

impl MetadataChannelExecutor {
    pub fn new(store: Arc<dyn MetadataStore>) -> Self {
        Self { store }
    }

    pub async fn execute(
        &self,
        query: &str,
        config: &ChannelConfig,
        global_filters: &[StructuredFilter],
        namespace: Option<&str>,
    ) -> Result<Vec<RetrievedChunk>, RagError> {
        let results = self
            .store
            .search_by_metadata(query, global_filters, config.top_k, namespace)
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
                channel: ChannelType::Metadata.as_str().to_string(),
                channel_score: score,
                metadata,
                embedding: None,
            })
            .collect();

        Ok(hits)
    }
}
