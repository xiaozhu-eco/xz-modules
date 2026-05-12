//! Summary generation layer.
//!
//! Provides LLM-driven session summarization with incremental updates.

#[cfg(feature = "summary")]
use xz_provider::traits::LlmProvider;

use crate::config::SummaryConfig;
use crate::error::MemoryError;
use crate::types::session::SessionSummary;

/// Quality metrics for a generated summary.
#[derive(Debug, Clone)]
pub struct SummaryQuality {
    /// Ratio of original tokens to summary tokens.
    pub compression_ratio: f32,
    /// Fraction of key points preserved (0..=1).
    pub key_point_coverage: f32,
    /// Estimated hallucination score (lower is better).
    pub hallucination_score: f32,
}

/// Thresholds for summary quality.
const MIN_COVERAGE: f32 = 0.7;
const MAX_HALLUCINATION: f32 = 0.3;

impl SummaryQuality {
    /// Returns true if the summary meets quality thresholds.
    pub fn is_acceptable(&self) -> bool {
        self.key_point_coverage >= MIN_COVERAGE && self.hallucination_score <= MAX_HALLUCINATION
    }
}

/// Summary manager for coordinating summary generation and quality checks.
pub struct SummaryManager {
    #[allow(dead_code)]
    config: SummaryConfig,
}

impl SummaryManager {
    pub fn new(config: SummaryConfig) -> Self {
        Self { config }
    }

    /// Check whether a summary should trigger based on message count.
    pub fn should_trigger(&self, message_count: usize) -> bool {
        message_count > 0 && message_count % self.config.trigger_at_message_count == 0
    }

    /// Generate a summary using the configured LLM provider.
    #[cfg(feature = "summary")]
    pub async fn generate(
        &self,
        provider: &dyn LlmProvider,
        conversation: &str,
        message_count: usize,
        token_count: usize,
    ) -> Result<SessionSummary, MemoryError> {
        let prompt = format!(
            "Summarize the following conversation concisely. Include at most {} characters.\n\
             Focus on key decisions, facts learned, and user preferences.\n\n\
             Conversation:\n{}",
            self.config.max_summary_length, conversation
        );

        let request = xz_provider::CompletionRequest {
            messages: vec![xz_provider::Message::user(&prompt)],
            ..Default::default()
        };

        let options = xz_provider::RequestOptions::default();
        let response = provider
            .complete(request, options)
            .await
            .map_err(|e| MemoryError::SummaryGeneration(e.to_string()))?;

        let summary_text = response.content.unwrap_or_default();

        // Truncate to max length if needed
        let summary_text = if summary_text.len() > self.config.max_summary_length {
            summary_text[..self.config.max_summary_length].to_string()
        } else {
            summary_text
        };

        Ok(SessionSummary {
            session_id: String::new(),
            user_id: String::new(),
            summary: summary_text,
            key_points: vec![],
            token_count,
            message_count,
            created_at: current_epoch_ms(),
            updated_at: current_epoch_ms(),
        })
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
