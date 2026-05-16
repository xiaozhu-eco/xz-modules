#[cfg(target_os = "windows")]
use notify_rust::{Notification, Timeout, Urgency};

use crate::error::ChannelError;

use super::{
    SystemNotificationBackend, SystemNotificationPlatform, SystemNotificationRequest,
};

#[derive(Debug, Default)]
pub(crate) struct WindowsToastBackend;

impl WindowsToastBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SystemNotificationBackend for WindowsToastBackend {
    async fn show(&self, request: SystemNotificationRequest) -> Result<(), ChannelError> {
        let mut notification = Notification::new();
        notification.summary(&request.title).body(&request.body);

        if let Some(subtitle) = &request.subtitle {
            notification.subtitle(subtitle);
        }

        if let Some(icon_path) = &request.app_icon_path {
            notification.image_path(icon_path);
        }

        if let Some(sound) = &request.sound {
            notification.sound_name(sound);
        }

        notification
            .urgency(Urgency::Normal)
            .timeout(Timeout::Milliseconds(5_000))
            .show()
            .map(|_| ())
            .map_err(|error| ChannelError::Connection(error.to_string()))
    }

    fn platform(&self) -> SystemNotificationPlatform {
        SystemNotificationPlatform::Windows
    }
}
