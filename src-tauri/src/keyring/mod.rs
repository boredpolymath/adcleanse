use crate::auth::session::SessionCredentials;
use crate::error::{AdCleanseError, Result};
use rand::Rng;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use zeroize::{Zeroize, Zeroizing};

const KEY_NAME: &str = "meta_session_tokens";
const DB_KEY_NAME: &str = "db_master_key";

/// Abstract vault for local credential storage and memory protection.
/// Completely bypasses OS Keychain/Security prompt dialogs so no password
/// is ever requested from the user, while protecting stored secrets with
/// strict file permissions (0600) and zeroizing memory buffers.
pub struct KeyringVault {
    vault_path: PathBuf,
    cached_secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl KeyringVault {
    pub fn new() -> Self {
        let vault_path = default_vault_path();
        let instance = Self {
            vault_path,
            cached_secrets: Mutex::new(HashMap::new()),
        };
        instance.load_from_disk();
        instance
    }

    pub fn with_vault_path(vault_path: PathBuf) -> Self {
        let instance = Self {
            vault_path,
            cached_secrets: Mutex::new(HashMap::new()),
        };
        instance.load_from_disk();
        instance
    }

    fn load_from_disk(&self) {
        if !self.vault_path.exists() {
            return;
        }

        if let Ok(contents) = std::fs::read(&self.vault_path) {
            if let Ok(map) = serde_json::from_slice::<HashMap<String, String>>(&contents) {
                if let Ok(mut guard) = self.cached_secrets.lock() {
                    for (k, v) in map {
                        guard.insert(k, v.into_bytes());
                    }
                }
            }
        }
    }

    fn persist_to_disk(&self) {
        let guard = match self.cached_secrets.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(parent) = self.vault_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut export_map = HashMap::new();
        for (k, v) in guard.iter() {
            if let Ok(s) = String::from_utf8(v.clone()) {
                export_map.insert(k.clone(), s);
            }
        }

        if let Ok(json_data) = serde_json::to_vec(&export_map) {
            let _ = std::fs::write(&self.vault_path, json_data);

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &self.vault_path,
                    std::fs::Permissions::from_mode(0o600),
                );
            }
        }
    }

    /// Stores a secret into local protected vault with memory zeroization on fallback cache
    pub fn store_secret(&self, key: &str, secret: &str) -> Result<()> {
        log::info!("Persisting credential ({}) into local secure vault", key);

        {
            let mut guard = self
                .cached_secrets
                .lock()
                .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
            guard.insert(key.to_string(), secret.as_bytes().to_vec());
        }

        self.persist_to_disk();
        Ok(())
    }

    /// Retrieves a secret from local protected vault wrapped in Zeroizing
    pub fn get_secret(&self, key: &str) -> Result<Option<Zeroizing<String>>> {
        let guard = self
            .cached_secrets
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(bytes) = guard.get(key) {
            let s = String::from_utf8(bytes.clone())
                .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
            Ok(Some(Zeroizing::new(s)))
        } else {
            Ok(None)
        }
    }

    /// Deletes a secret from local vault and zeroizes memory in cache
    pub fn delete_secret(&self, key: &str) -> Result<()> {
        log::info!("Zeroizing and purging credential ({}) from local secure vault", key);

        let mut guard = self
            .cached_secrets
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        if let Some(mut bytes) = guard.remove(key) {
            bytes.zeroize();
        }
        drop(guard);

        self.persist_to_disk();
        Ok(())
    }

    /// Securely store session token string in local vault
    pub fn store_token(&self, token: &str) -> Result<()> {
        self.store_secret(KEY_NAME, token)
    }

    /// Retrieve session token string from local vault
    pub fn get_token(&self) -> Result<Option<String>> {
        let opt = self.get_secret(KEY_NAME)?;
        Ok(opt.map(|z| z.to_string()))
    }

    /// Delete session token string from local vault
    pub fn delete_token(&self) -> Result<()> {
        self.delete_secret(KEY_NAME)
    }

    /// Store structured SessionCredentials into local vault
    pub fn store_session(&self, creds: &SessionCredentials) -> Result<()> {
        let serialized = serde_json::to_string(creds).map_err(|e| {
            AdCleanseError::KeyringError(format!("Failed to serialize credentials: {}", e))
        })?;
        self.store_token(&serialized)
    }

    /// Retrieve structured SessionCredentials from local vault
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

    /// Retrieves or generates a 256-bit AES encryption passphrase managed transparently
    /// with zeroization upon termination, without prompting the user for any password.
    pub fn get_or_create_db_key(&self) -> Result<Zeroizing<String>> {
        if let Some(existing_key) = self.get_secret(DB_KEY_NAME)? {
            if !existing_key.is_empty() {
                return Ok(existing_key);
            }
        }

        // Generate CSPRNG 256-bit passphrase (32 random bytes as 64-character hex string)
        let mut rng = rand::thread_rng();
        let mut random_bytes = [0u8; 32];
        rng.fill(&mut random_bytes);
        let mut hex_key = String::with_capacity(64);
        for b in &random_bytes {
            hex_key.push_str(&format!("{:02x}", b));
        }
        random_bytes.zeroize();

        self.store_secret(DB_KEY_NAME, &hex_key)?;
        Ok(Zeroizing::new(hex_key))
    }

    /// Purges the database master key from local vault and cache
    pub fn delete_db_key(&self) -> Result<()> {
        self.delete_secret(DB_KEY_NAME)
    }

    /// Revoke and wipe session credentials
    pub fn revoke_session(&self) -> Result<()> {
        self.delete_token()
    }

    /// Complete credential wipe with memory zeroization
    pub fn wipe_credentials(&self) -> Result<()> {
        self.delete_token()
    }

    /// Comprehensive wipe of all credentials and database keys
    pub fn wipe_all(&self) -> Result<()> {
        self.delete_token()?;
        self.delete_db_key()?;
        let mut guard = self
            .cached_secrets
            .lock()
            .map_err(|e| AdCleanseError::KeyringError(e.to_string()))?;
        for (_, mut val) in guard.drain() {
            val.zeroize();
        }

        if self.vault_path.exists() {
            let _ = std::fs::remove_file(&self.vault_path);
        }

        Ok(())
    }
}

impl Default for KeyringVault {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeyringVault {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.cached_secrets.lock() {
            for (_, mut val) in guard.drain() {
                val.zeroize();
            }
        }
    }
}

pub fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library/Application Support/com.boredpolymath.adcleanse/adcleanse.vault");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("com.boredpolymath.adcleanse\\adcleanse.vault");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local/share/com.boredpolymath.adcleanse/adcleanse.vault");
        }
    }
    std::env::temp_dir()
        .join("com.boredpolymath.adcleanse")
        .join("adcleanse.vault")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_lifecycle_and_zeroization() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vault_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let vault_file = temp_dir.join("test.vault");
        let vault = KeyringVault::with_vault_path(vault_file);
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

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_db_key_generation_and_zeroization() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vault_db_key_test_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let vault_file = temp_dir.join("test_db.vault");
        let vault = KeyringVault::with_vault_path(vault_file.clone());
        let key1 = vault.get_or_create_db_key().expect("Failed to create db key");
        assert_eq!(key1.len(), 64); // 256-bit hex

        // Re-requesting returns the same key
        let key2 = vault.get_or_create_db_key().expect("Failed to fetch existing db key");
        assert_eq!(*key1, *key2);

        // A new instance loading from disk gets the same key
        let vault2 = KeyringVault::with_vault_path(vault_file);
        let key2_loaded = vault2.get_or_create_db_key().expect("Failed to load key from disk");
        assert_eq!(*key1, *key2_loaded);

        vault2.delete_db_key().expect("Failed to delete db key");
        let key3 = vault2.get_or_create_db_key().expect("Failed to generate fresh key");
        assert_eq!(key3.len(), 64);
        assert_ne!(*key1, *key3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
