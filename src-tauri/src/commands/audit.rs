use crate::audit::diff::DifferentialEngine;
use crate::audit::extractor::PreferenceExtractor;
use crate::audit::models::{AuditSnapshot, DifferentialResult};
use crate::error::Result;

#[tauri::command]
pub async fn trigger_manual_audit() -> Result<AuditSnapshot> {
    log::info!("IPC: trigger_manual_audit requested");
    let extractor = PreferenceExtractor::new();
    let snapshot = extractor.extract_current_snapshot("mock_token").await?;
    Ok(snapshot)
}

#[tauri::command]
pub async fn get_latest_snapshot() -> Result<AuditSnapshot> {
    let extractor = PreferenceExtractor::new();
    extractor.extract_current_snapshot("mock_token").await
}

#[tauri::command]
pub async fn get_diff_history() -> Result<Vec<DifferentialResult>> {
    log::info!("IPC: get_diff_history requested");
    let extractor = PreferenceExtractor::new();
    let engine = DifferentialEngine::new();

    let current = extractor.extract_current_snapshot("mock_token").await?;
    // Generate scaffolded differential record for preview if no prior database history exists
    let diff = engine.compute_differential(None, &current);

    Ok(vec![diff])
}
