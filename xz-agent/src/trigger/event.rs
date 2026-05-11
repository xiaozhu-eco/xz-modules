use crate::error::AgentError;
use serde_json::Value;

/// Event trigger handler for event-driven agent execution.
#[derive(Debug)]
pub struct EventTrigger;

impl EventTrigger {
    pub fn new() -> Self {
        Self
    }

    /// Validate event filter against event data.
    pub fn matches_filter(filter: Option<&Value>, event: &Value) -> Result<bool, AgentError> {
        match filter {
            None => Ok(true),
            Some(filter) => {
                // Simple key-value matching
                if let (Some(filter_obj), Some(event_obj)) = (filter.as_object(), event.as_object()) {
                    for (key, expected) in filter_obj {
                        match event_obj.get(key) {
                            Some(actual) if actual == expected => continue,
                            _ => return Ok(false),
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

impl Default for EventTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_match() {
        let filter = serde_json::json!({"type": "user_message"});
        let event = serde_json::json!({"type": "user_message", "user": "alice"});
        assert!(EventTrigger::matches_filter(Some(&filter), &event).unwrap());
    }

    #[test]
    fn test_event_filter_no_match() {
        let filter = serde_json::json!({"type": "system_event"});
        let event = serde_json::json!({"type": "user_message"});
        assert!(!EventTrigger::matches_filter(Some(&filter), &event).unwrap());
    }

    #[test]
    fn test_event_filter_none() {
        let event = serde_json::json!({"type": "any"});
        assert!(EventTrigger::matches_filter(None, &event).unwrap());
    }
}
