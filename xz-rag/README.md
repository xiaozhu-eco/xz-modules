# xz-rag

Multi-channel Retrieval-Augmented Generation engine — BM25 + vector + HyDE + query expansion.

Composable retrieval pipeline that fuses results across semantic (vector), BM25 (lexical),
metadata (filtered), and graph (knowledge-graph) channels. Pluggable components for
embedding, vector storage, reranking, LLM generation, and in-memory caching.

## Features

| Feature flag        | Description                                          |
|---------------------|------------------------------------------------------|
| `bm25`              | Enable BM25 (lexical) retrieval channel              |
| `rerank`            | Enable cross-encoder reranking of fused results      |
| `llm-generation`    | Enable LLM-backed answer generation                  |
| `hyde`              | Enable HyDE (Hypothetical Document Embeddings) query preprocessing |
| `query-expansion`   | Enable LLM-based query expansion                     |
| `caching`           | Enable in-memory `moka`-backed retrieval cache       |

No features are enabled by default. Enable what you need:

```toml
[dependencies]
xz-rag = { version = "0.1", features = ["bm25", "rerank", "hyde", "caching"] }
```

## Quick Start

```rust
use xz_rag::{
    ChannelConfig, ChannelPipeline, ContextBuilder,
    DefaultRagEngine, DefaultRagEngineBuilder,
    PromptTemplate, RagEngine, RagRequest, RetrieveRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build the engine
    let engine = DefaultRagEngineBuilder::default()
        .name("my-rag")
        .pipeline(
            ChannelPipeline::new(vec![
                ChannelConfig::semantic(0.5, 10).with_min_score(0.2),
                ChannelConfig::metadata(0.3, 5),
            ])
            .with_rrf_k(60),
        )
        .context_builder(ContextBuilder::new(4096))
        .prompt_template(PromptTemplate::default_qa())
        .build();

    // 2. Retrieve documents across channels
    let retrieve_req = RetrieveRequest::builder("What is Rust?")
        .top_k(10)
        .build();

    let result = engine.retrieve(&retrieve_req).await?;
    println!("Found {} hits in {} ms", result.hits.len(), result.latency_ms);

    // 3. Retrieve + generate an answer
    let rag_req = RagRequest::builder("What is Rust?")
        .retrieve_config(retrieve_req)
        .system_prompt("You are a helpful Rust expert.")
        .build();

    let response = engine.retrieve_and_generate(&rag_req).await?;
    println!("Answer:\n{}", response.answer);
    println!("Citations: {:?}", response.citations);

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                    RagRequest                    │
└─────────────────┬───────────────────────────────┘
                  ▼
┌─────────────────────────────────────────────────┐
│               RagEngine.retrieve()               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ Semantic │ │ Metadata │ │   BM25   │  ...   │
│  │ Channel  │ │ Channel  │ │ Channel  │        │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘        │
│       │             │            │              │
│       ▼             ▼            ▼              │
│  ┌─────────────────────────────────────────┐    │
│  │     Score Normalization + RRF Fusion     │    │
│  └─────────────────┬───────────────────────┘    │
│                    ▼                             │
│  ┌─────────────────────────────────────────┐    │
│  │         Reranker (optional)              │    │
│  └─────────────────┬───────────────────────┘    │
│                    ▼                             │
│  ┌─────────────────────────────────────────┐    │
│  │       ContextBuilder (token budget)      │    │
│  └─────────────────┬───────────────────────┘    │
│                    ▼                             │
│  ┌─────────────────────────────────────────┐    │
│  │       LLM Generation (optional)          │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Channels

| Channel    | Description                                   | Requires                       |
|------------|-----------------------------------------------|--------------------------------|
| Semantic   | Vector similarity search                      | `Embedder` + `SemanticSearch` trait impls |
| BM25       | Lexical (keyword) retrieval                   | `bm25` feature flag            |
| Metadata   | Structured filter-based retrieval             | `MetadataStore` trait impl     |
| Graph      | Knowledge-graph traversal                     | `KnowledgeGraphSearch` trait impl |
| Custom     | Arbitrary channel via `ChannelType::Custom(s)` | User-provided executor         |

### Query Preprocessing

```rust
use xz_rag::types::retrieval::QueryPreprocessing;

// HyDE — generate a hypothetical document embedding
let req = RetrieveRequest::builder("how do lifetimes work?")
    .query_preprocessing(QueryPreprocessing::Hyde)
    .build();

// Query expansion — generate multiple search variants
let req = RetrieveRequest::builder("async runtime comparison")
    .query_preprocessing(QueryPreprocessing::QueryExpansion { count: 3 })
    .build();
```

Requires the `hyde` or `query-expansion` feature flags respectively, plus an `xz-provider` LLM provider wired into the engine.

### Structured Filters

```rust
use xz_rag::types::retrieval::StructuredFilter;

let filter = StructuredFilter::And(
    Box::new(StructuredFilter::MetadataEq {
        key: "category".into(),
        value: "rust".into(),
    }),
    Box::new(StructuredFilter::MetadataExists {
        key: "published_date".into(),
    }),
);
```

### Chunking

```rust
use xz_rag::{FixedSizeChunker, RecursiveCharacterChunker, SemanticChunker, ChunkStrategy};

let chunker = RecursiveCharacterChunker::new(512, 50);
let chunks = chunker.chunk("your document text...")?;

let fixed_chunker = FixedSizeChunker::new(256, 25);
```

### Streaming Generation

When `llm-generation` is enabled and a provider is wired in, use the streaming API:

```rust
use futures::StreamExt;

let rag_req = RagRequest::builder("Explain async/await")
    .system_prompt("You are a Rust expert.")
    .build();

let mut stream = engine.retrieve_and_generate_stream(&rag_req).await?;
while let Some(event) = stream.next().await {
    match event? {
        xz_rag::RagStreamEvent::ContentDelta { delta } => {
            print!("{}", delta);
        }
        xz_rag::RagStreamEvent::Done { citations, .. } => {
            println!("\n\nCitations: {:?}", citations);
        }
        _ => {}
    }
}
```

## Pluggable Traits

The engine delegates all I/O to user-provided trait implementations. Implement these to wire in your stack:

- `Embedder` — convert text to vectors (`semantic.rs:22`)
- `SemanticSearch` — search by embedding (`semantic.rs:12`)
- `MetadataStore` — structured metadata queries (`metadata.rs`)
- `KnowledgeGraphSearch` — graph-traversal retrieval (`graph.rs`)
- `Reranker` (behind `rerank` feature) — cross-encoder rescoring
- `LlmProvider` (behind `llm-generation` / `hyde` / `query-expansion`) — LLM completions

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0))
- MIT license ([LICENSE-MIT](http://opensource.org/licenses/MIT))

at your option.
