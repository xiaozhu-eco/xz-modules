use std::collections::HashMap;
use xz_rerank::*;

fn default_candidate() -> RerankCandidate {
    RerankCandidate {
        id: "test".into(),
        content: "test content".into(),
        metadata: HashMap::new(),
        retrieval_score: None,
        channel: None,
        created_at: None,
        embedding: None,
    }
}

#[tokio::test]
async fn test_keyword_overlap() {
    let signal = KeywordOverlapSignal;
    let candidate = RerankCandidate {
        content: "Rust is a systems programming language".into(),
        ..default_candidate()
    };

    let score = signal.score("systems programming", &candidate).await.unwrap();
    assert!(score > 0.3);
}

#[tokio::test]
async fn test_keyword_no_overlap() {
    let signal = KeywordOverlapSignal;
    let candidate = RerankCandidate {
        content: "Python data science machine learning".into(),
        ..default_candidate()
    };

    let score = signal.score("systems programming", &candidate).await.unwrap();
    assert_eq!(score, 0.0);
}

#[tokio::test]
async fn test_content_quality_optimal() {
    let signal = ContentQualitySignal;
    let candidate = RerankCandidate {
        content: "a".repeat(500),
        ..default_candidate()
    };

    let score = signal.score("query", &candidate).await.unwrap();
    assert!((score - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_content_quality_too_short() {
    let signal = ContentQualitySignal;
    let candidate = RerankCandidate {
        content: "hi".into(),
        ..default_candidate()
    };

    let score = signal.score("query", &candidate).await.unwrap();
    assert!(score < 1.0);
}

#[test]
fn test_signal_weights_default() {
    let w = SignalWeights::default();
    assert!(w.validate().is_ok());
}

#[test]
fn test_signal_weights_invalid() {
    let w = SignalWeights {
        keyword_overlap: 0.5,
        vector_similarity: 0.5,
        metadata_match: 0.5,
        content_quality: 0.5,
        recency: 0.5,
    };
    assert!(w.validate().is_err());
    let normalized = w.normalize();
    assert!(normalized.validate().is_ok());
}

#[tokio::test]
async fn test_recency_no_decay() {
    use xz_rerank::RecencyMode;
    let signal = RecencySignal::new(RecencyMode::NoDecay);
    let candidate = RerankCandidate {
        created_at: Some(0),
        ..default_candidate()
    };

    let score = signal.score("query", &candidate).await.unwrap();
    assert!((score - 1.0).abs() < 0.01);
}
