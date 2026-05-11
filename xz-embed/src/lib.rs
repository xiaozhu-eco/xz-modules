pub mod config;
pub mod error;
pub mod traits;
pub mod types;
pub mod batch_manager;
pub mod fusion;

pub mod embedder;
pub mod store;
pub mod index;
pub mod quantize;

// 重导出核心 API
pub use batch_manager::ConcurrentBatchManager;
pub use config::{EmbedConfig, IndexBuildConfig, IndexBuildMode, ModelConfig, RebuildTrigger, RetryConfig, StorageConfig};
pub use error::{EmbedError, StoreError};
pub use fusion::{rrf_fusion, FusionResult};
pub use traits::{DimensionReducer, EmbedModelInfo, EmbedPricing, EmbeddingModel, KeywordSearch, StoreLifecycle, TruncationStrategy, VectorStore};
pub use types::{BatchEmbedRequest, BatchEmbedResponse, MetadataFilter, SearchResult, StoreStats, VectorEntry};

pub use embedder::MockEmbedder;
#[cfg(feature = "openai")]
pub use embedder::OpenAiEmbedder;
pub use store::{InMemoryVectorStore, SqliteVecStore};
pub use index::IndexBuilder;
pub use quantize::{ProductQuantizer, ScalarQuantizer, VectorQuantizer};
