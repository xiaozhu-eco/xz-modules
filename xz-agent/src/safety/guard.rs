use crate::safety::types::{SafetyCheckType, SafetyRule, SafetySeverity, SafetyViolation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configurable safety rule engine that checks agent execution against
/// defined rules and blocks/pauses when violations occur.
///
/// # Example
///
/// ```rust
/// use xz_agent::safety::{SafetyGuard, SafetyRule, SafetyCheckType, SafetySeverity};
///
/// let guard = SafetyGuard::default();
/// let violations = guard.check_tool_calls(60);
/// assert_eq!(violations.len(), 1);
/// ```
pub struct SafetyGuard {
    rules: Vec<SafetyRule>,
}

impl SafetyGuard {
    /// Creates a new [`SafetyGuard`] from the given list of rules.
    pub fn new(rules: Vec<SafetyRule>) -> Self {
        Self { rules }
    }

    /// Creates a [`SafetyGuard`] pre-populated with sensible default rules:
    ///
    /// | Rule              | Threshold | Severity |
    /// |-------------------|-----------|----------|
    /// | `MaxToolCalls`    | 50        | Blocking |
    /// | `MaxRevisionRounds` | 5       | Blocking |
    /// | `MinOutputLength` | 100       | Warning  |
    /// | `MaxDuration`      | `"600s"`  | Warning  |
    /// | `ExcessiveQueries`  | 0.7      | Warning  |
    /// | `MaxToolCallRounds` | 50       | Blocking |
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        let rules = vec![
            SafetyRule::new(
                SafetyCheckType::MaxToolCalls,
                SafetySeverity::Blocking,
                serde_json::json!(50),
                "Maximum 50 tool calls per execution",
            ),
            SafetyRule::new(
                SafetyCheckType::MaxRevisionRounds,
                SafetySeverity::Blocking,
                serde_json::json!(5),
                "Maximum 5 revision rounds per execution",
            ),
            SafetyRule::new(
                SafetyCheckType::MinOutputLength,
                SafetySeverity::Warning,
                serde_json::json!(100),
                "Output must be at least 100 characters",
            ),
            SafetyRule::new(
                SafetyCheckType::MaxDuration,
                SafetySeverity::Warning,
                serde_json::json!("600s"),
                "Maximum 600 seconds execution duration",
            ),
            SafetyRule::new(
                SafetyCheckType::ExcessiveQueries,
                SafetySeverity::Warning,
                serde_json::json!(0.7),
                "Tool call ratio must not exceed 0.7 of total steps",
            ),
            SafetyRule::new(
                SafetyCheckType::MaxToolCallRounds,
                SafetySeverity::Blocking,
                serde_json::json!(50),
                "Maximum 50 tool calls per loop iteration",
            ),
        ];
        Self { rules }
    }

    /// Adds a new safety rule to the guard.
    pub fn add_rule(&mut self, rule: SafetyRule) {
        self.rules.push(rule);
    }

    /// Removes all safety rules matching the given [`SafetyCheckType`].
    pub fn remove_rule(&mut self, check_type: SafetyCheckType) {
        self.rules.retain(|r| r.check_type != check_type);
    }

    /// Checks if the current tool call count exceeds any `MaxToolCalls` rule.
    ///
    /// Returns a violation for each enabled `MaxToolCalls` rule whose
    /// threshold is exceeded by `current`.
    pub fn check_tool_calls(&self, current: u32) -> Vec<SafetyViolation> {
        self.check_count(SafetyCheckType::MaxToolCalls, current)
    }

    /// Checks if the current revision round count exceeds any
    /// `MaxRevisionRounds` rule.
    ///
    /// Returns a violation for each enabled `MaxRevisionRounds` rule whose
    /// threshold is exceeded by `current`.
    pub fn check_revision_rounds(&self, current: u32) -> Vec<SafetyViolation> {
        self.check_count(SafetyCheckType::MaxRevisionRounds, current)
    }

    /// Checks if the output length is below the `MinOutputLength` threshold.
    ///
    /// The `min_threshold` is the minimum acceptable length. If `current` is
    /// less than `min_threshold`, a warning violation is emitted for each
    /// enabled `MinOutputLength` rule.
    pub fn check_output_length(&self, current: u64, min_threshold: u64) -> Vec<SafetyViolation> {
        if current >= min_threshold {
            return vec![];
        }

        self.rules
            .iter()
            .filter(|r| r.check_type == SafetyCheckType::MinOutputLength && r.enabled)
            .map(|rule| SafetyViolation {
                rule: rule.check_type.clone(),
                severity: rule.severity.clone(),
                message: format!(
                    "{} — actual: {current}, required: {min_threshold}",
                    rule.description,
                ),
                current_value: serde_json::json!(current),
                threshold: serde_json::json!(min_threshold),
                timestamp: Utc::now(),
            })
            .collect()
    }

    /// Checks if the elapsed duration since `start` exceeds any `MaxDuration`
    /// rule.
    ///
    /// Parses each enabled rule's threshold string (e.g., `"600s"`, `"5m"`,
    /// `"1h"`) into seconds and compares against the wall-clock elapsed time.
    pub fn check_duration(&self, start: &DateTime<Utc>) -> Vec<SafetyViolation> {
        let elapsed = Utc::now() - *start;

        self.rules
            .iter()
            .filter(|r| r.check_type == SafetyCheckType::MaxDuration && r.enabled)
            .filter_map(|rule| {
                let threshold_secs = parse_duration_secs(&rule.threshold)?;
                if elapsed.num_seconds() > threshold_secs {
                    Some(SafetyViolation {
                        rule: rule.check_type.clone(),
                        severity: rule.severity.clone(),
                        message: format!(
                            "{} — elapsed: {}s, limit: {}s",
                            rule.description,
                            elapsed.num_seconds(),
                            threshold_secs,
                        ),
                        current_value: serde_json::json!(elapsed.num_seconds()),
                        threshold: serde_json::json!(threshold_secs),
                        timestamp: Utc::now(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Checks if the ratio of `tool_calls` to `total_steps` exceeds the
    /// `ExcessiveQueries` threshold ratio.
    ///
    /// When `total_steps` is zero the check is skipped (returns empty).
    pub fn check_excessive_queries(&self, tool_calls: u32, total_steps: u32) -> Vec<SafetyViolation> {
        if total_steps == 0 {
            return vec![];
        }

        let ratio = f64::from(tool_calls) / f64::from(total_steps);

        self.rules
            .iter()
            .filter(|r| r.check_type == SafetyCheckType::ExcessiveQueries && r.enabled)
            .filter_map(|rule| {
                let threshold = rule.threshold.as_f64()?;
                if ratio > threshold {
                    Some(SafetyViolation {
                        rule: rule.check_type.clone(),
                        severity: rule.severity.clone(),
                        message: format!(
                            "{} — ratio: {ratio:.3}, limit: {threshold}",
                            rule.description,
                        ),
                        current_value: serde_json::json!(ratio),
                        threshold: rule.threshold.clone(),
                        timestamp: Utc::now(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Checks if the current tool call round count exceeds any
    /// `MaxToolCallRounds` rule.
    ///
    /// Returns a violation for each enabled `MaxToolCallRounds` rule whose
    /// threshold is exceeded by `current`.
    pub fn check_tool_call_rounds(&self, current: u32) -> Vec<SafetyViolation> {
        self.check_count(SafetyCheckType::MaxToolCallRounds, current)
    }

    /// Runs **all** enabled rules against the provided
    /// [`SafetyCheckContext`] and collects every violation into a
    /// [`SafetyReport`].
    ///
    /// Even if a blocking violation is found, processing continues so that
    /// all violations are captured in the report.
    pub fn check_all(&self, context: &SafetyCheckContext) -> SafetyReport {
        let mut violations = Vec::new();

        violations.extend(self.check_tool_calls(context.current_tool_calls));
        violations.extend(self.check_revision_rounds(context.current_revision_rounds));
        violations.extend(self.check_output_length(context.output_length, context.min_output_length));
        violations.extend(self.check_duration(&context.started_at));
        violations.extend(self.check_excessive_queries(context.tool_call_steps, context.total_steps));
        violations.extend(self.check_tool_call_rounds(context.current_tool_call_rounds));

        let has_blocking = violations.iter().any(|v| v.severity == SafetySeverity::Blocking);
        let has_warnings = violations.iter().any(|v| v.severity == SafetySeverity::Warning);

        SafetyReport {
            violations,
            has_blocking,
            has_warnings,
        }
    }

    fn check_count(&self, check_type: SafetyCheckType, current: u32) -> Vec<SafetyViolation> {
        self.rules
            .iter()
            .filter(|r| r.check_type == check_type && r.enabled)
            .filter_map(|rule| {
                let threshold = rule.threshold.as_u64()?;
                if u64::from(current) > threshold {
                    Some(SafetyViolation {
                        rule: rule.check_type.clone(),
                        severity: rule.severity.clone(),
                        message: format!(
                            "{} — actual: {current}, limit: {threshold}",
                            rule.description,
                        ),
                        current_value: serde_json::json!(current),
                        threshold: serde_json::json!(threshold),
                        timestamp: Utc::now(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Input data used by [`SafetyGuard::check_all`] to run every enabled rule.
pub struct SafetyCheckContext {
    /// Current number of tool calls executed.
    pub current_tool_calls: u32,
    /// Current number of tool calls in the current loop iteration.
    pub current_tool_call_rounds: u32,
    /// Current number of revision rounds.
    pub current_revision_rounds: u32,
    /// Length of the agent output in characters/bytes.
    pub output_length: u64,
    /// Minimum acceptable output length (per the check).
    pub min_output_length: u64,
    /// Timestamp when execution started.
    pub started_at: DateTime<Utc>,
    /// Number of steps that were tool calls (for ratio computation).
    pub tool_call_steps: u32,
    /// Total number of steps executed (for ratio computation).
    pub total_steps: u32,
}

/// Results of running all safety checks via [`SafetyGuard::check_all`].
///
/// Contains the full list of violations and boolean summaries for quick
/// decision-making.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    /// All violations detected across every rule.
    pub violations: Vec<SafetyViolation>,
    /// `true` if at least one [`SafetySeverity::Blocking`] violation was found.
    pub has_blocking: bool,
    /// `true` if at least one [`SafetySeverity::Warning`] violation was found.
    pub has_warnings: bool,
}

impl SafetyReport {
    /// Returns only the blocking violations.
    pub fn blocking_violations(&self) -> Vec<&SafetyViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == SafetySeverity::Blocking)
            .collect()
    }

    /// Returns only the warning violations.
    pub fn warning_violations(&self) -> Vec<&SafetyViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == SafetySeverity::Warning)
            .collect()
    }

    /// Returns `true` when there are zero violations (no blocking and no
    /// warnings).
    pub fn is_all_clear(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Parse a duration string such as `"600s"`, `"5m"`, or `"1h"` into total
/// seconds.
///
/// Returns `None` if the string cannot be parsed.
fn parse_duration_secs(threshold: &serde_json::Value) -> Option<i64> {
    let s = threshold.as_str()?;
    if s.is_empty() {
        return None;
    }

    let (num_str, multiplier) = match s.as_bytes().last().copied()? {
        b's' | b'S' => (&s[..s.len() - 1], 1),
        b'm' | b'M' => (&s[..s.len() - 1], 60),
        b'h' | b'H' => (&s[..s.len() - 1], 3600),
        _ => (s, 1),
    };

    let value: i64 = num_str.parse().ok()?;
    Some(value * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that the default guard has exactly 6 rules.
    #[test]
    fn test_default_rules() {
        let guard = SafetyGuard::default();
        assert_eq!(guard.rules.len(), 6);
    }

    /// MaxToolCalls=3, 5 calls made → blocking violation.
    #[test]
    fn test_max_tool_calls_blocking() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls",
        )]);

        let violations = guard.check_tool_calls(5);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, SafetySeverity::Blocking);
        assert_eq!(violations[0].rule, SafetyCheckType::MaxToolCalls);
    }

    /// MaxToolCalls=3, 2 calls made → no violation.
    #[test]
    fn test_max_tool_calls_ok() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls",
        )]);

        let violations = guard.check_tool_calls(2);
        assert!(violations.is_empty());
    }

    /// MinOutputLength=100, output=50 → warning violation.
    #[test]
    fn test_output_length_warning() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MinOutputLength,
            SafetySeverity::Warning,
            serde_json::json!(100),
            "output too short",
        )]);

        let violations = guard.check_output_length(50, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, SafetySeverity::Warning);
        assert_eq!(violations[0].rule, SafetyCheckType::MinOutputLength);
    }

    /// check_all with all metrics → verify report contains expected
    /// violations.
    #[test]
    fn test_check_all() {
        let guard = SafetyGuard::new(vec![
            SafetyRule::new(
                SafetyCheckType::MaxToolCalls,
                SafetySeverity::Blocking,
                serde_json::json!(2),
                "max 2 tool calls",
            ),
            SafetyRule::new(
                SafetyCheckType::MinOutputLength,
                SafetySeverity::Warning,
                serde_json::json!(100),
                "output too short",
            ),
        ]);

        let context = SafetyCheckContext {
            current_tool_calls: 5,
            current_tool_call_rounds: 0,
            current_revision_rounds: 0,
            output_length: 20,
            min_output_length: 100,
            started_at: Utc::now(),
            tool_call_steps: 0,
            total_steps: 0,
        };

        let report = guard.check_all(&context);
        assert!(report.has_blocking);
        assert!(report.has_warnings);
        // 1 blocking (tool_calls) + 1 warning (output_length) = 2
        assert_eq!(report.violations.len(), 2);
        assert_eq!(report.blocking_violations().len(), 1);
        assert_eq!(report.warning_violations().len(), 1);
        assert!(!report.is_all_clear());
    }

    /// Disabled rule should never produce a violation.
    #[test]
    fn test_disabled_rule() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls",
        )
        .with_enabled(false)]);

        let violations = guard.check_tool_calls(100);
        assert!(violations.is_empty());
    }

    /// Adding then removing a rule means it is no longer checked.
    #[test]
    fn test_remove_rule() {
        let mut guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls",
        )]);

        guard.remove_rule(SafetyCheckType::MaxToolCalls);
        let violations = guard.check_tool_calls(100);
        assert!(violations.is_empty());
    }

    /// MaxToolCallRounds=3, 5 calls in current round → blocking violation.
    #[test]
    fn test_max_tool_call_rounds_blocking() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCallRounds,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls per round",
        )]);

        let violations = guard.check_tool_call_rounds(5);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, SafetySeverity::Blocking);
        assert_eq!(violations[0].rule, SafetyCheckType::MaxToolCallRounds);
    }

    /// MaxToolCallRounds=3, 2 calls in current round → no violation.
    #[test]
    fn test_max_tool_call_rounds_ok() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::MaxToolCallRounds,
            SafetySeverity::Blocking,
            serde_json::json!(3),
            "max 3 tool calls per round",
        )]);

        let violations = guard.check_tool_call_rounds(2);
        assert!(violations.is_empty());
    }

    /// ExcessiveQueries threshold 0.5, 10/15 = 0.667 → violation.
    #[test]
    fn test_excessive_queries() {
        let guard = SafetyGuard::new(vec![SafetyRule::new(
            SafetyCheckType::ExcessiveQueries,
            SafetySeverity::Warning,
            serde_json::json!(0.5),
            "too many queries",
        )]);

        let violations = guard.check_excessive_queries(10, 15);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, SafetySeverity::Warning);
        assert_eq!(violations[0].rule, SafetyCheckType::ExcessiveQueries);
    }

    /// SafetyReport helper methods work correctly.
    #[test]
    fn test_safety_report() {
        let empty_report = SafetyReport {
            violations: vec![],
            has_blocking: false,
            has_warnings: false,
        };
        assert!(empty_report.is_all_clear());
        assert!(!empty_report.has_blocking);
        assert!(!empty_report.has_warnings);
        assert!(empty_report.blocking_violations().is_empty());
        assert!(empty_report.warning_violations().is_empty());

        let warning = SafetyViolation {
            rule: SafetyCheckType::MinOutputLength,
            severity: SafetySeverity::Warning,
            message: "short output".into(),
            current_value: serde_json::json!(50),
            threshold: serde_json::json!(100),
            timestamp: Utc::now(),
        };
        let blocking = SafetyViolation {
            rule: SafetyCheckType::MaxToolCalls,
            severity: SafetySeverity::Blocking,
            message: "too many calls".into(),
            current_value: serde_json::json!(60),
            threshold: serde_json::json!(50),
            timestamp: Utc::now(),
        };

        let report = SafetyReport {
            violations: vec![warning.clone(), blocking.clone()],
            has_blocking: true,
            has_warnings: true,
        };

        assert!(report.has_blocking);
        assert!(report.has_warnings);
        assert!(!report.is_all_clear());
        assert_eq!(report.blocking_violations().len(), 1);
        assert_eq!(report.warning_violations().len(), 1);
        assert_eq!(
            report.blocking_violations()[0].rule,
            SafetyCheckType::MaxToolCalls,
        );
        assert_eq!(
            report.warning_violations()[0].rule,
            SafetyCheckType::MinOutputLength,
        );
    }
}
