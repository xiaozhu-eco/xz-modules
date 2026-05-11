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
async fn test_graph_neighbors() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Build a small graph: Alice -knows-> Bob -knows-> Carol
    kg.upsert_entity(make_entity("e1", "Alice", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_entity("e2", "Bob", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_entity("e3", "Carol", EntityType::Person))
        .await
        .unwrap();

    kg.upsert_relation(make_relation("r1", "e1", "e2", "knows"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r2", "e2", "e3", "knows"))
        .await
        .unwrap();

    // Depth 1 from Alice
    let sub = kg.get_neighbors("e1", 1).await.unwrap();
    assert_eq!(sub.entities.len(), 1); // Bob
    assert_eq!(sub.relations.len(), 1);

    // Depth 2 from Alice
    let sub2 = kg.get_neighbors("e1", 2).await.unwrap();
    assert_eq!(sub2.entities.len(), 2); // Bob and Carol
}

#[tokio::test]
async fn test_shortest_path() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Chain: A - B - C - D
    for (id, name) in &[("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")] {
        kg.upsert_entity(make_entity(id, name, EntityType::Person))
            .await
            .unwrap();
    }

    kg.upsert_relation(make_relation("r1", "a", "b", "connects"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r2", "b", "c", "connects"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r3", "c", "d", "connects"))
        .await
        .unwrap();

    let path = kg.shortest_path("a", "d").await.unwrap().unwrap();
    assert_eq!(path.len(), 3); // a->b, b->c, c->d
}

#[tokio::test]
async fn test_shortest_path_same_node() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());
    kg.upsert_entity(make_entity("a", "A", EntityType::Person))
        .await
        .unwrap();

    let path = kg.shortest_path("a", "a").await.unwrap().unwrap();
    assert!(path.is_empty());
}

#[tokio::test]
async fn test_shortest_path_not_found() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    kg.upsert_entity(make_entity("a", "A", EntityType::Person))
        .await
        .unwrap();
    kg.upsert_entity(make_entity("b", "B", EntityType::Person))
        .await
        .unwrap();

    let path = kg.shortest_path("a", "b").await.unwrap();
    assert!(path.is_none());
}

#[tokio::test]
async fn test_all_paths() {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // Diamond: A -> B -> D, A -> C -> D
    for (id, name) in &[("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")] {
        kg.upsert_entity(make_entity(id, name, EntityType::Person))
            .await
            .unwrap();
    }

    kg.upsert_relation(make_relation("r1", "a", "b", "to"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r2", "a", "c", "to"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r3", "b", "d", "to"))
        .await
        .unwrap();
    kg.upsert_relation(make_relation("r4", "c", "d", "to"))
        .await
        .unwrap();

    let paths = kg.all_paths("a", "d", 5).await.unwrap();
    assert_eq!(paths.len(), 2); // A-B-D and A-C-D
}

#[tokio::test]
async fn test_max_bfs_depth() {
    let mut config = KgConfig::default();
    config.max_bfs_depth = 2;
    let kg = InMemoryKnowledgeGraph::new(config);

    kg.upsert_entity(make_entity("a", "A", EntityType::Person))
        .await
        .unwrap();

    let result = kg.get_neighbors("a", 5).await;
    assert!(result.is_err());
}
