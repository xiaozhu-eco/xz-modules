use std::sync::atomic::{AtomicU64, Ordering};

pub struct TtsMetrics {
    pub sessions_total: AtomicU64,
    pub sessions_active: AtomicU64,
    pub audio_bytes_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub first_chunk_latency_us: AtomicU64,
}

impl TtsMetrics {
    pub fn new() -> Self {
        Self {
            sessions_total: AtomicU64::new(0),
            sessions_active: AtomicU64::new(0),
            audio_bytes_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            first_chunk_latency_us: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> TtsMetricsSnapshot {
        TtsMetricsSnapshot {
            sessions_total: self.sessions_total.load(Ordering::Relaxed),
            sessions_active: self.sessions_active.load(Ordering::Relaxed),
            audio_bytes_total: self.audio_bytes_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            first_chunk_latency_us: self.first_chunk_latency_us.load(Ordering::Relaxed),
        }
    }
}

impl Default for TtsMetrics {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Default)]
pub struct TtsMetricsSnapshot {
    pub sessions_total: u64,
    pub sessions_active: u64,
    pub audio_bytes_total: u64,
    pub errors_total: u64,
    pub first_chunk_latency_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_defaults_are_zero() {
        let m = TtsMetrics::new();
        let snap = m.snapshot();
        assert_eq!(snap.sessions_total, 0);
        assert_eq!(snap.sessions_active, 0);
        assert_eq!(snap.errors_total, 0);
    }
}
