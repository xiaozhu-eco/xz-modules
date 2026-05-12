use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

use crate::error::RagError;
use crate::types::config::RagEngineInfo;
use crate::types::rag::{RagRequest, RagResponse, RagStreamEvent};
use crate::types::retrieval::{RetrieveRequest, RetrieveResult};

/// Multi-channel RAG engine trait.
///
/// Core design principles:
/// - Composable retrieval channels (Semantic, BM25, Metadata, etc.)
/// - Pluggable external components (Embedder, VectorStore, Reranker, LLM)
/// - Context window management as engine's core responsibility
/// - Decoupled streaming generation from retrieval
#[async_trait]
pub trait RagEngine: Send + Sync {
    /// Multi-channel retrieval only (no generation).
    async fn retrieve(&self, request: &RetrieveRequest) -> Result<RetrieveResult, RagError>;

    /// End-to-end retrieval + generation.
    async fn retrieve_and_generate(&self, request: &RagRequest) -> Result<RagResponse, RagError>;

    /// Streaming retrieval + generation.
    async fn retrieve_and_generate_stream(
        &self,
        request: &RagRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RagStreamEvent, RagError>> + Send>>, RagError>;

    /// Get engine metadata.
    fn engine_info(&self) -> RagEngineInfo;
}
