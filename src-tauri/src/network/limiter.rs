use rand::Rng;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_BACKOFF_BASE_SECONDS: i64 = 60;
pub const MAX_BACKOFF_SECONDS: i64 = 3600;

/// Adaptive rate limiter and exponential anti-throttling controller
pub struct AdaptiveRateLimiter {
    cooldown_until_epoch: Arc<AtomicI64>,
    consecutive_throttles: Arc<AtomicU32>,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            cooldown_until_epoch: Arc::new(AtomicI64::new(0)),
            consecutive_throttles: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn is_throttled(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.cooldown_until_epoch.load(Ordering::SeqCst) > now
    }

    pub fn get_remaining_cooldown_seconds(&self) -> i64 {
        let now = chrono::Utc::now().timestamp();
        let target = self.cooldown_until_epoch.load(Ordering::SeqCst);
        if target > now {
            target - now
        } else {
            0
        }
    }

    /// Inspects HTTP response headers (e.g. Retry-After, x-app-usage) for rate-limit directives
    pub fn inspect_headers(&self, status: u16, retry_after_header: Option<&str>) {
        if status == 429 {
            let retry_after_seconds = retry_after_header
                .and_then(|h| h.trim().parse::<i64>().ok())
                .unwrap_or_else(|| {
                    let attempts = self.consecutive_throttles.fetch_add(1, Ordering::SeqCst);
                    let factor = (2_i64).saturating_pow(attempts.min(5));
                    (DEFAULT_BACKOFF_BASE_SECONDS * factor).min(MAX_BACKOFF_SECONDS)
                });

            self.handle_throttle(retry_after_seconds);
        } else if status >= 200 && status < 300 {
            // Success resets consecutive throttle backoff counter
            self.consecutive_throttles.store(0, Ordering::SeqCst);
        }
    }

    /// Triggers exponential backoff with randomized jitter on HTTP 429
    pub fn handle_throttle(&self, backoff_base_seconds: i64) {
        let mut rng = rand::thread_rng();
        let jitter: i64 = rng.gen_range(5..=30);
        let cooldown = (backoff_base_seconds + jitter).min(MAX_BACKOFF_SECONDS);
        let now = chrono::Utc::now().timestamp();
        self.cooldown_until_epoch
            .store(now + cooldown, Ordering::SeqCst);

        log::warn!(
            "Meta platform throttling activated: backoff active for {} seconds (cooldown expires at {})",
            cooldown,
            now + cooldown
        );
    }

    /// Resets cooldown and throttle count
    pub fn reset(&self) {
        self.cooldown_until_epoch.store(0, Ordering::SeqCst);
        self.consecutive_throttles.store(0, Ordering::SeqCst);
    }

    /// Generates randomized humanized delay between mutations (800ms - 2400ms) to prevent bot flags
    pub async fn human_delay(&self) {
        let mut rng = rand::thread_rng();
        let delay_ms = rng.gen_range(800..=2400);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_cooldown_and_jitter() {
        let limiter = AdaptiveRateLimiter::new();
        assert!(!limiter.is_throttled());
        assert_eq!(limiter.get_remaining_cooldown_seconds(), 0);

        // Handle 429 with 60s base
        limiter.handle_throttle(60);
        assert!(limiter.is_throttled());
        let rem = limiter.get_remaining_cooldown_seconds();
        assert!(rem >= 60 && rem <= 95);

        // Header inspection with explicit Retry-After
        limiter.inspect_headers(429, Some("120"));
        assert!(limiter.is_throttled());
        let rem_header = limiter.get_remaining_cooldown_seconds();
        assert!(rem_header >= 120 && rem_header <= 155);

        // Success resets limiter
        limiter.reset();
        assert!(!limiter.is_throttled());
    }
}
