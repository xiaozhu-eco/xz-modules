//! Style memory — tracks writing style metrics across chapters.
//!
//! This module provides data types and a high-level memory client for
//! recording chapter-level writing metrics, computing an aggregate style
//! profile, and detecting style drift between chapters.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{Confidence, Fact, FactCategory, FactRecallOptions, FactSortField};
use crate::types::query::{PageRequest, UpsertResult};

// ---------------------------------------------------------------------------
// Custom serde for `Range<u32>` — serialized as a `[start, end]` tuple
// ---------------------------------------------------------------------------

mod range_serde {
    use std::ops::Range;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(range: &Range<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (range.start, range.end).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Range<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (start, end) = <(u32, u32)>::deserialize(deserializer)?;
        Ok(start..end)
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Overall style profile for a novel.
///
/// Captures the aggregate writing style derived from per-chapter metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    /// Unique identifier for the novel.
    pub novel_id: String,
    /// Descriptive voice label (e.g., "Third person limited, literary").
    pub overall_voice: String,
    /// Typical word count range across chapters.
    #[serde(with = "range_serde")]
    pub typical_word_count: Range<u32>,
    /// Preferred pacing category: `"fast"`, `"balanced"`, or `"slow"`.
    pub preferred_pacing: String,
    /// Ratio of dialogue text (0.0–1.0).
    pub dialogue_ratio: f32,
    /// Ratio of description text (0.0–1.0).
    pub description_ratio: f32,
    /// Narrative techniques employed (e.g., `"flashback"`,
    /// `"stream of consciousness"`).
    pub narrative_techniques: Vec<String>,
    /// Timestamp of last profile update.
    pub updated_at: DateTime<Utc>,
}

/// Aggregate metrics for a single chapter's writing style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMetrics {
    /// Chapter number (1-based).
    pub chapter_number: u32,
    /// Total word count.
    pub word_count: u64,
    /// Total sentence count.
    pub sentence_count: u64,
    /// Average sentence length in words.
    pub avg_sentence_length: f32,
    /// Percentage of dialogue content (0.0–100.0).
    pub dialogue_pct: f32,
    /// Percentage of description content (0.0–100.0).
    pub description_pct: f32,
    /// Percentage of action content (0.0–100.0).
    pub action_pct: f32,
    /// Count of unique words used.
    pub unique_words: u64,
    /// Readability score (0.0–100.0, simplified Flesch).
    pub readability_score: f32,
    /// Pacing score (-1.0 slow to 1.0 fast).
    pub pacing_score: f32,
    /// Most frequent adjectives in this chapter.
    pub top_adjectives: Vec<String>,
    /// Most frequent verbs in this chapter.
    pub top_verbs: Vec<String>,
    /// Timestamp when metrics were recorded.
    pub created_at: DateTime<Utc>,
}

/// Detected style drift for a chapter relative to preceding chapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDrift {
    /// Chapter number being assessed.
    pub chapter_number: u32,
    /// Overall drift score (0.0 = no drift, 1.0 = severe drift).
    pub drift_score: f32,
    /// Names of metrics that changed significantly.
    pub changed_metrics: Vec<String>,
    /// Human-readable recommendation based on drift assessment.
    pub recommendation: String,
}

// ---------------------------------------------------------------------------
// StyleMemory — high-level client
// ---------------------------------------------------------------------------

/// High-level style memory for tracking and querying writing style metrics.
///
/// Stores per-chapter metrics as facts via the [`MemorySystem`] trait and
/// provides derived analyses such as aggregate style profiles and drift
/// detection.
pub struct StyleMemory {
    memory: Arc<dyn MemorySystem>,
    novel_id: String,
}

impl StyleMemory {
    /// Create a new [`StyleMemory`] for the given novel.
    pub fn new(memory: Arc<dyn MemorySystem>, novel_id: &str) -> Self {
        Self { memory, novel_id: novel_id.to_string() }
    }

    /// Retrieve or compute the aggregate style profile from recorded chapter
    /// metrics.
    ///
    /// Returns `Ok(None)` when no chapter metrics have been recorded yet.
    pub async fn get_style_profile(&self) -> Result<Option<StyleProfile>, MemoryError> {
        let metrics = self.load_all_metrics().await?;
        if metrics.is_empty() {
            return Ok(None);
        }

        let count = metrics.len() as f32;
        let avg_word_count =
            (metrics.iter().map(|m| m.word_count).sum::<u64>() as f32 / count) as u32;

        let min_word_count = metrics.iter().map(|m| m.word_count as u32).min().unwrap_or(0);
        let max_word_count = metrics.iter().map(|m| m.word_count as u32).max().unwrap_or(0);

        // Ensure range is meaningful even with a single chapter
        let range_end = max_word_count.max(avg_word_count).max(min_word_count + 1);

        let avg_dialogue = metrics.iter().map(|m| m.dialogue_pct).sum::<f32>() / count;
        let avg_description = metrics.iter().map(|m| m.description_pct).sum::<f32>() / count;
        let avg_pacing = metrics.iter().map(|m| m.pacing_score).sum::<f32>() / count;

        let preferred_pacing = if avg_pacing < -0.3 {
            "slow".to_string()
        } else if avg_pacing > 0.3 {
            "fast".to_string()
        } else {
            "balanced".to_string()
        };

        // Collect aggregate top adjectives and verbs across all chapters
        let mut adj_freq: HashMap<String, usize> = HashMap::new();
        let mut verb_freq: HashMap<String, usize> = HashMap::new();
        for m in &metrics {
            for adj in &m.top_adjectives {
                *adj_freq.entry(adj.clone()).or_insert(0) += 1;
            }
            for verb in &m.top_verbs {
                *verb_freq.entry(verb.clone()).or_insert(0) += 1;
            }
        }

        Ok(Some(StyleProfile {
            novel_id: self.novel_id.clone(),
            overall_voice: "Third person limited, literary".to_string(),
            typical_word_count: min_word_count..range_end,
            preferred_pacing,
            dialogue_ratio: (avg_dialogue / 100.0).clamp(0.0, 1.0),
            description_ratio: (avg_description / 100.0).clamp(0.0, 1.0),
            narrative_techniques: Vec::new(),
            updated_at: Utc::now(),
        }))
    }

    /// Record metrics for a chapter.
    ///
    /// If metrics for the same chapter already exist they are overwritten.
    pub async fn record_chapter_metrics(&self, metrics: ChapterMetrics) -> Result<(), MemoryError> {
        let fact = self.build_metrics_fact(&metrics)?;
        match self.memory.remember_fact(fact).await? {
            UpsertResult::Created | UpsertResult::Updated { .. } | UpsertResult::Unchanged => {
                Ok(())
            }
        }
    }

    /// Retrieve metrics for a specific chapter by number.
    ///
    /// Returns `Ok(None)` if no metrics have been recorded for that chapter.
    pub async fn get_chapter_metrics(
        &self,
        chapter_number: u32,
    ) -> Result<Option<ChapterMetrics>, MemoryError> {
        let all = self.load_all_metrics().await?;
        Ok(all.into_iter().find(|m| m.chapter_number == chapter_number))
    }

    /// Retrieve metrics for the most recent `last_n` chapters.
    ///
    /// Returns chapters sorted by number in descending order
    /// (most recent first), limited to `last_n` entries.
    pub async fn get_recent_metrics(
        &self,
        last_n: usize,
    ) -> Result<Vec<ChapterMetrics>, MemoryError> {
        let mut all = self.load_all_metrics().await?;
        all.sort_by_key(|m| std::cmp::Reverse(m.chapter_number));
        all.truncate(last_n);
        Ok(all)
    }

    /// Detect style drift for a given chapter compared to the preceding 3
    /// chapters.
    ///
    /// Returns `Ok(None)` if the chapter metrics don't exist or there are
    /// fewer than one preceding chapter to compare against.
    pub async fn detect_style_drift(
        &self,
        chapter_number: u32,
    ) -> Result<Option<StyleDrift>, MemoryError> {
        let current = match self.get_chapter_metrics(chapter_number).await? {
            Some(m) => m,
            None => return Ok(None),
        };

        // Get up to 3 chapters before this one
        let mut all = self.load_all_metrics().await?;
        all.sort_by_key(|m| std::cmp::Reverse(m.chapter_number));
        let prev: Vec<ChapterMetrics> =
            all.into_iter().filter(|m| m.chapter_number < chapter_number).take(3).collect();

        if prev.is_empty() {
            return Ok(None);
        }

        let n = prev.len() as f32;
        let avg_word_count = prev.iter().map(|m| m.word_count as f32).sum::<f32>() / n;
        let avg_sentence_count = prev.iter().map(|m| m.sentence_count as f32).sum::<f32>() / n;
        let avg_sentence_len = prev.iter().map(|m| m.avg_sentence_length).sum::<f32>() / n;
        let avg_dialogue = prev.iter().map(|m| m.dialogue_pct).sum::<f32>() / n;
        let avg_description = prev.iter().map(|m| m.description_pct).sum::<f32>() / n;
        let avg_action = prev.iter().map(|m| m.action_pct).sum::<f32>() / n;
        let avg_unique_words = prev.iter().map(|m| m.unique_words as f32).sum::<f32>() / n;
        let avg_readability = prev.iter().map(|m| m.readability_score).sum::<f32>() / n;
        let avg_pacing = prev.iter().map(|m| m.pacing_score).sum::<f32>() / n;

        let threshold = 0.20; // 20 % change considered significant
        let mut changed_metrics: Vec<String> = Vec::new();
        let mut drift_sum = 0.0f32;
        let mut drift_count = 0u32;

        macro_rules! check_drift {
            ($val:expr, $avg:expr, $name:expr) => {
                if $avg != 0.0 {
                    let change = (($val as f32) - ($avg as f32)).abs() / ($avg as f32).abs();
                    if change > threshold {
                        changed_metrics.push($name.to_string());
                    }
                    drift_sum += change.min(1.0);
                    drift_count += 1;
                }
            };
        }

        check_drift!(current.word_count, avg_word_count, "word_count");
        check_drift!(current.sentence_count, avg_sentence_count, "sentence_count");
        check_drift!(current.avg_sentence_length, avg_sentence_len, "avg_sentence_length");
        check_drift!(current.dialogue_pct, avg_dialogue, "dialogue_pct");
        check_drift!(current.description_pct, avg_description, "description_pct");
        check_drift!(current.action_pct, avg_action, "action_pct");
        check_drift!(current.unique_words, avg_unique_words, "unique_words");
        check_drift!(current.readability_score, avg_readability, "readability_score");
        check_drift!(current.pacing_score, avg_pacing, "pacing_score");

        let drift_score =
            if drift_count > 0 { (drift_sum / drift_count as f32).clamp(0.0, 1.0) } else { 0.0 };

        let recommendation = if drift_score > 0.5 {
            format!(
                "Significant style drift detected in chapter {}. Metrics changed: {}. \
                 Consider reviewing for consistency.",
                chapter_number,
                changed_metrics.join(", ")
            )
        } else if !changed_metrics.is_empty() {
            format!(
                "Minor style drift detected in chapter {}. Metrics: {}. \
                 May be worth monitoring.",
                chapter_number,
                changed_metrics.join(", ")
            )
        } else {
            format!("Chapter {} is consistent with the established style.", chapter_number)
        };

        Ok(Some(StyleDrift { chapter_number, drift_score, changed_metrics, recommendation }))
    }

    // ── Internal helpers ──

    /// Load all chapter metrics facts from the memory system.
    async fn load_all_metrics(&self) -> Result<Vec<ChapterMetrics>, MemoryError> {
        let options = FactRecallOptions {
            page: PageRequest { limit: 1000, offset: 0 },
            categories: Some(vec![FactCategory::Custom("style".to_string())]),
            sort_by: FactSortField::UpdatedAt,
            min_confidence: None,
        };
        let page = self.memory.recall_facts(&self.novel_id, "chapter", &options).await?;
        let mut metrics: Vec<ChapterMetrics> =
            page.items.iter().filter_map(|f| serde_json::from_str(&f.object).ok()).collect();
        metrics.sort_by_key(|m| m.chapter_number);
        Ok(metrics)
    }

    /// Build a [`Fact`] from chapter metrics for persistence.
    fn build_metrics_fact(&self, metrics: &ChapterMetrics) -> Result<Fact, MemoryError> {
        let object = serde_json::to_string(metrics).map_err(|e| {
            MemoryError::serialization_with_source(
                "failed to serialize chapter metrics",
                Box::new(e),
            )
        })?;
        let now = Utc::now().timestamp() as u64;
        Ok(Fact {
            id: format!("style_{}_{}", self.novel_id, metrics.chapter_number),
            user_id: self.novel_id.clone(),
            category: FactCategory::Custom("style".to_string()),
            subject: "chapter_metrics".to_string(),
            predicate: format!("chapter_{}", metrics.chapter_number),
            object,
            confidence: Confidence::High,
            source_session: None,
            created_at: now,
            updated_at: now,
            version: 1,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::InMemoryMemory;

    /// Helper to construct a [`ChapterMetrics`] with sensible defaults.
    fn make_metrics(chapter: u32, word_count: u64) -> ChapterMetrics {
        ChapterMetrics {
            chapter_number: chapter,
            word_count,
            sentence_count: (word_count / 15).max(1),
            avg_sentence_length: 15.0,
            dialogue_pct: 30.0,
            description_pct: 40.0,
            action_pct: 30.0,
            unique_words: (word_count / 2).max(10),
            readability_score: 60.0,
            pacing_score: 0.0,
            top_adjectives: vec!["dark".to_string(), "cold".to_string()],
            top_verbs: vec!["ran".to_string(), "said".to_string()],
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_record_and_get_metrics() {
        let mem = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        let sm = StyleMemory::new(mem, "novel_1");

        let m = make_metrics(1, 1000);
        sm.record_chapter_metrics(m.clone()).await.unwrap();

        let retrieved = sm.get_chapter_metrics(1).await.unwrap().unwrap();
        assert_eq!(retrieved.chapter_number, 1);
        assert_eq!(retrieved.word_count, 1000);
    }

    #[tokio::test]
    async fn test_get_recent_metrics() {
        let mem = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        let sm = StyleMemory::new(mem, "novel_2");

        for i in 1..=5 {
            sm.record_chapter_metrics(make_metrics(i, i as u64 * 500)).await.unwrap();
        }

        let recent = sm.get_recent_metrics(3).await.unwrap();
        assert_eq!(recent.len(), 3);
        // Most recent chapters first
        assert_eq!(recent[0].chapter_number, 5);
        assert_eq!(recent[1].chapter_number, 4);
        assert_eq!(recent[2].chapter_number, 3);
    }

    #[tokio::test]
    async fn test_style_profile_creation() {
        let mem = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        let sm = StyleMemory::new(mem, "novel_3");

        for i in 1..=3 {
            sm.record_chapter_metrics(make_metrics(i, (1000 + i * 100) as u64)).await.unwrap();
        }

        let profile = sm.get_style_profile().await.unwrap().unwrap();
        assert_eq!(profile.novel_id, "novel_3");
        assert!(profile.dialogue_ratio > 0.0);
        assert!(profile.description_ratio > 0.0);
        // Typical word count range should cover chapters 1–3
        assert!(profile.typical_word_count.start <= 1100);
        assert!(profile.typical_word_count.end >= 1300);
    }

    #[tokio::test]
    async fn test_no_metrics_returns_none() {
        let mem = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        let sm = StyleMemory::new(mem, "novel_4");

        let chapter = sm.get_chapter_metrics(99).await.unwrap();
        assert!(chapter.is_none());

        let profile = sm.get_style_profile().await.unwrap();
        assert!(profile.is_none());
    }

    #[tokio::test]
    async fn test_style_drift_detection() {
        let mem = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        let sm = StyleMemory::new(mem, "novel_5");

        // Record 3 consistent chapters
        for i in 1..=3 {
            sm.record_chapter_metrics(make_metrics(i, 1000)).await.unwrap();
        }

        // Record a divergent chapter — significantly different metrics
        let mut divergent = make_metrics(4, 1000);
        divergent.word_count = 3000;
        divergent.sentence_count = 300;
        divergent.unique_words = 1500;
        divergent.readability_score = 85.0;
        divergent.pacing_score = 0.8;
        sm.record_chapter_metrics(divergent).await.unwrap();

        let drift = sm.detect_style_drift(4).await.unwrap().unwrap();
        assert!(drift.drift_score > 0.0);
        assert!(drift.drift_score <= 1.0);
        assert!(!drift.changed_metrics.is_empty());
        assert!(drift.changed_metrics.contains(&"word_count".to_string()));

        // Chapter 1 should return None (no preceding chapters)
        let drift_none = sm.detect_style_drift(1).await.unwrap();
        assert!(drift_none.is_none());
    }
}
