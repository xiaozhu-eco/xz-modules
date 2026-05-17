use crate::error::RagError;

/// Query expansion: generate N equivalent queries for better recall.
pub struct QueryExpander;

impl QueryExpander {
    pub fn new() -> Self {
        Self
    }

    /// Generate expanded queries.
    #[cfg(feature = "query-expansion")]
    pub async fn expand(
        &self,
        query: &str,
        count: usize,
        provider: &dyn xz_provider::LlmProvider,
    ) -> Result<Vec<String>, RagError> {
        let prompt = format!(
            "Generate {} different ways to express the following search query:\n\nOriginal: {}\n\nVariations:\n1.",
            count, query
        );
        let request = xz_provider::CompletionRequest {
            messages: vec![xz_provider::Message::user(&prompt)],
            temperature: Some(0.8),
            max_tokens: Some(256),
            ..Default::default()
        };
        let response = provider
            .complete(request, xz_provider::RequestOptions::default())
            .await
            .map_err(|e| RagError::QueryPreprocessing(e.to_string()))?;

        let mut queries = vec![query.to_string()];
        for line in response.content.unwrap_or_default().lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Strip leading number prefix like "1. "
                let q = trimmed
                    .splitn(2, ". ")
                    .last()
                    .unwrap_or(trimmed);
                queries.push(q.to_string());
                if queries.len() >= count + 1 {
                    break;
                }
            }
        }

        Ok(queries)
    }

    /// Fallback: returns original query only.
    #[cfg(not(feature = "query-expansion"))]
    pub async fn expand(&self, query: &str, _count: usize) -> Result<Vec<String>, RagError> {
        Ok(vec![query.to_string()])
    }
}

impl Default for QueryExpander {
    fn default() -> Self {
        Self::new()
    }
}
