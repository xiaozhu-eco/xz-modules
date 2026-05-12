use std::collections::{HashMap, VecDeque};

use crate::types::entity::Entity;
use crate::types::graph::SubGraph;
use crate::types::relation::Relation;

/// BFS neighbor traversal from a center entity up to a given depth.
///
/// Works on in-memory entity/relation sets (already loaded by the caller).
pub fn bfs_neighbors_from_memory(
    center: Entity,
    entities: &HashMap<String, Entity>,
    relations: &[Relation],
    depth: u32,
) -> SubGraph {
    let entity_id = center.id.clone();
    let mut visited_entities: HashMap<String, Entity> = HashMap::new();
    let mut visited_relations: Vec<Relation> = Vec::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();

    visited_entities.insert(entity_id.clone(), center.clone());
    queue.push_back((entity_id.clone(), 0));

    while let Some((current_id, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }

        let neighbors: Vec<&Relation> = relations
            .iter()
            .filter(|r| r.source_id == current_id || r.target_id == current_id)
            .collect();

        for rel in neighbors {
            let neighbor_id = if rel.source_id == current_id {
                &rel.target_id
            } else {
                &rel.source_id
            };

            visited_relations.push(rel.clone());

            if !visited_entities.contains_key(neighbor_id) {
                if let Some(entity) = entities.get(neighbor_id).cloned() {
                    visited_entities.insert(neighbor_id.to_string(), entity);
                    queue.push_back((neighbor_id.to_string(), current_depth + 1));
                }
            }
        }
    }

    let result_entities: Vec<Entity> = visited_entities
        .into_iter()
        .filter(|(id, _)| id != &entity_id)
        .map(|(_, e)| e)
        .collect();

    SubGraph {
        center,
        entities: result_entities,
        relations: visited_relations,
    }
}
