#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("xz-notification system channel supports only macOS, Linux, and Windows targets");

use std::{fmt, sync::Arc};

use async_trait::async_trait;

use crate::{
    error::ChannelError,
    traits::NotificationChannel,
    types::{
        ChannelCapabilities, ChannelDeliveryStatus, ChannelHealth, ChannelKind, DeliveryReceipt,
        PreparedNotification,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChannelConfig {
    pub app_name: String,
    pub app_icon_path: Option<String>,
    pub sound: Option<String>,
}

impl Default for SystemChannelConfig {
    fn default() -> Self {
        Self {
            app_name: "xz-notification".into(),
            app_icon_path: None,
            sound: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SystemNotificationPlatform {
    MacOS,
    Linux,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemNotificationRequest {
    pub app_name: String,
    pub app_icon_path: Option<String>,
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
    pub sound: Option<String>,
    pub actions: Vec<SystemNotificationAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemNotificationAction {
    pub identifier: String,
    pub label: String,
}

#[async_trait]
pub(crate) trait SystemNotificationBackend: fmt::Debug + Send + Sync {
    async fn show(&self, request: SystemNotificationRequest) -> Result<(), ChannelError>;

    #[allow(dead_code)]
    async fn dismiss(&self) -> Result<(), ChannelError> {
        Ok(())
    }

    fn platform(&self) -> SystemNotificationPlatform;
}

pub struct SystemChannel {
    backend: Arc<dyn SystemNotificationBackend>,
    config: SystemChannelConfig,
}

impl fmt::Debug for SystemChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemChannel")
            .field("platform", &self.backend.platform())
            .field("config", &self.config)
            .finish()
    }
}

impl SystemChannel {
    pub fn new(config: SystemChannelConfig) -> Result<Self, ChannelError> {
        #[cfg(target_os = "macos")]
        let backend: Arc<dyn SystemNotificationBackend> = Arc::new(macos::MacOSBackend::new());

        #[cfg(target_os = "linux")]
        let backend: Arc<dyn SystemNotificationBackend> = Arc::new(linux::LinuxDBusBackend::new());

        #[cfg(target_os = "windows")]
        let backend: Arc<dyn SystemNotificationBackend> = Arc::new(windows::WindowsToastBackend::new());

        Ok(Self { backend, config })
    }

    #[cfg(test)]
    pub(crate) fn with_backend(
        config: SystemChannelConfig,
        backend: Arc<dyn SystemNotificationBackend>,
    ) -> Self {
        Self { backend, config }
    }

    fn notification_request(
        &self,
        notification: &PreparedNotification,
    ) -> Result<SystemNotificationRequest, ChannelError> {
        let title = notification
            .title
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| ChannelError::InvalidPayload("system notifications require a title".into()))?;
        let body = notification
            .body
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| ChannelError::InvalidPayload("system notifications require a body".into()))?;

        let actions = notification
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| SystemNotificationAction {
                identifier: format!("action-{index}"),
                label: action
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("action-{index}")),
            })
            .collect();

        Ok(SystemNotificationRequest {
            app_name: self.config.app_name.clone(),
            app_icon_path: self.config.app_icon_path.clone(),
            title,
            body,
            subtitle: notification.subtitle.clone(),
            sound: notification.sound.clone().or_else(|| self.config.sound.clone()),
            actions,
        })
    }

    #[cfg(test)]
    pub(crate) fn platform(&self) -> SystemNotificationPlatform {
        self.backend.platform()
    }
}

#[async_trait]
impl NotificationChannel for SystemChannel {
    fn id(&self) -> &str {
        "system"
    }

    fn kind(&self) -> ChannelKind {
        ChannelKind::System
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            supports_rich_text: true,
            supports_actions: true,
            supports_text_reply: false,
            supports_grouping: true,
            supports_sound: true,
            supports_expiration: true,
            supports_delivery_confirmation: false,
        }
    }

    async fn send(
        &self,
        notification: &PreparedNotification,
    ) -> Result<DeliveryReceipt, ChannelError> {
        let request = self.notification_request(notification)?;
        self.backend.show(request).await?;

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
    use crate::types::{ActionType, NotificationAction};
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct MockBackendState {
        requests: Vec<SystemNotificationRequest>,
        fail_with: Option<ChannelError>,
    }

    #[derive(Debug)]
    struct MockBackend {
        platform: SystemNotificationPlatform,
        state: Arc<Mutex<MockBackendState>>,
    }

    impl MockBackend {
        fn new(platform: SystemNotificationPlatform, state: Arc<Mutex<MockBackendState>>) -> Self {
            Self { platform, state }
        }
    }

    #[async_trait]
    impl SystemNotificationBackend for MockBackend {
        async fn show(&self, request: SystemNotificationRequest) -> Result<(), ChannelError> {
            let mut state = self.state.lock().unwrap();
            state.requests.push(request);
            match &state.fail_with {
                Some(error) => Err(match error {
                    ChannelError::Connection(message) => ChannelError::Connection(message.clone()),
                    ChannelError::Auth(message) => ChannelError::Auth(message.clone()),
                    ChannelError::RateLimited { retry_after_ms } => {
                        ChannelError::RateLimited { retry_after_ms: *retry_after_ms }
                    }
                    ChannelError::InvalidPayload(message) => {
                        ChannelError::InvalidPayload(message.clone())
                    }
                    ChannelError::DeviceNotRegistered(message) => {
                        ChannelError::DeviceNotRegistered(message.clone())
                    }
                    ChannelError::Timeout => ChannelError::Timeout,
                }),
                None => Ok(()),
            }
        }
        fn platform(&self) -> SystemNotificationPlatform {
            self.platform
        }
    }

    fn sample_notification() -> PreparedNotification {
        PreparedNotification {
            title: Some("Build finished".into()),
            body: Some("All checks passed".into()),
            subtitle: Some("CI".into()),
            sound: Some("default".into()),
            actions: vec![NotificationAction {
                action_type: ActionType::OpenUrl,
                label: Some("Open logs".into()),
                value: Some("https://example.com/logs".into()),
            }],
        }
    }

    #[tokio::test]
    async fn system_channel_send_forwards_rendered_notification_to_backend() {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let backend = Arc::new(MockBackend::new(SystemNotificationPlatform::MacOS, state.clone()));
        let channel = SystemChannel::with_backend(SystemChannelConfig::default(), backend);

        let receipt = channel.send(&sample_notification()).await.unwrap();
        assert_eq!(receipt.channel, "system");
        assert_eq!(receipt.status, ChannelDeliveryStatus::Delivered);

        let state = state.lock().unwrap();
        assert_eq!(state.requests.len(), 1);
        let request = &state.requests[0];
        assert_eq!(request.title, "Build finished");
        assert_eq!(request.body, "All checks passed");
        assert_eq!(request.subtitle.as_deref(), Some("CI"));
        assert_eq!(request.sound.as_deref(), Some("default"));
        assert_eq!(request.actions.len(), 1);
        assert_eq!(request.actions[0].label, "Open logs");
    }

    #[tokio::test]
    async fn system_channel_uses_config_sound_when_notification_sound_missing() {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let backend = Arc::new(MockBackend::new(SystemNotificationPlatform::Linux, state.clone()));
        let channel = SystemChannel::with_backend(
            SystemChannelConfig {
                app_name: "xz".into(),
                app_icon_path: Some("/tmp/icon.png".into()),
                sound: Some("ping".into()),
            },
            backend,
        );

        let mut notification = sample_notification();
        notification.sound = None;
        channel.send(&notification).await.unwrap();

        let state = state.lock().unwrap();
        let request = &state.requests[0];
        assert_eq!(request.app_name, "xz");
        assert_eq!(request.app_icon_path.as_deref(), Some("/tmp/icon.png"));
        assert_eq!(request.sound.as_deref(), Some("ping"));
    }

    #[tokio::test]
    async fn system_channel_rejects_missing_title_or_body() {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let backend = Arc::new(MockBackend::new(SystemNotificationPlatform::Windows, state));
        let channel = SystemChannel::with_backend(SystemChannelConfig::default(), backend);

        let mut missing_title = sample_notification();
        missing_title.title = None;
        assert!(matches!(
            channel.send(&missing_title).await,
            Err(ChannelError::InvalidPayload(_))
        ));

        let mut missing_body = sample_notification();
        missing_body.body = Some("   ".into());
        assert!(matches!(
            channel.send(&missing_body).await,
            Err(ChannelError::InvalidPayload(_))
        ));
    }

    #[tokio::test]
    async fn system_channel_surfaces_backend_errors() {
        let state = Arc::new(Mutex::new(MockBackendState {
            requests: Vec::new(),
            fail_with: Some(ChannelError::Connection("dbus unavailable".into())),
        }));
        let backend = Arc::new(MockBackend::new(SystemNotificationPlatform::Linux, state));
        let channel = SystemChannel::with_backend(SystemChannelConfig::default(), backend);

        let error = channel.send(&sample_notification()).await.unwrap_err();
        assert!(matches!(error, ChannelError::Connection(_)));
    }

    #[test]
    fn system_channel_reports_expected_kind_capabilities_and_platform() {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let backend = Arc::new(MockBackend::new(SystemNotificationPlatform::Windows, state));
        let channel = SystemChannel::with_backend(SystemChannelConfig::default(), backend);

        assert_eq!(channel.kind(), ChannelKind::System);
        assert_eq!(channel.platform(), SystemNotificationPlatform::Windows);
        let caps = channel.capabilities();
        assert!(caps.supports_rich_text);
        assert!(caps.supports_actions);
        assert!(caps.supports_grouping);
        assert!(caps.supports_sound);
    }
}
