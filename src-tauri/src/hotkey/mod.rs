use crate::audit::models::{AuditSnapshot, RiskLevel};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const DEFAULT_SPOTLIGHT_HOTKEY: &str = "CmdOrCtrl+Shift+P";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightMetricsSummary {
    pub is_visible: bool,
    pub drift_index: f64,
    pub active_topics_count: usize,
    pub high_risk_count: usize,
    pub high_risk_preview: Vec<String>,
    pub status_badge: String,
    pub hotkey: String,
}

/// Global hotkey controller & floating Spotlight panel manager
pub struct SpotlightManager {
    is_visible: Arc<AtomicBool>,
    hotkey: Arc<Mutex<String>>,
}

impl SpotlightManager {
    pub fn new() -> Self {
        Self {
            is_visible: Arc::new(AtomicBool::new(false)),
            hotkey: Arc::new(Mutex::new(DEFAULT_SPOTLIGHT_HOTKEY.to_string())),
        }
    }

    pub fn set_hotkey(&self, hotkey: &str) {
        let mut guard = self.hotkey.lock().unwrap();
        *guard = hotkey.to_string();
        log::info!("Spotlight global shortcut reconfigured to: {}", hotkey);
    }

    pub fn get_hotkey(&self) -> String {
        self.hotkey.lock().unwrap().clone()
    }

    pub fn toggle_spotlight(&self) -> Result<bool> {
        let currently_visible = self.is_visible.load(Ordering::SeqCst);
        let new_state = !currently_visible;
        self.is_visible.store(new_state, Ordering::SeqCst);
        log::info!("Spotlight floating panel toggled: visible = {}", new_state);
        Ok(new_state)
    }

    pub fn set_visible(&self, visible: bool) {
        self.is_visible.store(visible, Ordering::SeqCst);
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.load(Ordering::SeqCst)
    }

    /// Renders instantaneous privacy metrics and high-risk vectors for floating HUD glance
    pub fn build_spotlight_summary(
        &self,
        snapshot: Option<&AuditSnapshot>,
    ) -> SpotlightMetricsSummary {
        let is_visible = self.is_visible();
        let hotkey = self.get_hotkey();

        match snapshot {
            Some(s) => {
                let high_risk_topics: Vec<String> = s
                    .topics
                    .iter()
                    .filter(|t| t.risk_level == RiskLevel::Critical || t.risk_level == RiskLevel::High)
                    .map(|t| t.name.clone())
                    .collect();

                let status_badge = if s.drift_index > 0.0 || !high_risk_topics.is_empty() {
                    "Drift Detected".to_string()
                } else {
                    "Protected (Zero-Telemetry)".to_string()
                };

                SpotlightMetricsSummary {
                    is_visible,
                    drift_index: s.drift_index,
                    active_topics_count: s.total_topics,
                    high_risk_count: high_risk_topics.len(),
                    high_risk_preview: high_risk_topics.into_iter().take(5).collect(),
                    status_badge,
                    hotkey,
                }
            }
            None => SpotlightMetricsSummary {
                is_visible,
                drift_index: 0.0,
                active_topics_count: 0,
                high_risk_count: 0,
                high_risk_preview: vec![],
                status_badge: "Protected (Zero-Telemetry)".to_string(),
                hotkey,
            },
        }
    }
}

impl Default for SpotlightManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::models::{AdTopic, TopicOrigin};

    #[test]
    fn test_spotlight_toggle_and_summary_generation() {
        let manager = SpotlightManager::new();
        assert!(!manager.is_visible());
        assert_eq!(manager.get_hotkey(), DEFAULT_SPOTLIGHT_HOTKEY);

        let toggled = manager.toggle_spotlight().unwrap();
        assert!(toggled);
        assert!(manager.is_visible());

        manager.set_hotkey("Ctrl+Shift+P");
        assert_eq!(manager.get_hotkey(), "Ctrl+Shift+P");

        let snapshot = AuditSnapshot {
            id: "snap_hud".to_string(),
            timestamp_epoch: 1760000000,
            total_topics: 10,
            total_partners: 2,
            drift_index: 22.4,
            topics: vec![AdTopic {
                id: "t_hud_1".to_string(),
                name: "High Interest Loans".to_string(),
                category: "Finance".to_string(),
                origin: TopicOrigin::InferredBehavior,
                risk_level: RiskLevel::Critical,
                date_added_epoch: 1760000000,
                is_active: true,
            }],
            partners: vec![],
        };

        let summary = manager.build_spotlight_summary(Some(&snapshot));
        assert!(summary.is_visible);
        assert_eq!(summary.high_risk_count, 1);
        assert_eq!(summary.high_risk_preview[0], "High Interest Loans");
        assert_eq!(summary.status_badge, "Drift Detected");
    }
}
