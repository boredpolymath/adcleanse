use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Dynamic tray icon status indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrayIconStatus {
    /// Green: Protected, zero-telemetry active, no unreviewed drift
    Protected,
    /// Amber: Drift detected, new tracking categories detected by Meta
    DriftDetected,
    /// Red: Throttled by HTTP 429 rate limit or session expired
    Throttled,
    /// Red: Authentication session expired, user action required
    Expired,
}

impl TrayIconStatus {
    pub fn badge_label(&self) -> &'static str {
        match self {
            TrayIconStatus::Protected => "Protected (Zero-Telemetry)",
            TrayIconStatus::DriftDetected => "Drift Detected",
            TrayIconStatus::Throttled => "Throttled (Cooldown Active)",
            TrayIconStatus::Expired => "Session Expired",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            TrayIconStatus::Protected => "#10b981",    // Emerald Green
            TrayIconStatus::DriftDetected => "#f59e0b", // Amber
            TrayIconStatus::Throttled => "#ef4444",    // Crimson Red
            TrayIconStatus::Expired => "#dc2626",      // Dark Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayMenuItem {
    pub id: String,
    pub title: String,
    pub enabled: bool,
}

/// System tray daemon state maintaining minimal RAM footprint (< 35 MB RSS)
pub struct TrayDaemonState {
    status: Arc<Mutex<TrayIconStatus>>,
    active_topics_count: Arc<AtomicUsize>,
    high_risk_count: Arc<AtomicUsize>,
    last_audit_epoch: Arc<AtomicI64>,
    cooldown_seconds: Arc<AtomicI64>,
}

impl TrayDaemonState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(TrayIconStatus::Protected)),
            active_topics_count: Arc::new(AtomicUsize::new(0)),
            high_risk_count: Arc::new(AtomicUsize::new(0)),
            last_audit_epoch: Arc::new(AtomicI64::new(0)),
            cooldown_seconds: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn get_status(&self) -> TrayIconStatus {
        *self.status.lock().unwrap()
    }

    pub fn set_status(&self, new_status: TrayIconStatus) {
        let mut guard = self.status.lock().unwrap();
        *guard = new_status;
        log::info!("System tray status shifted to: {:?}", new_status);
    }

    pub fn update_metrics(
        &self,
        topics_count: usize,
        high_risk: usize,
        last_audit: i64,
        drift_delta: f64,
    ) {
        self.active_topics_count.store(topics_count, Ordering::SeqCst);
        self.high_risk_count.store(high_risk, Ordering::SeqCst);
        self.last_audit_epoch.store(last_audit, Ordering::SeqCst);

        let current_status = self.get_status();
        if current_status != TrayIconStatus::Throttled && current_status != TrayIconStatus::Expired {
            if drift_delta > 0.0 || high_risk > 0 {
                self.set_status(TrayIconStatus::DriftDetected);
            } else {
                self.set_status(TrayIconStatus::Protected);
            }
        }
    }

    pub fn set_throttled(&self, cooldown_sec: i64) {
        self.cooldown_seconds.store(cooldown_sec, Ordering::SeqCst);
        if cooldown_sec > 0 {
            self.set_status(TrayIconStatus::Throttled);
        } else {
            self.set_status(TrayIconStatus::Protected);
        }
    }

    pub fn get_topics_count(&self) -> usize {
        self.active_topics_count.load(Ordering::SeqCst)
    }

    pub fn get_high_risk_count(&self) -> usize {
        self.high_risk_count.load(Ordering::SeqCst)
    }

    pub fn get_cooldown_seconds(&self) -> i64 {
        self.cooldown_seconds.load(Ordering::SeqCst)
    }

    /// Generates structured context menu items for dynamic tray rendering
    pub fn build_context_menu_items(&self) -> Vec<TrayMenuItem> {
        let status = self.get_status();
        let topics = self.get_topics_count();
        let high_risk = self.get_high_risk_count();
        let cooldown = self.get_cooldown_seconds();

        let status_text = if cooldown > 0 {
            format!("Status: {} ({}s)", status.badge_label(), cooldown)
        } else {
            format!("Status: {}", status.badge_label())
        };

        vec![
            TrayMenuItem {
                id: "status".to_string(),
                title: status_text,
                enabled: false,
            },
            TrayMenuItem {
                id: "topics_count".to_string(),
                title: format!("Active Vectors: {} ({} High-Risk)", topics, high_risk),
                enabled: false,
            },
            TrayMenuItem {
                id: "quick_purge".to_string(),
                title: "⚡ Quick Scrub High-Risk Topics".to_string(),
                enabled: high_risk > 0,
            },
            TrayMenuItem {
                id: "open_console".to_string(),
                title: "Open AdCleanse Console".to_string(),
                enabled: true,
            },
            TrayMenuItem {
                id: "quit".to_string(),
                title: "Quit AdCleanse".to_string(),
                enabled: true,
            },
        ]
    }

    pub fn update_tray_menu(&self) -> Result<()> {
        log::info!(
            "Refreshing system tray context menu. Status: {:?}, Active: {}",
            self.get_status(),
            self.get_topics_count()
        );
        Ok(())
    }
}

impl Default for TrayDaemonState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_status_transitions() {
        let tray = TrayDaemonState::new();
        assert_eq!(tray.get_status(), TrayIconStatus::Protected);
        assert_eq!(tray.get_status().badge_label(), "Protected (Zero-Telemetry)");

        // Drift detected transition
        tray.update_metrics(45, 3, 1760000000, 15.0);
        assert_eq!(tray.get_status(), TrayIconStatus::DriftDetected);
        assert_eq!(tray.get_topics_count(), 45);
        assert_eq!(tray.get_high_risk_count(), 3);

        // Throttling transition
        tray.set_throttled(120);
        assert_eq!(tray.get_status(), TrayIconStatus::Throttled);
        assert_eq!(tray.get_cooldown_seconds(), 120);

        // Clear cooldown
        tray.set_throttled(0);
        assert_eq!(tray.get_status(), TrayIconStatus::Protected);

        // Context menu generation
        let items = tray.build_context_menu_items();
        assert!(items.iter().any(|i| i.id == "quick_purge"));
        assert!(items.iter().any(|i| i.id == "open_console"));
    }
}
