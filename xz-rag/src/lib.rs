pub mod cache;
pub mod channels;
pub mod context;
pub mod engine;
pub mod error;
pub mod generation;
pub mod indexing;
pub mod pipeline;
pub mod preprocessing;
pub mod traits;
pub mod types;

// Re-exports
pub use engine::{DefaultRagEngine, DefaultRagEngineBuilder};
pub use error::RagError;
pub use traits::RagEngine;
pub use types::chunk::{Chunk, ChunkMetadata};
pub use types::config::{RagConfig, RagEngineInfo};
pub use types::rag::{
    BuiltContext, ChatMessage, ChatRole, Citation, CitationFormat, ContextConfig, PromptTemplate,
    RagGenerationConfig, RagRequest, RagRequestBuilder, RagResponse, RagStreamEvent, RagTokenUsage,
    RequestOptions,
};
pub use types::retrieval::{
    ChannelStats, QueryPreprocessing, RetrieveRequest, RetrieveRequestBuilder, RetrieveResult,
    RetrievedChunk, StructuredFilter,
};

// Pipeline re-exports
pub use pipeline::channel::{ChannelConfig, ChannelPipeline, ChannelType};
pub use pipeline::fusion::RrfFusion;
pub use pipeline::normalize::{MinMaxNormalizer, ZScoreNormalizer};

// Context re-exports
pub use context::citation::format_citations_numeric;
pub use context::token_budget::ContextBuilder;

// Indexing re-exports
pub use indexing::chunker::fixed::FixedSizeChunker;
pub use indexing::chunker::recursive::RecursiveCharacterChunker;
pub use indexing::chunker::semantic::SemanticChunker;
pub use indexing::chunker::ChunkStrategy;
pub use indexing::{DocumentIndexer, IndexDocument};

// Channel re-exports
pub use channels::graph::{GraphChannelExecutor, KnowledgeGraphSearch};
pub use channels::metadata::{MetadataChannelExecutor, MetadataStore};
pub use channels::semantic::{Embedder, SemanticChannelExecutor, SemanticSearch};
