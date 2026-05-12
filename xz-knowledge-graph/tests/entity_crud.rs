use std::collections::HashMap;

use xz_knowledge_graph::{
    Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph,
};

fn make_test_entity(id: &str, name: &str, entity_type: EntityType) -> Entity {
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

#[tokio::test]
async fn test_entity_crud() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Create
    let entity = make_test_entity("e1", "Alice", EntityType::Person);
    let result = kg.upsert_entity(entity.clone()).await.unwrap();
    assert!(matches!(result, xz_knowledge_graph::UpsertResult::Created));

    // Read
    let found = kg.get_entity("e1").await.unwrap().unwrap();
    assert_eq!(found.name, "Alice");

    // Update
    let mut updated = entity.clone();
    updated.name = "Alice Smith".to_string();
    let result = kg.upsert_entity(updated).await.unwrap();
    assert!(matches!(
        result,
        xz_knowledge_graph::UpsertResult::Updated { .. }
    ));

    let found2 = kg.get_entity("e1").await.unwrap().unwrap();
    assert_eq!(found2.name, "Alice Smith");

    // Delete
    let deleted_relations = kg.delete_entity("e1").await.unwrap();
    assert_eq!(deleted_relations, 0);
    assert!(kg.get_entity("e1").await.unwrap().is_none());
}

#[tokio::test]
async fn test_entity_batch() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    for i in 0..5 {
        let entity = make_test_entity(&format!("e{}", i), &format!("Entity{}", i), EntityType::Item);
        kg.upsert_entity(entity).await.unwrap();
    }

    let batch = kg
        .get_entities_batch(&["e0", "e1", "e4", "e99"])
        .await
        .unwrap();
    assert_eq!(batch.len(), 3);
}

#[tokio::test]
async fn test_search_entities() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    for i in 0..10 {
        let entity = make_test_entity(
            &format!("e{}", i),
            &format!("Entity{}", i),
            EntityType::Item,
        );
        kg.upsert_entity(entity).await.unwrap();
    }

    let query = xz_knowledge_graph::EntityQuery {
        name_contains: Some("Entity1".into()),
        ..Default::default()
    };
    let page = kg.search_entities(&query).await.unwrap();
    assert!(page.items.len() >= 1);
}
