use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{Confidence, Fact, FactCategory, FactRecallOptions, FactSortField};
use crate::types::query::PageRequest;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Status of a plot arc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArcStatus {
    /// Arc is planned but has not yet begun.
    Planned,
    /// Arc is currently progressing.
    Active,
    /// Arc has been resolved or completed.
    Resolved,
    /// Arc has been abandoned.
    Abandoned,
}

/// A plot arc within a novel – a narrative thread with events and unresolved
/// story threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotArc {
    /// Unique identifier for this arc.
    pub arc_id: String,
    /// ID of the novel this arc belongs to.
    pub novel_id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the arc's purpose and content.
    pub description: String,
    /// Current status.
    pub status: ArcStatus,
    /// First chapter number (if known).
    pub start_chapter: Option<u32>,
    /// Final chapter number (if known).
    pub end_chapter: Option<u32>,
    /// Chapter numbers that belong to this arc.
    pub chapters_in_arc: Vec<u32>,
    /// Key plot events that have occurred.
    pub key_events: Vec<String>,
    /// Narrative threads still unresolved.
    pub unresolved_threads: Vec<String>,
    /// When the arc was created.
    pub created_at: DateTime<Utc>,
    /// When the arc was last modified.
    pub updated_at: DateTime<Utc>,
}

/// Progress snapshot for a single plot arc, derived from its resolved vs.
/// remaining narrative threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcProgress {
    /// The arc being measured.
    pub arc: PlotArc,
    /// Completion percentage (0.0 – 100.0).
    pub completion_pct: f32,
    /// Estimated chapters remaining based on current completion rate.
    pub estimated_chapters_remaining: u32,
    /// Number of narrative threads that have been resolved.
    pub threads_resolved: usize,
    /// Number of narrative threads still outstanding.
    pub threads_remaining: usize,
}

/// Context information for a story volume (e.g. a published book of a series).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeContext {
    /// Volume number within the novel.
    pub volume_number: u32,
    /// Title of the volume.
    pub title: String,
    /// Chapters contained in this volume.
    pub chapters: Vec<u32>,
    /// Plot arc IDs active in this volume.
    pub arcs_in_volume: Vec<String>,
    /// Summary of the volume's content.
    pub summary: String,
    /// Total word count for the volume.
    pub word_count: u64,
}

// ---------------------------------------------------------------------------
// PlotMemory
// ---------------------------------------------------------------------------

/// High-level memory interface for tracking plot arcs across a novel.
///
/// Each plot arc is stored as a [`Fact`] with
/// `category = Custom("PlotArc")`, using the novel's ID as the fact
/// `user_id`.  Volume contexts are stored similarly with
/// `category = Custom("VolumeContext")`.
#[derive(Debug)]
pub struct PlotMemory {
    memory: Arc<dyn MemorySystem>,
    novel_id: String,
}

impl PlotMemory {
    /// Create a new `PlotMemory` for the given novel.
    pub fn new(memory: Arc<dyn MemorySystem>, novel_id: &str) -> Self {
        Self { memory, novel_id: novel_id.to_string() }
    }

    // -- single arc operations -----------------------------------------------

    /// Insert or update a plot arc (upsert semantics by `arc_id`).
    pub async fn upsert_arc(&self, arc: PlotArc) -> Result<(), MemoryError> {
        let object = serde_json::to_string(&arc)
            .map_err(|e| MemoryError::serialization_with_source(e.to_string(), Box::new(e)))?;

        let now = epoch_millis();
        let fact = Fact {
            id: Uuid::new_v4().to_string(),
            user_id: self.novel_id.clone(),
            category: FactCategory::Custom("PlotArc".into()),
            subject: arc.arc_id.clone(),
            predicate: "plot_arc".into(),
            object,
            confidence: Confidence::High,
            source_session: None,
            created_at: now,
            updated_at: now,
            version: 1,
        };

        self.memory.remember_fact(fact).await?;
        Ok(())
    }

    /// Retrieve a single plot arc by its ID.
    pub async fn get_arc(&self, arc_id: &str) -> Result<Option<PlotArc>, MemoryError> {
        let opts = FactRecallOptions {
            page: PageRequest { offset: 0, limit: 10 },
            min_confidence: None,
            categories: Some(vec![FactCategory::Custom("PlotArc".into())]),
            sort_by: FactSortField::UpdatedAt,
        };
        let page = self.memory.recall_facts(&self.novel_id, arc_id, &opts).await?;
        let fact = page.items.into_iter().find(|f| f.subject == arc_id);
        match fact {
            Some(f) => deserialize_arc(f).map(Some),
            None => Ok(None),
        }
    }

    // -- collection queries --------------------------------------------------

    /// Return every plot arc for this novel.
    pub async fn get_all_arcs(&self) -> Result<Vec<PlotArc>, MemoryError> {
        let opts = FactRecallOptions {
            page: PageRequest { offset: 0, limit: 1000 },
            min_confidence: None,
            categories: Some(vec![FactCategory::Custom("PlotArc".into())]),
            sort_by: FactSortField::UpdatedAt,
        };
        let page = self.memory.recall_facts(&self.novel_id, "", &opts).await?;
        deserialize_arcs(page.items)
    }

    /// Return only arcs whose [`ArcStatus`] is `Active`.
    pub async fn get_active_arcs(&self) -> Result<Vec<PlotArc>, MemoryError> {
        let all = self.get_all_arcs().await?;
        Ok(all.into_iter().filter(|a| a.status == ArcStatus::Active).collect())
    }

    // -- progress ------------------------------------------------------------

    /// Calculate progress for a specific arc.
    ///
    /// The completion percentage is derived from the ratio of resolved threads
    /// (approximated by `key_events`) vs. total threads (key events +
    /// unresolved threads).  This heuristic is suitable for most creative
    /// writing workflows; callers may override it based on richer data.
    pub async fn get_arc_progress(&self, arc_id: &str) -> Result<Option<ArcProgress>, MemoryError> {
        let arc = match self.get_arc(arc_id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        let threads_remaining = arc.unresolved_threads.len();
        let threads_resolved = arc.key_events.len();
        let total = threads_resolved + threads_remaining;

        let completion_pct =
            if total > 0 { (threads_resolved as f32 / total as f32) * 100.0 } else { 100.0 };

        let chapters_done = arc.chapters_in_arc.len() as f32;
        let estimated_chapters_remaining =
            if completion_pct > 0.0 && completion_pct < 100.0 && chapters_done > 0.0 {
                let total_estimated = chapters_done / (completion_pct / 100.0);
                let remaining = total_estimated - chapters_done;
                if remaining > 0.0 { remaining.ceil() as u32 } else { 0 }
            } else if completion_pct >= 100.0 {
                0
            } else {
                10
            };

        Ok(Some(ArcProgress {
            arc,
            completion_pct,
            estimated_chapters_remaining,
            threads_resolved,
            threads_remaining,
        }))
    }

    // -- lifecycle updates ---------------------------------------------------

    /// Record a chapter write – each arc that already references
    /// `chapter_number` gets updated with the new data.
    ///
    /// * `chapter_number` – the chapter that was just written.
    /// * `resolved_threads` – narrative threads to remove from
    ///   `unresolved_threads`.
    /// * `new_events` – events to append to `key_events`.
    pub async fn update_after_chapter(
        &self,
        chapter_number: u32,
        resolved_threads: &[String],
        new_events: &[String],
    ) -> Result<(), MemoryError> {
        let all_arcs = self.get_all_arcs().await?;
        for mut arc in all_arcs {
            if !arc.chapters_in_arc.contains(&chapter_number)
                && !chapter_in_range(&arc, chapter_number)
            {
                continue;
            }

            // Append chapter if new.
            if !arc.chapters_in_arc.contains(&chapter_number) {
                arc.chapters_in_arc.push(chapter_number);
            }

            for thread in resolved_threads {
                arc.unresolved_threads.retain(|t| t != thread);
            }

            // Append new events (dedup).
            for event in new_events {
                if !arc.key_events.contains(event) {
                    arc.key_events.push(event.clone());
                }
            }

            arc.updated_at = Utc::now();
            self.upsert_arc(arc).await?;
        }

        Ok(())
    }

    // -- volume context ------------------------------------------------------

    /// Retrieve context for a given volume.
    ///
    /// Volume contexts are stored as facts with
    /// `category = Custom("VolumeContext")`.
    pub async fn get_volume_context(
        &self,
        volume_number: u32,
    ) -> Result<Option<VolumeContext>, MemoryError> {
        let subject = format!("vol_context:{}:{}", self.novel_id, volume_number);
        let opts = FactRecallOptions {
            page: PageRequest { offset: 0, limit: 10 },
            min_confidence: None,
            categories: Some(vec![FactCategory::Custom("VolumeContext".into())]),
            sort_by: FactSortField::UpdatedAt,
        };
        let page = self.memory.recall_facts(&self.novel_id, &subject, &opts).await?;
        let fact = page.items.into_iter().find(|f| f.subject == subject);
        match fact {
            Some(f) => {
                let ctx: VolumeContext = serde_json::from_str(&f.object).map_err(|e| {
                    MemoryError::serialization_with_source(e.to_string(), Box::new(e))
                })?;
                Ok(Some(ctx))
            }
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Check whether a chapter falls within an arc's declared range.
fn chapter_in_range(arc: &PlotArc, chapter: u32) -> bool {
    match (arc.start_chapter, arc.end_chapter) {
        (Some(start), Some(end)) => start <= chapter && chapter <= end,
        (Some(start), None) => start <= chapter,
        (None, Some(end)) => chapter <= end,
        (None, None) => false,
    }
}

/// Deserialize a single fact's `object` field into a `PlotArc`.
fn deserialize_arc(fact: Fact) -> Result<PlotArc, MemoryError> {
    serde_json::from_str(&fact.object)
        .map_err(|e| MemoryError::serialization_with_source(e.to_string(), Box::new(e)))
}

/// Deserialize a batch of facts.
fn deserialize_arcs(facts: Vec<Fact>) -> Result<Vec<PlotArc>, MemoryError> {
    facts.into_iter().map(deserialize_arc).collect()
}

/// Current time as milliseconds since Unix epoch.
fn epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::InMemoryMemory;

    fn make_plot_memory() -> PlotMemory {
        let memory = Arc::new(InMemoryMemory::new()) as Arc<dyn MemorySystem>;
        PlotMemory::new(memory, "novel-1")
    }

    fn sample_arc(arc_id: &str, status: ArcStatus) -> PlotArc {
        PlotArc {
            arc_id: arc_id.to_string(),
            novel_id: "novel-1".to_string(),
            name: format!("Arc {}", arc_id),
            description: "A test plot arc".into(),
            status,
            start_chapter: Some(1),
            end_chapter: None,
            chapters_in_arc: vec![1],
            key_events: vec![],
            unresolved_threads: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // -- test: upsert + get --------------------------------------------------

    #[tokio::test]
    async fn test_upsert_and_get_arc() {
        let pm = make_plot_memory();
        let arc = sample_arc("arc-1", ArcStatus::Planned);

        pm.upsert_arc(arc.clone()).await.expect("upsert should succeed");

        let retrieved = pm.get_arc("arc-1").await.expect("get should succeed");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Arc arc-1");
    }

    // -- test: get_active_arcs -----------------------------------------------

    #[tokio::test]
    async fn test_get_active_arcs() {
        let pm = make_plot_memory();

        let active = sample_arc("active-1", ArcStatus::Active);
        let resolved = sample_arc("resolved-1", ArcStatus::Resolved);
        let planned = sample_arc("planned-1", ArcStatus::Planned);

        pm.upsert_arc(active).await.unwrap();
        pm.upsert_arc(resolved).await.unwrap();
        pm.upsert_arc(planned).await.unwrap();

        let active_arcs = pm.get_active_arcs().await.expect("get_active_arcs");
        assert_eq!(active_arcs.len(), 1);
        assert_eq!(active_arcs[0].arc_id, "active-1");
    }

    // -- test: arc progress calculation --------------------------------------

    #[tokio::test]
    async fn test_arc_progress_calculation() {
        let pm = make_plot_memory();

        let arc = PlotArc {
            key_events: vec!["hero arrives".into(), "dragon sighted".into()],
            unresolved_threads: vec!["find the sword".into(), "defeat dragon".into()],
            chapters_in_arc: vec![1, 2],
            ..sample_arc("progress-1", ArcStatus::Active)
        };

        pm.upsert_arc(arc).await.unwrap();

        let progress = pm
            .get_arc_progress("progress-1")
            .await
            .expect("get_arc_progress")
            .expect("arc should exist");

        // 2 resolved (key_events) / 4 total = 50%
        assert!((progress.completion_pct - 50.0).abs() < f32::EPSILON);
        assert_eq!(progress.threads_resolved, 2);
        assert_eq!(progress.threads_remaining, 2);
    }

    // -- test: update_after_chapter ------------------------------------------

    #[tokio::test]
    async fn test_update_after_chapter() {
        let pm = make_plot_memory();

        let mut arc = sample_arc("update-1", ArcStatus::Active);
        arc.chapters_in_arc = vec![1, 2];
        arc.key_events = vec!["meet ally".into()];
        arc.unresolved_threads = vec!["find the sword".into(), "defeat dragon".into()];

        pm.upsert_arc(arc).await.unwrap();

        pm.update_after_chapter(2, &["find the sword".into()], &["found the sword".into()])
            .await
            .expect("update_after_chapter");

        let updated = pm.get_arc("update-1").await.expect("get_arc").expect("arc exists");

        // chapters_in_arc should still contain [1, 2] (2 already present)
        assert_eq!(updated.chapters_in_arc, vec![1, 2]);
        // resolved thread removed
        assert_eq!(updated.unresolved_threads, vec!["defeat dragon".to_string()]);
        // new event appended
        assert!(updated.key_events.contains(&"found the sword".into()));
    }
}
