# xz-search

External search abstraction — multi-engine aggregation + content extraction.

## Overview

`xz-search` provides a unified async interface over multiple web search backends (Tavily, SerpAPI) and content extraction services (Jina). It handles engine routing, result merging, deduplication, caching, rate limiting, query rewriting, and feedback collection — all behind a single `SearchRouter`.

## Features

- **Multi-engine aggregation** — query multiple search backends in parallel, merge and rank results
- **Content extraction** — fetch and extract clean markdown from result URLs (Jina Reader)
- **Result caching** — in-memory cache with TTL and LRU eviction
- **Deduplication** — exact URL dedup + near-duplicate detection via MinHash/LSH
- **Rate limiting** — token-bucket rate limiter per engine (QPS + daily quota)
- **Batch search** — concurrent bulk query processing with semaphore-controlled parallelism
- **Query rewriting** — heuristic keyword extraction, multi-perspective expansion, decomposition
- **Feedback collection** — record user clicks/irrelevance to influence future ranking

## Feature flags

| Flag      | Enables                 |
|-----------|-------------------------|
| `tavily`  | `TavilyEngine` backend  |
| `serpapi` | `SerpApiEngine` backend |
| `jina`    | `JinaExtractor`         |

All three pull in `reqwest` as a dependency. By default, no flags are enabled.

## Quick start

```rust
use xz_search::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a router with a mock engine (no API key needed)
    let router = SearchRouter::new();

    let config = SearchConfig {
        max_results: 5,
        engines: vec![],
        ..Default::default()
    };

    let result = router
        .aggregated_search("Rust async patterns", &config, &SearchOptions::default())
        .await?;

    println!("Found {} results in {}ms", result.total_results, result.latency_ms);
    for item in &result.items {
        println!("  [{:.2}] {} — {}", item.score, item.title, item.url);
    }
    Ok(())
}
```

If you omit `engines` (empty vec), the router uses all registered engines. Register real backends with feature flags:

```toml
[dependencies]
xz-search = { features = ["tavily", "jina"] }
```

```rust
use xz_search::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = SearchRouter::new();

    #[cfg(feature = "tavily")]
    router.register_engine(Box::new(TavilyEngine::new("your-tavily-api-key")));

    #[cfg(feature = "jina")]
    let router = router.with_extractor(Box::new(JinaExtractor::new()));

    let result = router
        .aggregated_search(
            "Rust 2024 edition migration guide",
            &SearchConfig {
                max_results: 5,
                auto_extract: true,
                ..Default::default()
            },
            &SearchOptions::default(),
        )
        .await?;

    for item in &result.items {
        println!("{}", item.title);
        if let Some(ref content) = item.extracted_content {
            println!("  excerpt: {}", content.excerpt);
        }
    }
    Ok(())
}
```

## Core API

### `SearchRouter`

The central orchestrator. Register engines, attach an extractor, configure caching and dedup:

```rust
let router = SearchRouter::new()
    .with_timeout(std::time::Duration::from_secs(10))
    .with_dedup(DedupStrategy::UrlExactWithNearDup { threshold: 0.95 })
    .with_cache(Box::new(MemorySearchCache::new(500)));
```

Primary method: `router.aggregated_search(query, &config, &options)`.

### `SearchConfig`

Data-plane parameters (dead simple — `Default` works out of the box):

| Field                   | Type                 | Default        |
|-------------------------|----------------------|----------------|
| `max_results`           | `usize`              | `10`           |
| `max_tokens`            | `usize`              | `1024`         |
| `engines`               | `Vec<String>`        | `[]` (all)     |
| `sources`               | `Vec<String>`        | `["web"]`      |
| `region`                | `Option<String>`     | `None`         |
| `time_range`            | `Option<TimeRange>`  | `None`         |
| `enable_cache`          | `bool`               | `true`         |
| `auto_extract`          | `bool`               | `false`        |
| `safe_search`           | `Option<SafeSearchLevel>` | `Moderate` |
| `offset`                | `usize`              | `0`            |

### `SearchResult`

| Field             | Type                 |
|-------------------|----------------------|
| `query`           | `String`             |
| `items`           | `Vec<SearchItem>`    |
| `total_results`   | `u64`                |
| `latency_ms`      | `u64`                |
| `cached`          | `bool`               |
| `engines_used`    | `Vec<String>`        |
| `rewritten_query` | `Option<String>`     |

### `SearchItem`

| Field               | Type                      |
|---------------------|---------------------------|
| `title`             | `String`                  |
| `url`               | `String`                  |
| `snippet`           | `String`                  |
| `source`            | `String`                  |
| `published_at`      | `Option<u64>`             |
| `score`             | `f32`                     |
| `domain`            | `String`                  |
| `extracted_content` | `Option<ExtractedContent>`|

## Content extraction

Attach a `JinaExtractor` (requires `jina` feature) and set `auto_extract: true`:

```rust
let router = SearchRouter::new()
    .with_extractor(Box::new(JinaExtractor::new()));
```

Or use the extractor directly:

```rust
let extractor = JinaExtractor::new();
let content = extractor.extract("https://example.com/article").await?;
println!("{}", content.content);
let batch = extractor.extract_batch(&["url1", "url2"], 4).await?;
```

## Batch search

Process many queries concurrently with `batch_search_with_arc`:

```rust
let router = Arc::new(SearchRouter::new());
let results = batch_search_with_arc(
    router,
    &["query one".into(), "query two".into()],
    &SearchConfig::default(),
    4, // concurrency
).await;
```

## Caching

```rust
let cache = MemorySearchCache::new(1000);
let router = SearchRouter::new().with_cache(Box::new(cache));
```

Cache stats via trait `CacheStats` — `hits`, `misses`, `size_bytes`, `entry_count`.

## Rate limiting

Gate engine calls with a token-bucket limiter:

```rust
let limiter = SearchRateLimiter::new(5.0, 1000); // 5 QPS, 1000/day
limiter.acquire("tavily").await?;
```

## Query rewriting

Heuristic rewriting without LLM dependency:

```rust
let rewriter = QueryRewriter::new("default");
let rewritten = rewriter
    .rewrite_with_template("best practices Rust async", RewriteTemplate::KeywordExtraction)
    .await?;
// → ["best practices rust async"]
```

## Feedback

```rust
let feedback = MemorySearchFeedback::new();
feedback.record_click("rust async", "https://example.com").await;
feedback.record_irrelevant("rust async", "https://spam.com").await;
let weight = feedback.get_url_weight("https://example.com").await;
```

## Error handling

All operations return `Result<_, SearchError>`:

| Variant            | Description                  |
|--------------------|------------------------------|
| `Api`              | Backend API error            |
| `Network`          | Transport failure            |
| `RateLimit`        | Rate limit hit               |
| `Extraction`       | Content extraction failure   |
| `Config`           | Invalid configuration        |
| `Auth`             | Authentication failure       |
| `AllEnginesFailed` | No engine returned results   |
| `Timeout`          | Request exceeded deadline    |
| `EngineUnavailable`| Engine not reachable         |

`SearchError::is_retryable()` returns `true` for `Network`, `RateLimit`, and `Timeout`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
