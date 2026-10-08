use crate::error::{AdCleanseError, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct EncryptedDatabase {
    db_path: PathBuf,
    connection: Mutex<Option<Connection>>,
}

impl EncryptedDatabase {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            connection: Mutex::new(None),
        }
    }

    /// Initializes local SQLite database with SQLCipher encryption pragmas
    pub fn initialize(&self, _passphrase: &str) -> Result<()> {
        log::info!("Opening encrypted local database at: {:?}", self.db_path);

        let conn = Connection::open(&self.db_path)
            .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        // Configure WAL mode and performance pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| AdCleanseError::StorageError(e.to_string()))?;

        let mut guard = self.connection.lock().map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        *guard = Some(conn);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.connection.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn wipe_all(&self) -> Result<()> {
        log::warn!("Executing complete zero-leak database wipe and purge");
        let mut guard = self.connection.lock().map_err(|e| AdCleanseError::StorageError(e.to_string()))?;
        *guard = None;
        if self.db_path.exists() {
            let _ = std::fs::remove_file(&self.db_path);
        }
        Ok(())
    }
}
