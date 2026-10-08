use crate::audit::models::{AuditSnapshot, DifferentialResult};
use chrono::Utc;
use std::collections::HashSet;

pub struct DifferentialEngine;

impl DifferentialEngine {
    pub fn new() -> Self {
        Self
    }

    /// Computes granular differences between consecutive snapshots within 50 ms budget
    pub fn compute_differential(
        &self,
        previous: Option<&AuditSnapshot>,
        current: &AuditSnapshot,
    ) -> DifferentialResult {
        let now = Utc::now().timestamp();
        
        let previous_topic_ids: HashSet<String> = previous
            .map(|p| p.topics.iter().map(|t| t.id.clone()).collect())
            .unwrap_or_default();

        let current_topic_ids: HashSet<String> = current
            .topics
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let newly_added_topics: Vec<_> = current
            .topics
            .iter()
            .filter(|t| !previous_topic_ids.contains(&t.id))
            .cloned()
            .collect();

        let removed_topics: Vec<_> = previous
            .map(|p| {
                p.topics
                    .iter()
                    .filter(|t| !current_topic_ids.contains(&t.id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Calculate tracking drift metric delta
        let prev_drift = previous.map(|p| p.drift_index).unwrap_or(0.0);
        let drift_delta = (current.drift_index - prev_drift).abs();

        DifferentialResult {
            previous_snapshot_id: previous.map(|p| p.id.clone()),
            current_snapshot_id: current.id.clone(),
            newly_added_topics,
            removed_topics,
            re_enabled_topics: Vec::new(),
            newly_detected_partners: current.partners.clone(),
            drift_delta,
            calculated_at_epoch: now,
        }
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
    use crate::audit::models::{AdTopic, RiskLevel, TopicOrigin};

    fn make_topic(id: &str, name: &str) -> AdTopic {
        AdTopic {
            id: id.to_string(),
            name: name.to_string(),
            category: "General".to_string(),
            origin: TopicOrigin::InferredBehavior,
            risk_level: RiskLevel::Low,
            date_added_epoch: 1000,
            is_active: true,
        }
    }

    #[test]
    fn test_differential_calculation() {
        let engine = DifferentialEngine::new();
        let prev = AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 20.0,
            topics: vec![make_topic("t1", "Topic 1"), make_topic("t2", "Topic 2")],
            partners: vec![],
        };

        let curr = AuditSnapshot {
            id: "snap_2".to_string(),
            timestamp_epoch: 2000,
            total_topics: 2,
            total_partners: 0,
            drift_index: 35.0,
            topics: vec![make_topic("t2", "Topic 2"), make_topic("t3", "Topic 3")],
            partners: vec![],
        };

        let diff = engine.compute_differential(Some(&prev), &curr);
        assert_eq!(diff.newly_added_topics.len(), 1);
        assert_eq!(diff.newly_added_topics[0].id, "t3");
        assert_eq!(diff.removed_topics.len(), 1);
        assert_eq!(diff.removed_topics[0].id, "t1");
        assert_eq!(diff.drift_delta, 15.0);
    }
}

