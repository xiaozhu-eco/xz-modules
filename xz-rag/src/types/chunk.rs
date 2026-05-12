use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Document chunk with metadata and optional embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: u32,
    pub content: String,
    pub summary: Option<String>,
    pub metadata: ChunkMetadata,
    pub embedding: Option<Vec<f32>>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

/// Metadata attached to each chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkMetadata {
    pub source: Option<String>,
    pub document_title: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<u64>,
    pub tags: Vec<String>,
    pub namespace: Option<String>,
    pub extra: HashMap<String, String>,
}

impl Chunk {
    pub fn new(id: String, document_id: String, content: String, chunk_index: u32) -> Self {
        Self {
            id,
            document_id,
            chunk_index,
            content,
            summary: None,
            metadata: ChunkMetadata::default(),
            embedding: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            expires_at: None,
        }
    }
}
