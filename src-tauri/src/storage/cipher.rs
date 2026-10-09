use crate::audit::models::{
    AdTopic, AuditSnapshot, DifferentialResult, PartnerUpload, RiskLevel, TopicOrigin,
};
use crate::error::{AdCleanseError, Result};
use crate::keyring::KeyringVault;
use crate::storage::migrations::{needs_migration, run_migrations};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroize;

pub const DEFAULT_MAX_BACKUPS: usize = 5;

pub struct EncryptedDatabase {
    db_path: PathBuf,
    backup_dir: PathBuf,
    max_backups: usize,
    connection: Mutex<Option<Connection>>,
}

static GLOBAL_DB: OnceLock<EncryptedDatabase> = OnceLock::new();

impl EncryptedDatabase {
    pub fn new(db_path: PathBuf) -> Self {
        let backup_dir = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        Self {
            db_path,
            backup_dir,
            max_backups: DEFAULT_MAX_BACKUPS,
            connection: Mutex::new(None),
        }
    }

    pub fn with_backup_dir(mut self, backup_dir: PathBuf) -> Self {
        self.backup_dir = backup_dir;
        self
    }

    pub fn with_max_backups(mut self, max: usize) -> Self {
        self.max_backups = max;
        self
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Access the global singleton instance configured with standard platform paths
    pub fn default_instance() -> &'static EncryptedDatabase {
        GLOBAL_DB.get_or_init(|| Self::new(default_db_path()))
    }

    /// Initializes local SQLite database with SQLCipher AES-256 encryption pragmas and WAL mode
    pub fn initialize(&self, passphrase: &str) -> Result<()> {
        log::info!("Opening encrypted local database at: {:?}", self.db_path);

        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let is_existing_db = self.db_path.exists()
            && std::fs::metadata(&self.db_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false);

        let conn = Connection::open(&self.db_path)
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        // Enforce SQLCipher AES-256 passphrase with immediate buffer zeroization
        if !passphrase.is_empty() {
            let mut pragma_stmt = format!("PRAGMA key = '{}';", passphrase.replace('\'', "''"));
            conn.execute_batch(&pragma_stmt)
                .map_err(|e| AdCleanseError::StorageError(format!("Failed to configure SQLCipher key: {}", e)))?;
            pragma_stmt.zeroize();
        }

        // Apply performance pragmas, WAL mode, busy timeout, and foreign keys
        conn.execute_batch(
            "PRAGMA cipher_page_size = 4096;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        // Verify key by checking sqlite_master
        let count_check: std::result::Result<i64, rusqlite::Error> =
            conn.query_row("SELECT count(*) FROM sqlite_master;", [], |r| r.get(0));
        if let Err(e) = count_check {
            return Err(AdCleanseError::StorageError(format!(
                "SQLCipher passphrase rejected or database corrupted: {}",
                e
            )));
        }

        // Automatic pre-migration backup rotation: if database exists and migrations are needed
        if is_existing_db && needs_migration(&conn)? {
            log::info!("Pending schema migrations detected. Executing pre-migration automated backup rotation.");
            if let Err(err) = self.rotate_backup_internal(&conn) {
                log::warn!("Pre-migration auto-backup encountered warning: {}", err);
            }
        }

        // Run atomic migrations
        run_migrations(&conn)?;

        let mut guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        *guard = Some(conn);
        Ok(())
    }

    /// Initializes encrypted database using the master key managed by system Keyring
    pub fn initialize_with_keyring(&self, vault: &KeyringVault) -> Result<()> {
        let master_key = vault.get_or_create_db_key()?;
        self.initialize(&master_key)
    }

    /// Executes atomic local backup of encrypted database to a specific target path
    pub fn backup_to(&self, target_path: &Path) -> Result<()> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        if let Some(conn) = guard.as_ref() {
            if let Some(parent) = target_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if target_path.exists() {
                let _ = std::fs::remove_file(target_path);
            }
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

    /// Rotates backups maintaining up to `max_backups` copies (backup.1.db through backup.5.db)
    pub fn rotate_and_create_backup(&self) -> Result<PathBuf> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_ref().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;
        self.rotate_backup_internal(conn)
    }

    fn rotate_backup_internal(&self, conn: &Connection) -> Result<PathBuf> {
        let _ = std::fs::create_dir_all(&self.backup_dir);
        let max = self.max_backups.max(1);

        // Remove oldest backup copy if present (e.g. adcleanse.backup.5.db)
        let oldest = self.backup_dir.join(format!("adcleanse.backup.{}.db", max));
        if oldest.exists() {
            let _ = std::fs::remove_file(&oldest);
        }

        // Shift existing backups (e.g. 4 -> 5, 3 -> 4, etc.)
        for i in (1..max).rev() {
            let current = self.backup_dir.join(format!("adcleanse.backup.{}.db", i));
            let next = self.backup_dir.join(format!("adcleanse.backup.{}.db", i + 1));
            if current.exists() {
                let _ = std::fs::rename(&current, &next);
            }
        }

        // Newest backup is written to backup.1.db
        let target = self.backup_dir.join("adcleanse.backup.1.db");
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }

        let path_str = target.to_string_lossy();
        conn.execute("VACUUM INTO ?1", rusqlite::params![path_str])
            .map_err(|e| {
                AdCleanseError::StorageError(format!("Automated backup rotation failed: {}", e))
            })?;

        log::info!("Automated backup rotation completed: {:?}", target);
        Ok(target)
    }

    /// Automatically triggers local database backup rotation before mass purge operations (> 50 items)
    pub fn backup_before_mass_purge(&self, purge_count: usize) -> Result<Option<PathBuf>> {
        if purge_count >= 50 {
            log::info!(
                "Mass purge operation triggered ({} topics). Executing pre-purge safety backup.",
                purge_count
            );
            let backup_path = self.rotate_and_create_backup()?;
            Ok(Some(backup_path))
        } else {
            Ok(None)
        }
    }

    /// Lists existing backup files in chronological order (newest first)
    pub fn list_backups(&self) -> Vec<PathBuf> {
        let mut backups = Vec::new();
        for i in 1..=self.max_backups {
            let p = self.backup_dir.join(format!("adcleanse.backup.{}.db", i));
            if p.exists() {
                backups.push(p);
            }
        }
        backups
    }

    /// Returns count of existing backup snapshots
    pub fn get_backup_count(&self) -> usize {
        self.list_backups().len()
    }

    /// Returns file size of primary database in bytes
    pub fn get_database_size_bytes(&self) -> u64 {
        std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0)
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

        let latest_id: Option<String> = conn
            .query_row(
                "SELECT id FROM snapshots ORDER BY timestamp_epoch DESC LIMIT 1;",
                [],
                |row| row.get(0),
            )
            .ok();

        match latest_id {
            Some(id) => Self::load_snapshot_internal(conn, &id),
            None => Ok(None),
        }
    }

    /// Retrieves all historical snapshots ordered chronologically
    pub fn get_all_snapshots(&self) -> Result<Vec<AuditSnapshot>> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_ref().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let mut stmt = conn
            .prepare("SELECT id FROM snapshots ORDER BY timestamp_epoch ASC;")
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let snap_ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        drop(stmt);

        let mut snapshots = Vec::new();
        for id in snap_ids {
            if let Some(snap) = Self::load_snapshot_internal(conn, &id)? {
                snapshots.push(snap);
            }
        }
        Ok(snapshots)
    }

    fn load_snapshot_internal(conn: &Connection, snap_id: &str) -> Result<Option<AuditSnapshot>> {
        let snapshot_row = conn.query_row(
            "SELECT id, timestamp_epoch, total_topics, total_partners, drift_index FROM snapshots WHERE id = ?1;",
            rusqlite::params![snap_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)? as usize,
                    row.get::<_, i64>(3)? as usize,
                    row.get::<_, f64>(4)?,
                ))
            },
        );

        let (id, ts, total_topics, total_partners, drift) = match snapshot_row {
            Ok(data) => data,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(AdCleanseError::StorageError(e.to_string())),
        };

        let mut topic_stmt = conn
            .prepare(
                "SELECT id, name, category, origin, risk_level, date_added_epoch, is_active FROM ad_topics WHERE snapshot_id = ?1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let topic_rows = topic_stmt
            .query_map(rusqlite::params![snap_id], |row| {
                let tid: String = row.get(0)?;
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
                    id: tid,
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

        let mut partner_stmt = conn
            .prepare(
                "SELECT id, company_name, upload_window_days, pixel_tracking_detected, opt_out_supported, opt_out_status, first_seen_epoch FROM partner_uploads WHERE snapshot_id = ?1;",
            )
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let partner_rows = partner_stmt
            .query_map(rusqlite::params![snap_id], |row| {
                let pid: String = row.get(0)?;
                let company_name: String = row.get(1)?;
                let upload_window_days: u32 = row.get(2)?;
                let pixel_int: i32 = row.get(3)?;
                let opt_out_int: i32 = row.get(4)?;
                let opt_out_status: String = row.get(5)?;
                let first_seen_epoch: i64 = row.get(6)?;

                Ok(PartnerUpload {
                    id: pid,
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
            id,
            timestamp_epoch: ts,
            total_topics,
            total_partners,
            drift_index: drift,
            topics,
            partners,
        }))
    }

    /// Returns count of all recorded snapshots
    pub fn get_snapshots_count(&self) -> Result<usize> {
        let guard = self
            .connection
            .lock()
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        let conn = guard.as_ref().ok_or_else(|| {
            AdCleanseError::StorageError("Database connection not initialized".to_string())
        })?;

        let count: i64 = conn
            .query_row("SELECT count(*) FROM snapshots;", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(count as usize)
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

    /// Performs a zero-leak permanent erase of database, WAL files, and backup history
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

        // Clean up WAL and SHM files
        let wal = self.db_path.with_extension("db-wal");
        if wal.exists() {
            let _ = std::fs::remove_file(wal);
        }
        let shm = self.db_path.with_extension("db-shm");
        if shm.exists() {
            let _ = std::fs::remove_file(shm);
        }

        // Clean up backup directory
        if self.backup_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.backup_dir);
        }

        Ok(())
    }
}

/// Computes standard default database storage path per OS
pub fn default_db_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library/Application Support/com.boredpolymath.adcleanse/adcleanse.encrypted.db");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("com.boredpolymath.adcleanse\\adcleanse.encrypted.db");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local/share/com.boredpolymath.adcleanse/adcleanse.encrypted.db");
        }
    }
    std::env::temp_dir()
        .join("com.boredpolymath.adcleanse")
        .join("adcleanse.encrypted.db")
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

    #[test]
    fn test_sqlcipher_encryption_and_keyring_lifecycle() {
        let dir = std::env::temp_dir().join(format!(
            "adcleanse_cipher_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let db_path = dir.join("encrypted.db");
        let _ = std::fs::create_dir_all(&dir);

        let vault = KeyringVault::new();
        let db = EncryptedDatabase::new(db_path.clone());

        // Initialize with keyring managed key
        db.initialize_with_keyring(&vault)
            .expect("Failed to initialize encrypted database with keyring key");
        assert!(db.is_ready());

        // Save a test record
        let snapshot = AuditSnapshot {
            id: "snap_enc_1".to_string(),
            timestamp_epoch: 1000,
            total_topics: 0,
            total_partners: 0,
            drift_index: 0.0,
            topics: vec![],
            partners: vec![],
        };
        db.save_snapshot(&snapshot).unwrap();

        // Drop connection and verify that opening with an invalid key fails
        drop(db);

        let unauthorized_db = EncryptedDatabase::new(db_path.clone());
        let result = unauthorized_db.initialize("wrong_key_123456789012345678901234");
        assert!(
            result.is_err(),
            "Opening encrypted database with wrong key must fail"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_automated_backup_rotation_and_mass_purge_trigger() {
        let dir = std::env::temp_dir().join(format!(
            "adcleanse_backup_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let db_path = dir.join("test_rot.db");
        let backup_dir = dir.join("backups");
        let _ = std::fs::create_dir_all(&dir);

        let db = EncryptedDatabase::new(db_path.clone())
            .with_backup_dir(backup_dir.clone())
            .with_max_backups(3);

        db.initialize("test_passphrase_rotation_123")
            .expect("Initialize failed");

        // Rotate 4 times when max is 3
        for _ in 0..4 {
            db.rotate_and_create_backup().expect("Rotation failed");
        }

        let backups = db.list_backups();
        assert_eq!(
            backups.len(),
            3,
            "Should enforce max_backups limit by pruning oldest"
        );
        assert_eq!(db.get_backup_count(), 3);

        // Mass purge trigger: < 50 topics does not trigger backup
        let result_under = db
            .backup_before_mass_purge(49)
            .expect("Mass purge check failed");
        assert!(result_under.is_none());

        // Mass purge trigger: >= 50 topics automatically triggers backup
        let result_over = db
            .backup_before_mass_purge(50)
            .expect("Mass purge check failed");
        assert!(result_over.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
