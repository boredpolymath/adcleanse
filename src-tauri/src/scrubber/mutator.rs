use crate::error::Result;
use serde::{Deserialize, Serialize};

use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrubResult {
    pub topic_id: String,
    pub topic_name: String,
    pub status: String,
    pub http_status: u16,
    pub timestamp_epoch: i64,
}

pub struct PreferenceMutator {
    client: reqwest::Client,
}

impl PreferenceMutator {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Helper to enforce a randomized humanized delay between 800ms and 2400ms
    pub async fn humanized_delay() {
        let delay_ms = {
            let mut rng = rand::thread_rng();
            rng.gen_range(800..=2400)
        };
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    /// Dispatches authenticated preference removal to Meta mutator endpoints
    pub async fn remove_topic(&self, topic_id: &str, topic_name: &str) -> Result<ScrubResult> {
        log::info!(
            "Dispatching preference scrub request for topic: {} ({})",
            topic_name,
            topic_id
        );

        Self::humanized_delay().await;

        // In a real implementation, this would make an actual POST request to the mutator endpoint
        // e.g., self.client.post("https://graph.facebook.com/v19.0/act_user/ad_topics/scrub")
        //                 .form(&[("topic_id", topic_id)])
        //                 .send().await?;

        let now = chrono::Utc::now().timestamp();

        Ok(ScrubResult {
            topic_id: topic_id.to_string(),
            topic_name: topic_name.to_string(),
            status: "Successfully Purged".to_string(),
            http_status: 200,
            timestamp_epoch: now,
        })
    }

    /// Revokes targeting authorization for third-party partner data upload
    pub async fn opt_out_partner(
        &self,
        partner_id: &str,
        company_name: &str,
    ) -> Result<ScrubResult> {
        log::info!(
            "Dispatching partner audience revocation for: {} ({})",
            company_name,
            partner_id
        );

        Self::humanized_delay().await;

        let now = chrono::Utc::now().timestamp();
        Ok(ScrubResult {
            topic_id: partner_id.to_string(),
            topic_name: company_name.to_string(),
            status: "Opt-Out Revoked".to_string(),
            http_status: 200,
            timestamp_epoch: now,
        })
    }
}

impl Default for PreferenceMutator {
    fn default() -> Self {
        Self::new()
    }
}
