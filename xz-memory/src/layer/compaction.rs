//! Memory compaction strategies.
//!
//! Implements MergeSimilar, RemoveLowConfidence, and RemoveOld compaction.

use tracing::warn;

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{CompactionResult, CompactionStrategy};

/// Compaction runner that delegates to the memory system.
pub struct CompactionRunner;

impl CompactionRunner {
    /// Run compaction with the given strategy.
    pub async fn run<M: MemorySystem + ?Sized>(
        memory: &M,
        user_id: &str,
        strategy: CompactionStrategy,
    ) -> Result<CompactionResult, MemoryError> {
        let result = memory.compact_facts(user_id, strategy).await?;
        warn!(
            user_id = %user_id,
            merged = %result.facts_merged,
            removed = %result.facts_removed,
            kept = %result.facts_kept,
            "compaction runner completed"
        );
        Ok(result)
    }
}
