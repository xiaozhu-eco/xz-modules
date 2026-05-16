# xz-notification

[![crates.io](https://img.shields.io/crates/v/xz-notification.svg)](https://crates.io/crates/xz-notification)
[![docs.rs](https://docs.rs/xz-notification/badge.svg)](https://docs.rs/xz-notification)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`xz-notification` is a flexible notification delivery crate for composing, routing, rendering, and delivering notifications across multiple channels. It provides a unified interface for handling notifications whether they are system alerts, emails, SMS messages, or push notifications.

## Quick Start

```rust
use std::error::Error;
use xz_notification::{
    DefaultNotificationManager, DeliveryMode, DeliveryPolicy, Notification, NotificationAction,
    NotificationCategory, NotificationId, NotificationManager, NotificationTarget, Priority,
    SystemChannel, SystemChannelConfig, ActionType, ChannelKind,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut manager = DefaultNotificationManager::new();
    
    // Register a delivery channel
    manager.register_channel(Box::new(SystemChannel::new(SystemChannelConfig::default())?));

    // Compose a notification
    let notification = Notification {
        id: NotificationId::new(),
        category: NotificationCategory::System,
        priority: Priority::Normal,
        template_key: "system.alert".into(),
        template_vars: serde_json::json!({"title": "Build finished", "body": "All checks passed"}),
        targets: vec![NotificationTarget::Broadcast],
        locale: None,
        actions: vec![NotificationAction {
            action_type: ActionType::Dismiss,
            label: Some("Dismiss".into()),
            value: None,
        }],
        group_key: None,
        data: serde_json::json!({}),
        delivery: DeliveryPolicy {
            channels: vec![ChannelKind::System],
            mode: DeliveryMode::Parallel,
            retry: Default::default(),
            channel_timeout: None,
        },
        ttl: None,
        created_at: 0,
    };

    // Deliver the notification
    let _handle = manager.notify(notification)?;
    
    Ok(())
}
```

## Channels

| Channel | Kind | Description | Feature Flag |
|---------|------|-------------|--------------|
| System | `system` | OS-native notifications | `system` |
| WebSocket | `websocket` | Real-time browser/app notifications | `websocket` |
| APNS | `apns` | Apple Push Notification service | `apns` |
| FCM | `fcm` | Firebase Cloud Messaging | `fcm` |
| Email | `email` | SMTP/API based email delivery | `email` |
| SMS | `sms` | Text message delivery | `sms` |
| Webhook | `webhook` | HTTP callbacks | `webhook` |

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `system` | OS-level notification support | Yes |
| `websocket` | WebSocket delivery support | No |
| `apns` | iOS/macOS push notifications | No |
| `fcm` | Android/cross-platform push notifications | No |
| `email` | Email delivery support | No |
| `sms` | SMS delivery support | No |
| `webhook` | Webhook delivery support | No |
| `full` | Enables all channels and features | No |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
