use crate::error::Result;
use crate::scrubber::mutator::PreferenceMutator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerRevocationResult {
    pub partner_id: String,
    pub company_name: String,
    pub success: bool,
    pub http_status: u16,
    pub timestamp_epoch: i64,
}

pub struct PartnerOptOutAutomator {
    mutator: PreferenceMutator,
}

impl PartnerOptOutAutomator {
    pub fn new() -> Self {
        Self {
            mutator: PreferenceMutator::new(),
        }
    }

    /// Automate revocation of targeting rights for specific third-party business lists and off-platform data uploads.
    pub async fn revoke_partner_access(&self, partner_id: &str, company_name: &str) -> Result<PartnerRevocationResult> {
        let scrub_result = self.mutator.opt_out_partner(partner_id, company_name).await?;
        
        // Track revocation success, failed attempts, and rate-limiting barriers per partner.
        let success = scrub_result.http_status >= 200 && scrub_result.http_status < 300;

        Ok(PartnerRevocationResult {
            partner_id: scrub_result.topic_id,
            company_name: scrub_result.topic_name,
            success,
            http_status: scrub_result.http_status,
            timestamp_epoch: scrub_result.timestamp_epoch,
        })
    }
}

impl Default for PartnerOptOutAutomator {
    fn default() -> Self {
        Self::new()
    }
}
