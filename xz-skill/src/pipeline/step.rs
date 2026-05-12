use serde::{Deserialize, Serialize};

/// A skill pipeline chains multiple skills together in sequence or parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPipeline {
    pub steps: Vec<PipelineStep>,
}

/// A single step in a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineStep {
    /// Execute a skill by ID.
    #[serde(rename = "skill")]
    Skill { id: String },

    /// Conditional branching based on a field value in the previous step's output.
    #[serde(rename = "condition")]
    Condition {
        field: String,
        op: String,
        value: String,
        then: Vec<PipelineStep>,
        #[serde(rename = "else")]
        else_: Vec<PipelineStep>,
    },

    /// Execute multiple branches in parallel (fan-out).
    #[serde(rename = "parallel")]
    Parallel { branches: Vec<Vec<PipelineStep>> },
}
