pub mod circular;
pub mod duplicate;
pub mod expiration;
pub mod orphan;

use crate::error::KgError;
use crate::types::consistency::ConsistencyIssue;

/// Consistency checker trait.
#[async_trait::async_trait]
pub trait ConsistencyChecker: Send + Sync {
    /// Run the check and return any issues found.
    async fn check(&self) -> Result<Vec<ConsistencyIssue>, KgError>;
}
