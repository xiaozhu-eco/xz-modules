use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use xz_notification::*;
use xz_notification::{
    error::{ChannelError, NotifError},
    manager::DefaultNotificationManager,
    preference::{self, CategoryPreference, QuietHours, UserPreferences as StoredUserPreferences},
    ratelimit::{ChannelRateLimit, RateLimitAction, RateLimitConfig, RateLimiter},
    template::engine::TemplateEngine,
    traits::{NotificationChannel, NotificationHook, NotificationManager},
};

#[path = "../src/queue/priority_queue.rs"]
mod priority_queue_impl;

use priority_queue_impl::{PriorityQueue, QueueItem};

#[derive(Debug, Clone, Default)]
struct ChannelState {
    sends: usize,
    payloads: Vec<PreparedNotification>,
}

#[derive(Debug)]
struct MockChannel {
    id: String,
    kind: ChannelKind,
    state: Arc<Mutex<ChannelState>>,
}

impl MockChannel {
    fn new(id: impl Into<String>, kind: ChannelKind, state: Arc<Mutex<ChannelState>>) -> Self {
        Self {
            id: id.into(),
            kind,
            state,
        }
    }
}

#[async_trait]
impl NotificationChannel for MockChannel {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> ChannelKind {
        self.kind.clone()
    }

    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities {
            supports_delivery_confirmation: true,
            ..ChannelCapabilities::default()
        }
    }

    async fn send(
        &self,
        notification: &PreparedNotification,
    ) -> Result<DeliveryReceipt, ChannelError> {
        let mut state = self.state.lock().expect("channel state lock poisoned");
        state.sends += 1;
        state.payloads.push(notification.clone());

        Ok(DeliveryReceipt {
            channel: self.id.clone(),
            status: ChannelDeliveryStatus::Delivered,
            delivered_at: None,
        })
    }

    async fn health_check(&self) -> Result<ChannelHealth, ChannelError> {
        Ok(ChannelHealth::default())
    }
}

#[derive(Debug, Clone, Default)]
struct HookState {
    rendered: Vec<PreparedNotification>,
    delivered_channels: Vec<String>,
}

struct RecordingHook {
    state: Arc<Mutex<HookState>>,
}

#[async_trait]
impl NotificationHook for RecordingHook {
    async fn on_rendered(&self, prepared: &PreparedNotification) {
        self.state
            .lock()
            .expect("hook state lock poisoned")
            .rendered
            .push(prepared.clone());
    }

    async fn on_delivered(&self, receipt: &DeliveryReceipt) {
        self.state
            .lock()
            .expect("hook state lock poisoned")
            .delivered_channels
            .push(receipt.channel.clone());
    }
}

#[derive(Debug, Clone)]
struct MockChannelSpec {
    id: &'static str,
    kind: ChannelKind,
    max_per_second: u32,
    burst: u32,
}

impl MockChannelSpec {
    fn success(id: &'static str, kind: ChannelKind) -> Self {
        Self {
            id,
            kind,
            max_per_second: 1_000,
            burst: 1_000,
        }
    }

    fn with_rate_limit(mut self, max_per_second: u32, burst: u32) -> Self {
        self.max_per_second = max_per_second;
        self.burst = burst;
        self
    }
}

struct TestHarness {
    manager: DefaultNotificationManager,
    channel_states: HashMap<String, Arc<Mutex<ChannelState>>>,
    hook_state: Arc<Mutex<HookState>>,
}

impl TestHarness {
    fn new(specs: Vec<MockChannelSpec>, templates: &[(&str, &str)]) -> Self {
        let mut engine = TemplateEngine::new();
        for (key, template) in templates {
            engine.register_template(key, template);
        }

        let mut rate_limit_channels = HashMap::new();
        for spec in &specs {
            rate_limit_channels.insert(
                spec.id.to_string(),
                ChannelRateLimit {
                    max_per_second: spec.max_per_second,
                    burst: spec.burst,
                    action: RateLimitAction::Drop,
                },
            );
        }

        let hook_state = Arc::new(Mutex::new(HookState::default()));
        let mut manager = DefaultNotificationManager::new()
            .with_template_engine(engine)
            .with_rate_limiter(RateLimiter::new(RateLimitConfig {
                channels: rate_limit_channels,
            }));
        manager.register_hook(Box::new(RecordingHook {
            state: hook_state.clone(),
        }));

        let mut channel_states = HashMap::new();
        for spec in specs {
            let state = Arc::new(Mutex::new(ChannelState::default()));
            channel_states.insert(spec.id.to_string(), state.clone());
            manager.register_channel(Box::new(MockChannel::new(spec.id, spec.kind, state)));
        }

        Self {
            manager,
            channel_states,
            hook_state,
        }
    }

    fn channel_state(&self, id: &str) -> ChannelState {
        self.channel_states[id]
            .lock()
            .expect("channel state lock poisoned")
            .clone()
    }

    fn rendered_notifications(&self) -> Vec<PreparedNotification> {
        self.hook_state
            .lock()
            .expect("hook state lock poisoned")
            .rendered
            .clone()
    }

    fn delivered_channels(&self) -> Vec<String> {
        self.hook_state
            .lock()
            .expect("hook state lock poisoned")
            .delivered_channels
            .clone()
    }
}

fn notification_with_vars(
    template_key: &str,
    template_vars: serde_json::Value,
    priority: Priority,
    channels: Vec<ChannelKind>,
    mode: DeliveryMode,
) -> Notification {
    Notification {
        id: NotificationId::new(),
        category: NotificationCategory::Alert,
        priority,
        template_key: template_key.to_string(),
        template_vars,
        targets: vec![NotificationTarget::User {
            user_id: "user-1".into(),
        }],
        locale: Some("en-US".into()),
        actions: vec![],
        group_key: Some("ops".into()),
        data: serde_json::json!({"source": "integration-test"}),
        delivery: DeliveryPolicy {
            channels,
            mode,
            retry: RetryConfig::default(),
            channel_timeout: None,
        },
        ttl: Some(30),
        created_at: Utc::now().timestamp(),
    }
}

async fn wait_for_delivery(mut handle: DeliveryHandle) -> DeliveryStatus {
    let mut receiver = handle
        .receiver
        .take()
        .expect("delivery handle should contain a receiver");

    loop {
        let status = receiver.borrow().clone();
        if matches!(
            status.phase,
            DeliveryPhase::Completed | DeliveryPhase::PartiallyFailed | DeliveryPhase::FullyFailed
        ) {
            return status;
        }

        receiver
            .changed()
            .await
            .expect("delivery sender should stay alive until terminal status");
    }
}

fn quiet_hours_around_now() -> QuietHours {
    let now = Utc::now();
    QuietHours {
        start: (now - ChronoDuration::hours(1)).time(),
        end: (now + ChronoDuration::hours(1)).time(),
    }
}

fn is_rate_limited_error(error: &NotifError) -> bool {
    matches!(
        error,
        NotifError::AllChannelsFailed(errors)
            if errors
                .iter()
                .any(|channel_error| matches!(channel_error, ChannelError::RateLimited { .. }))
    )
}

#[tokio::test]
async fn full_notification_lifecycle_updates_handle_and_completes_delivery() {
    let harness = TestHarness::new(
        vec![MockChannelSpec::success("system-primary", ChannelKind::System)],
        &[("lifecycle", "Hello {{name}}")],
    );
    let notification = notification_with_vars(
        "lifecycle",
        serde_json::json!({"title": "Lifecycle", "name": "Ada"}),
        Priority::High,
        vec![ChannelKind::System],
        DeliveryMode::Serial,
    );

    let handle = harness
        .manager
        .notify(notification.clone())
        .await
        .expect("notification should dispatch successfully");

    assert_eq!(handle.notification_id, notification.id);

    let snapshot = handle
        .receiver
        .as_ref()
        .expect("receiver should exist")
        .borrow()
        .clone();
    assert_eq!(snapshot.channels.len(), 1);

    let final_status = wait_for_delivery(handle).await;
    assert_eq!(final_status.phase, DeliveryPhase::Completed);
    assert_eq!(final_status.status, ChannelDeliveryStatus::Delivered);
    assert_eq!(final_status.channels[0].channel, ChannelKind::System);
    assert_eq!(final_status.channels[0].status, ChannelDeliveryStatus::Delivered);
    assert_eq!(harness.channel_state("system-primary").sends, 1);
}

#[tokio::test]
async fn multi_channel_delivery_reaches_system_and_websocket_channels() {
    let harness = TestHarness::new(
        vec![
            MockChannelSpec::success("system-primary", ChannelKind::System),
            MockChannelSpec::success("ws-primary", ChannelKind::WebSocket),
        ],
        &[("broadcast", "Deploy {{version}} finished")],
    );
    let notification = notification_with_vars(
        "broadcast",
        serde_json::json!({"title": "Release", "version": "v1.2.3"}),
        Priority::Normal,
        vec![ChannelKind::System, ChannelKind::WebSocket],
        DeliveryMode::Parallel,
    );

    let final_status = wait_for_delivery(
        harness
            .manager
            .notify(notification)
            .await
            .expect("multi-channel notification should succeed"),
    )
    .await;

    assert_eq!(final_status.phase, DeliveryPhase::Completed);
    assert_eq!(final_status.channels.len(), 2);
    assert!(final_status
        .channels
        .iter()
        .all(|record| record.status == ChannelDeliveryStatus::Delivered));
    assert_eq!(harness.channel_state("system-primary").sends, 1);
    assert_eq!(harness.channel_state("ws-primary").sends, 1);

    let delivered_channels = harness.delivered_channels();
    assert_eq!(delivered_channels.len(), 2);
    assert!(delivered_channels.iter().any(|channel| channel == "system-primary"));
    assert!(delivered_channels.iter().any(|channel| channel == "ws-primary"));
}

#[tokio::test]
async fn dnd_suppression_blocks_normal_priority_notifications() {
    let harness = TestHarness::new(
        vec![MockChannelSpec::success("system-primary", ChannelKind::System)],
        &[("quiet-hours", "Ping {{name}}")],
    );
    let notification = notification_with_vars(
        "quiet-hours",
        serde_json::json!({"title": "Quiet hours", "name": "Ada"}),
        Priority::Normal,
        vec![ChannelKind::System],
        DeliveryMode::Serial,
    );

    let mut category_preferences = HashMap::new();
    category_preferences.insert(
        NotificationCategory::Alert,
        CategoryPreference {
            enabled: true,
            allowed_channels: vec!["system-primary".into()],
            priority_override: None,
        },
    );
    let preferences = StoredUserPreferences {
        notifications_enabled: true,
        category_preferences,
        quiet_hours: Some(quiet_hours_around_now()),
        channel_priority: vec!["system-primary".into()],
    };

    let allowed = preference::should_deliver(
        &preferences,
        &notification.category,
        notification.priority,
        "system-primary",
    );

    assert!(!allowed, "normal-priority notifications should be suppressed during DND");
    assert_eq!(harness.channel_state("system-primary").sends, 0);
}

#[tokio::test]
async fn rate_limiter_rejects_rapid_burst_after_first_delivery() {
    let harness = TestHarness::new(
        vec![MockChannelSpec::success("system-primary", ChannelKind::System)
            .with_rate_limit(1, 1)],
        &[("burst", "Rate limit {{index}}")],
    );

    let results = vec![
        harness
            .manager
            .notify(notification_with_vars(
                "burst",
                serde_json::json!({"title": "Burst", "index": 1}),
                Priority::Normal,
                vec![ChannelKind::System],
                DeliveryMode::Serial,
            ))
            .await,
        harness
            .manager
            .notify(notification_with_vars(
                "burst",
                serde_json::json!({"title": "Burst", "index": 2}),
                Priority::Normal,
                vec![ChannelKind::System],
                DeliveryMode::Serial,
            ))
            .await,
        harness
            .manager
            .notify(notification_with_vars(
                "burst",
                serde_json::json!({"title": "Burst", "index": 3}),
                Priority::Normal,
                vec![ChannelKind::System],
                DeliveryMode::Serial,
            ))
            .await,
    ];

    let successful = results.iter().filter(|result| result.is_ok()).count();
    let rate_limited = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .filter(|error| is_rate_limited_error(error))
        .count();

    assert!(successful >= 1, "expected at least one delivery before rate limiting");
    assert!(rate_limited >= 1, "expected at least one rate-limited notification");
    assert_eq!(harness.channel_state("system-primary").sends, successful);
}

#[tokio::test]
async fn template_rendering_produces_expected_prepared_notification() {
    let harness = TestHarness::new(
        vec![MockChannelSpec::success("system-primary", ChannelKind::System)],
        &[("incident", "Service {{service}} is {{state}}")],
    );

    harness
        .manager
        .notify_and_wait(notification_with_vars(
            "incident",
            serde_json::json!({
                "title": "Incident detected",
                "service": "billing",
                "state": "degraded",
                "subtitle": "SEV-1",
                "sound": "alarm"
            }),
            Priority::High,
            vec![ChannelKind::System],
            DeliveryMode::Serial,
        ))
        .await
        .expect("template-backed notification should deliver");

    let rendered = harness.rendered_notifications();
    let prepared = rendered
        .last()
        .expect("render hook should capture the prepared notification");

    assert_eq!(prepared.title.as_deref(), Some("Incident detected"));
    assert_eq!(prepared.body.as_deref(), Some("Service billing is degraded"));
    assert_eq!(prepared.subtitle.as_deref(), Some("SEV-1"));
    assert_eq!(prepared.sound.as_deref(), Some("alarm"));

    let sent_payloads = harness.channel_state("system-primary").payloads;
    assert_eq!(sent_payloads.len(), 1);
    assert_eq!(sent_payloads[0], *prepared);
}

#[tokio::test]
async fn priority_queue_dequeues_critical_notifications_first() {
    let mut queue = PriorityQueue::new();
    queue.enqueue(QueueItem::new("normal", Priority::Normal, None));
    queue.enqueue(QueueItem::new("critical", Priority::Critical, None));
    queue.enqueue(QueueItem::new("high", Priority::High, None));
    queue.enqueue(QueueItem::new("background", Priority::Background, None));

    let first = queue.dequeue().expect("first item should exist");
    let second = queue.dequeue().expect("second item should exist");
    let third = queue.dequeue().expect("third item should exist");
    let fourth = queue.dequeue().expect("fourth item should exist");

    assert_eq!(first.notification_id, "critical");
    assert_eq!(first.priority, Priority::Critical);
    assert_eq!(second.notification_id, "high");
    assert_eq!(third.notification_id, "normal");
    assert_eq!(fourth.notification_id, "background");
}
