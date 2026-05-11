use std::collections::HashMap;

use xz_knowledge_graph::{
    Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph, Relation,
};

fn make_entity(id: &str, name: &str) -> Entity {
    Entity {
        id: id.to_string(),
        name: name.to_string(),
        aliases: vec![],
        entity_type: EntityType::Concept,
        attributes: HashMap::new(),
        description: None,
        created_at: 1000,
        updated_at: 2000,
        version: 1,
        source: None,
        tags: vec![],
    }
}

fn make_rel(id: &str, src: &str, tgt: &str) -> Relation {
    Relation {
        id: id.to_string(),
        source_id: src.to_string(),
        target_id: tgt.to_string(),
        relation_type: "connects".to_string(),
        properties: HashMap::new(),
        confidence: xz_knowledge_graph::Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: None,
    }
}

#[tokio::main]
async fn main() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Build a diamond-shaped graph: A → B → D, A → C → D
    for (id, name) in &[("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")] {
        kg.upsert_entity(make_entity(id, name)).await.unwrap();
    }

    kg.upsert_relation(make_rel("a-b", "a", "b")).await.unwrap();
    kg.upsert_relation(make_rel("a-c", "a", "c")).await.unwrap();
    kg.upsert_relation(make_rel("b-d", "b", "d")).await.unwrap();
    kg.upsert_relation(make_rel("c-d", "c", "d")).await.unwrap();

    // Shortest path
    let path = kg.shortest_path("a", "d").await.unwrap();
    if let Some(steps) = &path {
        println!("Shortest path ({} steps):", steps.len());
        for (i, step) in steps.iter().enumerate() {
            println!(
                "  {}: {} --[{}]--> {}",
                i + 1,
                step.relation.source_id,
                step.relation.relation_type,
                step.entity.name
            );
        }
    }

    // All paths
    let paths = kg.all_paths("a", "d", 5).await.unwrap();
    println!("\nAll paths from A to D: {}", paths.len());
    for (i, path) in paths.iter().enumerate() {
        let route: Vec<String> = path.iter().map(|s| s.entity.name.clone()).collect();
        println!("  Path {}: {}", i + 1, route.join(" → "));
    }

    // Neighbors at depth 1
    let sub = kg.get_neighbors("a", 1).await.unwrap();
    println!(
        "\nNeighbors of A (depth 1): {:?}",
        sub.entities.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
}
