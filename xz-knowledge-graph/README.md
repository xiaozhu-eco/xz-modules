# xz-knowledge-graph

Structured knowledge graph storage engine with graph traversal — entities, relations, property queries.

## Overview

- **Entities** — typed nodes (`Person`, `Location`, `Organization`, `Concept`, `Event`, `Custom`, etc.) with attributes, tags, aliases, provenance, and confidence scoring
- **Relations** — directed edges between entities with typed properties, temporal validity windows, and confidence-weighted traversal costs
- **Graph traversal** — BFS neighbor expansion, shortest-path, and bounded all-paths queries
- **Query** — paginated entity search with name/alias/tag/attribute/type filters and sorts
- **Batch import** — transactional bulk upserts with conflict reporting and merge strategies
- **Consistency checking** — detects orphans, circular references, duplicates, expired relations, and conflicting attributes
- **Storage backends** — in-memory (zero-setup) and SQLite (persistent)

## Quick start

```rust
use std::collections::HashMap;
use xz_knowledge_graph::{
    AttributeValue, Confidence, Entity, EntityQuery, EntityType,
    InMemoryKnowledgeGraph, KgConfig, KnowledgeGraph, Relation,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

    // --- Entities ---
    let alice = Entity {
        id: "alice".into(),
        name: "Alice".into(),
        aliases: vec!["Ali".into()],
        entity_type: EntityType::Person,
        attributes: {
            let mut m = HashMap::new();
            m.insert("role".into(), AttributeValue::new("engineer"));
            m
        },
        description: Some("Software engineer".into()),
        created_at: 1715000000,
        updated_at: 1715000000,
        version: 1,
        source: Some("demo".into()),
        tags: vec!["staff".into()],
    };

    let bob = Entity {
        id: "bob".into(),
        name: "Bob".into(),
        aliases: vec![],
        entity_type: EntityType::Person,
        attributes: HashMap::new(),
        description: Some("Designer".into()),
        created_at: 1715000000,
        updated_at: 1715000000,
        version: 1,
        source: Some("demo".into()),
        tags: vec![],
    };

    kg.upsert_entity(alice).await?;
    kg.upsert_entity(bob).await?;

    // --- Relation ---
    let knows = Relation {
        id: "r1".into(),
        source_id: "alice".into(),
        target_id: "bob".into(),
        relation_type: "knows".into(),
        properties: HashMap::new(),
        confidence: Confidence::High,
        provenance: None,
        valid_from: None,
        valid_to: None,
        created_at: 1715000000,
        weight: Some(0.9),
    };
    kg.upsert_relation(knows).await?;

    // --- Search ---
    let page = kg.search_entities(&EntityQuery {
        name_contains: Some("Ali".into()),
        ..Default::default()
    }).await?;
    assert_eq!(page.total, 1);

    // --- Traversal ---
    let neighbors = kg.get_neighbors("alice", 1).await?;
    assert_eq!(neighbors.entities.len(), 1);

    let path = kg.shortest_path("alice", "bob").await?;
    assert!(path.is_some());

    // --- Stats & consistency ---
    let stats = kg.stats().await?;
    println!("{stats:?}");

    let issues = kg.check_consistency().await?;
    for issue in &issues {
        println!("[{}] {}", match issue.severity {
            xz_knowledge_graph::IssueSeverity::Error => "ERROR",
            xz_knowledge_graph::IssueSeverity::Warning => "WARN",
            xz_knowledge_graph::IssueSeverity::Info => "INFO",
        }, issue.description);
    }

    Ok(())
}
```

## API reference

### `KnowledgeGraph` trait

| Method | Signature |
|--------|-----------|
| `upsert_entity` | `async fn(&self, entity: Entity) -> Result<UpsertResult, KgError>` |
| `get_entity` | `async fn(&self, id: &str) -> Result<Option<Entity>, KgError>` |
| `search_entities` | `async fn(&self, query: &EntityQuery) -> Result<EntityPage, KgError>` |
| `delete_entity` | `async fn(&self, id: &str) -> Result<usize, KgError>` |
| `get_entities_batch` | `async fn(&self, ids: &[&str]) -> Result<Vec<Entity>, KgError>` |
| `upsert_relation` | `async fn(&self, relation: Relation) -> Result<UpsertResult, KgError>` |
| `get_relations` | `async fn(&self, entity_id: &str) -> Result<Vec<Relation>, KgError>` |
| `query_relations` | `async fn(&self, query: &RelationQuery) -> Result<Vec<Relation>, KgError>` |
| `delete_relation` | `async fn(&self, id: &str) -> Result<(), KgError>` |
| `get_neighbors` | `async fn(&self, entity_id: &str, depth: u32) -> Result<SubGraph, KgError>` |
| `shortest_path` | `async fn(&self, from: &str, to: &str) -> Result<Option<Vec<PathStep>>, KgError>` |
| `all_paths` | `async fn(&self, from, to, max_depth) -> Result<Vec<Vec<PathStep>>, KgError>` |
| `batch_import` | `async fn(&self, entities, relations) -> Result<ImportResult, KgError>` |
| `check_consistency` | `async fn(&self) -> Result<Vec<ConsistencyIssue>, KgError>` |
| `stats` | `async fn(&self) -> Result<GraphStats, KgError>` |

### Storage backends

```rust
// In-memory (no persistence, ideal for tests and prototypes)
let kg = InMemoryKnowledgeGraph::new(KgConfig::default());

// SQLite (persistent, for production use)
let kg = SqliteKnowledgeGraph::new("./data/kg.db", KgConfig::default()).await?;
```

### Key types

| Type | Description |
|------|-------------|
| `Entity` | Graph node with id, name, type, attributes, tags, and provenance metadata |
| `EntityType` | `Person`, `Location`, `Item`, `Organization`, `Concept`, `Resource`, `Ability`, `Event(String)`, `Custom{category,label}` |
| `Relation` | Directed edge with source/target, type, confidence, weight, and temporal validity |
| `EntityQuery` | Paginated search with name/alias/tag/attribute/type filters and sort options |
| `RelationQuery` | Filter by source, target, type, confidence, or temporal validity |
| `Confidence` | `Speculative` (0.15), `Low` (0.35), `Medium` (0.60), `High` (0.85), `Confirmed` (1.0) |
| `AttributeValue` | Keyed property with confidence and provenance |
| `SubGraph` | Neighborhood result: center entity and connected entities/relations |
| `PathStep` | Single hop in a traversal path |
| `GraphStats` | Counts, degree distribution, orphan stats, DB size |
| `ImportResult` | Batch upsert summary with conflict entries |
| `ConsistencyIssue` | Severity-typed issue with affected entity/relation ids |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)

at your option.
