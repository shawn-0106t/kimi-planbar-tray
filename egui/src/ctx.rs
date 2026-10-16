// Process-internal application context, replacing the Tauri AppHandle: the
// shared AppState plus an event bus. The Tauri emits (quota-updated /
// update-status / theme-changed in lib.rs, polling.rs, theme_watch.rs) become
// AppEvent values; the Phase 2 GUI consumes the receiving end to drive the
// UI and the tray tooltip.

use crate::quota::QuotaResult;
use crate::state::AppState;
use crate::update::UpdateStatus;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub enum AppEvent {
    QuotaUpdated(QuotaResult),
    UpdateStatus(UpdateStatus),
    ThemeChanged(String),
}

/// Cheap-to-clone handle passed around like the Tauri edition's AppHandle.
#[derive(Clone)]
pub struct AppCtx {
    pub state: Arc<AppState>,
    pub events: UnboundedSender<AppEvent>,
}

impl AppCtx {
    pub fn new(state: AppState) -> (Self, UnboundedReceiver<AppEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            AppCtx {
                state: Arc::new(state),
                events: tx,
            },
            rx,
        )
    }
}
