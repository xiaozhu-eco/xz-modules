use async_trait::async_trait;

use crate::error::RerankError;
use crate::traits::SignalPlugin;
use crate::types::RerankCandidate;

/// 向量相似度信号
///
/// 需要 query embedding 由调用方在 rerank 前计算。
#[derive(Debug)]
pub struct VectorSimilaritySignal {
    /// query embedding（由调用方在 rerank 前计算）
    query_embedding: Option<Vec<f32>>,
}

impl VectorSimilaritySignal {
    pub fn new() -> Self {
        Self {
            query_embedding: None,
        }
    }

    pub fn with_query_embedding(embedding: Vec<f32>) -> Self {
        Self {
            query_embedding: Some(embedding),
        }
    }
}

impl Default for VectorSimilaritySignal {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignalPlugin for VectorSimilaritySignal {
    fn name(&self) -> &str {
        "vector_similarity"
    }

    async fn score(
        &self,
        _query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        let q = match self.query_embedding.as_ref() {
            Some(q) => q,
            None => return Err(RerankError::MissingQueryEmbedding),
        };
        let empty = vec![];
        let c = candidate.embedding.as_ref().unwrap_or(&empty);

        if q.is_empty() || c.is_empty() {
            return Ok(0.0);
        }

        let dot: f32 = q.iter().zip(c).map(|(a, b)| a * b).sum();
        let q_norm: f32 = q.iter().map(|x| x * x).sum::<f32>().sqrt();
        let c_norm: f32 = c.iter().map(|x| x * x).sum::<f32>().sqrt();

        if q_norm == 0.0 || c_norm == 0.0 {
            return Ok(0.0);
        }

        let similarity = dot / (q_norm * c_norm);
        Ok((similarity + 1.0) / 2.0) // [-1, 1] → [0, 1]
    }
}

#[async_trait]
impl SignalPlugin for &VectorSimilaritySignal {
    fn name(&self) -> &str {
        "vector_similarity"
    }

    async fn score(
        &self,
        _query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        VectorSimilaritySignal::score(self, _query, candidate).await
    }
}
