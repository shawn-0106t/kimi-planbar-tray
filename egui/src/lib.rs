// Kimi Planbar Tray — egui edition, headless business core (Phase 1).
// The modules below are 1:1 ports of rust/src-tauri/src with the Tauri
// facilities removed: AppHandle/emit -> AppCtx + AppEvent (ctx.rs), and
// tauri::async_runtime -> a plain tokio runtime owned by the caller. The GUI
// (eframe) lands in Phase 2; see docs/EGUI-MIGRATION.md.

pub mod credentials;
pub mod ctx;
pub mod polling;
pub mod quota;
pub mod settings;
pub mod skills;
pub mod state;
pub mod theme_watch;
pub mod update;

use ctx::{AppCtx, AppEvent};
use state::AppState;
use tokio::sync::mpsc::UnboundedReceiver;

/// Build the shared state from persisted settings (port of lib.rs
/// build_state). Returns the context and the receiving end of the event bus.
pub fn build_state() -> (AppCtx, UnboundedReceiver<AppEvent>) {
    let data = settings::load();
    let theme = theme_watch::effective(&data.theme);
    AppCtx::new(AppState::new(data, theme))
}

/// Update check + store + emit, port of lib.rs check_and_emit (SPEC 17).
pub async fn check_and_emit(ctx: &AppCtx) {
    let st = update::check().await;
    *ctx.state.update.write().unwrap() = st.clone();
    let _ = ctx.events.send(AppEvent::UpdateStatus(st));
}
