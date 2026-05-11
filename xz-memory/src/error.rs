use std::error::Error;

/// Memory system errors.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Database error: {message}")]
    Database {
        message: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Fact not found: {0}")]
    FactNotFound(String),

    #[error("Vector not found: {0}")]
    VectorNotFound(String),

    #[error("Vector memory not enabled")]
    VectorMemoryNotEnabled,

    #[error("Summary generation failed: {0}")]
    SummaryGeneration(String),

    #[error("Import format error: {0}")]
    ImportFormat(String),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Transaction failed: {message}")]
    Transaction {
        message: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },
}

impl MemoryError {
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            source: None,
        }
    }

    pub fn database_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Database {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn transaction(message: impl Into<String>) -> Self {
        Self::Transaction {
            message: message.into(),
            source: None,
        }
    }

    pub fn transaction_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Transaction {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            source: None,
        }
    }

    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Database { .. } | Self::Transaction { .. })
    }
}
