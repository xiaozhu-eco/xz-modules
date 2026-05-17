use chrono::Timelike;

use crate::error::AgentError;

/// Parse an optional cron field into individual values or detect step pattern (e.g. */5).
fn parse_field(field: &str) -> Option<Vec<u32>> {
    if field == "*" {
        return None; // wildcard = any value
    }
    if let Some(step_str) = field.strip_prefix("*/") {
        let step: u32 = step_str.parse().ok()?;
        if step == 0 {
            return None;
        }
        let max = if step >= 60 { 24 } else { 60 };
        let mut values = Vec::new();
        let mut v = 0;
        while v < max {
            values.push(v);
            v += step;
        }
        return Some(values);
    }
    if field.contains(',') {
        let values: Vec<u32> = field.split(',').filter_map(|s| s.parse().ok()).collect();
        return Some(values);
    }
    if let Ok(v) = field.parse::<u32>() {
        return Some(vec![v]);
    }
    None
}

/// Get the next fire time for a 5-field cron expression.
/// Uses chrono to compute the next matching minute.
fn next_cron_time(expression: &str) -> Option<u64> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return None;
    }

    // parse_field returns None for wildcard (*) = all values accepted.
    // Use unwrap_or_default to treat wildcard as an empty filter (not an error).
    let minutes = parse_field(fields[0]).unwrap_or_default();
    let hours = parse_field(fields[1]).unwrap_or_default();

    let now = chrono::Utc::now();
    let current_min = now.minute() as u32;
    let current_hour = now.hour() as u32;
    let current_sec = now.second() as u32;

    // Simple schedule: find next matching minute within the same or next hour/day.
    // If specific minutes are given: find the next minute >= current
    // If wildcard minute (*): return 60 (next minute boundary)
    // If step minute (*/N): find next step boundary

    {
        // specific minutes: find next
        for &m in &minutes {
            // Check if we match ANY hour or specific hours
            if !hours.is_empty() && !hours.contains(&current_hour) && current_min < m {
                // Same hour, future minute
                let secs = (m - current_min) as u64 * 60 - current_sec as u64;
                return Some(secs);
            }
            if hours.is_empty() && m > current_min {
                let secs = (m - current_min) as u64 * 60 - current_sec as u64;
                return Some(secs);
            }
        }
        // All minutes in current hour passed, try next hour
        let next_hour = if hours.is_empty() {
            let h = if current_hour == 23 { 0 } else { current_hour + 1 };
            Some(h)
        } else {
            hours.iter().find(|&&h| h > current_hour).copied()
        };
        if let Some(nh) = next_hour {
            if let Some(&first_min) = minutes.first() {
                let hour_diff = if nh > current_hour { nh - current_hour } else { 24 - current_hour + nh };
                let secs = hour_diff as u64 * 3600 + (first_min as u64 * 60) - current_sec as u64;
                return Some(secs);
            }
        }
        // Next day
        if let Some(&first_min) = minutes.first() {
            if let Some(&first_hour) = hours.first() {
                let secs = (24 - current_hour) as u64 * 3600
                    + (first_hour as u64 * 3600)
                    + (first_min as u64 * 60)
                    - current_sec as u64;
                return Some(secs);
            }
        }
    }

    // Fallback: 60s (next minute)
    Some(60)
}

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
    pub fn next_fire_seconds(expression: &str) -> Option<u64> {
        next_cron_time(expression)
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

    #[test]
    fn test_cron_next_fire_every_5_min() {
        let secs = CronTrigger::next_fire_seconds("*/5 * * * *");
        assert!(secs.is_some(), "expected Some, got None");
        if let Some(s) = secs {
            assert!(s > 0, "expected > 0, got {}", s);
        }
    }

    #[test]
    fn test_cron_next_fire_specific_minute() {
        let secs = CronTrigger::next_fire_seconds("30 * * * *");
        assert!(secs.is_some(), "expected Some, got None");
    }

    #[test]
    fn test_cron_invalid_returns_none() {
        assert!(CronTrigger::next_fire_seconds("invalid").is_none());
    }
}
