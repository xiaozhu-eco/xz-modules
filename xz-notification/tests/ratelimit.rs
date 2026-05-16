use std::{collections::HashMap, thread, time::Duration};

use xz_notification::error::NotifError;
use xz_notification::ratelimit::{ChannelRateLimit, RateLimitAction, RateLimitConfig, RateLimiter};

fn limiter_with(channel: &str, max_per_second: u32, burst: u32) -> RateLimiter {
    let mut channels = HashMap::new();
    channels.insert(
        channel.to_string(),
        ChannelRateLimit { max_per_second, burst, action: RateLimitAction::Drop },
    );
    RateLimiter::new(RateLimitConfig { channels })
}

#[test]
fn consumes_tokens_until_bucket_is_empty() {
    let mut limiter = limiter_with("push", 1, 5);

    for _ in 0..5 {
        assert!(limiter.check("push").is_ok());
    }

    match limiter.check("push") {
        Err(NotifError::RateLimited { channel, retry_after_ms }) => {
            assert_eq!(channel, "push");
            assert!(matches!(retry_after_ms, 999..=1000));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }
}

#[test]
fn refills_tokens_over_time() {
    let mut limiter = limiter_with("push", 10, 1);

    assert!(limiter.check("push").is_ok());
    thread::sleep(Duration::from_millis(120));
    assert!(limiter.check("push").is_ok());
}

#[test]
fn channels_are_isolated() {
    let mut channels = HashMap::new();
    channels.insert(
        "push".to_string(),
        ChannelRateLimit { max_per_second: 1, burst: 1, action: RateLimitAction::Drop },
    );
    channels.insert(
        "email".to_string(),
        ChannelRateLimit { max_per_second: 1, burst: 1, action: RateLimitAction::Drop },
    );

    let mut limiter = RateLimiter::new(RateLimitConfig { channels });

    assert!(limiter.check("push").is_ok());
    assert!(matches!(limiter.check("push"), Err(NotifError::RateLimited { .. })));
    assert!(limiter.check("email").is_ok());
}

#[test]
fn retry_after_ms_is_computed_from_remaining_fraction() {
    let mut limiter = limiter_with("push", 10, 1);

    assert!(limiter.check("push").is_ok());
        match limiter.check("push") {
            Err(NotifError::RateLimited { retry_after_ms, .. }) => {
                assert!(matches!(retry_after_ms, 99..=100));
            }
        other => panic!("expected rate limited error, got {other:?}"),
    }
}
