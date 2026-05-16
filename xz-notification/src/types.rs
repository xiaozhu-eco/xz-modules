use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tokio::sync::watch;
use uuid::Uuid;

/// Notification identifier backed by UUID v7.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NotificationId(pub Uuid);

impl NotificationId {
    /// Create a new time-sortable identifier.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for NotificationId {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for NotificationId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NotificationId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_u128().cmp(&other.0.as_u128())
    }
}

/// Notification priority.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Notification category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NotificationCategory {
    System,
    Message,
    Alert,
    Reminder,
    Task,
    Social,
    Marketing,
    Custom(String),
}

/// Notification target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationTarget {
    User { user_id: String },
    Device { device_id: String },
    Session { session_id: String },
    Email { address: String },
    Phone { number: String },
    Webhook { url: String },
    Broadcast,
}

/// A notification action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationAction {
    pub action_type: ActionType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Action type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    OpenUrl,
    Callback,
    TextReply,
    Dismiss,
}

/// Delivery mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Parallel,
    Serial,
    FirstAvailable,
}

impl Default for DeliveryMode {
    fn default() -> Self {
        Self::Parallel
    }
}

/// Backoff strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackoffStrategy {
    Fixed,
    Exponential { base_ms: u64, max_ms: u64 },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Fixed
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetryConfig {
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub backoff: BackoffStrategy,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self { max_retries: 0, backoff: BackoffStrategy::Fixed }
    }
}

/// Delivery policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryPolicy {
    #[serde(default)]
    pub channels: Vec<ChannelKind>,
    #[serde(default)]
    pub mode: DeliveryMode,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_timeout: Option<u64>,
}

impl Default for DeliveryPolicy {
    fn default() -> Self {
        Self { channels: Vec::new(), mode: DeliveryMode::Parallel, retry: RetryConfig::default(), channel_timeout: None }
    }
}

/// Overall delivery phase.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryPhase {
    Rendering,
    Queued,
    Dispatching,
    Completed,
    PartiallyFailed,
    FullyFailed,
}

/// Per-channel delivery state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelDeliveryStatus {
    Pending,
    Queued,
    Sending,
    Delivered,
    Failed,
    Expired,
    Cancelled,
}

/// Delivery channel record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelDeliveryRecord {
    pub channel: ChannelKind,
    pub status: ChannelDeliveryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Overall delivery status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryStatus {
    pub phase: DeliveryPhase,
    pub status: ChannelDeliveryStatus,
    #[serde(default)]
    pub channels: Vec<ChannelDeliveryRecord>,
}

impl Default for DeliveryStatus {
    fn default() -> Self {
        Self { phase: DeliveryPhase::Queued, status: ChannelDeliveryStatus::Pending, channels: Vec::new() }
    }
}

/// Handle to an active delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryHandle {
    pub notification_id: NotificationId,
    #[serde(skip, default)]
    pub receiver: Option<watch::Receiver<DeliveryStatus>>,
}

/// Delivery receipt for a channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryReceipt {
    pub channel: String,
    pub status: ChannelDeliveryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<std::time::SystemTime>,
}

/// Do-not-disturb status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DndStatus {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<std::time::SystemTime>,
}

impl Default for DndStatus {
    fn default() -> Self {
        Self { enabled: false, until: None }
    }
}

/// Connected session information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// User notification preferences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserPreferences {
    #[serde(default)]
    pub muted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dnd: Option<DndStatus>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self { muted: false, dnd: None }
    }
}

/// Message pushed to a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushMessage {
    pub notification_id: NotificationId,
    pub payload: serde_json::Value,
}

/// Channel health snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelHealth {
    pub healthy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_check: Option<std::time::SystemTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for ChannelHealth {
    fn default() -> Self {
        Self { healthy: true, last_check: None, message: None }
    }
}

/// Prepared notification for channel rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreparedNotification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
}

/// Channel capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelCapabilities {
    #[serde(default)]
    pub supports_rich_text: bool,
    #[serde(default)]
    pub supports_actions: bool,
    #[serde(default)]
    pub supports_text_reply: bool,
    #[serde(default)]
    pub supports_grouping: bool,
    #[serde(default)]
    pub supports_sound: bool,
    #[serde(default)]
    pub supports_expiration: bool,
    #[serde(default)]
    pub supports_delivery_confirmation: bool,
}

impl Default for ChannelCapabilities {
    fn default() -> Self {
        Self {
            supports_rich_text: false,
            supports_actions: false,
            supports_text_reply: false,
            supports_grouping: false,
            supports_sound: false,
            supports_expiration: false,
            supports_delivery_confirmation: false,
        }
    }
}

/// Channel kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    System,
    WebSocket,
    Apns,
    Fcm,
    Email,
    Sms,
    Webhook,
    Custom(String),
}

/// Device platform.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    MacOS,
    Windows,
    Linux,
    IOS,
    Android,
    Web,
}

/// Full notification record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub id: NotificationId,
    pub category: NotificationCategory,
    pub priority: Priority,
    pub template_key: String,
    #[serde(default)]
    pub template_vars: serde_json::Value,
    #[serde(default)]
    pub targets: Vec<NotificationTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
    pub delivery: DeliveryPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_notification() -> Notification {
        Notification {
            id: NotificationId::new(),
            category: NotificationCategory::Custom("ops".into()),
            priority: Priority::High,
            template_key: "welcome".into(),
            template_vars: serde_json::json!({"name": "Ada"}),
            targets: vec![
                NotificationTarget::User { user_id: "u1".into() },
                NotificationTarget::Email { address: "a@example.com".into() },
            ],
            locale: Some("en-US".into()),
            actions: vec![NotificationAction { action_type: ActionType::OpenUrl, label: Some("Open".into()), value: Some("https://example.com".into()) }],
            group_key: Some("group-1".into()),
            data: serde_json::json!({"source": "test"}),
            delivery: DeliveryPolicy {
                channels: vec![ChannelKind::Email, ChannelKind::WebSocket],
                mode: DeliveryMode::Serial,
                retry: RetryConfig { max_retries: 2, backoff: BackoffStrategy::Exponential { base_ms: 100, max_ms: 1000 } },
                channel_timeout: Some(5000),
            },
            ttl: Some(3600),
            created_at: 1_717_000_000,
        }
    }

    fn roundtrip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).unwrap();
        let decoded: T = serde_json::from_str(&json).unwrap();
        assert_eq!(&decoded, value);
    }

    #[test]
    fn serialize_deserialize_roundtrip_for_core_types() {
        roundtrip(&NotificationId::new());
        roundtrip(&Priority::Critical);
        roundtrip(&NotificationCategory::System);
        roundtrip(&NotificationCategory::Custom("x".into()));
        roundtrip(&NotificationTarget::Broadcast);
        roundtrip(&NotificationTarget::Webhook { url: "https://example.com/hook".into() });
        roundtrip(&ActionType::TextReply);
        roundtrip(&NotificationAction { action_type: ActionType::Dismiss, label: None, value: None });
        roundtrip(&DeliveryMode::FirstAvailable);
        roundtrip(&BackoffStrategy::Fixed);
        roundtrip(&RetryConfig { max_retries: 3, backoff: BackoffStrategy::Fixed });
        roundtrip(&DeliveryPolicy::default());
        roundtrip(&DeliveryPhase::Dispatching);
        roundtrip(&ChannelDeliveryStatus::Delivered);
        roundtrip(&ChannelDeliveryRecord { channel: ChannelKind::Apns, status: ChannelDeliveryStatus::Sending, error: Some("boom".into()) });
        roundtrip(&DeliveryStatus::default());
        roundtrip(&PreparedNotification {
            title: Some("T".into()),
            body: Some("B".into()),
            subtitle: Some("S".into()),
            sound: Some("ding".into()),
            actions: vec![NotificationAction { action_type: ActionType::Callback, label: None, value: Some("cb".into()) }],
        });
        roundtrip(&ChannelCapabilities::default());
        roundtrip(&ChannelKind::WebSocket);
        roundtrip(&ChannelKind::Custom("custom".into()));
        roundtrip(&DevicePlatform::MacOS);
        roundtrip(&sample_notification());
    }

    #[test]
    fn priority_orders_correctly() {
        assert!(Priority::Background < Priority::Low);
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn notification_id_is_uuid_v7_and_sortable() {
        let first = NotificationId::new();
        let second = NotificationId::new();

        assert_eq!(first.0.get_version_num(), 7);
        assert_eq!(second.0.get_version_num(), 7);

        let mut ids = vec![second.clone(), first.clone()];
        ids.sort();
        assert_eq!(ids, vec![first, second]);
    }

    #[test]
    fn notification_target_uses_type_tag() {
        let target = NotificationTarget::Email { address: "a@example.com".into() };
        let json = serde_json::to_value(&target).unwrap();
        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("email"));
    }

    #[test]
    fn delivery_policy_defaults_to_parallel() {
        assert_eq!(DeliveryPolicy::default().mode, DeliveryMode::Parallel);
    }

    #[test]
    fn channel_capabilities_field_names_match() {
        let caps = ChannelCapabilities {
            supports_rich_text: true,
            supports_actions: true,
            supports_text_reply: true,
            supports_grouping: true,
            supports_sound: true,
            supports_expiration: true,
            supports_delivery_confirmation: true,
        };

        assert!(caps.supports_rich_text);
        assert!(caps.supports_actions);
        assert!(caps.supports_text_reply);
        assert!(caps.supports_grouping);
        assert!(caps.supports_sound);
        assert!(caps.supports_expiration);
        assert!(caps.supports_delivery_confirmation);
    }

    #[test]
    fn prepared_notification_exposes_rendered_fields() {
        let rendered = PreparedNotification {
            title: Some("Hello".into()),
            body: Some("World".into()),
            subtitle: Some("Sub".into()),
            sound: Some("ding".into()),
            actions: vec![NotificationAction {
                action_type: ActionType::OpenUrl,
                label: Some("Open".into()),
                value: Some("https://example.com".into()),
            }],
        };

        assert_eq!(rendered.title.as_deref(), Some("Hello"));
        assert_eq!(rendered.body.as_deref(), Some("World"));
        assert_eq!(rendered.subtitle.as_deref(), Some("Sub"));
        assert_eq!(rendered.sound.as_deref(), Some("ding"));
        assert_eq!(rendered.actions.len(), 1);
    }

    #[test]
    fn delivery_handle_roundtrips_without_receiver() {
        let handle = DeliveryHandle {
            notification_id: NotificationId::new(),
            receiver: None,
        };

        let json = serde_json::to_string(&handle).unwrap();
        let decoded: DeliveryHandle = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.notification_id, handle.notification_id);
        assert!(decoded.receiver.is_none());
    }
}
