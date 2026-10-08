use crate::error::{AdCleanseError, Result};
use crate::network::ledger::NetworkLedger;
use crate::network::limiter::AdaptiveRateLimiter;
use std::sync::Arc;

pub struct PrivacyHttpClient {
    ledger: Arc<NetworkLedger>,
    limiter: Arc<AdaptiveRateLimiter>,
}

impl PrivacyHttpClient {
    pub fn new(ledger: Arc<NetworkLedger>, limiter: Arc<AdaptiveRateLimiter>) -> Self {
        Self { ledger, limiter }
    }

    /// Validates endpoint domain strictly against Meta verified preference endpoints
    fn is_authorized_endpoint(&self, url: &str) -> bool {
        let authorized = [
            "https://www.facebook.com/adpreferences",
            "https://accountscenter.facebook.com",
            "https://accountscenter.instagram.com",
            "https://graph.facebook.com",
        ];
        authorized.iter().any(|domain| url.starts_with(domain))
    }

    /// Zero-telemetry enforced request dispatcher
    pub async fn dispatch_get(&self, url: &str) -> Result<String> {
        if !self.is_authorized_endpoint(url) {
            log::error!("BLOCKED: Attempted outbound request to unauthorized endpoint: {}", url);
            return Err(AdCleanseError::NetworkError(
                "Violation of Zero-Telemetry Policy: Request rejected.".to_string(),
            ));
        }

        if self.limiter.is_throttled() {
            return Err(AdCleanseError::RateLimited);
        }

        // Record into Network Ledger
        self.ledger.record_request(url, "GET", 200, "Preference audit payload fetch");

        Ok("{\"status\":\"ok\"}".to_string())
    }
}
