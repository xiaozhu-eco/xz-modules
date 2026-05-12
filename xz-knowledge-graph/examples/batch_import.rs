use std::collections::HashMap;

use xz_knowledge_graph::{
    Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph, Relation,
};

fn make_entity(id: &str, name: &str) -> Entity {
    Entity {
        id: id.to_string(),
        name: name.to_string(),
        aliases: vec![],
        entity_type: EntityType::Item,
        attributes: HashMap::new(),
        description: None,
        created_at: 1000,
        updated_at: 2000,
        version: 1,
        source: None,
        tags: vec![],
    }
}

#[tokio::main]
async fn main() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Bulk create 100 entities
    let entities: Vec<Entity> = (0..100)
        .map(|i| make_entity(&format!("e{:03}", i), &format!("Entity-{}", i)))
        .collect();

    let relations = vec![
        Relation {
            id: "r0".to_string(),
            source_id: "e000".to_string(),
            target_id: "e001".to_string(),
            relation_type: "relates_to".to_string(),
            properties: HashMap::new(),
            confidence: xz_knowledge_graph::Confidence::High,
            provenance: None,
            valid_from: None,
            valid_to: None,
            created_at: 1000,
            weight: None,
        },
        Relation {
            id: "r1".to_string(),
            source_id: "e000".to_string(),
            target_id: "e002".to_string(),
            relation_type: "relates_to".to_string(),
            properties: HashMap::new(),
            confidence: xz_knowledge_graph::Confidence::Medium,
            provenance: None,
            valid_from: None,
            valid_to: None,
            created_at: 1000,
            weight: None,
        },
    ];

    let result = kg.batch_import(entities, relations).await.unwrap();
    println!("Import result:");
    println!("  Entities: {} created, {} updated, {} skipped",
        result.entities_created, result.entities_updated, result.entities_skipped);
    println!("  Relations: {} created, {} updated",
        result.relations_created, result.relations_updated);

    // Check stats
    let stats = kg.stats().await.unwrap();
    println!("\nKnowledge graph stats:");
    println!("  Total entities: {}", stats.total_entities);
    println!("  Total relations: {}", stats.total_relations);
    println!("  Entity types: {:?}", stats.entity_types);
    println!("  Avg degree: {:.2}", stats.avg_degree);
}
