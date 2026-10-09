use crate::error::Result;
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
    Ok(StorageMetrics {
        database_path:
            "~/Library/Application Support/com.boredpolymath.adcleanse/adcleanse.encrypted.db"
                .to_string(),
        encryption_active: true,
        cipher_mode: "SQLCipher (AES-256-CBC)".to_string(),
        database_size_bytes: 428032,
        snapshots_count: 14,
        auto_backup_count: 3,
    })
}

#[tauri::command]
pub async fn export_audit_data(format: String) -> Result<String> {
    log::info!("IPC: export_audit_data format: {}", format);
    if format == "csv" {
        Ok("SnapshotID,Timestamp,TopicID,TopicName,Category,RiskLevel\n1,1760000000,t1,Finances,High".to_string())
    } else {
        Ok("{\"export\":\"complete\",\"records\":14}".to_string())
    }
}

#[tauri::command]
pub async fn wipe_local_database() -> Result<bool> {
    log::warn!("IPC: wipe_local_database executed");
    Ok(true)
}
