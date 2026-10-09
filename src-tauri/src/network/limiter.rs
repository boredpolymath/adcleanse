use rand::Rng;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct AdaptiveRateLimiter {
    cooldown_until_epoch: Arc<AtomicI64>,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            cooldown_until_epoch: Arc::new(AtomicI64::new(0)),
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

    /// Trigger exponential backoff on HTTP 429
    pub fn handle_throttle(&self, backoff_base_seconds: i64) {
        let mut rng = rand::thread_rng();
        let jitter: i64 = rng.gen_range(5..30);
        let cooldown = backoff_base_seconds + jitter;
        let now = chrono::Utc::now().timestamp();
        self.cooldown_until_epoch
            .store(now + cooldown, Ordering::SeqCst);
        log::warn!(
            "Throttling activated: backoff active for {} seconds",
            cooldown
        );
    }

    /// Generates randomized human delay between mutations (800ms - 2400ms)
    pub async fn human_delay(&self) {
        let mut rng = rand::thread_rng();
        let delay_ms = rng.gen_range(800..2400);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
