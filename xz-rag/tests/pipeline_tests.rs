use std::collections::HashMap;

use xz_rag::{
    ChannelConfig, MinMaxNormalizer, RetrievedChunk, RrfFusion, ZScoreNormalizer,
};

fn make_hit(id: &str, score: f32, channel: &str) -> RetrievedChunk {
    RetrievedChunk {
        chunk_id: id.to_string(),
        document_id: format!("doc_{}", id),
        content: format!("content {}", id),
        score,
        channel: channel.to_string(),
        channel_score: score,
        metadata: xz_rag::ChunkMetadata::default(),
        embedding: None,
    }
}

#[test]
fn test_rrf_fusion_basic() {
    let fusion = RrfFusion::new(60);

    let mut channel_results = HashMap::new();
    channel_results.insert(
        "semantic".to_string(),
        vec![
            make_hit("c1", 0.9, "semantic"),
            make_hit("c2", 0.8, "semantic"),
        ],
    );
    channel_results.insert(
        "bm25".to_string(),
        vec![
            make_hit("c3", 0.7, "bm25"),
            make_hit("c2", 0.4, "bm25"),
        ],
    );

    let fused = fusion.fuse(channel_results);
    // c2 appears in both channels, should have higher RRF score
    assert!(fused.len() <= 3);

    let c2 = fused.iter().find(|h| h.chunk_id == "c2").unwrap();
    let c3 = fused.iter().find(|h| h.chunk_id == "c3").unwrap();
    // c2 appears in both channels -> higher RRF score than c3
    assert!(c2.score > c3.score);
}

#[test]
fn test_minmax_normalize() {
    let mut scores = vec![0.2, 0.5, 0.8];
    MinMaxNormalizer::normalize(&mut scores);
    assert!((scores[0] - 0.0).abs() < 0.01);
    assert!((scores[1] - 0.5).abs() < 0.01);
    assert!((scores[2] - 1.0).abs() < 0.01);
}

#[test]
fn test_minmax_normalize_single() {
    let mut scores = vec![0.5];
    MinMaxNormalizer::normalize(&mut scores);
    assert!((scores[0] - 0.5).abs() < 0.01); // All same -> 0.5
}

#[test]
fn test_zscore_normalize() {
    let mut scores = vec![0.2, 0.5, 0.8];
    ZScoreNormalizer::normalize(&mut scores);
    for s in &scores {
        assert!((0.0..=1.0).contains(s));
    }
}

#[test]
fn test_channel_pipeline() {
    let pipeline = xz_rag::ChannelPipeline::new(vec![
        ChannelConfig::semantic(0.5, 10).with_min_score(0.1),
        ChannelConfig::metadata(0.3, 5),
    ])
    .with_rrf_k(60)
    .with_normalize(true);

    assert_eq!(pipeline.channels.len(), 2);
    assert_eq!(pipeline.rrf_k, 60);
    assert!(pipeline.normalize_scores);
}
