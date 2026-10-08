use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopicOrigin {
    OffPlatformActivity,
    AdvertiserCustomerList,
    DirectEngagement,
    LookalikeAudience,
    InferredBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdTopic {
    pub id: String,
    pub name: String,
    pub category: String,
    pub origin: TopicOrigin,
    pub risk_level: RiskLevel,
    pub date_added_epoch: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerUpload {
    pub id: String,
    pub company_name: String,
    pub upload_window_days: u32,
    pub pixel_tracking_detected: bool,
    pub opt_out_supported: bool,
    pub opt_out_status: String,
    pub first_seen_epoch: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSnapshot {
    pub id: String,
    pub timestamp_epoch: i64,
    pub total_topics: usize,
    pub total_partners: usize,
    pub drift_index: f64,
    pub topics: Vec<AdTopic>,
    pub partners: Vec<PartnerUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialResult {
    pub previous_snapshot_id: Option<String>,
    pub current_snapshot_id: String,
    pub newly_added_topics: Vec<AdTopic>,
    pub removed_topics: Vec<AdTopic>,
    pub re_enabled_topics: Vec<AdTopic>,
    pub newly_detected_partners: Vec<PartnerUpload>,
    pub drift_delta: f64,
    pub calculated_at_epoch: i64,
}
