use std::collections::HashMap;

use crate::types::retrieval::RetrievedChunk;

/// RRF (Reciprocal Rank Fusion) for merging multi-channel results.
#[derive(Debug, Clone)]
pub struct RrfFusion {
    pub k: usize,
}

impl RrfFusion {
    pub fn new(k: usize) -> Self {
        Self { k }
    }

    /// Fuse results from multiple channels using RRF.
    ///
    /// `channel_results` maps channel name -> ranked hits.
    /// Returns fused hits sorted by RRF score descending.
    pub fn fuse(
        &self,
        channel_results: HashMap<String, Vec<RetrievedChunk>>,
    ) -> Vec<RetrievedChunk> {
        let mut rrf_scores: HashMap<String, f32> = HashMap::new();
        let mut best_per_chunk: HashMap<String, RetrievedChunk> = HashMap::new();

        for (_channel_name, hits) in &channel_results {
            for (rank, hit) in hits.iter().enumerate() {
                let rrf = 1.0 / (self.k as f32 + (rank + 1) as f32);
                *rrf_scores.entry(hit.chunk_id.clone()).or_default() += rrf;

                // Keep highest original score per chunk
                best_per_chunk
                    .entry(hit.chunk_id.clone())
                    .and_modify(|existing| {
                        if hit.score > existing.score {
                            *existing = hit.clone();
                        }
                    })
                    .or_insert_with(|| hit.clone());
            }
        }

        let mut fused: Vec<RetrievedChunk> = best_per_chunk
            .into_iter()
            .map(|(chunk_id, mut chunk)| {
                chunk.score = rrf_scores.get(&chunk_id).copied().unwrap_or(0.0);
                chunk
            })
            .collect();

        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hit(id: &str, score: f32, channel: &str) -> RetrievedChunk {
        RetrievedChunk {
            chunk_id: id.to_string(),
            document_id: format!("doc_{}", id),
            content: format!("content_{}", id),
            score,
            channel: channel.to_string(),
            channel_score: score,
            metadata: crate::types::chunk::ChunkMetadata::default(),
            embedding: None,
        }
    }

    #[test]
    fn test_rrf_fusion() {
        let fusion = RrfFusion::new(60);

        let mut channel_results = HashMap::new();
        channel_results.insert("semantic".to_string(), vec![
            make_hit("c1", 0.9, "semantic"),
            make_hit("c2", 0.8, "semantic"),
            make_hit("c3", 0.5, "semantic"),
        ]);
        channel_results.insert("bm25".to_string(), vec![
            make_hit("c3", 0.7, "bm25"),
            make_hit("c2", 0.4, "bm25"),
            make_hit("c4", 0.6, "bm25"),
        ]);

        let fused = fusion.fuse(channel_results);
        assert!(!fused.is_empty());

        // c2 appeared in both channels -> higher RRF score
        let c2 = fused.iter().find(|h| h.chunk_id == "c2").unwrap();
        let c4 = fused.iter().find(|h| h.chunk_id == "c4").unwrap();
        assert!(c2.score > c4.score);
    }
}
