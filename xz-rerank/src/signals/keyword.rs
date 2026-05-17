use async_trait::async_trait;
use std::collections::HashSet;

use crate::error::RerankError;
use crate::traits::SignalPlugin;
use crate::types::RerankCandidate;

/// 关键词重叠度信号（Jaccard 相似度）
#[derive(Debug)]
pub struct KeywordOverlapSignal;

fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .collect()
}

#[async_trait]
impl SignalPlugin for KeywordOverlapSignal {
    fn name(&self) -> &str {
        "keyword_overlap"
    }

    fn weight_key(&self) -> &'static str {
        "keyword_overlap"
    }

    async fn score(
        &self,
        query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        let query_tokens = tokenize(query);
        let candidate_tokens = tokenize(&candidate.content);

        if query_tokens.is_empty() || candidate_tokens.is_empty() {
            return Ok(0.0);
        }

        let intersection = query_tokens
            .iter()
            .filter(|t| candidate_tokens.contains(*t))
            .count();
        let union: HashSet<&String> = query_tokens
            .union(&candidate_tokens)
            .collect();

        if union.is_empty() {
            return Ok(0.0);
        }

        Ok(intersection as f32 / union.len() as f32)
    }
}
