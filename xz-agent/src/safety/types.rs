use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of safety checks that can be performed on agent execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetyCheckType {
    /// Maximum number of tool calls allowed per execution.
    MaxToolCalls,
    /// Maximum number of revision rounds allowed.
    MaxRevisionRounds,
    /// Minimum required output length for agent responses.
    MinOutputLength,
    /// Mandatory seed phrases that must appear in the output.
    MandatorySeeds,
    /// Excessive external queries detected.
    ExcessiveQueries,
    /// Maximum allowed execution duration.
    MaxDuration,
    /// Maximum number of tool calls allowed per single loop iteration.
    MaxToolCallRounds,
}

/// Severity level of a safety rule or violation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetySeverity {
    /// Non-blocking warning; execution may proceed with advisory notice.
    Warning,
    /// Blocking violation; execution must be halted.
    Blocking,
}

/// A configurable safety rule that governs agent execution constraints.
///
/// Each rule specifies what to check, how severe a violation is, the threshold
/// that triggers the rule, and a human-readable description.
///
/// The `threshold` field uses [`serde_json::Value`] for flexibility:
/// - For count-based checks (e.g., `MaxToolCalls`): a JSON number (`u32`)
/// - For duration-based checks (e.g., `MaxDuration`): a JSON string (e.g., `"30s"`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    /// The type of safety check this rule performs.
    pub check_type: SafetyCheckType,
    /// Severity level when this rule is violated.
    pub severity: SafetySeverity,
    /// Whether this rule is currently active.
    pub enabled: bool,
    /// Threshold value that triggers the rule.
    /// Can hold a count (`serde_json::Value::Number`) or a duration string.
    pub threshold: serde_json::Value,
    /// Human-readable description of what this rule checks.
    pub description: String,
}

impl SafetyRule {
    /// Creates a new [`SafetyRule`] with the given configuration.
    ///
    /// The rule is enabled by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::safety::{SafetyCheckType, SafetyRule, SafetySeverity};
    ///
    /// let rule = SafetyRule::new(
    ///     SafetyCheckType::MaxToolCalls,
    ///     SafetySeverity::Blocking,
    ///     serde_json::json!(50),
    ///     "Maximum 50 tool calls per execution",
    /// );
    /// assert!(rule.enabled);
    /// ```
    pub fn new(
        check_type: SafetyCheckType,
        severity: SafetySeverity,
        threshold: serde_json::Value,
        description: impl Into<String>,
    ) -> Self {
        Self {
            check_type,
            severity,
            enabled: true,
            threshold,
            description: description.into(),
        }
    }

    /// Sets the enabled state of this rule (builder-pattern style).
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::safety::{SafetyCheckType, SafetyRule, SafetySeverity};
    ///
    /// let rule = SafetyRule::new(
    ///     SafetyCheckType::MaxDuration,
    ///     SafetySeverity::Warning,
    ///     serde_json::json!("30s"),
    ///     "Max 30 seconds duration",
    /// )
    /// .with_enabled(false);
    ///
    /// assert!(!rule.enabled);
    /// ```
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Records a single safety violation that occurred during execution.
///
/// Contains the rule that was violated, the severity, a descriptive message,
/// the actual value at the time of violation, the configured threshold,
/// and a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafetyViolation {
    /// The type of safety check that was violated.
    pub rule: SafetyCheckType,
    /// Severity level of this violation.
    pub severity: SafetySeverity,
    /// Human-readable message describing the violation.
    pub message: String,
    /// The actual value observed at the time of violation.
    pub current_value: serde_json::Value,
    /// The threshold that was exceeded.
    pub threshold: serde_json::Value,
    /// When this violation occurred.
    pub timestamp: DateTime<Utc>,
}

/// The final verdict after evaluating all safety rules for an execution.
///
/// # Variants
///
/// * `Approved` — No violations; execution is clear to proceed.
/// * `ApprovedWithWarnings` — Non-blocking warnings were raised; execution
///   may proceed with advisory notes.
/// * `Rejected` — Blocking violations were detected; execution must not proceed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinalVerdict {
    /// No violations; execution is approved.
    Approved,
    /// Approved with non-blocking warnings.
    ApprovedWithWarnings {
        /// The warnings that were raised.
        warnings: Vec<SafetyViolation>,
    },
    /// Rejected due to blocking violations.
    Rejected {
        /// The violations that caused the rejection.
        violations: Vec<SafetyViolation>,
    },
}

impl FinalVerdict {
    /// Returns `true` if the verdict allows execution to proceed.
    ///
    /// Both `Approved` and `ApprovedWithWarnings` return `true`.
    /// Only `Rejected` returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use xz_agent::safety::FinalVerdict;
    ///
    /// assert!(FinalVerdict::Approved.is_approved());
    /// ```
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved | Self::ApprovedWithWarnings { .. })
    }

    /// Returns all warnings, if any.
    ///
    /// Returns an empty vec for `Approved` and `Rejected` variants that
    /// do not carry warnings.
    pub fn warnings(&self) -> Vec<&SafetyViolation> {
        match self {
            Self::ApprovedWithWarnings { warnings } => warnings.iter().collect(),
            _ => vec![],
        }
    }

    /// Returns all blocking violations, if any.
    ///
    /// Returns an empty vec for `Approved` and `ApprovedWithWarnings` variants
    /// that do not carry violations.
    pub fn violations(&self) -> Vec<&SafetyViolation> {
        match self {
            Self::Rejected { violations } => violations.iter().collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that a [`SafetyRule`] can be created and its fields are set correctly.
    #[test]
    fn test_safety_rule_creation() {
        let rule = SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(50),
            "Maximum 50 tool calls per execution",
        );

        assert_eq!(rule.check_type, SafetyCheckType::MaxToolCalls);
        assert_eq!(rule.severity, SafetySeverity::Blocking);
        assert!(rule.enabled);
        assert_eq!(rule.threshold, serde_json::json!(50));
        assert_eq!(rule.description, "Maximum 50 tool calls per execution");
    }

    /// Verifies that a [`SafetyViolation`] serializes and deserializes correctly,
    /// preserving all field values including the timestamp.
    #[test]
    fn test_safety_violation_serde() {
        let violation = SafetyViolation {
            rule: SafetyCheckType::MaxToolCalls,
            severity: SafetySeverity::Blocking,
            message: "Exceeded maximum tool calls".to_string(),
            current_value: serde_json::json!(75),
            threshold: serde_json::json!(50),
            timestamp: DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&violation).expect("serialization should succeed");
        let deserialized: SafetyViolation =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.rule, SafetyCheckType::MaxToolCalls);
        assert_eq!(deserialized.severity, SafetySeverity::Blocking);
        assert_eq!(deserialized.message, "Exceeded maximum tool calls");
        assert_eq!(deserialized.current_value, serde_json::json!(75));
        assert_eq!(deserialized.threshold, serde_json::json!(50));
        assert_eq!(
            deserialized.timestamp.to_rfc3339(),
            "2025-01-01T00:00:00+00:00"
        );
    }

    /// Verifies that `Approved` verdict reports `is_approved() == true` and
    /// returns empty warnings and violations.
    #[test]
    fn test_final_verdict_approved() {
        let verdict = FinalVerdict::Approved;

        assert!(verdict.is_approved());
        assert!(verdict.warnings().is_empty());
        assert!(verdict.violations().is_empty());
    }

    /// Verifies that `Rejected` verdict reports `is_approved() == false`
    /// and correctly exposes the contained violations.
    #[test]
    fn test_final_verdict_rejected() {
        let violation = SafetyViolation {
            rule: SafetyCheckType::MaxToolCalls,
            severity: SafetySeverity::Blocking,
            message: "Exceeded maximum tool calls".to_string(),
            current_value: serde_json::json!(75),
            threshold: serde_json::json!(50),
            timestamp: Utc::now(),
        };

        let verdict = FinalVerdict::Rejected {
            violations: vec![violation],
        };

        assert!(!verdict.is_approved());
        assert_eq!(verdict.violations().len(), 1);
        assert_eq!(
            verdict.violations()[0].message,
            "Exceeded maximum tool calls"
        );
        assert!(verdict.warnings().is_empty());
    }

    /// Verifies that each [`SafetyCheckType`] variant survives serde roundtrip.
    #[test]
    fn test_safety_check_type_serde() {
        let variants = [
            SafetyCheckType::MaxToolCalls,
            SafetyCheckType::MaxRevisionRounds,
            SafetyCheckType::MinOutputLength,
            SafetyCheckType::MandatorySeeds,
            SafetyCheckType::ExcessiveQueries,
            SafetyCheckType::MaxDuration,
        ];

        for variant in &variants {
            let json = serde_json::to_string(variant).expect("serialization should succeed");
            let deserialized: SafetyCheckType =
                serde_json::from_str(&json).expect("deserialization should succeed");
            assert_eq!(&deserialized, variant);
        }
    }
}
