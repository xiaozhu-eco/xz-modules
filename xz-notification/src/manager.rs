use std::{
    collections::HashMap,
    fmt,
    sync::Mutex,
    time::Duration,
};

use async_trait::async_trait;
use futures::future::join_all;
use tokio::sync::watch;

use crate::{
    error::{ChannelError, NotifError},
    queue::priority_queue::{PriorityQueue, QueueItem},
    ratelimit::RateLimiter,
    template::engine::TemplateEngine,
    traits::{NotificationChannel, NotificationHook, NotificationManager},
    types::{
        ChannelDeliveryRecord, ChannelDeliveryStatus, ChannelKind, DeliveryHandle,
        DeliveryMode, DeliveryPhase, DeliveryReceipt, DeliveryStatus, Notification,
        PreparedNotification,
    },
};

pub struct DefaultNotificationManager {
    channels: Vec<Box<dyn NotificationChannel>>,
    queue: Mutex<PriorityQueue>,
    template_engine: Option<TemplateEngine>,
    hooks: Vec<Box<dyn NotificationHook>>,
    rate_limiter: Option<Mutex<RateLimiter>>,
}

impl fmt::Debug for DefaultNotificationManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultNotificationManager")
            .field("channels", &self.channels.len())
            .field("queue", &"PriorityQueue")
            .field("template_engine", &self.template_engine.is_some())
            .field("hooks", &self.hooks.len())
            .field("rate_limiter", &self.rate_limiter.is_some())
            .finish()
    }
}

impl Default for DefaultNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultNotificationManager {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            queue: Mutex::new(PriorityQueue::new()),
            template_engine: None,
            hooks: Vec::new(),
            rate_limiter: None,
        }
    }

    pub fn with_template_engine(mut self, template_engine: TemplateEngine) -> Self {
        self.template_engine = Some(template_engine);
        self
    }

    pub fn with_rate_limiter(mut self, rate_limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(Mutex::new(rate_limiter));
        self
    }

    pub fn register_hook(&mut self, hook: Box<dyn NotificationHook>) {
        self.hooks.push(hook);
    }

    pub fn unregister_hook(&mut self, hook_index: usize) {
        if hook_index < self.hooks.len() {
            self.hooks.remove(hook_index);
        }
    }

    fn render_notification(
        &self,
        notification: &Notification,
    ) -> Result<PreparedNotification, NotifError> {
        let mut prepared = if let Some(engine) = &self.template_engine {
            let vars = notification_template_vars(notification);
            engine.render(&notification.template_key, &vars, notification.locale.as_deref())?
        } else {
            PreparedNotification {
                title: notification.template_vars.get("title").and_then(as_string),
                body: notification.template_vars.get("body").and_then(as_string),
                subtitle: notification.template_vars.get("subtitle").and_then(as_string),
                sound: notification.template_vars.get("sound").and_then(as_string),
                actions: Vec::new(),
            }
        };

        if prepared.actions.is_empty() {
            prepared.actions = notification.actions.clone();
        }

        Ok(prepared)
    }

    fn target_channels<'a>(
        &'a self,
        notification: &Notification,
    ) -> Result<Vec<&'a dyn NotificationChannel>, NotifError> {
        if self.channels.is_empty() {
            return Err(NotifError::ChannelNotFound("no channels registered".into()));
        }

        if notification.delivery.channels.is_empty() {
            return Ok(self.channels.iter().map(|channel| channel.as_ref()).collect());
        }

        let mut selected = Vec::new();
        for requested in &notification.delivery.channels {
            let matches: Vec<&dyn NotificationChannel> = self
                .channels
                .iter()
                .filter(|channel| channel.kind() == *requested)
                .map(|channel| channel.as_ref())
                .collect();

            if matches.is_empty() {
                return Err(NotifError::ChannelNotFound(channel_kind_label(requested)));
            }

            selected.extend(matches);
        }

        Ok(selected)
    }

    fn queue_notification(&self, notification: &Notification) {
        let mut queue = self.queue.lock().expect("queue lock poisoned");
        queue.enqueue(QueueItem::new(
            notification.id.0.to_string(),
            notification.priority,
            notification.ttl.map(Duration::from_secs),
        ));
        let _ = queue.dequeue();
    }

    fn check_rate_limit(&self, channel_id: &str) -> Result<(), NotifError> {
        match &self.rate_limiter {
            Some(rate_limiter) => rate_limiter
                .lock()
                .expect("rate limiter lock poisoned")
                .check(channel_id),
            None => Ok(()),
        }
    }

    fn send_status(sender: &watch::Sender<DeliveryStatus>, status: &DeliveryStatus) {
        let _ = sender.send(status.clone());
    }

    fn set_channel_status(
        status: &mut DeliveryStatus,
        index: usize,
        next_status: ChannelDeliveryStatus,
        error: Option<String>,
    ) {
        if let Some(record) = status.channels.get_mut(index) {
            record.status = next_status;
            record.error = error;
        }

        status.status = aggregate_status(&status.channels);
        status.phase = aggregate_phase(&status.channels);
    }

    async fn call_failed_hook(&self, channel_id: &str, error: &NotifError) {
        let channel_error = notif_error_to_channel_error(error);
        for hook in &self.hooks {
            hook.on_failed(channel_id, &channel_error).await;
        }
    }

    fn terminal_status(
        mut status: DeliveryStatus,
        receipts: &[DeliveryReceipt],
        failures: &[ChannelError],
    ) -> DeliveryStatus {
        status.status = if receipts.is_empty() {
            ChannelDeliveryStatus::Failed
        } else {
            ChannelDeliveryStatus::Delivered
        };

        status.phase = if !receipts.is_empty() && failures.is_empty() {
            DeliveryPhase::Completed
        } else if !receipts.is_empty() {
            DeliveryPhase::PartiallyFailed
        } else {
            DeliveryPhase::FullyFailed
        };

        status
    }
}

#[async_trait]
impl NotificationManager for DefaultNotificationManager {
    async fn notify(&self, notification: Notification) -> Result<DeliveryHandle, NotifError> {
        let (sender, receiver) = watch::channel(DeliveryStatus {
            phase: DeliveryPhase::Rendering,
            status: ChannelDeliveryStatus::Pending,
            channels: Vec::new(),
        });

        let handle = DeliveryHandle {
            notification_id: notification.id.clone(),
            receiver: Some(receiver),
        };

        for hook in &self.hooks {
            hook.on_created(&notification).await;
        }

        self.queue_notification(&notification);

        let mut prepared = self.render_notification(&notification)?;
        for hook in &self.hooks {
            hook.on_rendered(&prepared).await;
        }
        for hook in &self.hooks {
            hook.before_dispatch(&mut prepared).await;
        }

        let target_channels = self.target_channels(&notification)?;
        let mut status = DeliveryStatus {
            phase: DeliveryPhase::Queued,
            status: ChannelDeliveryStatus::Queued,
            channels: target_channels
                .iter()
                .map(|channel| ChannelDeliveryRecord {
                    channel: channel.kind(),
                    status: ChannelDeliveryStatus::Queued,
                    error: None,
                })
                .collect(),
        };
        Self::send_status(&sender, &status);

        let mut receipts = Vec::new();
        let mut failures = Vec::new();

        match notification.delivery.mode {
            DeliveryMode::Parallel => {
                for index in 0..status.channels.len() {
                    Self::set_channel_status(&mut status, index, ChannelDeliveryStatus::Sending, None);
                }
                Self::send_status(&sender, &status);

                let outcomes = join_all(target_channels.iter().enumerate().map(|(index, channel)| {
                    let prepared = prepared.clone();
                    async move {
                        let rate_limit = self.check_rate_limit(channel.id());
                        let result = match rate_limit {
                            Ok(()) => channel
                                .send(&prepared)
                                .await
                                .map_err(|error| channel_error_to_notif_error(channel.id(), error)),
                            Err(error) => Err(error),
                        };
                        (index, channel.id().to_string(), result)
                    }
                }))
                .await;

                for (index, channel_id, result) in outcomes {
                    match result {
                        Ok(receipt) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Delivered,
                                None,
                            );
                            Self::send_status(&sender, &status);
                            for hook in &self.hooks {
                                hook.on_delivered(&receipt).await;
                            }
                            receipts.push(receipt);
                        }
                        Err(error) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Failed,
                                Some(error.to_string()),
                            );
                            Self::send_status(&sender, &status);
                            self.call_failed_hook(&channel_id, &error).await;
                            failures.push(notif_error_to_channel_error(&error));
                        }
                    }
                }
            }
            DeliveryMode::Serial => {
                for (index, channel) in target_channels.iter().enumerate() {
                    Self::set_channel_status(&mut status, index, ChannelDeliveryStatus::Sending, None);
                    Self::send_status(&sender, &status);

                    let result = match self.check_rate_limit(channel.id()) {
                        Ok(()) => channel
                            .send(&prepared)
                            .await
                            .map_err(|error| channel_error_to_notif_error(channel.id(), error)),
                        Err(error) => Err(error),
                    };

                    match result {
                        Ok(receipt) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Delivered,
                                None,
                            );
                            Self::send_status(&sender, &status);
                            for hook in &self.hooks {
                                hook.on_delivered(&receipt).await;
                            }
                            receipts.push(receipt);
                        }
                        Err(error) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Failed,
                                Some(error.to_string()),
                            );
                            Self::send_status(&sender, &status);
                            self.call_failed_hook(channel.id(), &error).await;
                            failures.push(notif_error_to_channel_error(&error));
                            break;
                        }
                    }
                }
            }
            DeliveryMode::FirstAvailable => {
                for (index, channel) in target_channels.iter().enumerate() {
                    Self::set_channel_status(&mut status, index, ChannelDeliveryStatus::Sending, None);
                    Self::send_status(&sender, &status);

                    let result = match self.check_rate_limit(channel.id()) {
                        Ok(()) => channel
                            .send(&prepared)
                            .await
                            .map_err(|error| channel_error_to_notif_error(channel.id(), error)),
                        Err(error) => Err(error),
                    };

                    match result {
                        Ok(receipt) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Delivered,
                                None,
                            );
                            Self::send_status(&sender, &status);
                            for hook in &self.hooks {
                                hook.on_delivered(&receipt).await;
                            }
                            receipts.push(receipt);
                            break;
                        }
                        Err(error) => {
                            Self::set_channel_status(
                                &mut status,
                                index,
                                ChannelDeliveryStatus::Failed,
                                Some(error.to_string()),
                            );
                            Self::send_status(&sender, &status);
                            self.call_failed_hook(channel.id(), &error).await;
                            failures.push(notif_error_to_channel_error(&error));
                        }
                    }
                }
            }
        }

        let status = Self::terminal_status(status, &receipts, &failures);
        Self::send_status(&sender, &status);
        for hook in &self.hooks {
            hook.on_completed(&notification.id, &receipts).await;
        }

        if receipts.is_empty() {
            return Err(NotifError::AllChannelsFailed(failures));
        }

        Ok(handle)
    }

    async fn notify_and_wait(
        &self,
        notification: Notification,
    ) -> Result<Vec<DeliveryReceipt>, NotifError> {
        let mut handle = self.notify(notification).await?;
        let mut receiver = handle
            .receiver
            .take()
            .ok_or_else(|| NotifError::Internal(Box::new(std::io::Error::other("missing delivery receiver"))))?;

        loop {
            let status = receiver.borrow().clone();
            if is_terminal_phase(status.phase) {
                return Ok(receipts_from_status(&status));
            }

            if receiver.changed().await.is_err() {
                return Ok(receipts_from_status(&receiver.borrow().clone()));
            }
        }
    }

    async fn notify_batch(
        &self,
        notifications: Vec<Notification>,
    ) -> Vec<Result<DeliveryHandle, NotifError>> {
        let mut results = Vec::with_capacity(notifications.len());
        for notification in notifications {
            results.push(self.notify(notification).await);
        }
        results
    }

    fn register_channel(&mut self, channel: Box<dyn NotificationChannel>) {
        self.channels.push(channel);
    }

    fn unregister_channel(&mut self, channel_id: &str) {
        self.channels.retain(|channel| channel.id() != channel_id);
    }

    fn channels(&self) -> &[Box<dyn NotificationChannel>] {
        &self.channels
    }
}

fn notification_template_vars(notification: &Notification) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    if let Some(object) = notification.template_vars.as_object() {
        for (key, value) in object {
            vars.insert(key.clone(), json_value_to_string(value));
        }
    }
    vars
}

fn as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn json_value_to_string(value: &serde_json::Value) -> String {
    as_string(value).unwrap_or_default()
}

fn channel_error_to_notif_error(channel_id: &str, error: ChannelError) -> NotifError {
    match error {
        ChannelError::RateLimited { retry_after_ms } => NotifError::RateLimited {
            channel: channel_id.into(),
            retry_after_ms,
        },
        ChannelError::Timeout => NotifError::DeliveryTimeout {
            channel: channel_id.into(),
        },
        other => NotifError::ChannelUnavailable {
            channel: channel_id.into(),
            reason: other.to_string(),
        },
    }
}

fn notif_error_to_channel_error(error: &NotifError) -> ChannelError {
    match error {
        NotifError::RateLimited { retry_after_ms, .. } => ChannelError::RateLimited {
            retry_after_ms: *retry_after_ms,
        },
        NotifError::DeliveryTimeout { .. } => ChannelError::Timeout,
        NotifError::ChannelUnavailable { reason, .. } => ChannelError::Connection(reason.clone()),
        NotifError::ChannelNotFound(channel) => ChannelError::InvalidPayload(channel.clone()),
        NotifError::TemplateError(reason) => ChannelError::InvalidPayload(reason.clone()),
        NotifError::PreferenceError(reason) => ChannelError::InvalidPayload(reason.clone()),
        NotifError::AllChannelsFailed(errors) => errors
            .first()
            .map(clone_channel_error)
            .unwrap_or_else(|| ChannelError::InvalidPayload("all channels failed".into())),
        NotifError::DoNotDisturb { .. } => ChannelError::InvalidPayload(error.to_string()),
        NotifError::Internal(inner) => ChannelError::InvalidPayload(inner.to_string()),
    }
}

fn clone_channel_error(error: &ChannelError) -> ChannelError {
    match error {
        ChannelError::Connection(reason) => ChannelError::Connection(reason.clone()),
        ChannelError::Auth(reason) => ChannelError::Auth(reason.clone()),
        ChannelError::RateLimited { retry_after_ms } => ChannelError::RateLimited {
            retry_after_ms: *retry_after_ms,
        },
        ChannelError::InvalidPayload(reason) => ChannelError::InvalidPayload(reason.clone()),
        ChannelError::DeviceNotRegistered(device) => ChannelError::DeviceNotRegistered(device.clone()),
        ChannelError::Timeout => ChannelError::Timeout,
    }
}

fn channel_kind_label(kind: &ChannelKind) -> String {
    match kind {
        ChannelKind::System => "system".into(),
        ChannelKind::WebSocket => "web_socket".into(),
        ChannelKind::Apns => "apns".into(),
        ChannelKind::Fcm => "fcm".into(),
        ChannelKind::Email => "email".into(),
        ChannelKind::Sms => "sms".into(),
        ChannelKind::Webhook => "webhook".into(),
        ChannelKind::Custom(value) => value.clone(),
    }
}

fn aggregate_status(records: &[ChannelDeliveryRecord]) -> ChannelDeliveryStatus {
    if records
        .iter()
        .any(|record| record.status == ChannelDeliveryStatus::Sending)
    {
        ChannelDeliveryStatus::Sending
    } else if records
        .iter()
        .any(|record| record.status == ChannelDeliveryStatus::Failed)
        && records
            .iter()
            .all(|record| matches!(record.status, ChannelDeliveryStatus::Failed | ChannelDeliveryStatus::Delivered))
    {
        if records
            .iter()
            .any(|record| record.status == ChannelDeliveryStatus::Delivered)
        {
            ChannelDeliveryStatus::Delivered
        } else {
            ChannelDeliveryStatus::Failed
        }
    } else if records
        .iter()
        .all(|record| record.status == ChannelDeliveryStatus::Delivered)
    {
        ChannelDeliveryStatus::Delivered
    } else if records
        .iter()
        .any(|record| record.status == ChannelDeliveryStatus::Queued)
    {
        ChannelDeliveryStatus::Queued
    } else {
        ChannelDeliveryStatus::Pending
    }
}

fn aggregate_phase(records: &[ChannelDeliveryRecord]) -> DeliveryPhase {
    if records
        .iter()
        .any(|record| record.status == ChannelDeliveryStatus::Sending)
    {
        DeliveryPhase::Dispatching
    } else if records
        .iter()
        .all(|record| record.status == ChannelDeliveryStatus::Delivered)
    {
        DeliveryPhase::Completed
    } else if records
        .iter()
        .all(|record| record.status == ChannelDeliveryStatus::Failed)
    {
        DeliveryPhase::FullyFailed
    } else if records.iter().any(|record| record.status == ChannelDeliveryStatus::Failed)
        && records
            .iter()
            .any(|record| record.status == ChannelDeliveryStatus::Delivered)
    {
        DeliveryPhase::PartiallyFailed
    } else {
        DeliveryPhase::Queued
    }
}

fn is_terminal_phase(phase: DeliveryPhase) -> bool {
    matches!(
        phase,
        DeliveryPhase::Completed | DeliveryPhase::PartiallyFailed | DeliveryPhase::FullyFailed
    )
}

fn receipts_from_status(status: &DeliveryStatus) -> Vec<DeliveryReceipt> {
    status
        .channels
        .iter()
        .filter(|record| record.status == ChannelDeliveryStatus::Delivered)
        .map(|record| DeliveryReceipt {
            channel: channel_kind_label(&record.channel),
            status: record.status,
            delivered_at: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        error::ChannelError,
        traits::NotificationChannel,
        types::{
            ActionType, ChannelCapabilities, ChannelHealth, NotificationAction,
            NotificationCategory, NotificationId, NotificationTarget, Priority,
        },
    };

    #[derive(Debug, Clone)]
    enum MockSendOutcome {
        Success(DeliveryReceipt),
        Failure(MockChannelError),
    }

    #[derive(Debug, Clone)]
    enum MockChannelError {
        Connection(String),
        Timeout,
    }

    impl MockChannelError {
        fn into_channel_error(self) -> ChannelError {
            match self {
                Self::Connection(reason) => ChannelError::Connection(reason),
                Self::Timeout => ChannelError::Timeout,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct ChannelLog {
        sends: usize,
        bodies: Vec<Option<String>>,
        actions: Vec<Vec<NotificationAction>>,
    }

    #[derive(Debug)]
    struct MockNotificationChannel {
        id: &'static str,
        kind: ChannelKind,
        outcome: MockSendOutcome,
        log: Arc<Mutex<ChannelLog>>,
    }

    impl MockNotificationChannel {
        fn success(id: &'static str, kind: ChannelKind, log: Arc<Mutex<ChannelLog>>) -> Self {
            Self {
                id,
                kind,
                outcome: MockSendOutcome::Success(DeliveryReceipt {
                    channel: id.into(),
                    status: ChannelDeliveryStatus::Delivered,
                    delivered_at: None,
                }),
                log,
            }
        }

        fn failure(
            id: &'static str,
            kind: ChannelKind,
            error: MockChannelError,
            log: Arc<Mutex<ChannelLog>>,
        ) -> Self {
            Self {
                id,
                kind,
                outcome: MockSendOutcome::Failure(error),
                log,
            }
        }
    }

    #[async_trait]
    impl NotificationChannel for MockNotificationChannel {
        fn id(&self) -> &str {
            self.id
        }

        fn kind(&self) -> ChannelKind {
            self.kind.clone()
        }

        fn capabilities(&self) -> ChannelCapabilities {
            ChannelCapabilities::default()
        }

        async fn send(
            &self,
            notification: &PreparedNotification,
        ) -> Result<DeliveryReceipt, ChannelError> {
            let mut log = self.log.lock().expect("channel log lock poisoned");
            log.sends += 1;
            log.bodies.push(notification.body.clone());
            log.actions.push(notification.actions.clone());
            match self.outcome.clone() {
                MockSendOutcome::Success(receipt) => Ok(receipt),
                MockSendOutcome::Failure(error) => Err(error.into_channel_error()),
            }
        }

        async fn health_check(&self) -> Result<ChannelHealth, ChannelError> {
            Ok(ChannelHealth::default())
        }
    }

    struct RecordingHook {
        events: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl NotificationHook for RecordingHook {
        async fn on_created(&self, _notification: &Notification) {
            self.events.lock().unwrap().push("created".into());
        }

        async fn on_rendered(&self, _prepared: &PreparedNotification) {
            self.events.lock().unwrap().push("rendered".into());
        }

        async fn before_dispatch(&self, prepared: &mut PreparedNotification) {
            self.events.lock().unwrap().push("before_dispatch".into());
            prepared.body = Some(format!(
                "{} [hooked]",
                prepared.body.clone().unwrap_or_default()
            ));
        }

        async fn on_delivered(&self, receipt: &DeliveryReceipt) {
            self.events
                .lock()
                .unwrap()
                .push(format!("delivered:{}", receipt.channel));
        }

        async fn on_failed(&self, channel: &str, _error: &ChannelError) {
            self.events
                .lock()
                .unwrap()
                .push(format!("failed:{channel}"));
        }

        async fn on_completed(&self, _notification_id: &NotificationId, receipts: &[DeliveryReceipt]) {
            self.events
                .lock()
                .unwrap()
                .push(format!("completed:{}", receipts.len()));
        }
    }

    fn channel_log() -> Arc<Mutex<ChannelLog>> {
        Arc::new(Mutex::new(ChannelLog {
            sends: 0,
            bodies: Vec::new(),
            actions: Vec::new(),
        }))
    }

    fn sample_notification(mode: DeliveryMode, channels: Vec<ChannelKind>) -> Notification {
        Notification {
            id: NotificationId::new(),
            category: NotificationCategory::Alert,
            priority: Priority::High,
            template_key: "welcome".into(),
            template_vars: serde_json::json!({
                "name": "Ada",
                "title": "Hello",
                "body": "Body from vars"
            }),
            targets: vec![NotificationTarget::User {
                user_id: "u1".into(),
            }],
            locale: Some("en-US".into()),
            actions: vec![NotificationAction {
                action_type: ActionType::OpenUrl,
                label: Some("Open".into()),
                value: Some("https://example.com".into()),
            }],
            group_key: None,
            data: serde_json::json!({}),
            delivery: crate::types::DeliveryPolicy {
                channels,
                mode,
                retry: Default::default(),
                channel_timeout: None,
            },
            ttl: Some(30),
            created_at: 1,
        }
    }

    fn template_engine() -> TemplateEngine {
        let mut engine = TemplateEngine::new();
        engine.register_template("welcome", "Hello {{name}}");
        engine
    }

    #[tokio::test]
    async fn manager_notify_runs_hooks_and_updates_status() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = channel_log();
        let mut manager = DefaultNotificationManager::new().with_template_engine(template_engine());
        manager.register_hook(Box::new(RecordingHook {
            events: events.clone(),
        }));
        manager.register_channel(Box::new(MockNotificationChannel::success(
            "email-primary",
            ChannelKind::Email,
            log.clone(),
        )));

        let handle = manager
            .notify(sample_notification(DeliveryMode::Serial, vec![ChannelKind::Email]))
            .await
            .unwrap();

        let receiver = handle.receiver.expect("receiver should be present");
        let status = receiver.borrow().clone();
        assert_eq!(status.phase, DeliveryPhase::Completed);
        assert_eq!(status.status, ChannelDeliveryStatus::Delivered);
        assert_eq!(status.channels.len(), 1);
        assert_eq!(status.channels[0].status, ChannelDeliveryStatus::Delivered);

        let channel_log = log.lock().unwrap().clone();
        assert_eq!(channel_log.sends, 1);
        assert_eq!(channel_log.bodies[0].as_deref(), Some("Hello Ada [hooked]"));
        assert_eq!(channel_log.actions[0].len(), 1);

        assert_eq!(
            events.lock().unwrap().clone(),
            vec![
                "created",
                "rendered",
                "before_dispatch",
                "delivered:email-primary",
                "completed:1"
            ]
        );
    }

    #[tokio::test]
    async fn manager_notify_and_wait_returns_delivered_receipts() {
        let log = channel_log();
        let mut manager = DefaultNotificationManager::new().with_template_engine(template_engine());
        manager.register_channel(Box::new(MockNotificationChannel::success(
            "email-primary",
            ChannelKind::Email,
            log,
        )));

        let receipts = manager
            .notify_and_wait(sample_notification(DeliveryMode::Serial, vec![ChannelKind::Email]))
            .await
            .unwrap();

        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].channel, "email");
        assert_eq!(receipts[0].status, ChannelDeliveryStatus::Delivered);
    }

    #[tokio::test]
    async fn manager_first_available_stops_after_first_success() {
        let failing_log = channel_log();
        let success_log = channel_log();
        let skipped_log = channel_log();
        let mut manager = DefaultNotificationManager::new().with_template_engine(template_engine());
        manager.register_channel(Box::new(MockNotificationChannel::failure(
            "email-primary",
            ChannelKind::Email,
            MockChannelError::Connection("down".into()),
            failing_log.clone(),
        )));
        manager.register_channel(Box::new(MockNotificationChannel::success(
            "sms-primary",
            ChannelKind::Sms,
            success_log.clone(),
        )));
        manager.register_channel(Box::new(MockNotificationChannel::success(
            "webhook-primary",
            ChannelKind::Webhook,
            skipped_log.clone(),
        )));

        let handle = manager
            .notify(sample_notification(
                DeliveryMode::FirstAvailable,
                vec![ChannelKind::Email, ChannelKind::Sms, ChannelKind::Webhook],
            ))
            .await
            .unwrap();

        let status = handle.receiver.unwrap().borrow().clone();
        assert_eq!(status.phase, DeliveryPhase::PartiallyFailed);
        assert_eq!(failing_log.lock().unwrap().sends, 1);
        assert_eq!(success_log.lock().unwrap().sends, 1);
        assert_eq!(skipped_log.lock().unwrap().sends, 0);
    }

    #[tokio::test]
    async fn manager_serial_stops_on_first_error_and_returns_all_channels_failed() {
        let first_log = channel_log();
        let second_log = channel_log();
        let mut manager = DefaultNotificationManager::new().with_template_engine(template_engine());
        manager.register_channel(Box::new(MockNotificationChannel::failure(
            "email-primary",
            ChannelKind::Email,
            MockChannelError::Timeout,
            first_log.clone(),
        )));
        manager.register_channel(Box::new(MockNotificationChannel::success(
            "sms-primary",
            ChannelKind::Sms,
            second_log.clone(),
        )));

        let error = manager
            .notify(sample_notification(
                DeliveryMode::Serial,
                vec![ChannelKind::Email, ChannelKind::Sms],
            ))
            .await
            .unwrap_err();

        assert!(matches!(error, NotifError::AllChannelsFailed(_)));
        assert_eq!(first_log.lock().unwrap().sends, 1);
        assert_eq!(second_log.lock().unwrap().sends, 0);
    }

    #[tokio::test]
    async fn manager_returns_channel_not_found_when_no_channels_registered() {
        let manager = DefaultNotificationManager::new();

        let error = manager
            .notify(sample_notification(DeliveryMode::Parallel, vec![]))
            .await
            .unwrap_err();

        assert!(matches!(error, NotifError::ChannelNotFound(_)));
    }
}
