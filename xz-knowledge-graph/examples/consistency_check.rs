use std::collections::HashMap;

use xz_knowledge_graph::{
    ConsistencyIssueType, Entity, EntityType, InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph,
    Relation,
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

#[tokio::main]
async fn main() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Create entities and relations including issues
    kg.upsert_entity(make_entity("e1", "Valid1")).await.unwrap();
    kg.upsert_entity(make_entity("e2", "Valid2")).await.unwrap();
    kg.upsert_entity(make_entity("orphan", "Orphan")).await.unwrap();

    // Valid relation
    kg.upsert_relation(Relation {
        id: "valid_rel".to_string(),
        source_id: "e1".to_string(),
        target_id: "e2".to_string(),
        relation_type: "valid".to_string(),
        properties: HashMap::new(),
        confidence: xz_knowledge_graph::Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: None,
    })
    .await
    .unwrap();

    // Self-referencing relation
    kg.upsert_relation(Relation {
        id: "self_ref".to_string(),
        source_id: "e1".to_string(),
        target_id: "e1".to_string(),
        relation_type: "self".to_string(),
        properties: HashMap::new(),
        confidence: xz_knowledge_graph::Confidence::Low,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1000,
        weight: None,
    })
    .await
    .unwrap();

    let issues = kg.check_consistency().await.unwrap();
    println!("Found {} consistency issues:\n", issues.len());

    for issue in &issues {
        let severity = match issue.severity {
            xz_knowledge_graph::IssueSeverity::Error => "ERROR",
            xz_knowledge_graph::IssueSeverity::Warning => "WARNING",
            xz_knowledge_graph::IssueSeverity::Info => "INFO",
        };
        let issue_type = match &issue.issue_type {
            ConsistencyIssueType::OrphanRelation => "OrphanRelation",
            ConsistencyIssueType::SelfReferencing => "SelfReferencing",
            ConsistencyIssueType::CircularReference => "CircularReference",
            ConsistencyIssueType::DuplicateEntity => "DuplicateEntity",
            ConsistencyIssueType::ConflictingAttribute => "ConflictingAttribute",
            ConsistencyIssueType::ExpiredRelation => "ExpiredRelation",
            ConsistencyIssueType::OrphanEntity => "OrphanEntity",
        };
        println!("  [{}] {}: {}", severity, issue_type, issue.description);
    }
}
