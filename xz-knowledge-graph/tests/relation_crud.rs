use std::collections::HashMap;

use xz_knowledge_graph::{
    Confidence, Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph,
    Relation, RelationQuery,
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

fn make_relation(
    id: &str,
    source: &str,
    target: &str,
    rel_type: &str,
) -> Relation {
    Relation {
        id: id.to_string(),
        source_id: source.to_string(),
        target_id: target.to_string(),
        relation_type: rel_type.to_string(),
        properties: HashMap::new(),
        confidence: Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: None,
    }
}

#[tokio::test]
async fn test_relation_crud() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_test_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_test_entity("e2", "Bob", EntityType::Person))
        .await
        .unwrap();

    // Create relation
    let rel = make_relation("r1", "e1", "e2", "knows");
    let result = kg.upsert_relation(rel.clone()).await.unwrap();
    assert!(matches!(
        result,
        xz_knowledge_graph::UpsertResult::Created
    ));

    // Get relations for entity
    let rels = kg.get_relations("e1").await.unwrap();
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].relation_type, "knows");

    // Get relations for the other entity
    let rels2 = kg.get_relations("e2").await.unwrap();
    assert_eq!(rels2.len(), 1);

    // Delete relation
    kg.delete_relation("r1").await.unwrap();
    let rels3 = kg.get_relations("e1").await.unwrap();
    assert_eq!(rels3.len(), 0);
}

#[tokio::test]
async fn test_query_relations() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_test_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_test_entity("e2", "Bob", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_test_entity("e3", "Carol", EntityType::Person))
        .await
        .unwrap();

    kg.upsert_relation(make_relation("r1", "e1", "e2", "knows"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r2", "e1", "e3", "works_with"))
        .await
        .unwrap();

    let query = RelationQuery {
        source_id: Some("e1".to_string()),
        ..Default::default()
    };
    let rels = kg.query_relations(&query).await.unwrap();
    assert_eq!(rels.len(), 2);

    let query2 = RelationQuery {
        relation_type: Some("knows".to_string()),
        ..Default::default()
    };
    let rels2 = kg.query_relations(&query2).await.unwrap();
    assert_eq!(rels2.len(), 1);
}
