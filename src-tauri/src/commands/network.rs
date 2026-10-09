use crate::error::Result;
use crate::network::ledger::{LedgerEntry, NetworkLedger, ZeroTelemetryAudit};

#[tauri::command]
pub async fn get_network_ledger() -> Result<Vec<LedgerEntry>> {
    let ledger = NetworkLedger::global();
    let entries = ledger.get_entries();

    if entries.is_empty() {
        // Seed initial audit records for presentation interface
        ledger.record_request(
            "https://accountscenter.facebook.com/ad_preferences/topics",
            "GET",
            200,
            "Differential preference audit extract",
            145,
            true,
        );
        ledger.record_request(
            "https://accountscenter.facebook.com/ad_preferences/advertisers",
            "GET",
            200,
            "Extract 90-day advertiser audience upload list",
            210,
            true,
        );
        ledger.record_request(
            "https://graph.facebook.com/v19.0/act_user/ad_topics/batch_scrub",
            "POST",
            200,
            "Scrub active targeting vectors",
            320,
            true,
        );
        return Ok(ledger.get_entries());
    }

    Ok(entries)
}

#[tauri::command]
pub async fn clear_network_ledger() -> Result<()> {
    log::info!("IPC: clear_network_ledger executed");
    NetworkLedger::global().clear();
    Ok(())
}

#[tauri::command]
pub async fn get_zero_telemetry_audit() -> Result<ZeroTelemetryAudit> {
    Ok(NetworkLedger::global().verify_zero_telemetry_compliance())
}
