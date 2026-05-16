use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::types::Priority;

const DEFAULT_HIGH_CAPACITY: usize = 1_000;
const DEFAULT_NORMAL_CAPACITY: usize = 5_000;
const DEFAULT_LOW_CAPACITY: usize = 10_000;
const HIGH_BURST_LIMIT: usize = 5;
const LOW_STARVATION_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueueItem {
    pub(crate) notification_id: String,
    pub(crate) priority: Priority,
    pub(crate) enqueued_at: Instant,
    pub(crate) ttl: Option<Duration>,
}

impl QueueItem {
    pub(crate) fn new(
        notification_id: impl Into<String>,
        priority: Priority,
        ttl: Option<Duration>,
    ) -> Self {
        Self {
            notification_id: notification_id.into(),
            priority,
            enqueued_at: Instant::now(),
            ttl,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PriorityQueue {
    critical: VecDeque<QueueItem>,
    high: VecDeque<QueueItem>,
    normal: VecDeque<QueueItem>,
    low: VecDeque<QueueItem>,
    background: VecDeque<QueueItem>,
    high_capacity: usize,
    normal_capacity: usize,
    low_capacity: usize,
    consecutive_high_only: usize,
    consecutive_high_plus: usize,
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityQueue {
    pub(crate) fn new() -> Self {
        Self {
            critical: VecDeque::new(),
            high: VecDeque::new(),
            normal: VecDeque::new(),
            low: VecDeque::new(),
            background: VecDeque::new(),
            high_capacity: DEFAULT_HIGH_CAPACITY,
            normal_capacity: DEFAULT_NORMAL_CAPACITY,
            low_capacity: DEFAULT_LOW_CAPACITY,
            consecutive_high_only: 0,
            consecutive_high_plus: 0,
        }
    }

    pub(crate) fn with_capacities(high_capacity: usize, normal_capacity: usize, low_capacity: usize) -> Self {
        Self {
            high_capacity,
            normal_capacity,
            low_capacity,
            ..Self::new()
        }
    }

    pub(crate) fn enqueue(&mut self, item: QueueItem) {
        let mut item = item;

        loop {
            match item.priority {
                Priority::Critical => {
                    self.critical.push_back(item);
                    break;
                }
                Priority::High => {
                    if self.high.len() < self.high_capacity {
                        self.high.push_back(item);
                        break;
                    }

                    item.priority = Priority::Normal;
                }
                Priority::Normal => {
                    if self.normal.len() < self.normal_capacity {
                        self.normal.push_back(item);
                        break;
                    }

                    item.priority = Priority::Low;
                }
                Priority::Low => {
                    if self.low.len() < self.low_capacity {
                        self.low.push_back(item);
                        break;
                    }

                    item.priority = Priority::Background;
                }
                Priority::Background => {
                    self.background.push_back(item);
                    break;
                }
            }
        }
    }

    pub(crate) fn dequeue(&mut self) -> Option<QueueItem> {
        if let Some(item) = self.critical.pop_front() {
            self.consecutive_high_only = 0;
            self.consecutive_high_plus += 1;
            return Some(item);
        }

        if self.consecutive_high_plus >= LOW_STARVATION_LIMIT {
            if let Some(item) = self.low.pop_front() {
                self.reset_high_counters();
                return Some(item);
            }
        }

        if self.consecutive_high_only >= HIGH_BURST_LIMIT {
            if let Some(item) = self.normal.pop_front() {
                self.reset_high_counters();
                return Some(item);
            }
        }

        if let Some(item) = self.high.pop_front() {
            self.consecutive_high_only += 1;
            self.consecutive_high_plus += 1;
            return Some(item);
        }

        if let Some(item) = self.normal.pop_front() {
            self.reset_high_counters();
            return Some(item);
        }

        if let Some(item) = self.low.pop_front() {
            self.reset_high_counters();
            return Some(item);
        }

        if let Some(item) = self.background.pop_front() {
            self.reset_high_counters();
            return Some(item);
        }

        None
    }

    pub(crate) fn len(&self) -> usize {
        self.critical.len()
            + self.high.len()
            + self.normal.len()
            + self.low.len()
            + self.background.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn reset_high_counters(&mut self) {
        self.consecutive_high_only = 0;
        self.consecutive_high_plus = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(notification_id: &str, priority: Priority) -> QueueItem {
        QueueItem::new(notification_id, priority, Some(Duration::from_secs(30)))
    }

    #[test]
    fn critical_items_dequeue_before_normal_items() {
        let mut queue = PriorityQueue::new();
        queue.enqueue(item("normal", Priority::Normal));
        queue.enqueue(item("critical", Priority::Critical));

        assert_eq!(queue.dequeue().map(|item| item.notification_id), Some("critical".to_string()));
        assert_eq!(queue.dequeue().map(|item| item.notification_id), Some("normal".to_string()));
    }

    #[test]
    fn high_priority_yields_to_normal_after_five_consecutive_items() {
        let mut queue = PriorityQueue::new();

        for index in 0..6 {
            queue.enqueue(item(&format!("high-{index}"), Priority::High));
        }
        queue.enqueue(item("normal", Priority::Normal));

        let drained: Vec<String> = std::iter::from_fn(|| queue.dequeue().map(|item| item.notification_id)).collect();

        assert_eq!(drained[0..5], ["high-0", "high-1", "high-2", "high-3", "high-4"]);
        assert_eq!(drained[5], "normal");
        assert_eq!(drained[6], "high-5");
    }

    #[test]
    fn starvation_prevention_allows_low_priority_after_twenty_high_plus_items() {
        let mut queue = PriorityQueue::new();
        queue.enqueue(item("low", Priority::Low));

        for index in 0..25 {
            queue.enqueue(item(&format!("high-{index}"), Priority::High));
        }

        let drained: Vec<String> = std::iter::from_fn(|| queue.dequeue().map(|item| item.notification_id)).collect();
        let low_position = drained.iter().position(|id| id == "low").expect("low item should be dequeued");

        assert_eq!(low_position, 20, "expected low item on dequeue 21, got position {} in {:?}", low_position + 1, drained);
    }

    #[test]
    fn normal_overflow_downgrades_to_low_queue() {
        let mut queue = PriorityQueue::with_capacities(8, 2, 8);
        queue.enqueue(item("normal-1", Priority::Normal));
        queue.enqueue(item("normal-2", Priority::Normal));
        queue.enqueue(item("overflow", Priority::Normal));

        let first = queue.dequeue().unwrap();
        let second = queue.dequeue().unwrap();
        let overflow = queue.dequeue().unwrap();

        assert_eq!(first.notification_id, "normal-1");
        assert_eq!(second.notification_id, "normal-2");
        assert_eq!(overflow.notification_id, "overflow");
        assert_eq!(overflow.priority, Priority::Low);
    }

    #[test]
    fn low_overflow_downgrades_to_background_queue() {
        let mut queue = PriorityQueue::with_capacities(8, 8, 1);
        queue.enqueue(item("low-1", Priority::Low));
        queue.enqueue(item("overflow", Priority::Low));

        let first = queue.dequeue().unwrap();
        let second = queue.dequeue().unwrap();

        assert_eq!(first.notification_id, "low-1");
        assert_eq!(first.priority, Priority::Low);
        assert_eq!(second.notification_id, "overflow");
        assert_eq!(second.priority, Priority::Background);
    }

    #[test]
    fn preserves_fifo_within_same_priority_level() {
        let mut queue = PriorityQueue::new();
        queue.enqueue(item("first", Priority::High));
        queue.enqueue(item("second", Priority::High));
        queue.enqueue(item("third", Priority::High));

        assert_eq!(queue.dequeue().map(|item| item.notification_id), Some("first".to_string()));
        assert_eq!(queue.dequeue().map(|item| item.notification_id), Some("second".to_string()));
        assert_eq!(queue.dequeue().map(|item| item.notification_id), Some("third".to_string()));
    }

    #[test]
    fn dequeue_returns_none_for_empty_queue() {
        let mut queue = PriorityQueue::new();

        assert!(queue.dequeue().is_none());
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn queue_item_preserves_metadata() {
        let ttl = Some(Duration::from_secs(90));
        let item = QueueItem::new("notification-123", Priority::Background, ttl);

        assert_eq!(item.notification_id, "notification-123");
        assert_eq!(item.priority, Priority::Background);
        assert_eq!(item.ttl, ttl);
        assert!(item.enqueued_at <= Instant::now());
    }
}
