#![warn(missing_docs)]

//! Skill plugin system — skill registration, execution, and sandbox.
//!
//! This crate provides the core abstractions and runtime for managing "skills"
//! (plugin-like capabilities that extend the agent's functionality). It supports:
//!
//! - **Skill definition & discovery**: YAML-based skill manifests, file/sqlite registries
//! - **Execution sandboxing**: WASM runtime with configurable permissions
//! - **Pipeline orchestration**: Ordered step pipelines with caching
//! - **Tool execution**: HTTP tools, WASM-powered tools, and custom executors
//! - **Security**: Permission validation and sandbox configuration
//! - **Frontmatter parsing**: Parse `SKILL.md` files into [`SkillDefinition`] structs
//!
//! # Feature flags
//!
//! - `wasm-runtime`: Enable WASM-based skill execution (default)
//! - `http-tool`: Enable HTTP-based tool execution
//! - `sqlite-registry`: Enable SQLite-backed skill registry
//! - `hot-reload`: Enable file-watch based hot reloading
//! - `test-utils`: Utilities for integration testing
//!
//! # Note
//! This crate uses `#![warn(missing_docs)]` to encourage documentation.
//! Upgrade to `#![deny(missing_docs)]` once all public items are documented.

pub mod cache;
pub mod error;
pub mod pipeline;
pub mod registry;
pub mod runtime;
pub mod security;
pub mod traits;
pub mod types;
pub mod validation;

// Re-exports
pub use cache::tool_cache::ToolResultCache;
pub use error::SkillError;
pub use pipeline::step::{PipelineStep, SkillPipeline};
pub use registry::file::FileSkillRegistry;
pub use runtime::default::DefaultSkillRuntime;
pub use security::permissions::PermissionValidator;
pub use security::sandbox::SandboxConfig;
pub use traits::{SkillRegistry, SkillRuntime};
pub use types::context::ExecutionContext;
pub use types::filter::{PageRequest, PreflightWarning, SkillFilter, WarningSeverity};
pub use types::output::{SkillOutput, SkillSummary, TokenUsage, ToolCallRecord};
pub use types::skill::{
    Skill, SkillPermission, ToolDefinition, ToolType, UpsertResult,
};
pub use types::skill_def::{parse_skill_frontmatter, SkillDefinition};
pub use validation::validate_wasm;

#[cfg(feature = "sqlite-registry")]
pub use registry::sqlite::SqliteSkillRegistry;
#[cfg(feature = "wasm-runtime")]
pub use runtime::wasm::{WasmConfig, WasmRuntime};
#[cfg(feature = "http-tool")]
pub use runtime::http::HttpToolExecutor;
