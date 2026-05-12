#[cfg(feature = "jina")]
pub mod jina;
pub mod mock;

#[cfg(feature = "jina")]
pub use jina::JinaExtractor;
pub use mock::MockExtractor;

#[cfg(feature = "jina")]
pub use jina::JinaExtractor as Extractor;
