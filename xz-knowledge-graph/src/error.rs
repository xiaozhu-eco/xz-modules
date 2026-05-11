/// Knowledge graph errors.
#[derive(Debug, thiserror::Error)]
pub enum KgError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Relation not found: {0}")]
    RelationNotFound(String),

    #[error("Entity exists and update not allowed: {0}")]
    EntityExists(String),

    #[error("Import conflict: {0}")]
    ImportConflict(String),

    #[error("Path not found: {from} -> {to}")]
    PathNotFound { from: String, to: String },

    #[error("Max depth exceeded: {depth} > {max}")]
    MaxDepthExceeded { depth: u32, max: u32 },

    #[error("Circular reference detected: entity={0}")]
    CircularReference(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Transaction failed: {0}")]
    Transaction(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl KgError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, KgError::Database(_) | KgError::Transaction(_))
    }
}
