use crate::error::{AdCleanseError, Result};
use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: i32 = 2;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version: i32 = conn
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .unwrap_or(0);

    log::info!("Current database schema version: {}", current_version);

    if current_version < 1 {
        log::info!("Applying initial schema migration v1");
        conn.execute_batch(
            "BEGIN TRANSACTION;

            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                timestamp_epoch INTEGER NOT NULL,
                total_topics INTEGER NOT NULL,
                total_partners INTEGER NOT NULL,
                drift_index REAL NOT NULL,
                metadata TEXT
            );

            CREATE TABLE IF NOT EXISTS ad_topics (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                origin TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                date_added_epoch INTEGER NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS partner_uploads (
                id TEXT PRIMARY KEY,
                snapshot_id TEXT NOT NULL,
                company_name TEXT NOT NULL,
                upload_window_days INTEGER NOT NULL,
                pixel_tracking_detected INTEGER NOT NULL,
                opt_out_supported INTEGER NOT NULL,
                opt_out_status TEXT NOT NULL,
                first_seen_epoch INTEGER NOT NULL,
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS topic_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                is_regex INTEGER NOT NULL DEFAULT 0,
                auto_scrub_enabled INTEGER NOT NULL DEFAULT 1,
                created_at_epoch INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_ledger (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_epoch INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                target TEXT NOT NULL,
                status TEXT NOT NULL,
                http_status INTEGER,
                details TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_topics_snapshot ON ad_topics(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_partners_snapshot ON partner_uploads(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_ledger_timestamp ON audit_ledger(timestamp_epoch);

            PRAGMA user_version = 1;
            COMMIT;",
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
    }

    if current_version < 2 {
        log::info!("Applying schema migration v2: differential history tracking");
        conn.execute_batch(
            "BEGIN TRANSACTION;

            CREATE TABLE IF NOT EXISTS differential_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                previous_snapshot_id TEXT,
                current_snapshot_id TEXT NOT NULL,
                newly_added_count INTEGER NOT NULL,
                removed_count INTEGER NOT NULL,
                re_enabled_count INTEGER NOT NULL,
                newly_detected_partners_count INTEGER NOT NULL,
                drift_delta REAL NOT NULL,
                calculated_at_epoch INTEGER NOT NULL,
                diff_payload_json TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_diff_current ON differential_history(current_snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_diff_calc_epoch ON differential_history(calculated_at_epoch);

            PRAGMA user_version = 2;
            COMMIT;",
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
    }

    Ok(())
}
