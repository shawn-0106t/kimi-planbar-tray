// Kimi Planbar Tray — egui edition, headless business core (Phase 1) + the
// eframe GUI (Phase 2). The business modules below are 1:1 ports of
// rust/src-tauri/src with the Tauri facilities removed: AppHandle/emit ->
// AppCtx + AppEvent (ctx.rs), and tauri::async_runtime -> a plain tokio
// runtime owned by the caller.

pub mod app;
pub mod credentials;
pub mod ctx;
pub mod format_util;
pub mod icons;
pub mod panel_pos;
pub mod polling;
pub mod quota;
pub mod settings;
pub mod skills;
pub mod state;
pub mod theme_watch;
pub mod tray;
pub mod ui;
pub mod update;
pub mod windows;

use ctx::{AppCtx, AppEvent};
use state::AppState;
use tokio::sync::mpsc::UnboundedReceiver;

/// URLs opened from the panel (SPEC 12.6 / 12.7), shared with app.rs.
pub const RELEASES_URL: &str = "https://github.com/MoonshotAI/kimi-code/releases";
pub const CONSOLE_URL: &str = "https://www.kimi.com/code/console?from=kfc_overview_topbar";

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

// GUI-subsystem exes (release builds) have no console: print via the inherited
// STD_OUTPUT_HANDLE — a valid pipe under Git Bash, the parent's console buffer
// under cmd/PowerShell. This is the qt/ edition's WriteFile pattern
// (qt/src/main.cpp printStdout); AttachConsole + CONOUT$ is the fallback and a
// plain println! the last resort (debug builds are console-subsystem, where
// println! always works).
pub fn print_stdout(text: &str) {
    // `::windows` (not `windows`): the crate-root module named `windows`
    // (windows.rs) would otherwise shadow the extern crate here.
    use ::windows::core::w;
    use ::windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE, HANDLE};
    use ::windows::Win32::Storage::FileSystem::{
        CreateFileW, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use ::windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };

    let write = |h: HANDLE| -> bool {
        let bytes = text.as_bytes();
        let mut written: u32 = 0;
        unsafe { WriteFile(h, Some(bytes), Some(&mut written), None).is_ok() }
    };
    unsafe {
        if let Ok(h) = GetStdHandle(STD_OUTPUT_HANDLE) {
            if !h.is_invalid() && !h.0.is_null() && write(h) {
                return;
            }
        }
        if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() {
            if let Ok(h) = CreateFileW(
                w!("CONOUT$"),
                GENERIC_WRITE.0,
                FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                let owned = write(h);
                let _ = CloseHandle(h); // only a handle we opened gets closed
                if owned {
                    return;
                }
            }
        }
    }
    println!("{text}");
}
