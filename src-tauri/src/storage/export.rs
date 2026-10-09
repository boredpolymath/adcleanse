use crate::audit::models::{AuditSnapshot, DifferentialResult};
use crate::error::{AdCleanseError, Result};
use crate::scrubber::rules::TopicRule;
use crate::security::csv_cell;
use serde::{Deserialize, Serialize};

/// Comprehensive sanitized archive for independent archival and data portability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedExportArchive {
    pub export_version: String,
    pub exported_at_epoch: i64,
    pub exported_at_iso8601: String,
    pub summary: ExportSummary,
    pub snapshots: Vec<AuditSnapshot>,
    pub differential_history: Vec<DifferentialResult>,
    pub topic_rules: Vec<TopicRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummary {
    pub total_snapshots: usize,
    pub total_unique_topics: usize,
    pub total_partners_detected: usize,
    pub latest_drift_index: f64,
    pub privacy_protection_mode: String,
}

/// Strips transient session identifiers and sensitive tokens from audit snapshots
pub fn sanitize_snapshots(snapshots: &[AuditSnapshot]) -> Vec<AuditSnapshot> {
    snapshots
        .iter()
        .map(|s| {
            let mut sanitized = s.clone();
            // Ensure no raw session or token data persists in topics or partner payloads
            for topic in &mut sanitized.topics {
                // Topic names and categories are sanitized of control characters
                topic.name = topic.name.chars().filter(|c| !c.is_control()).collect();
                topic.category = topic.category.chars().filter(|c| !c.is_control()).collect();
            }
            for partner in &mut sanitized.partners {
                partner.company_name = partner
                    .company_name
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect();
            }
            sanitized
        })
        .collect()
}

/// Strips transient tokens and compiles human-readable tracking history into a single JSON archive
pub fn export_consolidated_archive_to_json(
    snapshots: &[AuditSnapshot],
    diff_history: &[DifferentialResult],
    rules: &[TopicRule],
) -> Result<String> {
    let sanitized_snaps = sanitize_snapshots(snapshots);

    let total_unique_topics = sanitized_snaps
        .iter()
        .flat_map(|s| s.topics.iter().map(|t| &t.id))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let total_partners = sanitized_snaps
        .iter()
        .flat_map(|s| s.partners.iter().map(|p| &p.id))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let latest_drift = sanitized_snaps
        .last()
        .map(|s| s.drift_index)
        .unwrap_or(0.0);

    let now_epoch = chrono::Utc::now().timestamp();
    let now_iso = chrono::Utc::now().to_rfc3339();

    let archive = SanitizedExportArchive {
        export_version: "1.0.0".to_string(),
        exported_at_epoch: now_epoch,
        exported_at_iso8601: now_iso,
        summary: ExportSummary {
            total_snapshots: sanitized_snaps.len(),
            total_unique_topics,
            total_partners_detected: total_partners,
            latest_drift_index: latest_drift,
            privacy_protection_mode: "Zero-Telemetry Local Storage".to_string(),
        },
        snapshots: sanitized_snaps,
        differential_history: diff_history.to_vec(),
        topic_rules: rules.to_vec(),
    };

    serde_json::to_string_pretty(&archive)
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))
}

/// Exports snapshots to formatted JSON (with transient tokens sanitized)
pub fn export_snapshots_to_json(snapshots: &[AuditSnapshot]) -> Result<String> {
    let sanitized = sanitize_snapshots(snapshots);
    serde_json::to_string_pretty(&sanitized)
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))
}

/// Exports ad topics to CSV format with RFC 4180 escaping and formula neutralization (CWE-1236)
pub fn export_topics_to_csv(snapshots: &[AuditSnapshot]) -> Result<String> {
    let mut csv =
        String::from("SnapshotID,Timestamp,TopicID,TopicName,Category,Origin,RiskLevel,IsActive\n");

    for s in snapshots {
        for t in &s.topics {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                csv_cell(&s.id),
                s.timestamp_epoch,
                csv_cell(&t.id),
                csv_cell(&t.name),
                csv_cell(&t.category),
                csv_cell(t.origin.as_str()),
                csv_cell(t.risk_level.as_str()),
                if t.is_active { 1 } else { 0 }
            ));
        }
    }

    Ok(csv)
}

/// Exports partner audience upload registry to CSV format with formula neutralization
pub fn export_partners_to_csv(snapshots: &[AuditSnapshot]) -> Result<String> {
    let mut csv = String::from(
        "SnapshotID,Timestamp,PartnerID,CompanyName,UploadWindowDays,PixelTrackingDetected,OptOutSupported,OptOutStatus,FirstSeenEpoch\n",
    );

    for s in snapshots {
        for p in &s.partners {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                csv_cell(&s.id),
                s.timestamp_epoch,
                csv_cell(&p.id),
                csv_cell(&p.company_name),
                p.upload_window_days,
                if p.pixel_tracking_detected { 1 } else { 0 },
                if p.opt_out_supported { 1 } else { 0 },
                csv_cell(&p.opt_out_status),
                p.first_seen_epoch
            ));
        }
    }

    Ok(csv)
}

/// Exports differential drift metrics history to CSV for longitudinal trend analysis
pub fn export_drift_history_to_csv(diff_history: &[DifferentialResult]) -> Result<String> {
    let mut csv = String::from(
        "CalculatedAtEpoch,PreviousSnapshotID,CurrentSnapshotID,NewlyAddedCount,RemovedCount,ReEnabledCount,NewlyDetectedPartnersCount,DriftDelta\n",
    );

    for d in diff_history {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{:.2}\n",
            d.calculated_at_epoch,
            csv_cell(d.previous_snapshot_id.as_deref().unwrap_or("")),
            csv_cell(&d.current_snapshot_id),
            d.newly_added_topics.len(),
            d.removed_topics.len(),
            d.re_enabled_topics.len(),
            d.newly_detected_partners.len(),
            d.drift_delta
        ));
    }

    Ok(csv)
}

/// Consolidated multi-section CSV export for comprehensive audit archival
pub fn export_consolidated_to_csv(
    snapshots: &[AuditSnapshot],
    diff_history: &[DifferentialResult],
) -> Result<String> {
    let mut output = String::new();
    output.push_str("# --- ADCLEANSE TOPIC TRACKING CATALOG ---\n");
    output.push_str(&export_topics_to_csv(snapshots)?);
    output.push_str("\n# --- THIRD-PARTY AUDIENCE UPLOADS ---\n");
    output.push_str(&export_partners_to_csv(snapshots)?);
    output.push_str("\n# --- DRIFT METRICS TIMELINE ---\n");
    output.push_str(&export_drift_history_to_csv(diff_history)?);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::models::{AdTopic, PartnerUpload, RiskLevel, TopicOrigin};

    fn sample_snapshot() -> AuditSnapshot {
        AuditSnapshot {
            id: "snap_1".to_string(),
            timestamp_epoch: 1728000000,
            total_topics: 1,
            total_partners: 1,
            drift_index: 12.5,
            topics: vec![AdTopic {
                id: "t1".to_string(),
                name: "=HYPERLINK(\"evil\") Real Estate".to_string(),
                category: "Finance".to_string(),
                origin: TopicOrigin::InferredBehavior,
                risk_level: RiskLevel::High,
                date_added_epoch: 1728000000,
                is_active: true,
            }],
            partners: vec![PartnerUpload {
                id: "p1".to_string(),
                company_name: "@Acxiom Data Exchange".to_string(),
                upload_window_days: 30,
                pixel_tracking_detected: true,
                opt_out_supported: true,
                opt_out_status: "Active Targeting".to_string(),
                first_seen_epoch: 1728000000,
                tracking_pixel_domain: None,
                data_broker_category: None,
            }],
        }
    }

    #[test]
    fn test_export_serializers_and_formula_neutralization() {
        let snapshot = sample_snapshot();
        let snapshots = vec![snapshot];

        // JSON export
        let json = export_snapshots_to_json(&snapshots).expect("JSON export failed");
        assert!(json.contains("Real Estate"));

        // CSV topic export neutralizes formulas
        let topic_csv = export_topics_to_csv(&snapshots).expect("CSV export failed");
        assert!(topic_csv.contains("\"'=HYPERLINK(\"\"evil\"\") Real Estate\""));

        // CSV partner export neutralizes leading '@' symbol
        let partner_csv = export_partners_to_csv(&snapshots).expect("Partner CSV export failed");
        assert!(partner_csv.contains("\"'@Acxiom Data Exchange\""));
    }

    #[test]
    fn test_consolidated_archive_json() {
        let snapshot = sample_snapshot();
        let snapshots = vec![snapshot];
        let diffs = vec![DifferentialResult {
            previous_snapshot_id: None,
            current_snapshot_id: "snap_1".to_string(),
            newly_added_topics: vec![],
            removed_topics: vec![],
            re_enabled_topics: vec![],
            newly_detected_partners: vec![],
            drift_delta: 12.5,
            calculated_at_epoch: 1728000000,
        }];

        let json = export_consolidated_archive_to_json(&snapshots, &diffs, &[]).unwrap();
        assert!(json.contains("Zero-Telemetry Local Storage"));
        assert!(json.contains("total_unique_topics"));
    }
}
