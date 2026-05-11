//! Layered memory storage engine.
//!
//! Provides four memory layers:
//! - **Short-term**: Session message window with pagination
//! - **Summary**: LLM-generated session summaries (feature: `summary`)
//! - **Fact**: Structured facts with FTS5 full-text search
//! - **Vector**: Embedding-based vector search (feature: `vector-memory`)
//!
//! # Features
//!
//! - `summary` (default): Enables LLM-driven summary generation via `xz-provider`
//! - `vector-memory`: Enables vector storage and search via `xz-embed`
//! - `test-utils`: Exposes in-memory store for unit testing

pub mod config;
pub mod error;
pub mod fts;
pub mod layer;
pub mod store;
pub mod traits;
pub mod types;
pub mod vector;

// Re-exports
pub use config::MemoryConfig;
pub use error::MemoryError;
pub use traits::MemorySystem;
pub use types::fact::{
    CompactionResult, CompactionStrategy, Confidence, Fact, FactCategory, FactPage,
    FactRecallOptions, FactSortField,
};
pub use types::message::{Message, Role};
pub use types::query::{ImportResult, MemoryExport, MemoryStats, MessagePage, PageRequest, UpsertResult};
pub use types::session::{SessionSummary, SessionSnapshot};
pub use types::vector::{SearchResult, VectorEntry};

pub use store::memory::InMemoryMemory;
pub use store::sqlite::SqliteMemory;
