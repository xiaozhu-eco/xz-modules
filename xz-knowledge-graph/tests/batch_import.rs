use std::collections::HashMap;

use xz_knowledge_graph::{
    Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph, Relation,
};

fn make_entity(id: &str, name: &str, entity_type: EntityType) -> Entity {
    Entity {
        id: id.to_string(),
        name: name.to_string(),
        aliases: vec![],
        entity_type,
        attributes: HashMap::new(),
        description: None,
        created_at: 1000,
        updated_at: 2000,
        version: 1,
        source: None,
        tags: vec![],
    }
}

fn make_relation(id: &str, source: &str, target: &str, rel_type: &str) -> Relation {
    Relation {
        id: id.to_string(),
        source_id: source.to_string(),
        target_id: target.to_string(),
        relation_type: rel_type.to_string(),
        properties: HashMap::new(),
        confidence: xz_knowledge_graph::Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: None,
    }
}

#[tokio::test]
async fn test_batch_import() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    let entities: Vec<Entity> = (0..5)
        .map(|i| make_entity(&format!("e{}", i), &format!("Entity{}", i), EntityType::Item))
        .collect();

    let relations = vec![
        make_relation("r0", "e0", "e1", "relates"),
        make_relation("r1", "e0", "e2", "relates"),
    ];

    let result = kg.batch_import(entities, relations).await.unwrap();
    assert_eq!(result.entities_created, 5);
    assert_eq!(result.relations_created, 2);
    assert_eq!(result.entities_updated, 0);
    assert_eq!(result.entities_skipped, 0);
}

#[tokio::test]
async fn test_batch_import_with_updates() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // First import
    let entities1: Vec<Entity> = vec![make_entity("e1", "Alice", EntityType::Person)];
    kg.batch_import(entities1, vec![]).await.unwrap();

    // Second import - update same entity
    let entities2: Vec<Entity> = vec![{
        let mut e = make_entity("e1", "Alice Updated", EntityType::Person);
        e.updated_at = 3000;
        e
    }];
    let result = kg.batch_import(entities2, vec![]).await.unwrap();
    assert_eq!(result.entities_updated, 1);
    assert_eq!(result.entities_created, 0);

    let found = kg.get_entity("e1").await.unwrap().unwrap();
    assert_eq!(found.name, "Alice Updated");
}
