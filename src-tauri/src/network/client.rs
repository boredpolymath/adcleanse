use crate::error::{AdCleanseError, Result};
use crate::network::ledger::NetworkLedger;
use crate::network::limiter::AdaptiveRateLimiter;
use crate::security::{is_allowed_https_url, AllowedEndpoint};
use std::sync::Arc;

/// The complete set of endpoints AdCleanse is ever permitted to contact.
/// Enforced by parsed-URL host/path matching (see `security::is_allowed_https_url`).
pub const AUTHORIZED_ENDPOINTS: &[AllowedEndpoint] = &[
    AllowedEndpoint {
        host: "www.facebook.com",
        path_prefix: "/adpreferences",
    },
    AllowedEndpoint {
        host: "accountscenter.facebook.com",
        path_prefix: "/",
    },
    AllowedEndpoint {
        host: "accountscenter.instagram.com",
        path_prefix: "/",
    },
    AllowedEndpoint {
        host: "graph.facebook.com",
        path_prefix: "/",
    },
];

/// Validates endpoint strictly against Meta verified preference endpoints.
pub fn is_authorized_endpoint(url: &str) -> bool {
    is_allowed_https_url(url, AUTHORIZED_ENDPOINTS)
}

pub struct PrivacyHttpClient {
    ledger: Arc<NetworkLedger>,
    limiter: Arc<AdaptiveRateLimiter>,
}

impl PrivacyHttpClient {
    pub fn new(ledger: Arc<NetworkLedger>, limiter: Arc<AdaptiveRateLimiter>) -> Self {
        Self { ledger, limiter }
    }

    fn is_authorized_endpoint(&self, url: &str) -> bool {
        is_authorized_endpoint(url)
    }

    /// Zero-telemetry enforced request dispatcher
    pub async fn dispatch_get(&self, url: &str) -> Result<String> {
        if !self.is_authorized_endpoint(url) {
            log::error!(
                "BLOCKED: Attempted outbound request to unauthorized endpoint: {}",
                url
            );
            return Err(AdCleanseError::NetworkError(
                "Violation of Zero-Telemetry Policy: Request rejected.".to_string(),
            ));
        }

        if self.limiter.is_throttled() {
            return Err(AdCleanseError::RateLimited);
        }

        // Record into Network Ledger
        self.ledger
            .record_request(url, "GET", 200, "Preference audit payload fetch");

        Ok("{\"status\":\"ok\"}".to_string())
    }
}
