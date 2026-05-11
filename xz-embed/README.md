# xz-embed

> Text embedding and vector storage abstraction — generate & search embeddings with pluggable backends

## Features

- Pluggable embedding backends — OpenAI (`openai` feature) and mock (for testing)
- `EmbeddingModel` trait for custom model integration
- Vector storage — in-memory (`InMemoryVectorStore`) and SQLite (`SqliteVecStore`, `sqlite-vec` feature)
- Metadata filtering with compound expressions (`Eq`, `Ne`, `In`, `And`, `Or`, `Not`, etc.)
- Concurrent batch embedding with retry and backoff (`ConcurrentBatchManager`)
- Reciprocal Rank Fusion (RRF) for hybrid vector + keyword search
- Vector quantization — scalar and product quantizers
- Batch embedding request/response types
- Index builder with configurable rebuild triggers

## Quick Start

```rust
use xz_embed::{MockEmbedder, EmbeddingModel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mock = MockEmbedder::new(1536, 2048);
    mock.set_output(vec![vec![0.1, 0.2, 0.3, 0.4]]);

    let vectors = mock.embed(&["Hello world"]).await?;
    println!("Dimensions: {}", vectors[0].len());

    // Or use the default embed_single helper
    let single = mock.embed_single("Hello world").await?;
    println!("Single: {}", single.len());

    Ok(())
}
```

### With OpenAI (requires `openai` feature)

```rust
use xz_embed::OpenAiEmbedder;

// From environment variable OPENAI_API_KEY
let embedder = OpenAiEmbedder::from_env()?;
let vectors = embedder.embed(&["Hello world"]).await?;
```

### Vector storage & search

```rust
use std::collections::HashMap;
use xz_embed::{InMemoryVectorStore, MockEmbedder, VectorEntry, VectorStore, StoreLifecycle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryVectorStore::new(4);
    store.initialize().await?;

    let mut mock = MockEmbedder::new(4, 32);
    mock.set_output(vec![vec![0.1, 0.2, 0.3, 0.4]]);

    let vectors = mock.embed(&["Rust programming"]).await?;

    let entry = VectorEntry {
        id: "doc-1".into(),
        vector: vectors[0].clone(),
        metadata: HashMap::from([("lang".into(), "rust".into())]),
        content: Some("Rust programming".into()),
        created_at: 0,
        expires_at: None,
        channel: Some("docs".into()),
    };
    store.insert(entry).await?;

    let query = mock.embed(&["systems programming"]).await?;
    let results = store.search(&query[0], 10).await?;

    for r in &results {
        println!("[{:.4}] {} — {:?}", r.score, r.id, r.content);
    }

    Ok(())
}
```

### Metadata-filtered search

```rust
use xz_embed::MetadataFilter;

let filter = MetadataFilter::and([
    MetadataFilter::in_values("lang", &["rust", "go"]),
    MetadataFilter::ne("status", "archived"),
]);

let results = store.search_with_filter(&query, &filter, 10).await?;
```

### Hybrid search with RRF fusion

```rust
use xz_embed::rrf_fusion;

// fuse vector results and keyword (BM25) results
let fused = rrf_fusion(&vector_results, &keyword_results, 60.0);
```

## Feature Flags

| Feature      | Description                        | Default |
|-------------|------------------------------------|---------|
| `openai`    | OpenAI embedding API via `reqwest` | off     |
| `sqlite-vec`| SQLite-backed vector store         | on      |

## License

MIT OR Apache-2.0
