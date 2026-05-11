pub mod config;
pub mod consistency;
pub mod error;
pub mod fts;
pub mod store;
pub mod traits;
pub mod traversal;
pub mod types;

// Re-exports
pub use config::KgConfig;
pub use error::KgError;
pub use traits::KnowledgeGraph;
pub use types::attribute::AttributeValue;
pub use types::confidence::Confidence;
pub use types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};
pub use types::entity::{Entity, EntityType};
pub use types::graph::{GraphStats, PathStep, SubGraph};
pub use types::import::{Conflict, ImportResult, MergeStrategy, UpsertResult};
pub use types::provenance::{ExtractionMethod, Provenance};
pub use types::query::{
    AttributeFilter, EntityPage, EntityQuery, EntitySortField, FilterOperator, PageRequest,
    RelationQuery, TagFilter, TagFilterMode,
};
pub use types::relation::{Relation, WeightStrategy};
pub use store::InMemoryKnowledgeGraph;
pub use store::SqliteKnowledgeGraph;
