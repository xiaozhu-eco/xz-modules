use crate::error::AgentError;

/// Interval trigger handler using tokio::time::interval.
pub struct IntervalTrigger;

impl IntervalTrigger {
    pub fn new() -> Self {
        Self
    }

    /// Validate interval seconds.
    pub fn validate_seconds(seconds: u64) -> Result<(), AgentError> {
        if seconds == 0 {
            return Err(AgentError::Io("interval cannot be zero".into()));
        }
        Ok(())
    }
}

impl Default for IntervalTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_interval() {
        assert!(IntervalTrigger::validate_seconds(60).is_ok());
        assert!(IntervalTrigger::validate_seconds(0).is_err());
    }
}
