/// RAG engine errors.
#[derive(Debug, thiserror::Error)]
pub enum RagError {
    #[error("Retrieval error: {0}")]
    Retrieve(String),

    #[error("Generation error: {0}")]
    Generation(String),

    #[error("Context overflow: {used}/{max} tokens")]
    ContextOverflow { used: usize, max: usize },

    #[error("No results found for query: {0}")]
    NoResults(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Rerank error: {0}")]
    Rerank(String),

    #[error("Chunking error: {0}")]
    Chunking(String),

    #[error("Query preprocessing error: {0}")]
    QueryPreprocessing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),
}

impl RagError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RagError::Retrieve(_) | RagError::Generation(_) | RagError::Store(_)
        )
    }
}
