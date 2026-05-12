use std::collections::HashMap;
use xz_rerank::*;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn make_candidate(id: &str, content: &str, retrieval_score: Option<f32>, created_at: u64) -> RerankCandidate {
    RerankCandidate {
        id: id.to_string(),
        content: content.to_string(),
        metadata: HashMap::from([
            ("source".into(), "test".into()),
        ]),
        retrieval_score,
        channel: Some("semantic".into()),
        created_at: Some(created_at),
        embedding: None,
    }
}

fn reranker_without_vector() -> LocalSignalReranker {
    let weights = SignalWeights::default();
    LocalSignalReranker::new(weights)
        .with_signal(Box::new(KeywordOverlapSignal))
        .with_signal(Box::new(MetadataMatchSignal::default()))
        .with_signal(Box::new(ContentQualitySignal))
        .with_signal(Box::new(RecencySignal::new(RecencyMode::ExponentialDecay { decay_rate: 0.01 })))
}

#[tokio::test]
async fn test_rerank_ordering() {
    let reranker = reranker_without_vector();

    let candidates = vec![
        make_candidate("a", "Rust programming language guide", Some(0.9), now_ms()),
        make_candidate("b", "Python for data science", Some(0.7), now_ms() - 86400_000),
        make_candidate("c", "Rust web framework comparison", Some(0.8), now_ms()),
    ];

    let result = reranker
        .rerank("Rust programming", candidates, &RerankConfig::default())
        .await
        .unwrap();

    assert_eq!(result.hits[0].candidate_id, "a");
    assert!(result.hits[0].score > result.hits[1].score);
    assert_eq!(result.reranker, "local-signal");
}

#[tokio::test]
async fn test_min_score_filter() {
    let reranker = reranker_without_vector();

    let candidates = vec![
        make_candidate("a", "Rust programming", Some(0.9), now_ms()),
        make_candidate("b", "Unrelated content", Some(0.1), now_ms()),
    ];

    let result = reranker
        .rerank(
            "Rust programming",
            candidates,
            &RerankConfig {
                min_score: Some(0.5),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].candidate_id, "a");
    assert_eq!(result.stats.filtered_out, 1);
}

#[tokio::test]
async fn test_empty_candidates_error() {
    let reranker = reranker_without_vector();
    let result = reranker
        .rerank("query", vec![], &RerankConfig::default())
        .await;
    assert!(matches!(result.unwrap_err(), RerankError::EmptyCandidates));
}

#[tokio::test]
async fn test_score_breakdown() {
    let weights = SignalWeights::default();
    let reranker = LocalSignalReranker::new(weights)
        .with_signal(Box::new(KeywordOverlapSignal))
        .with_signal(Box::new(VectorSimilaritySignal::with_query_embedding(vec![0.1_f32; 384])))
        .with_signal(Box::new(MetadataMatchSignal::default()))
        .with_signal(Box::new(ContentQualitySignal))
        .with_signal(Box::new(RecencySignal::new(RecencyMode::ExponentialDecay { decay_rate: 0.01 })));

    let candidates = vec![
        make_candidate("a", "Rust systems programming language", Some(0.9), now_ms()),
    ];

    let result = reranker
        .rerank(
            "systems programming",
            candidates,
            &RerankConfig {
                include_score_breakdown: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert!(result.hits[0].score_breakdown.is_some());

    let breakdown = result.hits[0].score_breakdown.as_ref().unwrap();
    assert!(!breakdown.signals.is_empty());
    // 验证贡献总和 ≈ final_score
    let total: f32 = breakdown.signals.iter().map(|s| s.contribution).sum();
    assert!((total - breakdown.final_score).abs() < 0.01);
}
