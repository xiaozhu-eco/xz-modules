/// Safety types for agent execution constraints and violation tracking.
pub mod types;

/// Safety guard rule engine for checking agent execution against defined rules.
pub mod guard;

pub use types::{
    FinalVerdict, SafetyCheckType, SafetyRule, SafetySeverity, SafetyViolation,
};

pub use guard::{SafetyCheckContext, SafetyGuard, SafetyReport};
