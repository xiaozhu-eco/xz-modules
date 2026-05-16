use std::{collections::HashMap, time::Duration};

use crate::{error::ChannelError, types::ChannelHealth};

pub fn emit_notification_event(channel: &str, notification_id: &str, latency: Duration, success: bool) {
    let latency_ms = latency.as_millis() as u64;
    tracing::info!(
        target: "xz_notification",
        channel = %channel,
        notification_id = %notification_id,
        latency_ms = latency_ms,
        success = success,
        "notification_event"
    );
}

pub fn emit_error_event(channel: &str, error: &ChannelError) {
    tracing::error!(
        target: "xz_notification",
        channel = %channel,
        error = %error,
        "notification_error"
    );
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationStats {
    pub total_notifications: u64,
    pub delivered: u64,
    pub failed: u64,
    pub retried: u64,
}

impl NotificationStats {
    pub fn accumulate(&mut self, other: &Self) {
        self.total_notifications += other.total_notifications;
        self.delivered += other.delivered;
        self.failed += other.failed;
        self.retried += other.retried;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    pub channels: HashMap<String, ChannelHealth>,
    pub queue_depth: usize,
    pub overall_healthy: bool,
}

impl Default for HealthReport {
    fn default() -> Self {
        Self { channels: HashMap::new(), queue_depth: 0, overall_healthy: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_stats_accumulate() {
        let mut lhs = NotificationStats { total_notifications: 1, delivered: 1, failed: 0, retried: 0 };
        let rhs = NotificationStats { total_notifications: 2, delivered: 1, failed: 1, retried: 3 };
        lhs.accumulate(&rhs);
        assert_eq!(lhs.total_notifications, 3);
        assert_eq!(lhs.delivered, 2);
        assert_eq!(lhs.failed, 1);
        assert_eq!(lhs.retried, 3);
    }

    #[test]
    fn health_report_defaults() {
        let report = HealthReport::default();
        assert!(report.channels.is_empty());
        assert_eq!(report.queue_depth, 0);
        assert!(report.overall_healthy);
    }

    #[test]
    fn tracing_helpers_emit_with_subscriber() {
        emit_notification_event("system", "n1", Duration::from_millis(12), true);
        emit_error_event("system", &crate::error::ChannelError::Timeout);
    }
}
