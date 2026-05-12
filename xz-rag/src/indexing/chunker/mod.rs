pub mod fixed;
pub mod recursive;
pub mod semantic;

/// Chunking strategy trait.
pub trait ChunkStrategy: Send + Sync {
    fn chunk(&self, text: &str) -> Vec<String>;
    fn name(&self) -> &str;
}
