use crate::error::Result;
use crate::hotkey::{SpotlightManager, SpotlightMetricsSummary};
use crate::notifications::NotificationDispatcher;
use crate::storage::cipher::EncryptedDatabase;
use crate::tray::TrayDaemonState;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static GLOBAL_TRAY: OnceLock<TrayDaemonState> = OnceLock::new();
static GLOBAL_SPOTLIGHT: OnceLock<SpotlightManager> = OnceLock::new();
static GLOBAL_NOTIFIER: OnceLock<NotificationDispatcher> = OnceLock::new();

pub fn get_global_tray() -> &'static TrayDaemonState {
    GLOBAL_TRAY.get_or_init(TrayDaemonState::new)
}

pub fn get_global_spotlight() -> &'static SpotlightManager {
    GLOBAL_SPOTLIGHT.get_or_init(SpotlightManager::new)
}

pub fn get_global_notifier() -> &'static NotificationDispatcher {
    GLOBAL_NOTIFIER.get_or_init(NotificationDispatcher::new)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub is_running: bool,
    pub background_polling_active: bool,
    pub tray_resident: bool,
    pub spotlight_visible: bool,
    pub cooldown_seconds_remaining: i64,
    pub tray_status: String,
    pub active_topics: usize,
}

#[tauri::command]
pub async fn get_system_status() -> Result<AppStatus> {
    let tray = get_global_tray();
    let spotlight = get_global_spotlight();

    Ok(AppStatus {
        is_running: true,
        background_polling_active: true,
        tray_resident: true,
        spotlight_visible: spotlight.is_visible(),
        cooldown_seconds_remaining: tray.get_cooldown_seconds(),
        tray_status: tray.get_status().badge_label().to_string(),
        active_topics: tray.get_topics_count(),
    })
}

#[tauri::command]
pub async fn toggle_spotlight_panel() -> Result<bool> {
    let spotlight = get_global_spotlight();
    spotlight.toggle_spotlight()
}

#[tauri::command]
pub async fn get_spotlight_summary() -> Result<SpotlightMetricsSummary> {
    let spotlight = get_global_spotlight();
    let db = EncryptedDatabase::default_instance();
    let snapshot = if db.is_ready() {
        db.get_latest_snapshot().ok().flatten()
    } else {
        None
    };
    Ok(spotlight.build_spotlight_summary(snapshot.as_ref()))
}
