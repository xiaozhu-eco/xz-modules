pub mod batch;
pub mod cache;
pub mod engines;
pub mod error;
pub mod extractors;
pub mod feedback;
pub mod near_dup;
pub mod rate_limiter;
pub mod rewrite;
pub mod router;
pub mod traits;
pub mod types;

// 重导出核心 API
pub use batch::{batch_search, batch_search_with_arc};
pub use cache::MemorySearchCache;
pub use engines::MockSearchEngine;
#[cfg(feature = "serpapi")]
pub use engines::SerpApiEngine;
#[cfg(feature = "tavily")]
pub use engines::TavilyEngine;
pub use error::SearchError;
#[cfg(feature = "jina")]
pub use extractors::JinaExtractor;
pub use extractors::MockExtractor;
pub use feedback::{MemorySearchFeedback, SearchFeedback};
pub use near_dup::NearDuplicateDetector;
pub use rate_limiter::SearchRateLimiter;
pub use rewrite::QueryRewriter;
pub use router::{DedupStrategy, SearchRouter};
pub use traits::{CacheStats, ContentExtractor, ExtractorInfo, SearchCache, SearchEngine, SearchEngineInfo, SearchPricing};
pub use types::{ExtractedContent, SafeSearchLevel, SearchConfig, SearchItem, SearchOptions, SearchResult, TimeRange};
