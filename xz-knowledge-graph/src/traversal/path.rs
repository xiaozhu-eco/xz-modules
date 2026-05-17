use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::types::entity::Entity;
use crate::types::graph::PathStep;
use crate::types::relation::{Relation, WeightStrategy};

/// Priority queue entry for Dijkstra. Uses Reverse ordering so BinaryHeap behaves as min-heap.
#[derive(PartialEq)]
struct PathCost(f32, String);

impl Eq for PathCost {}

impl PartialOrd for PathCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for PathCost {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Build adjacency map from a slice of relations.
pub fn build_adjacency(
    relations: &[Relation],
) -> HashMap<String, Vec<(String, Relation)>> {
    let mut adj: HashMap<String, Vec<(String, Relation)>> = HashMap::new();
    for rel in relations {
        adj.entry(rel.source_id.clone())
            .or_default()
            .push((rel.target_id.clone(), rel.clone()));
        adj.entry(rel.target_id.clone())
            .or_default()
            .push((rel.source_id.clone(), rel.clone()));
    }
    adj
}

/// Dijkstra shortest path between two entities using in-memory data.
pub fn dijkstra_shortest_path(
    from: &str,
    to: &str,
    entities: &HashMap<String, Entity>,
    adj: &HashMap<String, Vec<(String, Relation)>>,
    weight_strategy: WeightStrategy,
) -> Option<Vec<PathStep>> {
    if from == to {
        return Some(vec![]);
    }

    let mut dist: HashMap<String, f32> = HashMap::new();
    let mut prev: HashMap<String, (String, Relation)> = HashMap::new();

    for id in entities.keys() {
        dist.insert(id.clone(), f32::MAX);
    }
    dist.insert(from.to_string(), 0.0);

    let mut queue: BinaryHeap<PathCost> = BinaryHeap::new();
    queue.push(PathCost(0.0, from.to_string()));

    while let Some(PathCost(_d, u)) = queue.pop() {
        if let Some(neighbors) = adj.get(&u) {
            for (v, rel) in neighbors {
                let weight = weight_strategy.relation_cost(rel);
                let alt = dist.get(&u).copied().unwrap_or(f32::MAX) + weight;
                if alt < dist.get(v).copied().unwrap_or(f32::MAX) {
                    dist.insert(v.clone(), alt);
                    prev.insert(v.clone(), (u.clone(), rel.clone()));
                    queue.push(PathCost(alt, v.clone()));
                }
            }
        }
    }

    if !prev.contains_key(to) && from != to {
        return None;
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
    Some(path)
}

/// DFS all-paths between two entities using in-memory data.
pub fn dfs_all_paths(
    from: &str,
    to: &str,
    max_depth: u32,
    entities: &HashMap<String, Entity>,
    adj: &HashMap<String, Vec<(String, Relation)>>,
) -> Vec<Vec<PathStep>> {
    let mut all_paths = Vec::new();
    let mut visited = HashSet::new();
    let mut current_path = Vec::new();

    dfs_inner(
        from, to, max_depth, entities, adj, &mut visited,
        &mut current_path, &mut all_paths,
    );

    all_paths
}

fn dfs_inner(
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
                dfs_inner(
                    neighbor, target, max_depth, entities, adj,
                    visited, current_path, all_paths,
                );
                current_path.pop();
            }
        }
    }

    visited.remove(current);
}
