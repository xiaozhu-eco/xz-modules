use crate::error::RagError;

/// HYDE (Hypothetical Document Embeddings) query expansion.
///
/// Generates a hypothetical answer passage for vector retrieval.
/// Requires LLM integration (xz-provider feature).
pub struct HydeExpander;

impl HydeExpander {
    pub fn new() -> Self {
        Self
    }

    /// Generate a hypothetical answer passage for the given query.
    #[cfg(feature = "hyde")]
    pub async fn expand(
        &self,
        query: &str,
        provider: &dyn xz_provider::LlmProvider,
    ) -> Result<String, RagError> {
        let prompt = format!(
            "Please write a passage that answers the following question:\n\nQuestion: {}\n\nPassage:",
            query
        );
        let request = xz_provider::CompletionRequest {
            messages: vec![xz_provider::Message::user(&prompt)],
            temperature: Some(0.7),
            max_tokens: Some(256),
            ..Default::default()
        };
        let response = provider
            .complete(request, xz_provider::RequestOptions::default())
            .await
            .map_err(|e| RagError::QueryPreprocessing(e.to_string()))?;

        Ok(response.content.unwrap_or_default())
    }

    /// Fallback when LLM is not available: returns query as-is.
    #[cfg(not(feature = "hyde"))]
    pub async fn expand(&self, query: &str) -> Result<String, RagError> {
        Ok(query.to_string())
    }
}

impl Default for HydeExpander {
    fn default() -> Self {
        Self::new()
    }
}
