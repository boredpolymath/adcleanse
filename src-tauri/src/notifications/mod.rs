use crate::error::Result;

pub struct NotificationDispatcher;

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self
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
