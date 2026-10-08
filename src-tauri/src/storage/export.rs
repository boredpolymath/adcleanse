use crate::audit::models::AuditSnapshot;
use crate::error::{AdCleanseError, Result};

pub fn export_snapshots_to_json(snapshots: &[AuditSnapshot]) -> Result<String> {
    serde_json::to_string_pretty(snapshots)
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))
}

pub fn export_topics_to_csv(snapshots: &[AuditSnapshot]) -> Result<String> {
    let mut csv = String::from("SnapshotID,Timestamp,TopicID,TopicName,Category,Origin,RiskLevel,IsActive\n");
    for s in snapshots {
        for t in &s.topics {
            csv.push_str(&format!(
                "\"{}\",{},\"{}\",\"{}\",\"{}\",\"{:?}\",\"{:?}\",{}\n",
                s.id, s.timestamp_epoch, t.id, t.name, t.category, t.origin, t.risk_level, t.is_active
            ));
        }
    }
    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::models::{AdTopic, RiskLevel, TopicOrigin};

    #[test]
    fn test_export_serializers() {
        let snapshot = AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 1,
            total_partners: 0,
            drift_index: 10.0,
            topics: vec![AdTopic {
                id: "t1".to_string(),
                name: "Real Estate".to_string(),
                category: "Finance".to_string(),
                origin: TopicOrigin::InferredBehavior,
                risk_level: RiskLevel::High,
                date_added_epoch: 1000,
                is_active: true,
            }],
            partners: vec![],
        };

        let json = export_snapshots_to_json(&[snapshot.clone()]).expect("JSON export failed");
        assert!(json.contains("Real Estate"));

        let csv = export_topics_to_csv(&[snapshot]).expect("CSV export failed");
        assert!(csv.contains("Real Estate"));
        assert!(csv.contains("Finance"));
    }
}

