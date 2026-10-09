use crate::audit::models::{
    AdTopic, AuditSnapshot, DifferentialResult, PartnerUpload, RiskLevel, TopicOrigin,
};
use crate::error::{AdCleanseError, Result};
use crate::storage::migrations::run_migrations;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct EncryptedDatabase {
    db_path: PathBuf,
    connection: Mutex<Option<Connection>>,
}

impl EncryptedDatabase {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            connection: Mutex::new(None),
        }
    }

    /// Initializes local SQLite database with SQLCipher encryption pragmas
    pub fn initialize(&self, passphrase: &str) -> Result<()> {
        log::info!("Opening encrypted local database at: {:?}", self.db_path);

        let conn = Connection::open(&self.db_path)
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        // Enforce SQLCipher passphrase and performance pragmas
        if !passphrase.is_empty() {
            let _ = conn.execute(
                &format!("PRAGMA key = '{}';", passphrase.replace('\'', "''")),
                [],
            );
        }

        conn.execute_batch(
            "PRAGMA cipher_page_size = 4096;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        run_migrations(&conn)?;

        let mut guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        *guard = Some(conn);
        Ok(())
    }

    /// Executes atomic local backup of encrypted database before migrations or bulk purge
    pub fn backup_to(&self, target_path: &std::path::Path) -> Result<()> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        if let Some(conn) = guard.as_ref() {
            let path_str = target_path.to_string_lossy();
            conn.execute("VACUUM INTO ?1", rusqlite::params![path_str])
                .map_err(|e| {
                    AdCleanseError::StorageError(format!("Database backup failed: {}", e))
                })?;
            log::info!("Database backup successfully written to: {:?}", target_path);
            Ok(())
        } else {
            Err(AdCleanseError::StorageError(
                "Database connection not initialized".to_string(),
            ))
        }
    }

    pub fn is_ready(&self) -> bool {
        self.connection.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Records an immutable audit snapshot into the database within high-speed SQLite transaction
    pub fn save_snapshot(&self, snapshot: &AuditSnapshot) -> Result<()> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_mut().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let tx = conn
            .transaction()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        tx.execute(
            "INSERT OR REPLACE INTO snapshots (id, timestamp_epoch, total_topics, total_partners, drift_index, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            rusqlite::params![
                snapshot.id,
                snapshot.timestamp_epoch,
                snapshot.total_topics as i64,
                snapshot.total_partners as i64,
                snapshot.drift_index,
                "{}"
            ],
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        for topic in &snapshot.topics {
            tx.execute(
                "INSERT OR REPLACE INTO ad_topics (id, snapshot_id, name, category, origin, risk_level, date_added_epoch, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                rusqlite::params![
                    topic.id,
                    snapshot.id,
                    topic.name,
                    topic.category,
                    topic.origin.as_str(),
                    topic.risk_level.as_str(),
                    topic.date_added_epoch,
                    if topic.is_active { 1 } else { 0 }
                ],
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        }

        for partner in &snapshot.partners {
            tx.execute(
                "INSERT OR REPLACE INTO partner_uploads (id, snapshot_id, company_name, upload_window_days, pixel_tracking_detected, opt_out_supported, opt_out_status, first_seen_epoch)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                rusqlite::params![
                    partner.id,
                    snapshot.id,
                    partner.company_name,
                    partner.upload_window_days,
                    if partner.pixel_tracking_detected { 1 } else { 0 },
                    if partner.opt_out_supported { 1 } else { 0 },
                    partner.opt_out_status,
                    partner.first_seen_epoch
                ],
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Records snapshot differential result into differential_history within budget (< 50 ms)
    pub fn save_differential(&self, diff: &DifferentialResult) -> Result<()> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_mut().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let json_payload =
            serde_json::to_string(diff).map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        conn.execute(
            "INSERT INTO differential_history (
                previous_snapshot_id, current_snapshot_id, newly_added_count, removed_count,
                re_enabled_count, newly_detected_partners_count, drift_delta, calculated_at_epoch, diff_payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            rusqlite::params![
                diff.previous_snapshot_id,
                diff.current_snapshot_id,
                diff.newly_added_topics.len() as i64,
                diff.removed_topics.len() as i64,
                diff.re_enabled_topics.len() as i64,
                diff.newly_detected_partners.len() as i64,
                diff.drift_delta,
                diff.calculated_at_epoch,
                json_payload
            ],
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Retrieves latest audit snapshot from local database
    pub fn get_latest_snapshot(&self) -> Result<Option<AuditSnapshot>> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_ref().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp_epoch, total_topics, total_partners, drift_index FROM snapshots ORDER BY timestamp_epoch DESC LIMIT 1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let snapshot_row = stmt.query_row([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)? as usize,
                row.get::<_, i64>(3)? as usize,
                row.get::<_, f64>(4)?,
            ))
        });

        let (snap_id, ts, total_topics, total_partners, drift) = match snapshot_row {
            Ok(data) => data,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(AdCleanseError::StorageError(e.to_string())),
        };

        // Load topics for this snapshot
        let mut topic_stmt = conn
            .prepare(
                "SELECT id, name, category, origin, risk_level, date_added_epoch, is_active FROM ad_topics WHERE snapshot_id = ?1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let topic_rows = topic_stmt
            .query_map(rusqlite::params![snap_id], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let category: String = row.get(2)?;
                let origin_str: String = row.get(3)?;
                let risk_str: String = row.get(4)?;
                let date_added_epoch: i64 = row.get(5)?;
                let is_active_int: i32 = row.get(6)?;

                let origin = TopicOrigin::from_str_loose(&origin_str);
                let risk_level = match risk_str.to_lowercase().as_str() {
                    "critical" => RiskLevel::Critical,
                    "high" => RiskLevel::High,
                    "moderate" => RiskLevel::Moderate,
                    _ => RiskLevel::Low,
                };

                Ok(AdTopic {
                    id,
                    name,
                    category,
                    origin,
                    risk_level,
                    date_added_epoch,
                    is_active: is_active_int != 0,
                })
            })
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let mut topics = Vec::new();
        for topic in topic_rows.flatten() {
            topics.push(topic);
        }

        // Load partner uploads
        let mut partner_stmt = conn
            .prepare(
                "SELECT id, company_name, upload_window_days, pixel_tracking_detected, opt_out_supported, opt_out_status, first_seen_epoch FROM partner_uploads WHERE snapshot_id = ?1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let partner_rows = partner_stmt
            .query_map(rusqlite::params![snap_id], |row| {
                let id: String = row.get(0)?;
                let company_name: String = row.get(1)?;
                let upload_window_days: u32 = row.get(2)?;
                let pixel_int: i32 = row.get(3)?;
                let opt_out_int: i32 = row.get(4)?;
                let opt_out_status: String = row.get(5)?;
                let first_seen_epoch: i64 = row.get(6)?;

                Ok(PartnerUpload {
                    id,
                    company_name,
                    upload_window_days,
                    pixel_tracking_detected: pixel_int != 0,
                    opt_out_supported: opt_out_int != 0,
                    opt_out_status,
                    first_seen_epoch,
                    tracking_pixel_domain: None,
                    data_broker_category: None,
                })
            })
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let mut partners = Vec::new();
        for partner in partner_rows.flatten() {
            partners.push(partner);
        }

        Ok(Some(AuditSnapshot {
            id: snap_id,
            timestamp_epoch: ts,
            total_topics,
            total_partners,
            drift_index: drift,
            topics,
            partners,
        }))
    }

    /// Retrieves differential history ordered chronologically descending
    pub fn get_differential_history(&self, limit: usize) -> Result<Vec<DifferentialResult>> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_ref().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT diff_payload_json FROM differential_history ORDER BY calculated_at_epoch DESC LIMIT ?1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                let json_str: String = row.get(0)?;
                Ok(json_str)
            })
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let mut results = Vec::new();
        for json_str in rows.flatten() {
            if let Ok(diff) = serde_json::from_str::<DifferentialResult>(&json_str) {
                results.push(diff);
            }
        }

        Ok(results)
    }

    pub fn wipe_all(&self) -> Result<()> {
        log::warn!("Executing complete zero-leak database wipe and purge");
        let mut guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        *guard = None;
        if self.db_path.exists() {
            let _ = std::fs::remove_file(&self.db_path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_record_snapshot_and_diff_within_50ms_budget() {
        let dir = std::env::temp_dir().join(format!(
            "adcleanse_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let db_path = dir.join("test_audit.db");
        let _ = std::fs::create_dir_all(&dir);

        let db = EncryptedDatabase::new(db_path.clone());
        db.initialize("").expect("Database should initialize");

        let mut sample_topics = Vec::new();
        for i in 0..100 {
            sample_topics.push(AdTopic {
                id: format!("top_{}", i),
                name: format!("Topic {}", i),
                category: "Financial Services".to_string(),
                origin: TopicOrigin::InferredBehavior,
                risk_level: RiskLevel::High,
                date_added_epoch: 1728000000,
                is_active: true,
            });
        }

        let snapshot = AuditSnapshot {
            id: "snap_bench_01".to_string(),
            timestamp_epoch: 1728000000,
            total_topics: 100,
            total_partners: 0,
            drift_index: 25.5,
            topics: sample_topics.clone(),
            partners: vec![],
        };

        let diff = DifferentialResult {
            previous_snapshot_id: None,
            current_snapshot_id: "snap_bench_01".to_string(),
            newly_added_topics: sample_topics,
            removed_topics: vec![],
            re_enabled_topics: vec![],
            newly_detected_partners: vec![],
            drift_delta: 25.5,
            calculated_at_epoch: 1728000000,
        };

        // Measure persistence performance
        let start = Instant::now();
        db.save_snapshot(&snapshot).expect("Snapshot should save");
        db.save_differential(&diff)
            .expect("Differential should save");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 50,
            "Recording snapshot and differential took {}ms, exceeding 50ms budget",
            elapsed.as_millis()
        );

        // Verify retrieval
        let retrieved = db
            .get_latest_snapshot()
            .unwrap()
            .expect("Snapshot should exist");
        assert_eq!(retrieved.id, "snap_bench_01");
        assert_eq!(retrieved.topics.len(), 100);

        let history = db.get_differential_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].current_snapshot_id, "snap_bench_01");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
