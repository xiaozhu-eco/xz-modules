//! Agent trajectory recording and serialization.
//!
//! This module provides types for capturing the step-by-step execution
//! history of an autonomous agent, including thoughts, tool calls,
//! chapter completions, and safety interventions.
//!
//! # Example
//!
//! ```rust
//! use xz_agent::trajectory::{AgentTrajectory, TrajectoryAction};
//!
//! let mut trajectory = AgentTrajectory::new("session-1", "novel-42", 3);
//! trajectory.record_step(
//!     TrajectoryAction::Thought { content: "Analyzing plot...".into() },
//!     None,
//! );
//! trajectory.mark_completed();
//!
//! let log = trajectory.to_display_log();
//! println!("{}", log);
//! ```

use serde::{Deserialize, Serialize};

/// Represents a single action taken by the agent during its trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrajectoryAction {
    /// The agent produced an internal thought or reasoning step.
    Thought {
        /// The content of the thought.
        content: String,
    },
    /// The agent invoked an external tool.
    ToolCall {
        /// Name of the tool that was called.
        tool_name: String,
        /// Arguments passed to the tool as a JSON value.
        arguments: serde_json::Value,
        /// The result returned by the tool, if any.
        result: Option<String>,
        /// Duration of the tool call in milliseconds.
        duration_ms: u64,
    },
    /// A chapter was completed by the agent.
    ChapterComplete {
        /// The chapter number that was completed.
        chapter_number: u32,
        /// Total word count of the completed chapter.
        word_count: u64,
        /// The final verdict on the chapter, if one was rendered.
        verdict: Option<crate::safety::FinalVerdict>,
    },
    /// A safety check triggered an intervention.
    SafetyIntervention {
        /// The type of safety check that was performed.
        check_type: String,
        /// Description of the violation that was detected.
        violation: String,
        /// Severity level: `"warning"` or `"blocking"`.
        severity: String,
    },
}

/// A single step in the agent's execution trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    /// Monotonically increasing step number (1-based).
    pub step_number: u32,
    /// When this step occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// The action that was performed in this step.
    pub action: TrajectoryAction,
    /// The raw LLM output that triggered this step, if captured.
    pub llm_output: Option<String>,
}

impl TrajectoryStep {
    /// Returns a one-line human-readable summary of this step.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::{TrajectoryAction, TrajectoryStep};
    /// use chrono::Utc;
    ///
    /// let step = TrajectoryStep {
    ///     step_number: 1,
    ///     timestamp: Utc::now(),
    ///     action: TrajectoryAction::Thought { content: "hello".into() },
    ///     llm_output: None,
    /// };
    /// assert!(step.format_summary().contains("Step 1"));
    /// ```
    pub fn format_summary(&self) -> String {
        let action_summary = match &self.action {
            TrajectoryAction::Thought { .. } => "Thought".to_string(),
            TrajectoryAction::ToolCall { tool_name, duration_ms, .. } => {
                format!("ToolCall({tool_name}, {duration_ms}ms)")
            }
            TrajectoryAction::ChapterComplete {
                chapter_number,
                word_count,
                verdict,
            } => {
                let verdict_str = match verdict {
                    Some(v) if v.is_approved() => " [approved]",
                    Some(_) => " [rejected]",
                    None => "",
                };
                format!(
                    "ChapterComplete(ch={chapter_number}, {word_count} words{verdict_str})"
                )
            }
            TrajectoryAction::SafetyIntervention {
                check_type,
                severity,
                ..
            } => {
                format!("SafetyIntervention({check_type}, severity={severity})")
            }
        };
        format!("Step {}: {action_summary}", self.step_number)
    }
}

/// The complete execution trajectory of an autonomous agent session.
///
/// Captures every step taken by the agent, from thoughts and tool calls
/// to chapter completions and safety interventions. Supports serialization
/// to both JSON and human-readable log format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrajectory {
    /// Unique identifier for the agent session.
    pub session_id: String,
    /// Identifier for the novel being written.
    pub novel_id: String,
    /// The chapter number the agent is working on.
    pub chapter_number: u32,
    /// When the agent session was started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the agent session completed, if it has finished.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// All steps recorded during this trajectory.
    pub steps: Vec<TrajectoryStep>,
    /// Total number of tool calls made.
    pub total_tool_calls: u32,
    /// Total wall-clock duration of all tool calls in milliseconds.
    pub total_duration_ms: u64,
}

impl AgentTrajectory {
    /// Creates a new empty trajectory for a given session, novel, and chapter.
    ///
    /// The `started_at` timestamp is set to the current UTC time.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::AgentTrajectory;
    ///
    /// let trajectory = AgentTrajectory::new("sess-1", "novel-42", 3);
    /// assert_eq!(trajectory.session_id, "sess-1");
    /// assert_eq!(trajectory.novel_id, "novel-42");
    /// assert_eq!(trajectory.chapter_number, 3);
    /// assert!(trajectory.steps.is_empty());
    /// ```
    pub fn new(
        session_id: impl Into<String>,
        novel_id: impl Into<String>,
        chapter_number: u32,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            novel_id: novel_id.into(),
            chapter_number,
            started_at: chrono::Utc::now(),
            completed_at: None,
            steps: Vec::new(),
            total_tool_calls: 0,
            total_duration_ms: 0,
        }
    }

    /// Records a new step in the trajectory.
    ///
    /// Automatically assigns the next step number and sets the timestamp
    /// to the current UTC time. If the action is a [`TrajectoryAction::ToolCall`],
    /// the tool call counters are updated accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::{AgentTrajectory, TrajectoryAction};
    ///
    /// let mut trajectory = AgentTrajectory::new("sess-1", "novel-1", 1);
    /// trajectory.record_step(
    ///     TrajectoryAction::Thought { content: "thinking...".into() },
    ///     None,
    /// );
    /// assert_eq!(trajectory.steps.len(), 1);
    /// assert_eq!(trajectory.steps[0].step_number, 1);
    /// ```
    pub fn record_step(
        &mut self,
        action: TrajectoryAction,
        llm_output: Option<String>,
    ) {
        let step_number = (self.steps.len() as u32) + 1;

        // Update tool call counters if this is a tool call
        if let TrajectoryAction::ToolCall { duration_ms, .. } = &action {
            self.total_tool_calls += 1;
            self.total_duration_ms += duration_ms;
        }

        let step = TrajectoryStep {
            step_number,
            timestamp: chrono::Utc::now(),
            action,
            llm_output,
        };
        self.steps.push(step);
    }

    /// Formats the entire trajectory as a human-readable multi-line log.
    ///
    /// Each step is rendered with its step number, the action summary,
    /// and any associated LLM output.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::{AgentTrajectory, TrajectoryAction};
    ///
    /// let mut trajectory = AgentTrajectory::new("sess-1", "novel-1", 1);
    /// trajectory.record_step(
    ///     TrajectoryAction::Thought { content: "hello".into() },
    ///     None,
    /// );
    /// let log = trajectory.to_display_log();
    /// assert!(log.contains("Session: sess-1"));
    /// assert!(log.contains("Step 1"));
    /// ```
    pub fn to_display_log(&self) -> String {
        let mut log = String::new();

        // Header
        log.push_str(&format!(
            "=== Agent Trajectory Log ===\n\
             Session: {}\n\
             Novel:   {}\n\
             Chapter: {}\n\
             Started: {}\n",
            self.session_id, self.novel_id, self.chapter_number, self.started_at
        ));

        if let Some(completed_at) = self.completed_at {
            let dur = self.duration();
            log.push_str(&format!(
                "Completed: {}\n\
                 Duration:  {}ms\n",
                completed_at, dur.num_milliseconds()
            ));
        }

        log.push_str(&format!(
            "Steps:    {} | Tool calls: {} | Tool time: {}ms\n\
             ==============================\n",
            self.steps.len(),
            self.total_tool_calls,
            self.total_duration_ms
        ));

        // Steps
        if self.steps.is_empty() {
            log.push_str("(no steps recorded)\n");
        } else {
            for step in &self.steps {
                // Timestamp as HH:MM:SS.mmm
                let ts = step.timestamp.format("%H:%M:%S%.3f");
                log.push_str(&format!(
                    "[{ts}] {}",
                    step.format_summary()
                ));

                // Show action detail
                match &step.action {
                    TrajectoryAction::Thought { content } => {
                        log.push_str(&format!(": {content}"));
                    }
                    TrajectoryAction::ToolCall {
                        tool_name: _,
                        arguments,
                        result,
                        ..
                    } => {
                        log.push_str(&format!(
                            "\n  Args: {arguments}\n  Result: {}",
                            result.as_deref().unwrap_or("(none)")
                        ));
                    }
                    TrajectoryAction::ChapterComplete {
                        chapter_number,
                        word_count,
                        verdict,
                    } => {
                        log.push_str(&format!(
                            "\n  Chapter: {chapter_number}, Words: {word_count}"
                        ));
                        if let Some(v) = verdict {
                            log.push_str(&format!("\n  Verdict: {v:?}"));
                        }
                    }
                    TrajectoryAction::SafetyIntervention {
                        check_type,
                        violation,
                        severity,
                    } => {
                        log.push_str(&format!(
                            "\n  Type: {check_type}, Severity: {severity}\n  Violation: {violation}"
                        ));
                    }
                }

                if let Some(ref llm_output) = step.llm_output {
                    log.push_str(&format!("\n  LLM output: {llm_output}"));
                }

                log.push('\n');
            }
        }

        log
    }

    /// Serializes the trajectory to pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::{AgentTrajectory, TrajectoryAction};
    ///
    /// let mut trajectory = AgentTrajectory::new("sess-1", "novel-1", 1);
    /// trajectory.record_step(
    ///     TrajectoryAction::Thought { content: "hello".into() },
    ///     None,
    /// );
    /// let json = trajectory.to_json().expect("serialization should succeed");
    /// assert!(json.contains("\"session_id\""));
    /// ```
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Marks the trajectory as completed, recording the current time.
    ///
    /// Sets [`completed_at`](Self::completed_at) to the current UTC time.
    /// Safe to call multiple times — subsequent calls update the timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::trajectory::AgentTrajectory;
    ///
    /// let mut trajectory = AgentTrajectory::new("sess-1", "novel-1", 1);
    /// assert!(trajectory.completed_at.is_none());
    /// trajectory.mark_completed();
    /// assert!(trajectory.completed_at.is_some());
    /// assert!(trajectory.duration().num_milliseconds() >= 0);
    /// ```
    pub fn mark_completed(&mut self) {
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Returns the total duration of the trajectory.
    ///
    /// If the trajectory has been [marked completed](Self::mark_completed),
    /// returns the elapsed time between start and completion. Otherwise,
    /// returns the elapsed time from start to now.
    pub fn duration(&self) -> chrono::Duration {
        let end = self.completed_at.unwrap_or_else(chrono::Utc::now);
        end - self.started_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test trajectory.
    fn test_trajectory() -> AgentTrajectory {
        AgentTrajectory::new("test-session", "test-novel", 1)
    }

    #[test]
    fn test_record_thought() {
        let mut trajectory = test_trajectory();
        trajectory.record_step(
            TrajectoryAction::Thought {
                content: "I should write the opening scene.".into(),
            },
            None,
        );
        assert_eq!(trajectory.steps.len(), 1);
        assert_eq!(trajectory.steps[0].step_number, 1);
        match &trajectory.steps[0].action {
            TrajectoryAction::Thought { content } => {
                assert!(content.contains("opening scene"));
            }
            _ => panic!("expected Thought action"),
        }
    }

    #[test]
    fn test_record_tool_call() {
        let mut trajectory = test_trajectory();
        trajectory.record_step(
            TrajectoryAction::ToolCall {
                tool_name: "search_web".into(),
                arguments: serde_json::json!({"query": "Rust async patterns"}),
                result: Some("Found 42 results".into()),
                duration_ms: 150,
            },
            None,
        );
        assert_eq!(trajectory.steps.len(), 1);
        assert_eq!(trajectory.total_tool_calls, 1);
        assert_eq!(trajectory.total_duration_ms, 150);

        match &trajectory.steps[0].action {
            TrajectoryAction::ToolCall { tool_name, .. } => {
                assert_eq!(tool_name, "search_web");
            }
            _ => panic!("expected ToolCall action"),
        }
    }

    #[test]
    fn test_to_display_log() {
        let mut trajectory = test_trajectory();

        trajectory.record_step(
            TrajectoryAction::Thought {
                content: "Planning chapter structure.".into(),
            },
            None,
        );

        trajectory.record_step(
            TrajectoryAction::ToolCall {
                tool_name: "write_draft".into(),
                arguments: serde_json::json!({"section": "intro"}),
                result: Some("Draft written successfully".into()),
                duration_ms: 320,
            },
            None,
        );

        trajectory.record_step(
            TrajectoryAction::ChapterComplete {
                chapter_number: 1,
                word_count: 2500,
                verdict: Some(crate::safety::FinalVerdict::Approved),
            },
            None,
        );

        trajectory.mark_completed();
        let log = trajectory.to_display_log();

        // Verify header
        assert!(log.contains("Session: test-session"));
        assert!(log.contains("Novel:   test-novel"));
        assert!(log.contains("Chapter: 1"));

        // Verify steps
        assert!(log.contains("Step 1: Thought"));
        assert!(log.contains("Step 2: ToolCall(write_draft"));
        assert!(log.contains("Step 3: ChapterComplete"));

        // Verify step details
        assert!(log.contains("Planning chapter structure"));
        assert!(log.contains("Draft written successfully"));
        assert!(log.contains("2500 words"));
        assert!(log.contains("approved"));
    }

    #[test]
    fn test_to_json_roundtrip() {
        let mut trajectory = test_trajectory();

        trajectory.record_step(
            TrajectoryAction::Thought {
                content: "Starting chapter.".into(),
            },
            Some("raw llm response".into()),
        );

        trajectory.record_step(
            TrajectoryAction::ToolCall {
                tool_name: "generate_text".into(),
                arguments: serde_json::json!({"prompt": "write story"}),
                result: Some("Once upon a time...".into()),
                duration_ms: 500,
            },
            None,
        );

        // Serialize
        let json = trajectory.to_json().expect("serialization should succeed");
        assert!(json.contains("\"session_id\""));
        assert!(json.contains("\"novel_id\""));
        assert!(json.contains("\"Thought\""));
        assert!(json.contains("\"generate_text\""));
        assert!(json.contains("raw llm response"));

        // Deserialize back
        let deserialized: AgentTrajectory =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.session_id, trajectory.session_id);
        assert_eq!(deserialized.novel_id, trajectory.novel_id);
        assert_eq!(deserialized.chapter_number, trajectory.chapter_number);
        assert_eq!(deserialized.steps.len(), trajectory.steps.len());
        assert_eq!(deserialized.total_tool_calls, trajectory.total_tool_calls);
        assert_eq!(deserialized.total_duration_ms, trajectory.total_duration_ms);
    }

    #[test]
    fn test_mark_completed() {
        let mut trajectory = test_trajectory();
        assert!(trajectory.completed_at.is_none());

        trajectory.record_step(
            TrajectoryAction::Thought {
                content: "Done.".into(),
            },
            None,
        );

        trajectory.mark_completed();
        assert!(trajectory.completed_at.is_some());
        assert!(trajectory.duration().num_milliseconds() >= 0);
    }

    #[test]
    fn test_format_summary_thought() {
        use chrono::Utc;
        let step = TrajectoryStep {
            step_number: 5,
            timestamp: Utc::now(),
            action: TrajectoryAction::Thought {
                content: "test thought".into(),
            },
            llm_output: None,
        };
        let summary = step.format_summary();
        assert!(summary.contains("Step 5"));
        assert!(summary.contains("Thought"));
    }

    #[test]
    fn test_format_summary_tool_call() {
        use chrono::Utc;
        let step = TrajectoryStep {
            step_number: 2,
            timestamp: Utc::now(),
            action: TrajectoryAction::ToolCall {
                tool_name: "fetch".into(),
                arguments: serde_json::json!({}),
                result: None,
                duration_ms: 100,
            },
            llm_output: None,
        };
        let summary = step.format_summary();
        assert!(summary.contains("Step 2"));
        assert!(summary.contains("ToolCall(fetch, 100ms)"));
    }

    #[test]
    fn test_format_summary_chapter_complete() {
        use chrono::Utc;
        let step = TrajectoryStep {
            step_number: 3,
            timestamp: Utc::now(),
            action: TrajectoryAction::ChapterComplete {
                chapter_number: 2,
                word_count: 5000,
                verdict: Some(crate::safety::FinalVerdict::Approved),
            },
            llm_output: None,
        };
        let summary = step.format_summary();
        assert!(summary.contains("Step 3"));
        assert!(summary.contains("ChapterComplete"));
        assert!(summary.contains("5000 words"));
        assert!(summary.contains("approved"));
    }

    #[test]
    fn test_format_summary_safety_intervention() {
        use chrono::Utc;
        let step = TrajectoryStep {
            step_number: 1,
            timestamp: Utc::now(),
            action: TrajectoryAction::SafetyIntervention {
                check_type: "content_filter".into(),
                violation: "inappropriate language".into(),
                severity: "blocking".into(),
            },
            llm_output: None,
        };
        let summary = step.format_summary();
        assert!(summary.contains("Step 1"));
        assert!(summary.contains("SafetyIntervention"));
        assert!(summary.contains("content_filter"));
        assert!(summary.contains("blocking"));
    }

    #[test]
    fn test_multiple_tool_calls_accumulate() {
        let mut trajectory = test_trajectory();
        trajectory.record_step(
            TrajectoryAction::ToolCall {
                tool_name: "tool_a".into(),
                arguments: serde_json::json!({}),
                result: None,
                duration_ms: 100,
            },
            None,
        );
        trajectory.record_step(
            TrajectoryAction::ToolCall {
                tool_name: "tool_b".into(),
                arguments: serde_json::json!({}),
                result: None,
                duration_ms: 200,
            },
            None,
        );
        assert_eq!(trajectory.total_tool_calls, 2);
        assert_eq!(trajectory.total_duration_ms, 300);
    }

    #[test]
    fn test_final_verdict_display() {
        let approved = crate::safety::FinalVerdict::Approved;
        let rejected = crate::safety::FinalVerdict::Rejected { violations: vec![] };
        assert!(approved.is_approved());
        assert!(!rejected.is_approved());
    }
}
