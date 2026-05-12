use crate::error::AgentError;

/// Cron trigger handler.
///
/// Parses cron expressions and schedules agent execution.
pub struct CronTrigger;

impl CronTrigger {
    pub fn new() -> Self {
        Self
    }

    /// Parse and validate a cron expression.
    pub fn validate_expression(expression: &str) -> Result<(), AgentError> {
        if expression.is_empty() {
            return Err(AgentError::Io("empty cron expression".into()));
        }
        // Validate basic format: 5 fields
        let fields: Vec<&str> = expression.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(AgentError::Io(format!(
                "cron expression must have 5 fields, got {}",
                fields.len()
            )));
        }
        Ok(())
    }

    /// Get the next scheduled time from a cron expression.
    /// Returns seconds from now.
    pub fn next_fire_seconds(_expression: &str) -> Option<u64> {
        // Placeholder: returns 60s (real impl uses cron library)
        Some(60)
    }
}

impl Default for CronTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_cron() {
        assert!(CronTrigger::validate_expression("0 8 * * *").is_ok());
    }

    #[test]
    fn test_validate_invalid_cron() {
        assert!(CronTrigger::validate_expression("invalid").is_err());
        assert!(CronTrigger::validate_expression("").is_err());
    }
}
