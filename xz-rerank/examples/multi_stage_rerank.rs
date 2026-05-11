use std::collections::HashMap;
use xz_rerank::*;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 粗排：本地快速过滤 500 → 50
    let coarse = LocalSignalReranker::default();

    // 精排：Mock（实际使用 CohereReranker::new("key")?）
    let fine = MockReranker::new("mock-fine");

    let multi_stage = MultiStageReranker::new(coarse, fine, 50);

    let candidates: Vec<RerankCandidate> = (0..100)
        .map(|i| RerankCandidate {
            id: format!("doc{i}"),
            content: format!("Document {i} about Rust programming"),
            metadata: HashMap::from([("idx".into(), i.to_string())]),
            retrieval_score: Some(0.5 + (i as f32 * 0.005)),
            channel: Some("semantic".into()),
            created_at: Some(now_ms() - i * 10000),
            embedding: None,
        })
        .collect();

    let result = multi_stage
        .rerank("Rust programming", candidates, &RerankConfig::default())
        .await?;

    println!("Multi-stage results ({}ms):", result.latency_ms);
    for hit in &result.hits {
        println!("  [{:.4}] {}", hit.score, hit.candidate_id);
    }

    Ok(())
}
