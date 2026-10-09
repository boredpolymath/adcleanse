use crate::audit::extractor::PreferenceExtractor;
use crate::error::Result;
use crate::keyring::KeyringVault;
use crate::storage::cipher::EncryptedDatabase;
use crate::storage::export::{
    export_consolidated_archive_to_json, export_consolidated_to_csv,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub database_path: String,
    pub encryption_active: bool,
    pub cipher_mode: String,
    pub database_size_bytes: u64,
    pub snapshots_count: usize,
    pub auto_backup_count: usize,
}

#[tauri::command]
pub async fn get_storage_metrics() -> Result<StorageMetrics> {
    let db = EncryptedDatabase::default_instance();

    let db_path_str = db.db_path().to_string_lossy().to_string();
    let size_bytes = db.get_database_size_bytes();
    let backup_count = db.get_backup_count();
    let snapshots_count = db.get_snapshots_count().unwrap_or(0);

    Ok(StorageMetrics {
        database_path: db_path_str,
        encryption_active: true,
        cipher_mode: "SQLCipher (AES-256-CBC)".to_string(),
        database_size_bytes: size_bytes,
        snapshots_count,
        auto_backup_count: backup_count,
    })
}

#[tauri::command]
pub async fn export_audit_data(format: String) -> Result<String> {
    log::info!("IPC: export_audit_data format: {}", format);

    let db = EncryptedDatabase::default_instance();

    // Load snapshots from database if ready, otherwise fallback to current extractor
    let snapshots = if db.is_ready() {
        let all = db.get_all_snapshots()?;
        if all.is_empty() {
            let extractor = PreferenceExtractor::new();
            vec![extractor.extract_current_snapshot("mock_token").await?]
        } else {
            all
        }
    } else {
        let extractor = PreferenceExtractor::new();
        vec![extractor.extract_current_snapshot("mock_token").await?]
    };

    let diff_history = if db.is_ready() {
        db.get_differential_history(50)?
    } else {
        vec![]
    };

    if format.to_lowercase() == "csv" {
        export_consolidated_to_csv(&snapshots, &diff_history)
    } else {
        export_consolidated_archive_to_json(&snapshots, &diff_history, &[])
    }
}

#[tauri::command]
pub async fn wipe_local_database() -> Result<bool> {
    log::warn!("IPC: wipe_local_database executed");
    let db = EncryptedDatabase::default_instance();
    db.wipe_all()?;

    let vault = KeyringVault::new();
    vault.wipe_all()?;

    Ok(true)
}
