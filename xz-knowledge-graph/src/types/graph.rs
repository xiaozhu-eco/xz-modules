use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::entity::Entity;
use super::relation::Relation;

/// A subgraph containing entities and relations within it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraph {
    pub center: Entity,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

/// A step in a path between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    pub entity: Entity,
    pub relation: Relation,
}

/// Graph statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub entity_types: HashMap<String, usize>,
    pub relation_types: HashMap<String, usize>,
    pub avg_degree: f64,
    pub max_degree: usize,
    pub orphan_entities: usize,
    pub db_size_bytes: u64,
}
