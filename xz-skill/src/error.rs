use std::fmt::Debug;

use crate::types::skill::SkillPermission;

/// Skill system errors
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// The requested skill was not found.
    #[error("Skill not found: {0}")]
    NotFound(String),

    /// Execution exceeded the configured timeout.
    #[error("Execution timed out ({0}ms)")]
    Timeout(u64),

    /// The operation was denied due to insufficient permissions.
    #[error("Insufficient permissions: requires {required:?}")]
    PermissionDenied {
        /// The permissions that would be required.
        required: Vec<SkillPermission>,
    },

    /// A tool failed during execution.
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    /// An error occurred in the WASM runtime.
    #[error("WASM error: {0}")]
    Wasm(String),

    /// Configuration validation failed.
    #[error("Configuration validation failed: {0}")]
    ConfigValidation(String),

    /// The skill requires a newer agent version.
    #[error("Version mismatch: skill requires >= {required}")]
    VersionMismatch {
        /// The minimum required version.
        required: String,
    },

    /// A preflight check failed before execution.
    #[error("Preflight check failed: {0}")]
    PreflightFailed(String),

    /// The skill is disabled and cannot be used.
    #[error("Skill is disabled: {0}")]
    Disabled(String),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse YAML content.
    #[error("YAML parse error: {0}")]
    Yaml(String),

    /// The WASM binary is invalid.
    #[error("Invalid WASM binary: {0}")]
    InvalidWasm(String),

    /// An HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(String),

    /// A parse error occurred while reading a skill definition file.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// A required field is missing from a skill definition.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// The skill definition file had an invalid format.
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

impl SkillError {
    /// Returns `true` if this error can be retried.
    ///
    /// Transient errors (timeout, tool execution failure, HTTP failures,
    /// IO errors) are retryable. All other errors are permanent and
    /// should not be retried without intervention.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SkillError::Timeout(_)
                | SkillError::ToolExecution(_)
                | SkillError::Http(_)
                | SkillError::Io(_)
        )
    }
}
