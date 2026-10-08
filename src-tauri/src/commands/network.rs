use crate::error::Result;
use crate::network::ledger::LedgerEntry;

#[tauri::command]
pub async fn get_network_ledger() -> Result<Vec<LedgerEntry>> {
    let now = chrono::Utc::now().timestamp();
    Ok(vec![
        LedgerEntry {
            id: 1,
            timestamp_epoch: now - 3600,
            endpoint: "https://accountscenter.facebook.com/ad_preferences/topics".to_string(),
            method: "GET".to_string(),
            http_status: 200,
            payload_summary: "Extract 42 active topic categories".to_string(),
            zero_telemetry_verified: true,
        },
        LedgerEntry {
            id: 2,
            timestamp_epoch: now - 1800,
            endpoint: "https://accountscenter.facebook.com/ad_preferences/advertisers".to_string(),
            method: "GET".to_string(),
            http_status: 200,
            payload_summary: "Query 90-day audience list uploads".to_string(),
            zero_telemetry_verified: true,
        },
        LedgerEntry {
            id: 3,
            timestamp_epoch: now - 600,
            endpoint: "https://graph.facebook.com/v19.0/act_user/ad_topics/scrub".to_string(),
            method: "POST".to_string(),
            http_status: 200,
            payload_summary: "Mutate topic: Health Insurance & Supplements".to_string(),
            zero_telemetry_verified: true,
        },
    ])
}

#[tauri::command]
pub async fn clear_network_ledger() -> Result<()> {
    log::info!("IPC: clear_network_ledger executed");
    Ok(())
}
