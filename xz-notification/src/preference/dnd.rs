use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuietHours {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DndStatus {
    pub active: bool,
    pub until: Option<DateTime<Utc>>,
}

pub fn is_dnd_active(quiet_hours: &QuietHours, now: DateTime<Utc>) -> DndStatus {
    let current = now.time();
    let start = quiet_hours.start;
    let end = quiet_hours.end;

    let active = if start < end {
        current >= start && current < end
    } else {
        current >= start || current < end
    };

    if !active {
        return DndStatus { active: false, until: None };
    }

    let until = if start < end {
        now.date_naive().and_time(end)
    } else if current >= start {
        (now.date_naive() + chrono::Duration::days(1)).and_time(end)
    } else {
        now.date_naive().and_time(end)
    };

    DndStatus { active: true, until: Some(DateTime::from_naive_utc_and_offset(until, Utc)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap()
    }

    #[test]
    fn detects_dnd_within_overnight_hours() {
        let quiet_hours = QuietHours {
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        };

        let status = is_dnd_active(&quiet_hours, dt(2026, 5, 16, 23, 0, 0));

        assert!(status.active);
        assert_eq!(status.until, Some(dt(2026, 5, 17, 7, 0, 0)));
    }

    #[test]
    fn detects_dnd_within_same_day_hours() {
        let quiet_hours = QuietHours {
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        };

        let status = is_dnd_active(&quiet_hours, dt(2026, 5, 16, 10, 30, 0));

        assert!(status.active);
        assert_eq!(status.until, Some(dt(2026, 5, 16, 17, 0, 0)));
    }

    #[test]
    fn detects_dnd_in_early_morning_overnight_window() {
        let quiet_hours = QuietHours {
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        };

        let status = is_dnd_active(&quiet_hours, dt(2026, 5, 16, 3, 0, 0));

        assert!(status.active);
        assert_eq!(status.until, Some(dt(2026, 5, 16, 7, 0, 0)));
    }

    #[test]
    fn reports_dnd_inactive_outside_window() {
        let quiet_hours = QuietHours {
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        };

        let status = is_dnd_active(&quiet_hours, dt(2026, 5, 16, 8, 0, 0));

        assert!(!status.active);
        assert_eq!(status.until, None);
    }
}
