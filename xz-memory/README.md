# xz-memory

Layered memory storage engine — working, short-term, long-term, core memory for AI agents.

## Architecture

```
┌──────────────────────────────────────────┐
│              Short-Term Memory           │
│  Session message window with pagination  │
│  append → retrieve → evict               │
├──────────────────────────────────────────┤
│              Summary Memory              │
│  LLM-generated session summaries         │
│  auto-triggered at message threshold     │
├──────────────────────────────────────────┤
│               Fact Memory                │
│  Structured facts (SPO triples)          │
│  FTS5 full-text search                   │
│  confidence-weighted                     │
├──────────────────────────────────────────┤
│              Vector Memory               │
│  Embedding-based semantic search         │
│  cosine similarity, threshold filtering  │
└──────────────────────────────────────────┘
```

## Features

- **4-layer memory** — Short-term (message window), summary (LLM-compressed), fact (structured triples), vector (semantic search)
- **Session management** — Multi-session message storage with per-session pagination and eviction
- **Auto-summarization** — LLM-driven summary generation triggered at configurable message thresholds
- **Fact recall** — FTS5-backed full-text search over subject/predicate/object triples with confidence scoring
- **Memory compaction** — Merge similar facts, prune low-confidence entries, remove stale data
- **Export/Import** — Full memory snapshot for backup, migration, or cross-agent transfer
- **Dual backend** — SQLite for production, in-memory store for testing
- **Feature-gated** — `summary` (default) adds LLM integration; `vector-memory` adds embedding search

## Quick Start

```rust
use xz_memory::{
    Confidence, Fact, FactCategory, FactRecallOptions, MemoryConfig,
    MemorySystem, Message, Role, SqliteMemory,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SQLite-backed memory
    let config = MemoryConfig::default();
    let memory = SqliteMemory::new("memory.db", config).await?;

    // --- Short-term: append messages to a session ---
    let msg = Message::new(
        uuid::Uuid::new_v4().to_string(),
        "sess_1".into(),
        "user_1".into(),
        Role::User,
        "I love sci-fi novels".into(),
        5,
    );
    memory.append_message("sess_1", msg).await?;

    let msg = Message::new(
        uuid::Uuid::new_v4().to_string(),
        "sess_1".into(),
        "user_1".into(),
        Role::Assistant,
        "Great taste! Any favorite authors?".into(),
        8,
    );
    memory.append_message("sess_1", msg).await?;

    // --- Retrieve recent messages ---
    let recent = memory.get_recent_messages("sess_1", 10).await?;
    for m in &recent {
        println!("[{}] {}", m.role, m.content);
    }

    // --- Fact memory: store structured knowledge ---
    let fact = Fact {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: "user_1".into(),
        category: FactCategory::Preference,
        subject: "user".into(),
        predicate: "likes".into(),
        object: "sci-fi novels".into(),
        confidence: Confidence::High,
        source_session: Some("sess_1".into()),
        created_at: 1000,
        updated_at: 1000,
        version: 1,
    };
    let result = memory.remember_fact(fact).await?;
    println!("Fact stored: {:?}", result);

    // --- Search facts via FTS5 ---
    let results = memory
        .recall_facts("user_1", "sci-fi", &FactRecallOptions::default())
        .await?;
    println!("Found {} facts", results.total);

    // --- Get user preferences ---
    let prefs = memory.get_user_preferences("user_1").await?;
    for p in &prefs {
        println!("  {} → {}", p.predicate, p.object);
    }

    // --- Compact memory ---
    let compaction = memory
        .compact_facts("user_1", xz_memory::CompactionStrategy::MergeSimilar)
        .await?;
    println!("Compacted: merged={}, kept={}", compaction.facts_merged, compaction.facts_kept);

    // --- Evict old messages (keep last 50) ---
    let evicted = memory.evict_oldest_messages("sess_1", 50).await?;
    println!("Evicted {} old messages", evicted);

    // --- Statistics ---
    let stats = memory.stats("user_1").await?;
    println!(
        "Memory: {} sessions, {} messages, {} facts | {} tokens",
        stats.total_sessions, stats.total_messages, stats.total_facts, stats.total_tokens_approx
    );

    // --- Export for backup ---
    let export = memory.export("user_1").await?;
    println!("Exported {} sessions, {} facts", export.sessions.len(), export.facts.len());

    Ok(())
}
```

### Summary generation (requires `summary` feature, enabled by default)

```rust
use xz_memory::{MemorySystem, SqliteMemory, MemoryConfig};

// let provider = /* your xz_provider::LlmProvider impl */;
// let summary = memory.get_or_create_summary("sess_1", provider).await?;
// println!("Session summary: {}", summary.summary);
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `summary` | yes | LLM-driven summary generation via `xz-provider` |
| `vector-memory` | no | Vector storage and cosine-similarity search via `xz-embed` |
| `test-utils` | no | Exposes `InMemoryMemory` for unit testing |

## Configuration

```rust
use xz_memory::MemoryConfig;

let config = MemoryConfig {
    short_term: ShortTermConfig {
        max_messages_per_session: 100,
        message_retention_days: 30,
    },
    compaction: CompactionConfig {
        default_strategy: "merge_similar".into(),
        low_confidence_threshold: 0.3,
        auto_compact_interval_hrs: 24,
    },
    ..Default::default()
};
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
