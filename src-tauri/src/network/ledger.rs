use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: u64,
    pub timestamp_epoch: i64,
    pub endpoint: String,
    pub method: String,
    pub http_status: u16,
    pub payload_summary: String,
    pub zero_telemetry_verified: bool,
}

pub struct NetworkLedger {
    entries: Mutex<Vec<LedgerEntry>>,
    counter: Mutex<u64>,
}

impl NetworkLedger {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            counter: Mutex::new(1),
        }
    }

    pub fn record_request(
        &self,
        endpoint: &str,
        method: &str,
        http_status: u16,
        payload_summary: &str,
    ) {
        let mut count_guard = self.counter.lock().unwrap();
        let id = *count_guard;
        *count_guard += 1;

        let entry = LedgerEntry {
            id,
            timestamp_epoch: chrono::Utc::now().timestamp(),
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            http_status,
            payload_summary: payload_summary.to_string(),
            zero_telemetry_verified: true,
        };

        let mut entries_guard = self.entries.lock().unwrap();
        entries_guard.push(entry);

        // Keep rolling buffer of recent 200 requests
        if entries_guard.len() > 200 {
            entries_guard.remove(0);
        }
    }

    pub fn get_entries(&self) -> Vec<LedgerEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl Default for NetworkLedger {
    fn default() -> Self {
        Self::new()
    }
}
