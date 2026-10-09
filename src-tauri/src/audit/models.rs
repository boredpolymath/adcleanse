use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl RiskLevel {
    pub fn weight(&self) -> f64 {
        match self {
            RiskLevel::Critical => 10.0,
            RiskLevel::High => 5.0,
            RiskLevel::Moderate => 2.0,
            RiskLevel::Low => 0.5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Moderate => "Moderate",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopicOrigin {
    OffPlatformActivity,
    AdvertiserCustomerList,
    DirectEngagement,
    LookalikeAudience,
    InferredBehavior,
}

impl TopicOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            TopicOrigin::OffPlatformActivity => "Off-Platform Activity",
            TopicOrigin::AdvertiserCustomerList => "Advertiser Customer List",
            TopicOrigin::DirectEngagement => "Direct Engagement",
            TopicOrigin::LookalikeAudience => "Lookalike Audience",
            TopicOrigin::InferredBehavior => "Inferred Behavior",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("off_platform")
            || lower.contains("off_facebook")
            || lower.contains("off-platform")
            || lower.contains("pixel")
            || lower.contains("sdk")
        {
            TopicOrigin::OffPlatformActivity
        } else if lower.contains("customer_list")
            || lower.contains("advertiser")
            || lower.contains("uploaded")
            || lower.contains("custom_audience")
        {
            TopicOrigin::AdvertiserCustomerList
        } else if lower.contains("direct")
            || lower.contains("engagement")
            || lower.contains("likes")
            || lower.contains("clicks")
        {
            TopicOrigin::DirectEngagement
        } else if lower.contains("lookalike")
            || lower.contains("modeled")
            || lower.contains("similar")
        {
            TopicOrigin::LookalikeAudience
        } else {
            TopicOrigin::InferredBehavior
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdTopic {
    pub id: String,
    pub name: String,
    pub category: String,
    pub origin: TopicOrigin,
    #[serde(alias = "risk")]
    pub risk_level: RiskLevel,
    pub date_added_epoch: i64,
    #[serde(default = "default_active", alias = "active")]
    pub is_active: bool,
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartnerUpload {
    pub id: String,
    #[serde(alias = "name")]
    pub company_name: String,
    #[serde(alias = "window")]
    pub upload_window_days: u32,
    #[serde(alias = "pixel")]
    pub pixel_tracking_detected: bool,
    #[serde(default = "default_opt_out_supported")]
    pub opt_out_supported: bool,
    #[serde(default = "default_opt_out_status", alias = "rights")]
    pub opt_out_status: String,
    pub first_seen_epoch: i64,
    #[serde(default)]
    pub tracking_pixel_domain: Option<String>,
    #[serde(default)]
    pub data_broker_category: Option<String>,
}

fn default_opt_out_supported() -> bool {
    true
}

fn default_opt_out_status() -> String {
    "Active Targeting".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditSnapshot {
    pub id: String,
    pub timestamp_epoch: i64,
    pub total_topics: usize,
    pub total_partners: usize,
    pub drift_index: f64,
    pub topics: Vec<AdTopic>,
    pub partners: Vec<PartnerUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
