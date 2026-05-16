//! # xz-notification
//!
//! `xz-notification` is a notification delivery crate for composing, routing,
//! rendering, and delivering notifications across multiple channels.
//!
//! It centers on two core traits:
//! - [`crate::NotificationChannel`]: a transport implementation for a concrete channel
//! - [`crate::NotificationManager`]: orchestration for rendering, routing, and delivery
//!
//! Supported channel kinds include system, email, SMS, webhook, APNS, FCM,
//! WebSocket, and custom channel implementations.
//!
//! ## Quick start
//!
//! ```rust
//! use std::boxed::Box;
//!
//! use xz_notification::{
//!     DefaultNotificationManager, DeliveryMode, DeliveryPolicy, Notification, NotificationAction,
//!     NotificationCategory, NotificationId, NotificationManager, NotificationTarget, Priority,
//!     SystemChannel, SystemChannelConfig,
//! };
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut manager = DefaultNotificationManager::new();
//! manager.register_channel(Box::new(SystemChannel::new(SystemChannelConfig::default())?));
//!
//! let notification = Notification {
//!     id: NotificationId::new(),
//!     category: NotificationCategory::System,
//!     priority: Priority::Normal,
//!     template_key: "system.alert".into(),
//!     template_vars: serde_json::json!({"title": "Build finished", "body": "All checks passed"}),
//!     targets: vec![NotificationTarget::Broadcast],
//!     locale: None,
//!     actions: vec![NotificationAction {
//!         action_type: xz_notification::ActionType::Dismiss,
//!         label: Some("Dismiss".into()),
//!         value: None,
//!     }],
//!     group_key: None,
//!     data: serde_json::json!({}),
//!     delivery: DeliveryPolicy {
//!         channels: vec![xz_notification::ChannelKind::System],
//!         mode: DeliveryMode::Parallel,
//!         retry: Default::default(),
//!         channel_timeout: None,
//!     },
//!     ttl: None,
//!     created_at: 0,
//! };
//!
//! let _handle = manager.notify(notification).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod channel;
pub mod manager;
pub mod queue;
pub mod ratelimit;
pub mod types;
pub mod traits;
pub mod observability;
pub mod preference;
pub mod template;

pub use error::{ChannelError, NotifError, RetryStrategy};
pub use manager::DefaultNotificationManager;

pub use observability::{emit_error_event, emit_notification_event, HealthReport, NotificationStats};

pub use preference::{CategoryPreference, DndStatus, QuietHours, UserPreferences};
pub use ratelimit::{ChannelRateLimit, RateLimitAction, RateLimitConfig, RateLimiter};

pub use template::engine::TemplateEngine;

pub use traits::{NotificationChannel, NotificationHook, NotificationManager, SessionRegistry, UserPreferenceStore};

pub use types::{
    ActionType, BackoffStrategy, ChannelCapabilities, ChannelDeliveryStatus, ChannelHealth,
    ChannelKind, DeliveryHandle, DeliveryMode, DeliveryPhase, DeliveryPolicy, DeliveryReceipt,
    DeliveryStatus, DevicePlatform, Notification, NotificationAction, NotificationCategory,
    NotificationId, NotificationTarget, PreparedNotification, Priority, PushMessage, RetryConfig,
    SessionInfo,
};

#[cfg(feature = "system")]
pub use channel::system::{SystemChannel, SystemChannelConfig};
