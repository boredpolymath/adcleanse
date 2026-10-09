use crate::error::{AdCleanseError, Result};
use crate::network::ledger::NetworkLedger;
use crate::network::limiter::AdaptiveRateLimiter;
use crate::security::{is_allowed_https_url, AllowedEndpoint};
use reqwest::tls::Version;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use zeroize::Zeroize;

/// The complete set of endpoints AdCleanse is ever permitted to contact.
/// Enforced by parsed-URL host/path matching (see `security::is_allowed_https_url`).
pub const AUTHORIZED_ENDPOINTS: &[AllowedEndpoint] = &[
    AllowedEndpoint {
        host: "www.facebook.com",
        path_prefix: "/adpreferences",
    },
    AllowedEndpoint {
        host: "www.facebook.com",
        path_prefix: "/login",
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

/// Zero-telemetry HTTP client enforcing TLS 1.3 and strict destination allowlisting
pub struct PrivacyHttpClient {
    client: Client,
    ledger: Arc<NetworkLedger>,
    limiter: Arc<AdaptiveRateLimiter>,
}

impl PrivacyHttpClient {
    pub fn new(ledger: Arc<NetworkLedger>, limiter: Arc<AdaptiveRateLimiter>) -> Result<Self> {
        let client = Client::builder()
            .min_tls_version(Version::TLS_1_3)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AdCleanseError::NetworkError(format!("Failed to build TLS 1.3 client: {}", e)))?;

        Ok(Self {
            client,
            ledger,
            limiter,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn limiter(&self) -> &AdaptiveRateLimiter {
        &self.limiter
    }

    pub fn ledger(&self) -> &NetworkLedger {
        &self.ledger
    }

    /// Dispatches an authenticated GET request with zero-telemetry boundary enforcement
    pub async fn dispatch_get(&self, url: &str, token: Option<&str>) -> Result<String> {
        let start = Instant::now();

        // 1. Zero-telemetry boundary check: reject any request to external trackers
        if !is_authorized_endpoint(url) {
            log::error!(
                "BLOCKED: Attempted outbound request to unauthorized endpoint: {}",
                url
            );
            self.ledger.record_request(
                url,
                "GET",
                403,
                "BLOCKED: Unauthorized destination (Zero-Telemetry Violation)",
                0,
                false,
            );
            return Err(AdCleanseError::NetworkError(
                "Violation of Zero-Telemetry Policy: Outbound request blocked.".to_string(),
            ));
        }

        // 2. Adaptive rate limiter throttle check
        if self.limiter.is_throttled() {
            log::warn!("Request blocked by active anti-throttling cooldown");
            return Err(AdCleanseError::RateLimited);
        }

        let mut req_builder = self.client.get(url);
        if let Some(t) = token {
            req_builder = req_builder.bearer_auth(t);
        }

        let res = match req_builder.send().await {
            Ok(response) => response,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                self.ledger.record_request(
                    url,
                    "GET",
                    500,
                    &format!("Network Error: {}", e),
                    duration,
                    true,
                );
                return Err(AdCleanseError::NetworkError(e.to_string()));
            }
        };

        let status = res.status().as_u16();
        let retry_header = res
            .headers()
            .get("Retry-After")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        self.limiter.inspect_headers(status, retry_header.as_deref());

        let mut body = res
            .text()
            .await
            .map_err(|e| AdCleanseError::NetworkError(e.to_string()))?;

        let duration = start.elapsed().as_millis() as u64;
        self.ledger.record_request(
            url,
            "GET",
            status,
            "Fetched preference audit response",
            duration,
            true,
        );

        if status == 429 {
            body.zeroize();
            return Err(AdCleanseError::RateLimited);
        }

        Ok(body)
    }

    /// Dispatches an authenticated POST mutation with humanized delay and zero-telemetry enforcement
    pub async fn dispatch_post(
        &self,
        url: &str,
        params: &[(&str, &str)],
        token: Option<&str>,
    ) -> Result<String> {
        let start = Instant::now();

        // 1. Zero-telemetry boundary check
        if !is_authorized_endpoint(url) {
            log::error!(
                "BLOCKED: Attempted POST mutation to unauthorized endpoint: {}",
                url
            );
            self.ledger.record_request(
                url,
                "POST",
                403,
                "BLOCKED: Unauthorized destination (Zero-Telemetry Violation)",
                0,
                false,
            );
            return Err(AdCleanseError::NetworkError(
                "Violation of Zero-Telemetry Policy: Outbound request blocked.".to_string(),
            ));
        }

        // 2. Check throttling
        if self.limiter.is_throttled() {
            return Err(AdCleanseError::RateLimited);
        }

        // 3. Humanized anti-bot delay
        self.limiter.human_delay().await;

        let mut req_builder = self.client.post(url).form(&params);
        if let Some(t) = token {
            req_builder = req_builder.bearer_auth(t);
        }

        let res = match req_builder.send().await {
            Ok(response) => response,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                self.ledger.record_request(
                    url,
                    "POST",
                    500,
                    &format!("Network Error: {}", e),
                    duration,
                    true,
                );
                return Err(AdCleanseError::NetworkError(e.to_string()));
            }
        };

        let status = res.status().as_u16();
        let retry_header = res
            .headers()
            .get("Retry-After")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        self.limiter.inspect_headers(status, retry_header.as_deref());

        let mut body = res
            .text()
            .await
            .map_err(|e| AdCleanseError::NetworkError(e.to_string()))?;

        let duration = start.elapsed().as_millis() as u64;
        self.ledger.record_request(
            url,
            "POST",
            status,
            "Executed topic/partner mutation",
            duration,
            true,
        );

        if status == 429 {
            body.zeroize();
            return Err(AdCleanseError::RateLimited);
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_telemetry_endpoint_authorization() {
        assert!(is_authorized_endpoint("https://graph.facebook.com/v19.0/act_user/ad_topics"));
        assert!(is_authorized_endpoint("https://accountscenter.facebook.com/ad_preferences"));
        assert!(is_authorized_endpoint("https://accountscenter.instagram.com/ad_preferences"));
        assert!(is_authorized_endpoint("https://www.facebook.com/adpreferences/ad_settings"));

        // Reject analytics / telemetry / evil domains
        assert!(!is_authorized_endpoint("https://analytics.google.com/collect"));
        assert!(!is_authorized_endpoint("https://sentry.io/api/123/envelope/"));
        assert!(!is_authorized_endpoint("https://graph.facebook.com.evil.com/"));
        assert!(!is_authorized_endpoint("https://telemetry.boredpolymath.com/log"));
        assert!(!is_authorized_endpoint("http://graph.facebook.com/insecure"));
    }

    #[tokio::test]
    async fn test_client_blocks_unauthorized_destinations() {
        let ledger = Arc::new(NetworkLedger::new());
        let limiter = Arc::new(AdaptiveRateLimiter::new());
        let client = PrivacyHttpClient::new(ledger.clone(), limiter).unwrap();

        let res = client
            .dispatch_get("https://unauthorized-telemetry-server.com/api", None)
            .await;
        assert!(res.is_err());

        let entries = ledger.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].http_status, 403);
        assert!(!entries[0].zero_telemetry_verified);
    }
}
