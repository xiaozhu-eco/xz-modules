use std::collections::HashMap;

use xz_knowledge_graph::{
    Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph, Relation,
};

#[tokio::main]
async fn main() {
    let config = KgConfig::default();
    let kg = InMemoryKnowledgeGraph::new(config);

    // Create entities
    let alice = Entity {
        id: "alice".to_string(),
        name: "Alice".to_string(),
        aliases: vec!["Ali".to_string()],
        entity_type: EntityType::Person,
        attributes: HashMap::new(),
        description: Some("A software engineer".to_string()),
        created_at: 1000,
        updated_at: 2000,
        version: 1,
        source: Some("demo".to_string()),
        tags: vec!["engineer".to_string()],
    };

    let bob = Entity {
        id: "bob".to_string(),
        name: "Bob".to_string(),
        aliases: vec![],
        entity_type: EntityType::Person,
        attributes: HashMap::new(),
        description: Some("A designer".to_string()),
        created_at: 1000,
        updated_at: 2000,
        version: 1,
        source: Some("demo".to_string()),
        tags: vec![],
    };

    kg.upsert_entity(alice.clone()).await.unwrap();
    kg.upsert_entity(bob.clone()).await.unwrap();

    // Create relation
    let knows = Relation {
        id: "r1".to_string(),
        source_id: "alice".to_string(),
        target_id: "bob".to_string(),
        relation_type: "knows".to_string(),
        properties: HashMap::new(),
        confidence: xz_knowledge_graph::Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: Some(0.9),
    };
    kg.upsert_relation(knows).await.unwrap();

    // Query
    let found = kg.get_entity("alice").await.unwrap().unwrap();
    println!("Found: {:?}", found.name);

    // Search
    let query = xz_knowledge_graph::EntityQuery {
        name_contains: Some("Ali".to_string()),
        ..Default::default()
    };
    let page = kg.search_entities(&query).await.unwrap();
    println!("Search results: {} (total: {})", page.items.len(), page.total);

    // Neighbors
    let sub = kg.get_neighbors("alice", 1).await.unwrap();
    println!("Neighbors: {:?}", sub.entities.iter().map(|e| &e.name).collect::<Vec<_>>());

    // Stats
    let stats = kg.stats().await.unwrap();
    println!("Stats: {} entities, {} relations", stats.total_entities, stats.total_relations);
}
