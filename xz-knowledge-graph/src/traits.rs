use async_trait::async_trait;
use std::fmt::Debug;

use crate::error::KgError;
use crate::types::consistency::ConsistencyIssue;
use crate::types::entity::Entity;
use crate::types::graph::{GraphStats, PathStep, SubGraph};
use crate::types::import::{ImportResult, UpsertResult};
use crate::types::query::{EntityPage, EntityQuery, RelationQuery};
use crate::types::relation::Relation;

/// Knowledge graph core interface.
///
/// Design principles:
/// - Upsert semantics by ID with configurable merge strategy
/// - Graph traversal as first-class citizen (BFS neighbors, shortest path)
/// - Batch import within transactions with detailed statistics
/// - Consistency checking as independent concern
/// - delete_entity cascades to related relations
#[async_trait]
pub trait KnowledgeGraph: Send + Sync + Debug {
    // === Entity Operations ===

    async fn upsert_entity(&self, entity: Entity) -> Result<UpsertResult, KgError>;
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>, KgError>;
    async fn search_entities(&self, query: &EntityQuery) -> Result<EntityPage, KgError>;
    async fn delete_entity(&self, id: &str) -> Result<usize, KgError>;
    async fn get_entities_batch(&self, ids: &[&str]) -> Result<Vec<Entity>, KgError>;

    // === Relation Operations ===

    async fn upsert_relation(&self, relation: Relation) -> Result<UpsertResult, KgError>;
    async fn get_relations(&self, entity_id: &str) -> Result<Vec<Relation>, KgError>;
    async fn query_relations(&self, query: &RelationQuery) -> Result<Vec<Relation>, KgError>;
    async fn delete_relation(&self, id: &str) -> Result<(), KgError>;

    // === Graph Traversal ===

    async fn get_neighbors(&self, entity_id: &str, depth: u32) -> Result<SubGraph, KgError>;
    async fn shortest_path(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Option<Vec<PathStep>>, KgError>;
    async fn all_paths(
        &self,
        from: &str,
        to: &str,
        max_depth: u32,
    ) -> Result<Vec<Vec<PathStep>>, KgError>;

    // === Batch Operations ===

    async fn batch_import(
        &self,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
    ) -> Result<ImportResult, KgError>;

    // === Consistency ===

    async fn check_consistency(&self) -> Result<Vec<ConsistencyIssue>, KgError>;

    // === Statistics ===

    async fn stats(&self) -> Result<GraphStats, KgError>;
}
