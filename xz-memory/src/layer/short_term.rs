//! Short-term memory management.
//!
//! Handles message appending, retrieval, and session window management.

use tracing::info;

use crate::config::ShortTermConfig;
use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::query::PageRequest;

/// Short-term memory helper for enforcing retention policies.
pub struct ShortTermManager {
    config: ShortTermConfig,
}

impl ShortTermManager {
    pub fn new(config: ShortTermConfig) -> Self {
        Self { config }
    }

    /// Evict older messages if the session exceeds max_messages_per_session.
    pub async fn enforce_window<M: MemorySystem + ?Sized>(
        &self,
        memory: &M,
        session_id: &str,
    ) -> Result<(), MemoryError> {
        let page = memory
            .get_session_messages(session_id, PageRequest {
                limit: 1,
                offset: 0,
            })
            .await?;

        if page.total > self.config.max_messages_per_session {
            let excess = page.total - self.config.max_messages_per_session;
            info!(
                session_id = %session_id,
                excess,
                total = page.total,
                max = self.config.max_messages_per_session,
                "evicting excess messages"
            );

            let evicted = memory
                .evict_oldest_messages(session_id, self.config.max_messages_per_session)
                .await?;

            info!(
                session_id = %session_id,
                evicted,
                remaining = self.config.max_messages_per_session,
                "window enforcement complete"
            );
        }

        Ok(())
    }
}
