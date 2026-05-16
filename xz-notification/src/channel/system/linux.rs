#[cfg(target_os = "linux")]
use notify_rust::{Hint, Notification, Timeout, Urgency};

use crate::error::ChannelError;

use super::{
    SystemNotificationBackend, SystemNotificationPlatform, SystemNotificationRequest,
};

#[derive(Debug, Default)]
pub(crate) struct LinuxDBusBackend;

impl LinuxDBusBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SystemNotificationBackend for LinuxDBusBackend {
    async fn show(&self, request: SystemNotificationRequest) -> Result<(), ChannelError> {
        let mut notification = Notification::new();
        notification
            .appname(&request.app_name)
            .summary(&request.title)
            .body(&request.body)
            .hint(Hint::Category("system".into()))
            .hint(Hint::Custom(
                "x-dunst-stack-tag".into(),
                request.app_name.clone(),
            ))
            .urgency(Urgency::Normal)
            .timeout(Timeout::Milliseconds(5_000));

        if let Some(icon_path) = &request.app_icon_path {
            notification.icon(icon_path);
            notification.image_path(icon_path);
        }

        if let Some(sound) = &request.sound {
            notification.hint(Hint::SoundName(sound.clone()));
        }

        for action in &request.actions {
            notification.action(&action.identifier, &action.label);
        }

        notification
            .show()
            .map(|_| ())
            .map_err(|error| ChannelError::Connection(error.to_string()))
    }

    async fn dismiss(&self) -> Result<(), ChannelError> {
        Ok(())
    }

    fn platform(&self) -> SystemNotificationPlatform {
        SystemNotificationPlatform::Linux
    }
}
