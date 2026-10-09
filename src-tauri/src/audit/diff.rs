use crate::audit::models::{AdTopic, AuditSnapshot, DifferentialResult, PartnerUpload, RiskLevel};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

pub struct DifferentialEngine;

impl DifferentialEngine {
    pub fn new() -> Self {
        Self
    }

    /// Computes granular differences between consecutive snapshots within a strict 50 ms budget
    pub fn compute_differential(
        &self,
        previous: Option<&AuditSnapshot>,
        current: &AuditSnapshot,
    ) -> DifferentialResult {
        self.compute_differential_with_purged_history(previous, current, &HashSet::new())
    }

    /// Differential comparison with detection of re-enabled topics from historical purge records
    pub fn compute_differential_with_purged_history(
        &self,
        previous: Option<&AuditSnapshot>,
        current: &AuditSnapshot,
        historically_purged_ids: &HashSet<String>,
    ) -> DifferentialResult {
        let now = Utc::now().timestamp();

        let previous_topic_map: HashMap<String, &AdTopic> = previous
            .map(|p| p.topics.iter().map(|t| (t.id.clone(), t)).collect())
            .unwrap_or_default();

        let current_topic_map: HashMap<String, &AdTopic> =
            current.topics.iter().map(|t| (t.id.clone(), t)).collect();

        // 1. Newly added topics (not in previous snapshot and not in historically purged list)
        let mut newly_added_topics = Vec::new();
        // 2. Re-enabled topics:
        //    a) Present in previous as inactive (`is_active == false`) but now active in current (`is_active == true`)
        //    b) Or absent from previous but present in historically_purged_ids and now resurfaced in current!
        let mut re_enabled_topics = Vec::new();

        for current_topic in &current.topics {
            if let Some(prev_t) = previous_topic_map.get(&current_topic.id) {
                if !prev_t.is_active && current_topic.is_active {
                    re_enabled_topics.push(current_topic.clone());
                }
            } else if historically_purged_ids.contains(&current_topic.id) {
                re_enabled_topics.push(current_topic.clone());
            } else {
                newly_added_topics.push(current_topic.clone());
            }
        }

        // 3. Removed topics (present in previous but absent in current, or transitioned to is_active == false)
        let mut removed_topics: Vec<AdTopic> = Vec::new();
        if let Some(prev) = previous {
            for prev_t in &prev.topics {
                if let Some(curr_t) = current_topic_map.get(&prev_t.id) {
                    if prev_t.is_active && !curr_t.is_active {
                        removed_topics.push((*curr_t).clone());
                    }
                } else {
                    removed_topics.push(prev_t.clone());
                }
            }
        }

        // 4. Partner uploads differential
        let previous_partner_ids: HashSet<String> = previous
            .map(|p| p.partners.iter().map(|part| part.id.clone()).collect())
            .unwrap_or_default();

        let newly_detected_partners: Vec<PartnerUpload> = current
            .partners
            .iter()
            .filter(|p| !previous_partner_ids.contains(&p.id))
            .cloned()
            .collect();

        // 5. Calculate Tracking Vector Drift Metric (quantified drift score 0.0 to 100.0)
        let calculated_drift = self.calculate_drift_score(
            &newly_added_topics,
            &re_enabled_topics,
            &newly_detected_partners,
        );

        let prev_drift = previous.map(|p| p.drift_index).unwrap_or(0.0);
        let drift_delta = if previous.is_some() {
            (current.drift_index - prev_drift).abs()
        } else {
            calculated_drift
        };

        DifferentialResult {
            previous_snapshot_id: previous.map(|p| p.id.clone()),
            current_snapshot_id: current.id.clone(),
            newly_added_topics,
            removed_topics,
            re_enabled_topics,
            newly_detected_partners,
            drift_delta,
            calculated_at_epoch: now,
        }
    }

    /// Quantifies tracking vector drift (0.0 to 100.0) based on new topics, re-enabled categories, and partner tracking
    pub fn calculate_drift_score(
        &self,
        added_topics: &[AdTopic],
        re_enabled_topics: &[AdTopic],
        new_partners: &[PartnerUpload],
    ) -> f64 {
        let mut raw_points: f64 = 0.0;

        // Added topics weighted by risk level
        for topic in added_topics {
            raw_points += match topic.risk_level {
                RiskLevel::Critical => 10.0,
                RiskLevel::High => 5.0,
                RiskLevel::Moderate => 2.0,
                RiskLevel::Low => 0.5,
            };
        }

        // Re-enabled topics carry a high drift penalty (platform silently re-assigning purged interests)
        for _ in re_enabled_topics {
            raw_points += 8.0;
        }

        // New partners: base 3.0 pts, +2.0 if tracking pixel active, +5.0 if data broker
        for partner in new_partners {
            raw_points += 3.0;
            if partner.pixel_tracking_detected {
                raw_points += 2.0;
            }
            if partner.data_broker_category.is_some() {
                raw_points += 5.0;
            }
        }

        (raw_points).clamp(0.0, 100.0)
    }
}

impl Default for DifferentialEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::models::{RiskLevel, TopicOrigin};
    use std::time::Instant;

    fn make_test_topic(id: &str, name: &str, risk: RiskLevel, active: bool) -> AdTopic {
        AdTopic {
            id: id.to_string(),
            name: name.to_string(),
            category: "General".to_string(),
            origin: TopicOrigin::InferredBehavior,
            risk_level: risk,
            date_added_epoch: 1000,
            is_active: active,
        }
    }

    fn make_test_partner(id: &str, name: &str, pixel: bool) -> PartnerUpload {
        PartnerUpload {
            id: id.to_string(),
            company_name: name.to_string(),
            upload_window_days: 30,
            pixel_tracking_detected: pixel,
            opt_out_supported: true,
            opt_out_status: "Active Targeting".to_string(),
            first_seen_epoch: 1000,
            tracking_pixel_domain: None,
            data_broker_category: None,
        }
    }

    #[test]
    fn test_differential_calculation_added_removed() {
        let engine = DifferentialEngine::new();
        let prev = AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 20.0,
            topics: vec![
                make_test_topic("t1", "Topic 1", RiskLevel::Low, true),
                make_test_topic("t2", "Topic 2", RiskLevel::Moderate, true),
            ],
            partners: vec![],
        };

        let curr = AuditSnapshot {
            id: "snap_2".to_string(),
            timestamp_epoch: 2000,
            total_topics: 2,
            total_partners: 1,
            drift_index: 35.0,
            topics: vec![
                make_test_topic("t2", "Topic 2", RiskLevel::Moderate, true),
                make_test_topic("t3", "Topic 3", RiskLevel::High, true),
            ],
            partners: vec![make_test_partner("p1", "LiveRamp", true)],
        };

        let diff = engine.compute_differential(Some(&prev), &curr);
        assert_eq!(diff.newly_added_topics.len(), 1);
        assert_eq!(diff.newly_added_topics[0].id, "t3");
        assert_eq!(diff.removed_topics.len(), 1);
        assert_eq!(diff.removed_topics[0].id, "t1");
        assert_eq!(diff.newly_detected_partners.len(), 1);
        assert_eq!(diff.newly_detected_partners[0].id, "p1");
        assert_eq!(diff.drift_delta, 15.0);
    }

    #[test]
    fn test_re_enabled_topics_detection() {
        let engine = DifferentialEngine::new();
        let prev = AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 10.0,
            topics: vec![
                make_test_topic("t1", "Topic 1", RiskLevel::Critical, false), // Was disabled/inactive!
                make_test_topic("t2", "Topic 2", RiskLevel::Low, true),
            ],
            partners: vec![],
        };

        let curr = AuditSnapshot {
            id: "snap_2".to_string(),
            timestamp_epoch: 2000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 25.0,
            topics: vec![
                make_test_topic("t1", "Topic 1", RiskLevel::Critical, true), // Silently re-enabled!
                make_test_topic("t2", "Topic 2", RiskLevel::Low, true),
            ],
            partners: vec![],
        };

        let diff = engine.compute_differential(Some(&prev), &curr);
        assert_eq!(diff.re_enabled_topics.len(), 1);
        assert_eq!(diff.re_enabled_topics[0].id, "t1");
        assert_eq!(diff.newly_added_topics.len(), 0);
    }

    #[test]
    fn test_historically_purged_re_enabled_resurface() {
        let engine = DifferentialEngine::new();
        let prev = AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 1,
            total_partners: 0,
            drift_index: 5.0,
            topics: vec![make_test_topic("t2", "Topic 2", RiskLevel::Low, true)],
            partners: vec![],
        };

        let curr = AuditSnapshot {
            id: "snap_2".to_string(),
            timestamp_epoch: 2000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 20.0,
            topics: vec![
                make_test_topic("t1_purged", "Subprime Mortgages", RiskLevel::Critical, true),
                make_test_topic("t2", "Topic 2", RiskLevel::Low, true),
            ],
            partners: vec![],
        };

        let mut purged_history = HashSet::new();
        purged_history.insert("t1_purged".to_string());

        let diff =
            engine.compute_differential_with_purged_history(Some(&prev), &curr, &purged_history);

        assert_eq!(diff.re_enabled_topics.len(), 1);
        assert_eq!(diff.re_enabled_topics[0].id, "t1_purged");
        assert_eq!(diff.newly_added_topics.len(), 0);
    }

    #[test]
    fn test_performance_budget_under_50ms() {
        let engine = DifferentialEngine::new();

        // Synthesize large baseline with 500 topics and 100 partners
        let mut prev_topics = Vec::new();
        for i in 0..500 {
            prev_topics.push(make_test_topic(
                &format!("topic_{}", i),
                &format!("Test Topic {}", i),
                RiskLevel::Moderate,
                true,
            ));
        }

        let mut curr_topics = Vec::new();
        for i in 100..600 {
            curr_topics.push(make_test_topic(
                &format!("topic_{}", i),
                &format!("Test Topic {}", i),
                RiskLevel::Moderate,
                true,
            ));
        }

        let prev = AuditSnapshot {
            id: "snap_baseline".to_string(),
            timestamp_epoch: 1000,
            total_topics: prev_topics.len(),
            total_partners: 0,
            drift_index: 30.0,
            topics: prev_topics,
            partners: vec![],
        };

        let curr = AuditSnapshot {
            id: "snap_current".to_string(),
            timestamp_epoch: 2000,
            total_topics: curr_topics.len(),
            total_partners: 0,
            drift_index: 45.0,
            topics: curr_topics,
            partners: vec![],
        };

        let start = Instant::now();
        let diff = engine.compute_differential(Some(&prev), &curr);
        let elapsed = start.elapsed();

        // Must execute well under 50 milliseconds
        assert!(
            elapsed.as_millis() < 50,
            "Differential calculation took {}ms, exceeding 50ms budget",
            elapsed.as_millis()
        );
        assert_eq!(diff.newly_added_topics.len(), 100);
        assert_eq!(diff.removed_topics.len(), 100);
        println!("Differential calculation took: {:?}", elapsed);
    }
}
