use std::fmt::Debug;

use async_trait::async_trait;
use futures::future::join_all;

use crate::error::{ChannelError, NotifError};
use crate::types::{
    ChannelCapabilities, ChannelHealth, ChannelKind, DeliveryHandle, DeliveryReceipt, DndStatus,
    Notification, NotificationId, PreparedNotification, PushMessage, SessionInfo, UserPreferences,
};

/// A concrete delivery channel (email, SMS, push, webhook, etc.).
#[async_trait]
pub trait NotificationChannel: Debug + Send + Sync {
    /// Stable channel identifier.
    fn id(&self) -> &str;

    /// High-level channel kind.
    fn kind(&self) -> ChannelKind;

    /// Static capability declaration for routing and batching.
    fn capabilities(&self) -> ChannelCapabilities;

    /// Send one prepared notification through this channel.
    async fn send(
        &self,
        notification: &PreparedNotification,
    ) -> Result<DeliveryReceipt, ChannelError>;

    /// Send a batch of prepared notifications, defaulting to individual sends.
    async fn send_batch(
        &self,
        notifications: &[PreparedNotification],
    ) -> Vec<Result<DeliveryReceipt, ChannelError>> {
        join_all(notifications.iter().map(|notification| self.send(notification))).await
    }

    /// Run a channel health check.
    async fn health_check(&self) -> Result<ChannelHealth, ChannelError>;
}

/// Coordinates notification creation, routing, and delivery.
#[async_trait]
pub trait NotificationManager: Debug + Send + Sync {
    /// Queue and dispatch a single notification.
    async fn notify(&self, notification: Notification) -> Result<DeliveryHandle, NotifError>;

    /// Dispatch a notification and wait for all receipts.
    async fn notify_and_wait(
        &self,
        notification: Notification,
    ) -> Result<Vec<DeliveryReceipt>, NotifError>;

    /// Queue many notifications, returning per-item dispatch results.
    async fn notify_batch(
        &self,
        notifications: Vec<Notification>,
    ) -> Vec<Result<DeliveryHandle, NotifError>>;

    /// Register a new delivery channel.
    fn register_channel(&mut self, channel: Box<dyn NotificationChannel>);

    /// Unregister a delivery channel by id.
    fn unregister_channel(&mut self, channel_id: &str);

    /// Access the currently registered channels.
    fn channels(&self) -> &[Box<dyn NotificationChannel>];
}

/// Backing store for per-user notification preferences.
#[async_trait]
pub trait UserPreferenceStore: Debug + Send + Sync {
    /// Load the stored preferences for a user.
    async fn get_preferences(&self, user_id: &str) -> Result<UserPreferences, NotifError>;

    /// Persist updated preferences for a user.
    async fn set_preferences(
        &self,
        user_id: &str,
        prefs: UserPreferences,
    ) -> Result<(), NotifError>;

    /// Read the user's do-not-disturb state.
    async fn get_dnd_status(&self, user_id: &str) -> Result<DndStatus, NotifError>;
}

/// Lifecycle hooks around notification preparation and delivery.
#[async_trait]
pub trait NotificationHook: Send + Sync {
    /// Called when a notification is created.
    async fn on_created(&self, _notification: &Notification) {}

    /// Called after a notification is rendered into a prepared payload.
    async fn on_rendered(&self, _prepared: &PreparedNotification) {}

    /// Called before dispatch, allowing in-place mutation.
    async fn before_dispatch(&self, _prepared: &mut PreparedNotification) {}

    /// Called after a successful delivery receipt is produced.
    async fn on_delivered(&self, _receipt: &DeliveryReceipt) {}

    /// Called after a channel reports an error.
    async fn on_failed(&self, _channel: &str, _error: &ChannelError) {}

    /// Called after all delivery attempts complete.
    async fn on_completed(&self, _notification_id: &NotificationId, _receipts: &[DeliveryReceipt]) {}
}

/// Registry of connected user sessions for push-style delivery.
#[async_trait]
pub trait SessionRegistry: Send + Sync {
    /// Find all known sessions for a user.
    fn find_sessions(&self, user_id: &str) -> Vec<SessionInfo>;

    /// Push a message to a single session.
    async fn push(&self, session_id: &str, message: PushMessage) -> Result<(), ChannelError>;

    /// Broadcast a message to all eligible sessions.
    async fn broadcast(&self, message: PushMessage) -> Result<Vec<String>, ChannelError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChannelDeliveryStatus, ChannelKind};

    #[derive(Debug)]
    struct DummyChannel;

    #[async_trait]
    impl NotificationChannel for DummyChannel {
        fn id(&self) -> &str {
            "dummy"
        }

        fn kind(&self) -> ChannelKind {
            ChannelKind::Custom("dummy".into())
        }

        fn capabilities(&self) -> ChannelCapabilities {
            ChannelCapabilities {
                supports_rich_text: true,
                supports_actions: true,
                supports_text_reply: true,
                supports_grouping: true,
                supports_sound: true,
                supports_expiration: true,
                supports_delivery_confirmation: true,
            }
        }

        async fn send(
            &self,
            notification: &PreparedNotification,
        ) -> Result<DeliveryReceipt, ChannelError> {
            let _ = notification;
            Ok(DeliveryReceipt {
                channel: self.id().to_string(),
                status: ChannelDeliveryStatus::Delivered,
                delivered_at: None,
            })
        }

        async fn health_check(&self) -> Result<ChannelHealth, ChannelError> {
            Ok(ChannelHealth { healthy: true, last_check: None, message: None })
        }
    }

    #[test]
    fn notification_channel_is_object_safe() {
        let channel: Box<dyn NotificationChannel> = Box::new(DummyChannel);
        assert_eq!(channel.id(), "dummy");
        assert_eq!(channel.kind(), ChannelKind::Custom("dummy".into()));
    }

    #[tokio::test]
    async fn send_batch_defaults_to_individual_sends() {
        let channel = DummyChannel;
        let notifications = vec![
            PreparedNotification { title: Some("n1".into()), body: Some("a".into()), subtitle: None, sound: None, actions: vec![] },
            PreparedNotification { title: Some("n2".into()), body: Some("b".into()), subtitle: None, sound: None, actions: vec![] },
        ];

        let receipts: Vec<Result<DeliveryReceipt, ChannelError>> = channel.send_batch(&notifications).await;
        assert_eq!(receipts.len(), 2);
        assert!(receipts.into_iter().all(|r| r.is_ok()));
    }
}
