#[cfg(feature = "vector-memory")]
pub use xz_embed::types::{SearchResult, VectorEntry};

/// Wrapper for feature-disabled builds.
#[cfg(not(feature = "vector-memory"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: std::collections::HashMap<String, String>,
    pub content: Option<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub channel: Option<String>,
}

#[cfg(not(feature = "vector-memory"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: std::collections::HashMap<String, String>,
    pub content: Option<String>,
    pub channel: Option<String>,
}
