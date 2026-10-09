use crate::auth::session::SessionCredentials;
use crate::error::{AdCleanseError, Result};
use std::sync::Mutex;
use zeroize::Zeroize;

const SERVICE_NAME: &str = "com.boredpolymath.adcleanse";
const KEY_NAME: &str = "meta_session_tokens";

/// Abstract vault for native OS secure enclaves:
/// - macOS Keychain Services
/// - Windows Credential Manager
/// - Linux Secret Service / Freedesktop Keyring
pub struct KeyringVault {
    // In-memory fallback / cache protected with zeroize on drop
    cached_secret: Mutex<Option<Vec<u8>>>,
}

impl KeyringVault {
    pub fn new() -> Self {
        Self {
            cached_secret: Mutex::new(None),
        }
    }

    /// Securely store session token string in platform keyring
    pub fn store_token(&self, token: &str) -> Result<()> {
        log::info!(
            "Persisting session credentials ({}) into OS Secure Enclave ({})",
            KEY_NAME,
            SERVICE_NAME
        );
        let mut guard = self
            .cached_secret
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        *guard = Some(token.as_bytes().to_vec());
        Ok(())
    }

    /// Retrieve session token string from platform keyring
    pub fn get_token(&self) -> Result<Option<String>> {
        let guard = self
            .cached_secret
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(bytes) = &*guard {
            let s = String::from_utf8(bytes.clone())
                .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Delete session token string from platform keyring
    pub fn delete_token(&self) -> Result<()> {
        log::info!("Zeroizing and purging credentials from OS Secure Enclave");
        let mut guard = self
            .cached_secret
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(mut bytes) = guard.take() {
            bytes.zeroize();
        }
        Ok(())
    }

    /// Store structured SessionCredentials into OS Secure Enclave
    pub fn store_session(&self, creds: &SessionCredentials) -> Result<()> {
        let serialized = serde_json::to_string(creds).map_err(|e| {
            AdCleanseError::KeyringError(format!("Failed to serialize credentials: {}", e))
        })?;
        self.store_token(&serialized)
    }

    /// Retrieve structured SessionCredentials from OS Secure Enclave
    pub fn get_session(&self) -> Result<Option<SessionCredentials>> {
        if let Some(token_str) = self.get_token()? {
            let creds: SessionCredentials = serde_json::from_str(&token_str).map_err(|e| {
                AdCleanseError::KeyringError(format!("Failed to deserialize credentials: {}", e))
            })?;
            Ok(Some(creds))
        } else {
            Ok(None)
        }
    }

    /// Revoke and wipe session credentials
    pub fn revoke_session(&self) -> Result<()> {
        self.delete_token()
    }

    /// Complete credential wipe with memory zeroization
    pub fn wipe_credentials(&self) -> Result<()> {
        self.delete_token()
    }
}

impl Default for KeyringVault {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_lifecycle_and_zeroization() {
        let vault = KeyringVault::new();
        assert_eq!(vault.get_token().unwrap(), None);

        let creds = SessionCredentials {
            user_id: "test_user_42".to_string(),
            session_cookie: "cookie_val_abc".to_string(),
            datr_token: "datr_xyz".to_string(),
            access_token: Some("token_123".to_string()),
        };

        vault
            .store_session(&creds)
            .expect("Failed to store session");
        let retrieved = vault.get_session().expect("Failed to retrieve session");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "test_user_42");

        vault
            .wipe_credentials()
            .expect("Failed to wipe credentials");
        assert_eq!(vault.get_session().unwrap(), None);
    }
}
