use std::fmt::Debug;

use crate::types::skill::SkillPermission;

/// Skill system errors
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Execution timed out ({0}ms)")]
    Timeout(u64),

    #[error("Insufficient permissions: requires {required:?}")]
    PermissionDenied {
        required: Vec<SkillPermission>,
    },

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("WASM error: {0}")]
    Wasm(String),

    #[error("Configuration validation failed: {0}")]
    ConfigValidation(String),

    #[error("Version mismatch: skill requires >= {required}")]
    VersionMismatch { required: String },

    #[error("Preflight check failed: {0}")]
    PreflightFailed(String),

    #[error("Skill is disabled: {0}")]
    Disabled(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(String),

    #[error("HTTP error: {0}")]
    Http(String),
}
