use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdCleanseError {
    #[error("Keyring credential error: {0}")]
    KeyringError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Database storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limited by platform (HTTP 429). Cooldown active.")]
    RateLimited,

    #[error("Topic parser error: {0}")]
    ParseError(String),

    #[error("Invalid session state: {0}")]
    InvalidSession(String),

    #[error("Internal system error: {0}")]
    InternalError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl Serialize for AdCleanseError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AdCleanseError>;
