pub mod chunker;

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::RagError;
use crate::types::chunk::{Chunk, ChunkMetadata};
use crate::indexing::chunker::ChunkStrategy;

/// Document to be indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDocument {
    pub id: String,
    pub content: String,
    pub title: Option<String>,
    pub metadata: ChunkMetadata,
}

/// Document indexing pipeline.
pub struct DocumentIndexer {
    chunk_strategy: Arc<dyn ChunkStrategy>,
    namespace: String,
}

impl DocumentIndexer {
    pub fn new(chunk_strategy: Arc<dyn ChunkStrategy>, namespace: impl Into<String>) -> Self {
        Self {
            chunk_strategy,
            namespace: namespace.into(),
        }
    }

    /// Index a single document into chunks.
    pub fn index_document(&self, doc: IndexDocument) -> Result<Vec<Chunk>, RagError> {
        let texts = self.chunk_strategy.chunk(&doc.content);
        let mut chunks = Vec::new();

        for (i, text) in texts.into_iter().enumerate() {
            let chunk_id = format!("{}-{}", doc.id, i);
            let mut chunk = Chunk::new(
                chunk_id,
                doc.id.clone(),
                text,
                i as u32,
            );
            chunk.metadata = doc.metadata.clone();
            chunk.metadata.document_title = doc.title.clone();
            chunk.metadata.namespace = Some(self.namespace.clone());
            chunks.push(chunk);
        }

        Ok(chunks)
    }
}
