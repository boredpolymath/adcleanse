use crate::audit::extractor::PreferenceExtractor;
use crate::audit::models::{AuditSnapshot, DifferentialResult};
use crate::error::Result;

#[tauri::command]
pub async fn trigger_manual_audit() -> Result<AuditSnapshot> {
    log::info!("IPC: trigger_manual_audit requested");
    let extractor = PreferenceExtractor::new();
    extractor.extract_current_snapshot("mock_token").await
}

#[tauri::command]
pub async fn get_latest_snapshot() -> Result<AuditSnapshot> {
    let extractor = PreferenceExtractor::new();
    extractor.extract_current_snapshot("mock_token").await
}

#[tauri::command]
pub async fn get_diff_history() -> Result<Vec<DifferentialResult>> {
    log::info!("IPC: get_diff_history requested");
    Ok(Vec::new())
}
