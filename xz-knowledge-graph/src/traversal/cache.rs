use std::collections::HashMap;
use std::sync::RwLock;

use crate::types::entity::Entity;
use crate::types::graph::{PathStep, SubGraph};
use crate::types::relation::Relation;

/// Simple in-memory traversal cache.
#[derive(Debug, Default)]
pub struct TraversalCache {
    neighbors: RwLock<HashMap<String, SubGraph>>,
    paths: RwLock<HashMap<(String, String), Vec<Vec<PathStep>>>>,
    entities: RwLock<HashMap<String, Entity>>,
    relations: RwLock<HashMap<String, Vec<Relation>>>,
}

impl TraversalCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_neighbors(&self, entity_id: &str) -> Option<SubGraph> {
        self.neighbors.read().unwrap().get(entity_id).cloned()
    }

    pub fn put_neighbors(&self, entity_id: String, subgraph: SubGraph) {
        self.neighbors.write().unwrap().insert(entity_id, subgraph);
    }

    pub fn get_path(&self, from: &str, to: &str) -> Option<Vec<Vec<PathStep>>> {
        self.paths.read().unwrap().get(&(from.to_string(), to.to_string())).cloned()
    }

    pub fn put_path(&self, from: String, to: String, paths: Vec<Vec<PathStep>>) {
        self.paths.write().unwrap().insert((from, to), paths);
    }

    pub fn get_entity(&self, id: &str) -> Option<Entity> {
        self.entities.read().unwrap().get(id).cloned()
    }

    pub fn put_entity(&self, id: String, entity: Entity) {
        self.entities.write().unwrap().insert(id, entity);
    }

    pub fn get_relations(&self, entity_id: &str) -> Option<Vec<Relation>> {
        self.relations.read().unwrap().get(entity_id).cloned()
    }

    pub fn put_relations(&self, entity_id: String, relations: Vec<Relation>) {
        self.relations.write().unwrap().insert(entity_id, relations);
    }

    /// Invalidate all cached data.
    pub fn invalidate_all(&self) {
        self.neighbors.write().unwrap().clear();
        self.paths.write().unwrap().clear();
        self.entities.write().unwrap().clear();
        self.relations.write().unwrap().clear();
    }
}
