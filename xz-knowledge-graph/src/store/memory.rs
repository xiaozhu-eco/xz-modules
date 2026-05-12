use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

use crate::config::KgConfig;
use crate::error::KgError;
use crate::traits::KnowledgeGraph;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};
use crate::types::entity::Entity;
use crate::types::graph::{GraphStats, PathStep, SubGraph};
use crate::types::import::{ImportResult, MergeStrategy, UpsertResult};
use crate::types::query::{EntityPage, EntityQuery, RelationQuery};
use crate::types::relation::{Relation, WeightStrategy};

/// In-memory knowledge graph implementation (for testing).
#[derive(Debug)]
pub struct InMemoryKnowledgeGraph {
    entities: RwLock<HashMap<String, Entity>>,
    relations: RwLock<HashMap<String, Relation>>,
    #[allow(dead_code)]
    merge_strategy: MergeStrategy,
    weight_strategy: WeightStrategy,
    max_bfs_depth: u32,
    max_path_search: u32,
}

impl InMemoryKnowledgeGraph {
    pub fn new(config: KgConfig) -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            relations: RwLock::new(HashMap::new()),
            merge_strategy: config.merge_strategy,
            weight_strategy: config.weight_strategy,
            max_bfs_depth: config.max_bfs_depth,
            max_path_search: config.max_path_search,
        }
    }
}

#[async_trait::async_trait]
impl KnowledgeGraph for InMemoryKnowledgeGraph {
    async fn upsert_entity(&self, entity: Entity) -> Result<UpsertResult, KgError> {
        let mut entities = self.entities.write().unwrap();
        if let Some(_existing) = entities.get(&entity.id) {
            entities.insert(entity.id.clone(), entity);
            Ok(UpsertResult::Updated {
                changed_fields: vec!["*".into()],
                conflicts: vec![],
            })
        } else {
            entities.insert(entity.id.clone(), entity);
            Ok(UpsertResult::Created)
        }
    }

    async fn get_entity(&self, id: &str) -> Result<Option<Entity>, KgError> {
        Ok(self.entities.read().unwrap().get(id).cloned())
    }

    async fn search_entities(&self, query: &EntityQuery) -> Result<EntityPage, KgError> {
        let entities = self.entities.read().unwrap();
        let mut items: Vec<Entity> = entities.values().cloned().collect();

        // Filter by name_contains
        if let Some(ref name) = query.name_contains {
            let name = name.to_lowercase();
            items.retain(|e| e.name.to_lowercase().contains(&name));
        }
        // Filter by entity_types
        if let Some(ref types) = query.entity_types {
            if !types.is_empty() {
                items.retain(|e| types.contains(&e.entity_type));
            }
        }
        // Filter by source
        if let Some(ref source) = query.source {
            items.retain(|e| e.source.as_deref() == Some(source.as_str()));
        }

        let total = items.len();
        items.sort_by_key(|e| std::cmp::Reverse(e.updated_at));

        let page = &query.page;
        let has_more = page.offset + page.limit < total;
        let items = items
            .into_iter()
            .skip(page.offset)
            .take(page.limit)
            .collect();

        Ok(EntityPage { items, total, has_more })
    }

    async fn delete_entity(&self, id: &str) -> Result<usize, KgError> {
        let mut entities = self.entities.write().unwrap();
        let mut relations = self.relations.write().unwrap();

        let relation_count = relations
            .values()
            .filter(|r| r.source_id == id || r.target_id == id)
            .count();

        relations.retain(|_, r| r.source_id != id && r.target_id != id);
        entities.remove(id);

        Ok(relation_count)
    }

    async fn get_entities_batch(&self, ids: &[&str]) -> Result<Vec<Entity>, KgError> {
        let entities = self.entities.read().unwrap();
        Ok(ids.iter().filter_map(|id| entities.get(*id).cloned()).collect())
    }

    // === Relation Operations ===

    async fn upsert_relation(&self, relation: Relation) -> Result<UpsertResult, KgError> {
        let mut relations = self.relations.write().unwrap();
        let existed = relations.contains_key(&relation.id);
        relations.insert(relation.id.clone(), relation);
        if existed {
            Ok(UpsertResult::Updated {
                changed_fields: vec!["*".into()],
                conflicts: vec![],
            })
        } else {
            Ok(UpsertResult::Created)
        }
    }

    async fn get_relations(&self, entity_id: &str) -> Result<Vec<Relation>, KgError> {
        let relations = self.relations.read().unwrap();
        Ok(relations
            .values()
            .filter(|r| r.source_id == entity_id || r.target_id == entity_id)
            .cloned()
            .collect())
    }

    async fn query_relations(&self, query: &RelationQuery) -> Result<Vec<Relation>, KgError> {
        let relations = self.relations.read().unwrap();
        let mut items: Vec<Relation> = relations.values().cloned().collect();

        if let Some(ref sid) = query.source_id {
            items.retain(|r| &r.source_id == sid);
        }
        if let Some(ref tid) = query.target_id {
            items.retain(|r| &r.target_id == tid);
        }
        if let Some(ref eid) = query.entity_id {
            items.retain(|r| &r.source_id == eid || &r.target_id == eid);
        }
        if let Some(ref rt) = query.relation_type {
            items.retain(|r| &r.relation_type == rt);
        }

        Ok(items)
    }

    async fn delete_relation(&self, id: &str) -> Result<(), KgError> {
        let mut relations = self.relations.write().unwrap();
        if relations.remove(id).is_none() {
            return Err(KgError::RelationNotFound(id.to_string()));
        }
        Ok(())
    }

    // === Graph Traversal ===

    async fn get_neighbors(&self, entity_id: &str, depth: u32) -> Result<SubGraph, KgError> {
        if depth > self.max_bfs_depth {
            return Err(KgError::MaxDepthExceeded {
                depth,
                max: self.max_bfs_depth,
            });
        }

        let entities = self.entities.read().unwrap();
        let relations = self.relations.read().unwrap();

        let center = entities
            .get(entity_id)
            .cloned()
            .ok_or_else(|| KgError::EntityNotFound(entity_id.to_string()))?;

        let mut visited_entities: HashMap<String, Entity> = HashMap::new();
        let mut visited_relations: Vec<Relation> = Vec::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();

        visited_entities.insert(entity_id.to_string(), center.clone());
        queue.push_back((entity_id.to_string(), 0));

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            let neighbors: Vec<Relation> = relations
                .values()
                .filter(|r| r.source_id == current_id || r.target_id == current_id)
                .cloned()
                .collect();

            for rel in neighbors {
                let neighbor_id = if rel.source_id == current_id {
                    rel.target_id.clone()
                } else {
                    rel.source_id.clone()
                };

                visited_relations.push(rel);

                if !visited_entities.contains_key(&neighbor_id) {
                    if let Some(entity) = entities.get(&neighbor_id).cloned() {
                        visited_entities.insert(neighbor_id.clone(), entity);
                        queue.push_back((neighbor_id, current_depth + 1));
                    }
                }
            }
        }

        let result_entities: Vec<Entity> = visited_entities
            .into_iter()
            .filter(|(id, _)| id != entity_id)
            .map(|(_, e)| e)
            .collect();

        Ok(SubGraph {
            center,
            entities: result_entities,
            relations: visited_relations,
        })
    }

    async fn shortest_path(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Option<Vec<PathStep>>, KgError> {
        if from == to {
            return Ok(Some(vec![]));
        }

        let entities = self.entities.read().unwrap();
        let relations = self.relations.read().unwrap();

        // Build adjacency
        let mut adj: HashMap<String, Vec<(String, Relation)>> = HashMap::new();
        for rel in relations.values() {
            adj.entry(rel.source_id.clone())
                .or_default()
                .push((rel.target_id.clone(), rel.clone()));
            adj.entry(rel.target_id.clone())
                .or_default()
                .push((rel.source_id.clone(), rel.clone()));
        }

        let mut dist: HashMap<String, f32> = HashMap::new();
        let mut prev: HashMap<String, (String, Relation)> = HashMap::new();

        for id in entities.keys() {
            dist.insert(id.clone(), f32::MAX);
        }
        dist.insert(from.to_string(), 0.0);

        let mut queue: Vec<(f32, String)> = vec![(0.0, from.to_string())];

        while let Some((_d, u)) = queue.pop() {
            if let Some(neighbors) = adj.get(&u) {
                for (v, rel) in neighbors {
                    let weight = self.weight_strategy.relation_cost(rel);
                    let alt = dist.get(&u).copied().unwrap_or(f32::MAX) + weight;
                    if alt < dist.get(v).copied().unwrap_or(f32::MAX) {
                        dist.insert(v.clone(), alt);
                        prev.insert(v.clone(), (u.clone(), rel.clone()));
                        queue.push((-alt, v.clone()));
                    }
                }
            }
            queue.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        if !prev.contains_key(to) && from != to {
            return Ok(None);
        }

        let mut path = Vec::new();
        let mut current = to.to_string();
        while current != from {
            if let Some((prev_node, rel)) = prev.get(&current) {
                let entity = entities.get(&current).cloned().unwrap();
                path.push(PathStep { entity, relation: rel.clone() });
                current = prev_node.clone();
            } else {
                break;
            }
        }
        path.reverse();
        Ok(Some(path))
    }

    async fn all_paths(
        &self,
        from: &str,
        to: &str,
        max_depth: u32,
    ) -> Result<Vec<Vec<PathStep>>, KgError> {
        if max_depth > self.max_path_search {
            return Err(KgError::MaxDepthExceeded {
                depth: max_depth,
                max: self.max_path_search,
            });
        }

        let entities = self.entities.read().unwrap();
        let relations = self.relations.read().unwrap();

        let mut adj: HashMap<String, Vec<(String, Relation)>> = HashMap::new();
        for rel in relations.values() {
            adj.entry(rel.source_id.clone())
                .or_default()
                .push((rel.target_id.clone(), rel.clone()));
            adj.entry(rel.target_id.clone())
                .or_default()
                .push((rel.source_id.clone(), rel.clone()));
        }

        let mut all_paths = Vec::new();
        let mut visited = HashSet::new();
        let mut current_path = Vec::new();

        dfs_memory(from, to, max_depth, &entities, &adj, &mut visited, &mut current_path, &mut all_paths);

        all_paths.sort_by(|a, b| {
            let a_cost: f32 = a.iter().map(|step| self.weight_strategy.relation_cost(&step.relation)).sum();
            let b_cost: f32 = b.iter().map(|step| self.weight_strategy.relation_cost(&step.relation)).sum();
            a_cost.partial_cmp(&b_cost).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(all_paths)
    }

    // === Batch Operations ===

    async fn batch_import(
        &self,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
    ) -> Result<ImportResult, KgError> {
        let mut result = ImportResult::default();

        for entity in entities {
            match self.upsert_entity(entity).await? {
                UpsertResult::Created => result.entities_created += 1,
                UpsertResult::Updated { .. } => result.entities_updated += 1,
                UpsertResult::Unchanged => result.entities_skipped += 1,
            }
        }
        for rel in relations {
            match self.upsert_relation(rel).await? {
                UpsertResult::Created => result.relations_created += 1,
                UpsertResult::Updated { .. } => result.relations_updated += 1,
                _ => {}
            }
        }

        Ok(result)
    }

    // === Consistency ===

    async fn check_consistency(&self) -> Result<Vec<ConsistencyIssue>, KgError> {
        let entities = self.entities.read().unwrap();
        let relations = self.relations.read().unwrap();
        let mut issues = Vec::new();

        // Orphan relations
        for rel in relations.values() {
            if !entities.contains_key(&rel.source_id) || !entities.contains_key(&rel.target_id) {
                issues.push(ConsistencyIssue {
                    severity: IssueSeverity::Error,
                    issue_type: ConsistencyIssueType::OrphanRelation,
                    description: format!("Relation {} references a non-existent entity", rel.id),
                    related_entities: vec![rel.source_id.clone(), rel.target_id.clone()],
                    related_relations: vec![rel.id.clone()],
                });
            }
        }

        // Self-referencing
        for rel in relations.values() {
            if rel.source_id == rel.target_id {
                issues.push(ConsistencyIssue {
                    severity: IssueSeverity::Warning,
                    issue_type: ConsistencyIssueType::SelfReferencing,
                    description: format!("Relation {} self-references entity {}", rel.id, rel.source_id),
                    related_entities: vec![rel.source_id.clone()],
                    related_relations: vec![rel.id.clone()],
                });
            }
        }

        // Orphan entities
        for (id, entity) in entities.iter() {
            let has_relation = relations.values().any(|r| r.source_id == *id || r.target_id == *id);
            if !has_relation {
                issues.push(ConsistencyIssue {
                    severity: IssueSeverity::Info,
                    issue_type: ConsistencyIssueType::OrphanEntity,
                    description: format!("Entity {} ({}) has no relations", entity.name, id),
                    related_entities: vec![id.clone()],
                    related_relations: vec![],
                });
            }
        }

        // Expired relations
        let now = current_epoch_ms();
        for rel in relations.values() {
            if let Some(valid_to) = rel.valid_to {
                if valid_to < now {
                    issues.push(ConsistencyIssue {
                        severity: IssueSeverity::Warning,
                        issue_type: ConsistencyIssueType::ExpiredRelation,
                        description: format!("Relation {} has expired (valid_to < now)", rel.id),
                        related_entities: vec![rel.source_id.clone(), rel.target_id.clone()],
                        related_relations: vec![rel.id.clone()],
                    });
                }
            }
        }

        Ok(issues)
    }

    // === Statistics ===

    async fn stats(&self) -> Result<GraphStats, KgError> {
        let entities = self.entities.read().unwrap();
        let relations = self.relations.read().unwrap();

        let mut entity_types: HashMap<String, usize> = HashMap::new();
        for e in entities.values() {
            *entity_types.entry(e.entity_type.as_str()).or_default() += 1;
        }

        let mut relation_types: HashMap<String, usize> = HashMap::new();
        for r in relations.values() {
            *relation_types.entry(r.relation_type.clone()).or_default() += 1;
        }

        let mut degrees: HashMap<String, usize> = HashMap::new();
        for r in relations.values() {
            *degrees.entry(r.source_id.clone()).or_default() += 1;
            *degrees.entry(r.target_id.clone()).or_default() += 1;
        }

        let degree_values: Vec<usize> = degrees.values().copied().collect();
        let avg_degree = if degree_values.is_empty() {
            0.0
        } else {
            degree_values.iter().sum::<usize>() as f64 / degree_values.len() as f64
        };
        let max_degree = degree_values.iter().max().copied().unwrap_or(0);

        let orphan_entities = entities
            .keys()
            .filter(|id| !relations.values().any(|r| &&r.source_id == id || &&r.target_id == id))
            .count();

        Ok(GraphStats {
            total_entities: entities.len(),
            total_relations: relations.len(),
            entity_types,
            relation_types,
            avg_degree,
            max_degree,
            orphan_entities,
            db_size_bytes: 0,
        })
    }
}

fn dfs_memory(
    current: &str,
    target: &str,
    max_depth: u32,
    entities: &HashMap<String, Entity>,
    adj: &HashMap<String, Vec<(String, Relation)>>,
    visited: &mut HashSet<String>,
    current_path: &mut Vec<PathStep>,
    all_paths: &mut Vec<Vec<PathStep>>,
) {
    if current == target {
        all_paths.push(current_path.clone());
        return;
    }
    if current_path.len() >= max_depth as usize {
        return;
    }
    visited.insert(current.to_string());

    if let Some(neighbors) = adj.get(current) {
        for (neighbor, rel) in neighbors {
            if visited.contains(neighbor.as_str()) {
                continue;
            }
            if let Some(entity) = entities.get(neighbor).cloned() {
                current_path.push(PathStep {
                    entity,
                    relation: rel.clone(),
                });
                dfs_memory(neighbor, target, max_depth, entities, adj, visited, current_path, all_paths);
                current_path.pop();
            }
        }
    }

    visited.remove(current);
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
