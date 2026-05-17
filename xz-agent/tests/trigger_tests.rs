use xz_agent::*;

#[test]
fn test_cron_validate_valid() {
    assert!(CronTrigger::validate_expression("0 8 * * *").is_ok());
}

#[test]
fn test_cron_validate_invalid() {
    assert!(CronTrigger::validate_expression("").is_err());
    assert!(CronTrigger::validate_expression("* * *").is_err());
    assert!(CronTrigger::validate_expression("* * * * * *").is_err());
}

#[test]
fn test_cron_next_fire() {
    let secs = CronTrigger::next_fire_seconds("0 0 * * *");
    assert!(secs.is_some(), "expected some value, got None");
    assert!(secs.unwrap() > 0, "expected > 0");
}

#[test]
fn test_interval_validate() {
    assert!(IntervalTrigger::validate_seconds(60).is_ok());
    assert!(IntervalTrigger::validate_seconds(0).is_err());
}

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
