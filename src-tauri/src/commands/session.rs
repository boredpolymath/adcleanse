use crate::auth::session::{AuthState, SessionStatus};
use crate::error::Result;

#[tauri::command]
pub async fn initiate_login() -> Result<SessionStatus> {
    log::info!("IPC: initiate_login requested");
    Ok(SessionStatus {
        state: AuthState::Authenticated,
        user_id: Some("100084920491823".to_string()),
        expires_at: Some(chrono::Utc::now().timestamp() + 86400 * 30),
        last_verified_at: Some(chrono::Utc::now().timestamp()),
    })
}

#[tauri::command]
pub async fn get_session_status() -> Result<SessionStatus> {
    Ok(SessionStatus {
        state: AuthState::Authenticated,
        user_id: Some("100084920491823".to_string()),
        expires_at: Some(chrono::Utc::now().timestamp() + 86400 * 30),
        last_verified_at: Some(chrono::Utc::now().timestamp()),
    })
}

#[tauri::command]
pub async fn revoke_session() -> Result<()> {
    log::info!("IPC: revoke_session requested");
    Ok(())
}
