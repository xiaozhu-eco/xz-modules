pub mod graph;
pub mod metadata;
pub mod semantic;

#[cfg(feature = "bm25")]
pub mod bm25;

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub namespace: Option<String>,
}
