use crate::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SpotlightManager {
    is_visible: Arc<AtomicBool>,
}

impl SpotlightManager {
    pub fn new() -> Self {
        Self {
            is_visible: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn toggle_spotlight(&self) -> Result<bool> {
        let currently_visible = self.is_visible.load(Ordering::SeqCst);
        let new_state = !currently_visible;
        self.is_visible.store(new_state, Ordering::SeqCst);
        log::info!("Spotlight floating panel toggled: visible = {}", new_state);
        Ok(new_state)
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.load(Ordering::SeqCst)
    }
}

impl Default for SpotlightManager {
    fn default() -> Self {
        Self::new()
    }
}
