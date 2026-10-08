use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub is_running: bool,
    pub background_polling_active: bool,
    pub tray_resident: bool,
    pub spotlight_visible: bool,
    pub cooldown_seconds_remaining: i64,
}

#[tauri::command]
pub async fn get_system_status() -> Result<AppStatus> {
    Ok(AppStatus {
        is_running: true,
        background_polling_active: true,
        tray_resident: true,
        spotlight_visible: false,
        cooldown_seconds_remaining: 0,
    })
}

#[tauri::command]
pub async fn toggle_spotlight_panel() -> Result<bool> {
    log::info!("IPC: toggle_spotlight_panel");
    Ok(true)
}
