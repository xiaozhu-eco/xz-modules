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
async fn test_consistency_no_issues() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_entity("e2", "Bob", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r1", "e1", "e2", "knows"))
        .await
        .unwrap();

    let issues = kg.check_consistency().await.unwrap();
    // No orphan relations or entities
    assert!(issues.is_empty());
}

#[tokio::test]
async fn test_orphan_entity() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_entity("e2", "Bob", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r1", "e1", "e2", "knows"))
        .await
        .unwrap();
    // e3 has no relations
    kg.upsert_entity(make_entity("e3", "Carol", EntityType::Person))
        .await
        .unwrap();

    let issues = kg.check_consistency().await.unwrap();
    let orphan_issues: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.issue_type, xz_knowledge_graph::ConsistencyIssueType::OrphanEntity))
        .collect();
    assert!(!orphan_issues.is_empty());
}

#[tokio::test]
async fn test_self_referencing_relation() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r1", "e1", "e1", "self"))
        .await
        .unwrap();

    let issues = kg.check_consistency().await.unwrap();
    let self_ref_issues: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.issue_type, xz_knowledge_graph::ConsistencyIssueType::SelfReferencing))
        .collect();
    assert!(!self_ref_issues.is_empty());
}
