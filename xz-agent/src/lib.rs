//! Agent scheduling system with DAG execution and multi-trigger support.
//!
//! Provides autonomous agent loop, conversation management, DAG-based task scheduling,
//! tool integration, safety checks, and trajectory tracking.
//!
//! # Features
//!
//! - `code-exec`: WASM-based code execution sandbox (via `wasmtime`)
//! - `web-search`: Web search capability (via `xz-search`)
//! - `web-extract`: Web page extraction (via `reqwest`)
//! - `skill-integration`: Skill plugin integration (via `xz-skill`)
//! - `system-notify`: System desktop notifications (via `notify-rust`)

#![deny(missing_docs)]

pub mod action;
pub mod autonomous;
pub mod conversation;
pub mod error;
pub mod executor;
pub mod fork;
pub mod safety;
pub mod scheduler;
pub mod tool;
pub mod traits;
pub mod trajectory;
pub mod trigger;
pub mod types;

// Re-exports
pub use autonomous::{AutonomousConfig, AutonomousLoop, AutonomousResult};
pub use conversation::{ConversationConfig, ConversationManager, ConversationResponse};
pub use error::AgentError;
pub use tool::{AgentTool, ToolContext, ToolOutput, ToolRegistry};
pub use traits::AgentScheduler;
pub use types::agent::{Agent, AgentConfig, AgentTrigger};
pub use types::result::{AgentRunResult, StepResult, TokenUsage};
pub use types::status::{AgentFilter, AgentStatus, PageRequest, UpsertResult};
pub use types::step::{
    AgentAction, AgentStep, NotificationMethod, OnFailure, ReportFormat,
};

// Safety re-exports
pub use safety::{FinalVerdict, SafetyCheckContext, SafetyCheckType, SafetyGuard, SafetyReport, SafetyRule, SafetySeverity, SafetyViolation};

// Trajectory re-exports
pub use trajectory::{AgentTrajectory, TrajectoryAction, TrajectoryStep};

// Scheduler re-exports
pub use scheduler::config::SchedulerConfig;
pub use scheduler::memory::InMemoryAgentScheduler;

// Executor re-exports
pub use executor::dag::{topological_sort, ExecutionContext};
pub use executor::retry::execute_with_retry;

// Trigger re-exports
pub use trigger::cron::CronTrigger;
pub use trigger::event::EventTrigger;
pub use trigger::interval::IntervalTrigger;

// Fork re-exports
pub use fork::{ForkConfig, ForkHandle, ForkManager, ForkResult, ForkStatus};
