#[cfg(target_os = "macos")]
use notify_rust::Notification;

use crate::error::ChannelError;

use super::{
    SystemNotificationBackend, SystemNotificationPlatform, SystemNotificationRequest,
};

#[derive(Debug, Default)]
pub(crate) struct MacOSBackend;

impl MacOSBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SystemNotificationBackend for MacOSBackend {
    async fn show(&self, request: SystemNotificationRequest) -> Result<(), ChannelError> {
        let mut notification = Notification::new();
        notification.summary(&request.title);
        notification.body(&request.body);

        if let Some(subtitle) = &request.subtitle {
            notification.subtitle(subtitle);
        }

        if let Some(sound) = &request.sound {
            notification.sound_name(sound);
        }

        if let Some(icon_path) = &request.app_icon_path {
            notification.image_path(icon_path);
        }

        notification
            .show()
            .map(|_| ())
            .map_err(|error| ChannelError::Connection(error.to_string()))
    }

    fn platform(&self) -> SystemNotificationPlatform {
        SystemNotificationPlatform::MacOS
    }
}
