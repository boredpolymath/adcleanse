use crate::error::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct TrayDaemonState {
    active_topics_count: Arc<AtomicUsize>,
}

impl TrayDaemonState {
    pub fn new() -> Self {
        Self {
            active_topics_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_topics_count(&self, count: usize) {
        self.active_topics_count.store(count, Ordering::SeqCst);
    }

    pub fn get_topics_count(&self) -> usize {
        self.active_topics_count.load(Ordering::SeqCst)
    }

    pub fn update_tray_menu(&self) -> Result<()> {
        log::info!(
            "Updating system tray menu with active count: {}",
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
