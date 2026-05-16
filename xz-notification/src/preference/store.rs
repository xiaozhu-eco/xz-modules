use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{NotificationCategory, Priority};

use super::dnd::QuietHours;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryPreference {
    pub enabled: bool,
    pub allowed_channels: Vec<String>,
    pub priority_override: Option<Priority>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserPreferences {
    pub notifications_enabled: bool,
    pub category_preferences: HashMap<NotificationCategory, CategoryPreference>,
    pub quiet_hours: Option<QuietHours>,
    pub channel_priority: Vec<String>,
}

pub fn should_deliver(
    prefs: &UserPreferences,
    category: &NotificationCategory,
    priority: Priority,
    channel: &str,
) -> bool {
    if !prefs.notifications_enabled {
        return false;
    }

    let category_pref = match prefs.category_preferences.get(category) {
        Some(pref) => pref,
        None => return false,
    };

    if !category_pref.enabled {
        return false;
    }

    if !category_pref.allowed_channels.is_empty()
        && !category_pref.allowed_channels.iter().any(|allowed| allowed == channel)
    {
        return false;
    }

    if priority == Priority::Critical {
        return true;
    }

    if let Some(quiet_hours) = &prefs.quiet_hours {
        let dnd = super::dnd::is_dnd_active(quiet_hours, chrono::Utc::now());
        if dnd.active {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn prefs_with(category_enabled: bool, allowed_channels: Vec<&str>, quiet_hours: Option<QuietHours>) -> UserPreferences {
        let mut category_preferences = HashMap::new();
        category_preferences.insert(
            NotificationCategory::Alert,
            CategoryPreference {
                enabled: category_enabled,
                allowed_channels: allowed_channels.into_iter().map(str::to_string).collect(),
                priority_override: None,
            },
        );

        UserPreferences {
            notifications_enabled: true,
            category_preferences,
            quiet_hours,
            channel_priority: vec!["email".into(), "push".into()],
        }
    }

    fn quiet_hours_around_now() -> QuietHours {
        let now = Utc::now();
        let start = (now - Duration::hours(1)).time();
        let end = (now + Duration::hours(1)).time();
        QuietHours { start, end }
    }

    #[test]
    fn critical_priority_bypasses_dnd() {
        let prefs = prefs_with(true, vec!["email"], Some(quiet_hours_around_now()));

        assert!(should_deliver(&prefs, &NotificationCategory::Alert, Priority::Critical, "email"));
    }

    #[test]
    fn disabled_category_blocks_delivery() {
        let prefs = prefs_with(false, vec!["email"], None);

        assert!(!should_deliver(&prefs, &NotificationCategory::Alert, Priority::Normal, "email"));
    }

    #[test]
    fn disallowed_channel_blocks_delivery() {
        let prefs = prefs_with(true, vec!["sms"], None);

        assert!(!should_deliver(&prefs, &NotificationCategory::Alert, Priority::Normal, "email"));
    }

    #[test]
    fn notifications_disabled_blocks_delivery() {
        let mut prefs = prefs_with(true, vec!["email"], None);
        prefs.notifications_enabled = false;

        assert!(!should_deliver(&prefs, &NotificationCategory::Alert, Priority::Normal, "email"));
    }

    #[test]
    fn quiet_hours_active_block_non_critical_delivery() {
        let prefs = prefs_with(true, vec!["email"], Some(quiet_hours_around_now()));

        assert!(!should_deliver(&prefs, &NotificationCategory::Alert, Priority::Normal, "email"));
    }
}
