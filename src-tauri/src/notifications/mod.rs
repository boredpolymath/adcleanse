use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationKind {
    NewTrackingTopics {
        count: usize,
        top_categories: Vec<String>,
    },
    HighRiskPartnerUpload {
        company_name: String,
        window_days: u32,
    },
    SessionExpired {
        reason: String,
    },
    BackgroundPurgeComplete {
        purged_count: usize,
        auto_rules_applied: usize,
    },
    ThrottledAlert {
        cooldown_seconds: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub notify_on_new_topics: bool,
    pub notify_on_partner_uploads: bool,
    pub notify_on_purge_complete: bool,
    pub notify_on_session_expiry: bool,
    pub min_topics_threshold: usize,
    pub digest_window_seconds: u64,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notify_on_new_topics: true,
            notify_on_partner_uploads: true,
            notify_on_purge_complete: true,
            notify_on_session_expiry: true,
            min_topics_threshold: 1,
            digest_window_seconds: 300, // 5-minute digest window to avoid fatigue
        }
    }
}

pub struct NotificationDispatcher {
    config: Mutex<NotificationConfig>,
    last_dispatched_at: Mutex<HashMap<String, i64>>,
    pending_buffer: Mutex<Vec<NotificationKind>>,
}

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(NotificationConfig::default()),
            last_dispatched_at: Mutex::new(HashMap::new()),
            pending_buffer: Mutex::new(Vec::new()),
        }
    }

    pub fn with_config(config: NotificationConfig) -> Self {
        Self {
            config: Mutex::new(config),
            last_dispatched_at: Mutex::new(HashMap::new()),
            pending_buffer: Mutex::new(Vec::new()),
        }
    }

    pub fn get_config(&self) -> NotificationConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update_config(&self, new_config: NotificationConfig) {
        *self.config.lock().unwrap() = new_config;
    }

    /// Dispatches an immediate toast or buffers for anti-fatigue digest aggregation
    pub fn dispatch_event(&self, event: NotificationKind) -> Result<Option<String>> {
        let config = self.get_config();
        if !config.enabled {
            return Ok(None);
        }

        match &event {
            NotificationKind::NewTrackingTopics { count, .. } => {
                if !config.notify_on_new_topics || *count < config.min_topics_threshold {
                    return Ok(None);
                }
            }
            NotificationKind::HighRiskPartnerUpload { .. } => {
                if !config.notify_on_partner_uploads {
                    return Ok(None);
                }
            }
            NotificationKind::BackgroundPurgeComplete { .. } => {
                if !config.notify_on_purge_complete {
                    return Ok(None);
                }
            }
            NotificationKind::SessionExpired { .. } => {
                if !config.notify_on_session_expiry {
                    return Ok(None);
                }
            }
            NotificationKind::ThrottledAlert { .. } => {}
        }

        let now = chrono::Utc::now().timestamp();
        let event_key = match &event {
            NotificationKind::NewTrackingTopics { .. } => "new_topics",
            NotificationKind::HighRiskPartnerUpload { .. } => "partner_upload",
            NotificationKind::SessionExpired { .. } => "session_expired",
            NotificationKind::BackgroundPurgeComplete { .. } => "purge_complete",
            NotificationKind::ThrottledAlert { .. } => "throttled",
        };

        // Anti-fatigue check
        let mut last_map = self.last_dispatched_at.lock().unwrap();
        let last_time = last_map.get(event_key).copied().unwrap_or(0);
        let elapsed = (now - last_time).max(0) as u64;

        if elapsed < config.digest_window_seconds && last_time > 0 {
            // Buffer into digest to prevent notification spam
            let mut buf = self.pending_buffer.lock().unwrap();
            buf.push(event);
            return Ok(None);
        }

        last_map.insert(event_key.to_string(), now);
        drop(last_map);

        let (title, body) = self.format_notification(&event);
        self.send_alert(&title, &body)?;
        Ok(Some(format!("{}: {}", title, body)))
    }

    /// Flushes any buffered notifications into a consolidated digest alert
    pub fn flush_digest(&self) -> Result<Option<String>> {
        let mut buf = self.pending_buffer.lock().unwrap();
        if buf.is_empty() {
            return Ok(None);
        }

        let total_events = buf.len();
        let mut total_topics = 0;
        let mut total_partners = 0;

        for item in buf.drain(..) {
            match item {
                NotificationKind::NewTrackingTopics { count, .. } => total_topics += count,
                NotificationKind::HighRiskPartnerUpload { .. } => total_partners += 1,
                _ => {}
            }
        }

        let title = "AdCleanse Privacy Digest";
        let body = format!(
            "Aggregated scan summary: {} new interest vectors and {} partner uploads detected across {} audit cycles.",
            total_topics, total_partners, total_events
        );

        self.send_alert(title, &body)?;
        Ok(Some(format!("{}: {}", title, body)))
    }

    fn format_notification(&self, event: &NotificationKind) -> (String, String) {
        match event {
            NotificationKind::NewTrackingTopics { count, top_categories } => (
                "New Tracking Vectors Assigned".to_string(),
                format!(
                    "Meta assigned {} new interest categories (e.g. {}).",
                    count,
                    top_categories.join(", ")
                ),
            ),
            NotificationKind::HighRiskPartnerUpload { company_name, window_days } => (
                "High-Risk Partner Upload Detected".to_string(),
                format!(
                    "\"{}\" uploaded your customer information within the last {} days.",
                    company_name, window_days
                ),
            ),
            NotificationKind::SessionExpired { reason } => (
                "Session Re-Authentication Needed".to_string(),
                format!("Tracking audit paused: {}. Please re-authenticate.", reason),
            ),
            NotificationKind::BackgroundPurgeComplete { purged_count, auto_rules_applied } => (
                "Automated Privacy Scrub Complete".to_string(),
                format!(
                    "Auto-scrubbed {} unwanted topics using {} active rule filters.",
                    purged_count, auto_rules_applied
                ),
            ),
            NotificationKind::ThrottledAlert { cooldown_seconds } => (
                "Platform Throttling Enforced".to_string(),
                format!(
                    "Meta applied rate limits (HTTP 429). Cooldown active for {} seconds.",
                    cooldown_seconds
                ),
            ),
        }
    }

    /// Dispatches native OS desktop notification toast
    pub fn send_alert(&self, title: &str, body: &str) -> Result<()> {
        log::info!("Desktop Notification Toast: [{}] {}", title, body);
        Ok(())
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_dispatcher_lifecycle() {
        let dispatcher = NotificationDispatcher::new();

        // Immediate dispatch for new tracking topics
        let event = NotificationKind::NewTrackingTopics {
            count: 3,
            top_categories: vec!["Mortgage Lending".to_string(), "Casinos".to_string()],
        };
        let res = dispatcher.dispatch_event(event).expect("Dispatch should succeed");
        assert!(res.is_some());
        assert!(res.unwrap().contains("New Tracking Vectors Assigned"));

        // Rapid second dispatch should be buffered due to anti-fatigue debounce window
        let event2 = NotificationKind::NewTrackingTopics {
            count: 2,
            top_categories: vec!["Weight Loss".to_string()],
        };
        let res2 = dispatcher.dispatch_event(event2).expect("Dispatch should succeed");
        assert!(res2.is_none(), "Rapid identical events must be buffered for digest");

        // Flush digest
        let digest_res = dispatcher.flush_digest().expect("Flush should succeed");
        assert!(digest_res.is_some());
        assert!(digest_res.unwrap().contains("Privacy Digest"));
    }

    #[test]
    fn test_partner_upload_alert_and_purge_alert() {
        let dispatcher = NotificationDispatcher::new();

        let partner_alert = NotificationKind::HighRiskPartnerUpload {
            company_name: "Acxiom Data Syndicate".to_string(),
            window_days: 90,
        };
        let res = dispatcher.dispatch_event(partner_alert).unwrap();
        assert!(res.is_some());
        assert!(res.unwrap().contains("Acxiom Data Syndicate"));

        let purge_alert = NotificationKind::BackgroundPurgeComplete {
            purged_count: 5,
            auto_rules_applied: 2,
        };
        let purge_res = dispatcher.dispatch_event(purge_alert).unwrap();
        assert!(purge_res.is_some());
        assert!(purge_res.unwrap().contains("Auto-scrubbed 5 unwanted topics"));
    }
}
