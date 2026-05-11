# xz-rerank

Search result re-ranking — local multi-signal fusion + remote Rerank API (Cohere, Jina).

## Features

- **Local re-ranking** — `LocalSignalReranker` fuses five independent scoring signals with configurable weights.
- **Remote providers** — `CohereReranker` and `JinaReranker` behind feature flags (`cohere`, `jina`).
- **Multi-stage pipeline** — `MultiStageReranker<S1, S2>` chains a fast coarse ranker with a precise fine ranker.
- **Pluggable signals** — implement `SignalPlugin` to add custom scoring logic.
- **Score breakdown** — opt-in per-hit signal attribution via `RerankConfig::include_score_breakdown`.
- **Recency decay** — linear or exponential time-decay, with per-channel rules.
- **LRU cache** — `MemoryRerankCache` avoids redundant re-ranking for repeated queries.

### Built-in signals

| Signal | Description |
| --- | --- |
| `KeywordOverlapSignal` | Jaccard similarity between query and candidate tokens |
| `VectorSimilaritySignal` | Cosine similarity against an externally-supplied query embedding |
| `MetadataMatchSignal` | Weighted match against candidate metadata fields |
| `ContentQualitySignal` | Heuristic score based on content length |
| `RecencySignal` | Time-decay score (`NoDecay` / `LinearDecay` / `ExponentialDecay`) |

Default weights: `keyword_overlap=0.30`, `vector_similarity=0.25`, `metadata_match=0.20`, `content_quality=0.10`, `recency=0.15`.

## Feature flags

- **`cohere`** — enable `CohereReranker` (+ `reqwest`).
- **`jina`** — enable `JinaReranker` (+ `reqwest`).

Both are disabled by default.

```toml
[dependencies]
xz-rerank = { version = "0.1", features = ["cohere"] }
```

## Quick start

```rust
use std::collections::HashMap;
use xz_rerank::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reranker = LocalSignalReranker::default();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let candidates = vec![
        RerankCandidate {
            id: "doc1".into(),
            content: "Rust is a systems programming language with zero-cost abstractions.".into(),
            metadata: HashMap::from([("source".into(), "docs".into())]),
            retrieval_score: Some(0.85),
            channel: Some("semantic".into()),
            created_at: Some(now - 3_600_000),
            embedding: None,
        },
        RerankCandidate {
            id: "doc2".into(),
            content: "Python is used for machine learning and data analysis.".into(),
            metadata: HashMap::from([("source".into(), "blog".into())]),
            retrieval_score: Some(0.72),
            channel: Some("semantic".into()),
            created_at: Some(now - 86_400_000),
            embedding: None,
        },
    ];

    let result = reranker
        .rerank(
            "systems programming language",
            candidates,
            &RerankConfig {
                top_k: 5,
                include_score_breakdown: true,
                ..Default::default()
            },
        )
        .await?;

    println!("Results ({}ms):", result.latency_ms);
    for hit in &result.hits {
        println!("  [{:.4}] {} — {}", hit.score, hit.candidate_id, hit.candidate.content);
        if let Some(bd) = &hit.score_breakdown {
            for signal in &bd.signals {
                println!("    {}: raw={:.4} w={:.2} contrib={:.4}",
                    signal.name, signal.raw_score, signal.weight, signal.contribution);
            }
        }
    }

    Ok(())
}
```

## Custom signals

```rust
use async_trait::async_trait;
use xz_rerank::*;

#[derive(Debug)]
struct BoostSignal;

#[async_trait]
impl SignalPlugin for BoostSignal {
    fn name(&self) -> &str { "boost" }

    async fn score(&self, _query: &str, candidate: &RerankCandidate) -> Result<f32, RerankError> {
        Ok(if candidate.metadata.contains_key("pinned") { 1.0 } else { 0.0 })
    }
}

let reranker = LocalSignalReranker::new(SignalWeights {
    boost: 0.10,         // custom weight
    ..Default::default()
})
.with_signal(Box::new(BoostSignal));
```

## Multi-stage reranking

```rust
let coarse = LocalSignalReranker::default();
let fine    = MockReranker::new("fine-ranker");

let pipeline = MultiStageReranker::new(coarse, fine, 50);

let result = pipeline
    .rerank("Rust programming", candidates, &RerankConfig::default())
    .await?;
```

## Remote providers

```rust
#[cfg(feature = "cohere")]
{
    let reranker = CohereReranker::new("your-api-key")?
        .with_model("rerank-english-v3.0");

    let result = reranker
        .rerank("query", candidates, &RerankConfig::default())
        .await?;
}

#[cfg(feature = "jina")]
{
    let reranker = JinaReranker::new("your-api-key")?
        .with_model("jina-reranker-v2-base-multilingual");

    let result = reranker
        .rerank("query", candidates, &RerankConfig::default())
        .await?;
}
```

## Recency decay

```rust
use xz_rerank::{RecencyMode, ChannelRecencyRule};

let reranker = LocalSignalReranker::default()
    .with_recency_mode(RecencyMode::LinearDecay { max_age_days: 30.0 })
    .with_channel_recency(vec![
        ChannelRecencyRule {
            channel: "news".into(),
            mode: RecencyMode::ExponentialDecay { decay_rate: 0.05 },
        },
    ]);
```

## Result cache

```rust
use std::time::Duration;
use xz_rerank::{MemoryRerankCache, RerankCache, RerankCandidate};

let cache = MemoryRerankCache::new(1000);
let ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();

if let Some(cached) = cache.get("my query", &ids).await {
    return Ok(cached);
}

let result = reranker.rerank("my query", candidates, &Default::default()).await?;
cache.set("my query", &ids, &result, Duration::from_secs(300)).await;
```

## License

Licensed under either of [MIT](LICENSE) or [Apache-2.0](LICENSE) at your option.
