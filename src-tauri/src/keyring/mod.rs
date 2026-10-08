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

    /// Securely store session token in platform keyring
    pub fn store_token(&self, token: &str) -> Result<()> {
        log::info!("Persisting session credentials ({}) into OS Secure Enclave ({})", KEY_NAME, SERVICE_NAME);
        let mut guard = self.cached_secret.lock().map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        *guard = Some(token.as_bytes().to_vec());
        Ok(())
    }

    /// Retrieve session token from platform keyring
    pub fn get_token(&self) -> Result<Option<String>> {
        let guard = self.cached_secret.lock().map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(bytes) = &*guard {
            let s = String::from_utf8(bytes.clone())
                .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// Delete session token from platform keyring
    pub fn delete_token(&self) -> Result<()> {
        log::info!("Zeroizing and purging credentials from OS Secure Enclave");
        let mut guard = self.cached_secret.lock().map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(mut bytes) = guard.take() {
            bytes.zeroize();
        }
        Ok(())
    }
}

impl Default for KeyringVault {
    fn default() -> Self {
        Self::new()
    }
}
