pub mod audit;
pub mod auth;
pub mod commands;
pub mod error;
pub mod hotkey;
pub mod keyring;
pub mod network;
pub mod notifications;
pub mod scrubber;
pub mod security;
pub mod storage;
pub mod tray;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    log::info!("Initializing AdCleanse Core Engine (Zero-Telemetry Mode Active)");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            initiate_login,
            get_session_status,
            revoke_session,
            trigger_manual_audit,
            get_latest_snapshot,
            get_diff_history,
            scrub_topic,
            batch_scrub_topics,
            opt_out_partner,
            get_topic_rules,
            add_topic_rule,
            get_storage_metrics,
            export_audit_data,
            wipe_local_database,
            get_network_ledger,
            clear_network_ledger,
            get_system_status,
            toggle_spotlight_panel,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to initialize AdCleanse desktop runtime");
}
