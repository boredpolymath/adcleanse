use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthState {
    Unauthenticated,
    Authenticating,
    Authenticated,
    SessionExpired,
    Throttled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SessionCredentials {
    pub user_id: String,
    pub session_cookie: String,
    pub datr_token: String,
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub state: AuthState,
    pub user_id: Option<String>,
    pub expires_at: Option<i64>,
    pub last_verified_at: Option<i64>,
}
