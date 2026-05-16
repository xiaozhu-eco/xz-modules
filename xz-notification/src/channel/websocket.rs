use std::{fmt, sync::Arc};

use async_trait::async_trait;

use crate::{
    error::ChannelError,
    traits::{NotificationChannel, SessionRegistry},
    types::{ChannelCapabilities, ChannelDeliveryStatus, ChannelHealth, ChannelKind, DeliveryReceipt, PreparedNotification, PushMessage},
};

#[derive(Clone)]
pub struct WebSocketChannel {
    registry: Arc<dyn SessionRegistry>,
}

impl fmt::Debug for WebSocketChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocketChannel").finish_non_exhaustive()
    }
}

impl WebSocketChannel {
    pub fn new(registry: Arc<dyn SessionRegistry>) -> Self {
        Self { registry }
    }

    fn push_message(&self, notification: &PreparedNotification) -> Result<PushMessage, ChannelError> {
        let payload = serde_json::to_value(notification)
            .map_err(|error| ChannelError::InvalidPayload(format!("failed to serialize websocket payload: {error}")))?;

        Ok(PushMessage { notification_id: crate::types::NotificationId::new(), payload })
    }
}

#[async_trait]
impl NotificationChannel for WebSocketChannel {
    fn id(&self) -> &str {
        "websocket"
    }

    fn kind(&self) -> ChannelKind {
        ChannelKind::WebSocket
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            supports_rich_text: true,
            supports_actions: true,
            supports_text_reply: true,
            supports_grouping: true,
            supports_sound: false,
            supports_expiration: true,
            supports_delivery_confirmation: true,
        }
    }

    async fn send(&self, notification: &PreparedNotification) -> Result<DeliveryReceipt, ChannelError> {
        let sessions = self.registry.find_sessions(self.id());
        let message = self.push_message(notification)?;

        for session in sessions {
            self.registry.push(&session.session_id, message.clone()).await?;
        }

        Ok(DeliveryReceipt {
            channel: self.id().into(),
            status: ChannelDeliveryStatus::Delivered,
            delivered_at: Some(std::time::SystemTime::now()),
        })
    }

    async fn health_check(&self) -> Result<ChannelHealth, ChannelError> {
        Ok(ChannelHealth::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SessionInfo;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct MockState {
        find_sessions_calls: Vec<String>,
        pushed: Vec<(String, PushMessage)>,
        sessions: Vec<SessionInfo>,
    }

    #[derive(Debug)]
    struct MockSessionRegistry {
        state: Arc<Mutex<MockState>>,
    }

    impl MockSessionRegistry {
        fn new(state: Arc<Mutex<MockState>>) -> Self {
            Self { state }
        }
    }

    #[async_trait]
    impl SessionRegistry for MockSessionRegistry {
        fn find_sessions(&self, user_id: &str) -> Vec<SessionInfo> {
            let mut state = self.state.lock().unwrap();
            state.find_sessions_calls.push(user_id.to_string());
            state.sessions.clone()
        }

        async fn push(&self, session_id: &str, message: PushMessage) -> Result<(), ChannelError> {
            let mut state = self.state.lock().unwrap();
            state.pushed.push((session_id.to_string(), message));
            Ok(())
        }

        async fn broadcast(&self, _message: PushMessage) -> Result<Vec<String>, ChannelError> {
            Ok(vec![])
        }
    }

    fn sample_notification() -> PreparedNotification {
        PreparedNotification {
            title: Some("Deploy complete".into()),
            body: Some("Preview is live".into()),
            subtitle: Some("web".into()),
            sound: None,
            actions: vec![],
        }
    }

    #[tokio::test]
    async fn websocket_channel_pushes_to_all_sessions() {
        let state = Arc::new(Mutex::new(MockState {
            sessions: vec![
                SessionInfo { session_id: "s-1".into(), device_id: Some("d-1".into()) },
                SessionInfo { session_id: "s-2".into(), device_id: None },
            ],
            ..Default::default()
        }));
        let channel = WebSocketChannel::new(Arc::new(MockSessionRegistry::new(state.clone())));

        let receipt = channel.send(&sample_notification()).await.unwrap();
        assert_eq!(receipt.channel, "websocket");
        assert_eq!(receipt.status, ChannelDeliveryStatus::Delivered);

        let state = state.lock().unwrap();
        assert_eq!(state.find_sessions_calls, vec!["websocket"]);
        assert_eq!(state.pushed.len(), 2);
        assert_eq!(state.pushed[0].0, "s-1");
        assert_eq!(state.pushed[1].0, "s-2");
        assert_eq!(state.pushed[0].1.payload["title"], "Deploy complete");
    }

    #[tokio::test]
    async fn websocket_channel_reports_expected_kind_and_capabilities() {
        let channel = WebSocketChannel::new(Arc::new(MockSessionRegistry::new(Arc::new(Mutex::new(
            MockState::default(),
        )))));

        assert_eq!(channel.kind(), ChannelKind::WebSocket);
        let caps = channel.capabilities();
        assert!(caps.supports_rich_text);
        assert!(caps.supports_actions);
        assert!(caps.supports_text_reply);
        assert!(caps.supports_delivery_confirmation);
    }
}
