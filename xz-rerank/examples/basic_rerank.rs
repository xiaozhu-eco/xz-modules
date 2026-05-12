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
    let reranker = LocalSignalReranker::default();

    let candidates = vec![
        RerankCandidate {
            id: "doc1".into(),
            content: "Rust is a systems programming language with zero-cost abstractions.".into(),
            metadata: HashMap::from([("source".into(), "docs".into())]),
            retrieval_score: Some(0.85),
            channel: Some("semantic".into()),
            created_at: Some(now_ms() - 3600_000),
            embedding: None,
        },
        RerankCandidate {
            id: "doc2".into(),
            content: "Python is used for machine learning and data analysis.".into(),
            metadata: HashMap::from([("source".into(), "blog".into())]),
            retrieval_score: Some(0.72),
            channel: Some("semantic".into()),
            created_at: Some(now_ms() - 86400_000),
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
                println!("    {}: raw={:.4} weight={:.2} contrib={:.4}",
                    signal.name, signal.raw_score, signal.weight, signal.contribution);
            }
        }
    }

    Ok(())
}
