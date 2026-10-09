use crate::audit::models::PartnerUpload;
use crate::error::{AdCleanseError, Result};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

const KNOWN_DATA_BROKERS: &[&str] = &[
    "acxiom",
    "liveramp",
    "oracle",
    "bluekai",
    "experian",
    "epsilon",
    "criteo",
    "neustar",
    "equifax",
    "transunion",
    "datalogix",
    "tapad",
];

pub struct PartnerInspector;

impl PartnerInspector {
    pub fn new() -> Self {
        Self
    }

    /// Queries or inspects partner custom audience uploads
    pub async fn inspect_partners(
        &self,
        _auth_token: &str,
        raw_payload: Option<&str>,
    ) -> Result<Vec<PartnerUpload>> {
        if let Some(payload) = raw_payload {
            self.parse_partner_json(payload)
        } else {
            Ok(self.default_scaffold_partners())
        }
    }

    /// Parses partner data from Meta Accounts Center / Ad Preferences JSON responses
    pub fn parse_partner_json(&self, payload: &str) -> Result<Vec<PartnerUpload>> {
        let value: Value = serde_json::from_str(payload)
            .map_err(|e| AdCleanseError::ParseError(format!("Invalid partner JSON: {}", e)))?;

        let mut partners = Vec::new();

        // 1. Array of partner objects at root or within common envelope keys
        if let Some(arr) = value.as_array() {
            for item in arr {
                if let Some(partner) = self.parse_single_partner(item) {
                    partners.push(partner);
                }
            }
        } else if let Some(obj) = value.as_object() {
            // Check known Meta GraphQL or Accounts Center keys
            let candidate_arrays = [
                "advertisers_who_uploaded_a_list",
                "custom_audience_advertisers",
                "partner_uploads",
                "data",
                "advertisers",
            ];

            for key in candidate_arrays {
                if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(partner) = self.parse_single_partner(item) {
                            partners.push(partner);
                        }
                    }
                }
            }

            // GraphQL relay format: data.viewer.ad_preferences.advertisers.edges[].node
            if let Some(edges) = value
                .pointer("/data/viewer/ad_preferences/advertisers/edges")
                .and_then(|v| v.as_array())
            {
                for edge in edges {
                    if let Some(node) = edge.get("node") {
                        if let Some(partner) = self.parse_single_partner(node) {
                            partners.push(partner);
                        }
                    }
                }
            }
        }

        if partners.is_empty() {
            log::warn!("Partner JSON parser found 0 partner entries in payload");
        }

        Ok(partners)
    }

    fn parse_single_partner(&self, item: &Value) -> Option<PartnerUpload> {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                item.get("advertiser_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_partner")
            })
            .to_string();

        let company_name = item
            .get("company_name")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("name").and_then(|v| v.as_str()))
            .or_else(|| item.get("advertiser_name").and_then(|v| v.as_str()))?
            .to_string();

        let upload_window_days = item
            .get("upload_window_days")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                item.get("window")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
            })
            .unwrap_or(90) as u32;

        let pixel_tracking_detected = item
            .get("pixel_tracking_detected")
            .and_then(|v| v.as_bool())
            .or_else(|| item.get("pixel").and_then(|v| v.as_bool()))
            .or_else(|| item.get("has_pixel").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        let opt_out_supported = item
            .get("opt_out_supported")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let opt_out_status = item
            .get("opt_out_status")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("rights").and_then(|v| v.as_str()))
            .unwrap_or("Active Targeting")
            .to_string();

        let first_seen_epoch = item
            .get("first_seen_epoch")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| Utc::now().timestamp() - (upload_window_days as i64 * 86400));

        let tracking_pixel_domain = item
            .get("tracking_pixel_domain")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("domain").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let data_broker_category = self.classify_data_broker(&company_name);

        Some(PartnerUpload {
            id,
            company_name,
            upload_window_days,
            pixel_tracking_detected,
            opt_out_supported,
            opt_out_status,
            first_seen_epoch,
            tracking_pixel_domain,
            data_broker_category,
        })
    }

    /// Classifies if the business entity is a known third-party data broker
    pub fn classify_data_broker(&self, company_name: &str) -> Option<String> {
        let lower = company_name.to_lowercase();
        for broker in KNOWN_DATA_BROKERS {
            if lower.contains(broker) {
                return Some("Third-Party Data Syndicate / Broker".to_string());
            }
        }
        None
    }

    /// Filters businesses that uploaded customer audience data within preceding 90-day window
    pub fn filter_90_day_activity(&self, partners: &[PartnerUpload]) -> Vec<PartnerUpload> {
        partners
            .iter()
            .filter(|p| p.upload_window_days <= 90)
            .cloned()
            .collect()
    }

    /// Indexes third-party tracking pixels and brand tracking associations
    /// Returns mapping of pixel domain/id to associated advertiser names
    pub fn index_tracking_pixels(
        &self,
        partners: &[PartnerUpload],
    ) -> HashMap<String, Vec<String>> {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();

        for partner in partners {
            if partner.pixel_tracking_detected {
                let key = partner
                    .tracking_pixel_domain
                    .clone()
                    .unwrap_or_else(|| format!("pixel_for_{}", partner.id));
                index
                    .entry(key)
                    .or_default()
                    .push(partner.company_name.clone());
            }
        }

        index
    }

    /// Calculates broker exposure index based on active syndicates, pixels, and window
    pub fn calculate_broker_exposure_index(&self, partners: &[PartnerUpload]) -> f64 {
        if partners.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        for p in partners {
            let mut partner_score = 10.0;
            if p.data_broker_category.is_some() {
                partner_score += 20.0;
            }
            if p.pixel_tracking_detected {
                partner_score += 15.0;
            }
            if p.upload_window_days <= 30 {
                partner_score += 10.0;
            } else if p.upload_window_days <= 60 {
                partner_score += 5.0;
            }
            score += partner_score;
        }

        // Normalize between 0.0 and 100.0
        (score / (partners.len() as f64 * 55.0) * 100.0).clamp(0.0, 100.0)
    }

    pub fn default_scaffold_partners(&self) -> Vec<PartnerUpload> {
        let now = Utc::now().timestamp();
        vec![
            PartnerUpload {
                id: "part_001".to_string(),
                company_name: "Acxiom / LiveRamp Data Exchange".to_string(),
                upload_window_days: 30,
                pixel_tracking_detected: true,
                opt_out_supported: true,
                opt_out_status: "Active Targeting".to_string(),
                first_seen_epoch: now - (30 * 86400),
                tracking_pixel_domain: Some("liveramp.com".to_string()),
                data_broker_category: Some("Third-Party Data Syndicate / Broker".to_string()),
            },
            PartnerUpload {
                id: "part_002".to_string(),
                company_name: "Oracle Advertising / BlueKai".to_string(),
                upload_window_days: 90,
                pixel_tracking_detected: true,
                opt_out_supported: true,
                opt_out_status: "Active Targeting".to_string(),
                first_seen_epoch: now - (90 * 86400),
                tracking_pixel_domain: Some("bluekai.com".to_string()),
                data_broker_category: Some("Third-Party Data Syndicate / Broker".to_string()),
            },
            PartnerUpload {
                id: "part_003".to_string(),
                company_name: "Experian Consumer Marketing".to_string(),
                upload_window_days: 60,
                pixel_tracking_detected: false,
                opt_out_supported: true,
                opt_out_status: "Active Targeting".to_string(),
                first_seen_epoch: now - (60 * 86400),
                tracking_pixel_domain: None,
                data_broker_category: Some("Third-Party Data Syndicate / Broker".to_string()),
            },
            PartnerUpload {
                id: "part_004".to_string(),
                company_name: "Epsilon Audience Syndicate".to_string(),
                upload_window_days: 90,
                pixel_tracking_detected: true,
                opt_out_supported: true,
                opt_out_status: "Active Targeting".to_string(),
                first_seen_epoch: now - (90 * 86400),
                tracking_pixel_domain: Some("epsilon.com".to_string()),
                data_broker_category: Some("Third-Party Data Syndicate / Broker".to_string()),
            },
        ]
    }
}

impl Default for PartnerInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_partner_json_array() {
        let inspector = PartnerInspector::new();
        let json = r#"[
            {
                "id": "p1",
                "company_name": "Acxiom Corporation",
                "upload_window_days": 30,
                "pixel_tracking_detected": true,
                "opt_out_supported": true,
                "opt_out_status": "Active Targeting",
                "first_seen_epoch": 1728000000,
                "tracking_pixel_domain": "acxiom.com"
            },
            {
                "id": "p2",
                "name": "Local Auto Dealer",
                "window": "120 Days",
                "pixel": false,
                "rights": "Active Targeting"
            }
        ]"#;

        let partners = inspector.parse_partner_json(json).unwrap();
        assert_eq!(partners.len(), 2);
        assert_eq!(partners[0].company_name, "Acxiom Corporation");
        assert_eq!(
            partners[0].data_broker_category,
            Some("Third-Party Data Syndicate / Broker".to_string())
        );
        assert!(partners[0].pixel_tracking_detected);
        assert_eq!(partners[1].upload_window_days, 120);

        let filtered_90 = inspector.filter_90_day_activity(&partners);
        assert_eq!(filtered_90.len(), 1);
        assert_eq!(filtered_90[0].id, "p1");

        let index = inspector.index_tracking_pixels(&partners);
        assert!(index.contains_key("acxiom.com"));
        assert_eq!(index["acxiom.com"], vec!["Acxiom Corporation"]);
    }

    #[test]
    fn test_parse_graphql_partner_envelope() {
        let inspector = PartnerInspector::new();
        let json = r#"{
            "data": {
                "viewer": {
                    "ad_preferences": {
                        "advertisers": {
                            "edges": [
                                {
                                    "node": {
                                        "id": "node_bluekai",
                                        "name": "Oracle BlueKai DMP",
                                        "upload_window_days": 60,
                                        "has_pixel": true
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        }"#;

        let partners = inspector.parse_partner_json(json).unwrap();
        assert_eq!(partners.len(), 1);
        assert_eq!(partners[0].company_name, "Oracle BlueKai DMP");
        assert!(partners[0].pixel_tracking_detected);
        assert!(partners[0].data_broker_category.is_some());
    }
}
