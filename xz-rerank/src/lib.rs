pub mod cache;
pub mod channel;
pub mod error;
pub mod local;
pub mod multi_stage;
pub mod providers;
pub mod signals;
pub mod traits;
pub mod types;

// 重导出核心 API
pub use cache::{MemoryRerankCache, RerankCache};
pub use channel::ChannelRecencyRule;
pub use error::RerankError;
pub use local::LocalSignalReranker;
pub use multi_stage::MultiStageReranker;
#[cfg(feature = "cohere")]
pub use providers::CohereReranker;
#[cfg(feature = "jina")]
pub use providers::JinaReranker;
pub use providers::MockReranker;
pub use signals::{
    ContentQualitySignal, KeywordOverlapSignal, MetadataMatchSignal, RecencySignal,
    VectorSimilaritySignal,
};
pub use traits::{Reranker, RerankerBackendType, RerankerInfo, RerankerPricing, SignalPlugin};
pub use types::{
    RecencyMode, RerankCandidate, RerankConfig,
    RerankHit, RerankResult, RerankStats, ScoreBreakdown, SignalScore, SignalWeights,
};
