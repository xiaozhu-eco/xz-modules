use async_trait::async_trait;
use std::fmt::Debug;

use crate::error::SkillError;
use crate::types::context::ExecutionContext;
use crate::types::filter::{PreflightWarning, SkillFilter};
use crate::types::output::{SkillOutput, SkillSummary};
use crate::types::skill::{Skill, UpsertResult};

/// Skill lifecycle management: register, unregister, enable, disable, search, list, count.
#[async_trait]
pub trait SkillRegistry: Send + Sync + Debug {
    /// Register or update a skill. Returns whether it was created, updated, or unchanged.
    async fn register(&self, skill: Skill) -> Result<UpsertResult, SkillError>;

    /// Remove a skill by ID.
    async fn unregister(&self, id: &str) -> Result<(), SkillError>;

    /// Get a full Skill by ID.
    async fn get(&self, id: &str) -> Result<Option<Skill>, SkillError>;

    /// List skill summaries with optional filtering and pagination.
    async fn list(&self, filter: &SkillFilter) -> Result<Vec<SkillSummary>, SkillError>;

    /// Free-text search across skill names, descriptions, authors.
    async fn search(&self, query: &str) -> Result<Vec<SkillSummary>, SkillError>;

    /// Enable or disable a skill by ID. Disabled skills cannot be executed.
    async fn enable(&self, id: &str, enabled: bool) -> Result<(), SkillError>;

    /// Total number of registered skills.
    async fn count(&self) -> Result<usize, SkillError>;
}

/// Skill execution: prompt injection, tool-calling loop, permission validation.
#[async_trait]
pub trait SkillRuntime: Send + Sync + Debug {
    /// Execute a skill: injects the skill prompt into the LLM context,
    /// runs the tool-calling loop, and returns the final output.
    async fn execute(
        &self,
        skill_id: &str,
        input: &str,
        context: &ExecutionContext,
    ) -> Result<SkillOutput, SkillError>;

    /// Execute a single named tool directly (bypasses LLM prompt injection).
    async fn execute_tool(&self, tool_name: &str, args: serde_json::Value)
        -> Result<serde_json::Value, SkillError>;

    /// Validate that the execution context satisfies all permissions required by the skill.
    async fn validate_permissions(
        &self,
        skill: &Skill,
        context: &ExecutionContext,
    ) -> Result<(), SkillError>;

    /// Run preflight checks on a skill before it is enabled or executed.
    /// Returns a list of warnings (non-blocking) or errors (blocking).
    async fn preflight_check(&self, skill: &Skill) -> Result<Vec<PreflightWarning>, SkillError>;
}
