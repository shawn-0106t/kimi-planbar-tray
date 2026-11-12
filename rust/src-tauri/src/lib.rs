// Kimi Planbar Tray — Tauri 2 (Rust) port of the WPF reference implementation.
// Startup order mirrors App.OnStartup (SPEC section 20); self-check command
// line args are handled in main.rs BEFORE the single-instance check.

pub mod credentials;
pub mod panel;
pub mod polling;
pub mod quota;
pub mod recovery;
pub mod settings;
pub mod skills;
pub mod state;
pub mod theme_watch;
pub mod tray;
pub mod update;

use quota::QuotaResult;
use serde::{Deserialize, Serialize};
use settings::SettingsData;
use state::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use update::UpdateStatus;

// on_window_event delivers tauri::Window; commands receive WebviewWindow.

const RELEASES_URL: &str = "https://github.com/MoonshotAI/kimi-code/releases";
const CONSOLE_URL: &str = "https://www.kimi.com/code/console?from=kfc_overview_topbar";

/// Opt every window out of Windows 11 DWM auto corner rounding. The CSS draws
/// its own 14px/12px radius on a transparent window; DWM would otherwise clip
/// the window corners at ~8px and fight the painted radius.
pub(crate) fn apply_no_round(window: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    if let Ok(hwnd) = window.hwnd() {
        let pref = DWMWCP_DONOTROUND;
        unsafe {
            let _ = DwmSetWindowAttribute(
                HWND(hwnd.0 as _),
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&pref) as u32,
            );
        }
    }
}

fn disable_dwm_corner_rounding(app: &tauri::App) {
    for window in app.webview_windows().values() {
        apply_no_round(window);
    }
}

/// Native quit-only tray menu installed after auto-recovery is disabled: the
/// webview menu may be a dead shell, and the app must keep a way to exit
/// (SPEC 22.4). `show_menu` stands down once this is active.
pub(crate) fn install_fallback_tray_menu(app: &AppHandle) {
    use tauri::menu::{Menu, MenuItem};
    let Ok(quit) = MenuItem::with_id(
        app,
        "kpt-fallback-quit",
        "Quit Kimi Planbar Tray",
        true,
        None::<&str>,
    ) else {
        return;
    };
    let Ok(menu) = Menu::with_items(app, &[&quit]) else {
        return;
    };
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_menu(Some(menu));
    }
    app.state::<AppState>()
        .recovery
        .fallback_menu
        .store(true, Ordering::SeqCst);
}

// ---- IPC DTOs ----

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub theme: String,
    pub refresh_minutes: i64,
    pub auto_start: bool,
}

impl From<&SettingsData> for SettingsDto {
    fn from(d: &SettingsData) -> Self {
        SettingsDto {
            theme: d.theme.clone(),
            refresh_minutes: d.refresh_minutes,
            auto_start: d.auto_start,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub quota: Option<QuotaResult>,
    pub update: UpdateStatus,
    pub settings: SettingsDto,
    pub theme: String, // effective theme: light | dark
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsArgs {
    pub theme: String,
    pub refresh_minutes: i64,
    pub auto_start: bool,
}

// ---- Commands ----

#[tauri::command]
fn get_state(app: AppHandle) -> AppStateDto {
    let st = app.state::<AppState>();
    let quota = st.last_quota.read().unwrap().clone();
    let update = st.update.read().unwrap().clone();
    let settings = SettingsDto::from(&*st.settings.read().unwrap());
    let theme = st.effective_theme.read().unwrap().clone();
    AppStateDto {
        quota,
        update,
        settings,
        theme,
    }
}

/// Panel "Refresh" button / tray-menu "Refresh": SafeRefresh + CheckAsync (SPEC 12.7 / 14).
/// Debounced: invocations within 2s of the previous one are ignored.
#[tauri::command]
fn refresh_now(app: AppHandle) {
    {
        let st = app.state::<AppState>();
        let mut last = st.last_manual_refresh.lock().unwrap();
        let now = std::time::Instant::now();
        if last.is_some_and(|t| now.duration_since(t) < std::time::Duration::from_secs(2)) {
            return;
        }
        *last = Some(now);
    }
    let a1 = app.clone();
    tauri::async_runtime::spawn(async move {
        polling::safe_refresh(&a1).await;
    });
    tauri::async_runtime::spawn(async move {
        check_and_emit(&app).await;
    });
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings_window(&app);
}

/// Shared by the open_settings command and the recovery rebuild restore path.
pub(crate) fn show_settings_window(app: &AppHandle) {
    if !recovery::gate_show(app, "settings") {
        return;
    }
    app.state::<AppState>()
        .settings_open
        .store(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("settings") {
        // Same DPI-inflation guard as show_panel: pin the size on every open.
        let _ = w.set_size(tauri::LogicalSize::new(404.0, 464.0));
        let _ = w.show();
        let _ = w.set_focus();
    }
    // The settings window is reused (hidden, never destroyed): tell the
    // frontend to backfill the current settings every time it opens (SPEC 13.2)
    recovery::note_ui_event(app, "settings", "show");
    recovery::set_last_visible(app, "settings", true);
    let _ = app.emit("settings-show", ());
}

#[tauri::command]
fn close_settings(app: AppHandle) {
    app.state::<AppState>()
        .settings_open
        .store(false, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.hide();
    }
    recovery::note_ui_event(&app, "settings", "hide");
    recovery::set_last_visible(&app, "settings", false);
}

/// Skills window: same reuse pattern as settings (SPEC 21). The scan itself
/// is lazy — it only runs when the frontend calls get_skills.
#[tauri::command]
fn open_skills(app: AppHandle) {
    show_skills_window(&app);
}

/// Shared by the open_skills command and the recovery rebuild restore path.
pub(crate) fn show_skills_window(app: &AppHandle) {
    if !recovery::gate_show(app, "skills") {
        return;
    }
    app.state::<AppState>()
        .skills_open
        .store(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("skills") {
        // Same DPI-inflation guard as show_panel: pin the size on every open.
        let _ = w.set_size(tauri::LogicalSize::new(404.0, 520.0));
        let _ = w.show();
        let _ = w.set_focus();
    }
    recovery::note_ui_event(app, "skills", "show");
    recovery::set_last_visible(app, "skills", true);
    let _ = app.emit("skills-show", ());
}

#[tauri::command]
fn close_skills(app: AppHandle) {
    app.state::<AppState>()
        .skills_open
        .store(false, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("skills") {
        let _ = w.hide();
    }
    recovery::note_ui_event(&app, "skills", "hide");
    recovery::set_last_visible(&app, "skills", false);
}

/// Cached skill list; `refresh` forces a rescan. Read-only (SPEC 21.2).
/// Async so the file scan never blocks the main thread / webviews.
#[tauri::command]
async fn get_skills(app: AppHandle, refresh: bool) -> Result<Vec<skills::SkillInfo>, ()> {
    let st = app.state::<AppState>();
    if !refresh {
        if let Some(cached) = st.skills_cache.read().unwrap().clone() {
            return Ok(cached);
        }
    }
    let list = tauri::async_runtime::spawn_blocking(skills::scan)
        .await
        .unwrap_or_default();
    *st.skills_cache.write().unwrap() = Some(list.clone());
    Ok(list)
}

/// Save -> persist -> ApplyAutoStart -> apply theme -> Reschedule -> close (SPEC 13.2).
#[tauri::command]
fn save_settings(app: AppHandle, settings: SaveSettingsArgs) {
    {
        let st = app.state::<AppState>();
        let mut cur = st.settings.write().unwrap();
        cur.theme = settings.theme;
        // Clamp at the IPC boundary: polling multiplies this by 60 seconds
        cur.refresh_minutes = settings.refresh_minutes.clamp(1, 30);
        cur.auto_start = settings.auto_start;
        settings::save(&cur);
        settings::apply_auto_start(&cur);
    }
    theme_watch::apply_now(&app);
    app.state::<AppState>().reschedule.notify_one();
    close_settings(app);
}

#[tauri::command]
fn open_releases(app: AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(RELEASES_URL, None::<&str>);
}

#[tauri::command]
fn open_console(app: AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(CONSOLE_URL, None::<&str>);
}

#[tauri::command]
fn finish_hide_panel(app: AppHandle) {
    panel::finish_hide(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Tray menu items (SPEC 14): Open/Refresh/Settings/Skills/Exit; menu window closes first.
#[tauri::command]
fn menu_action(app: AppHandle, action: String) {
    if let Some(w) = app.get_webview_window("menu") {
        let _ = w.hide();
    }
    recovery::note_ui_event(&app, "menu", "hide");
    recovery::set_last_visible(&app, "menu", false);
    match action.as_str() {
        "open" => panel::toggle_panel(&app, false),
        "refresh" => refresh_now(app),
        "settings" => open_settings(app),
        "skills" => open_skills(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Frontend-reported content height of the menu (WPF SizeToContent equivalent).
#[tauri::command]
fn menu_height(app: AppHandle, height: f64) {
    if (20.0..600.0).contains(&height) {
        *app.state::<AppState>().menu_height.lock().unwrap() = height;
    }
}

#[tauri::command]
fn start_drag(window: WebviewWindow) {
    let _ = window.start_dragging();
}

// ---- Update check with event emission ----

pub async fn check_and_emit(app: &AppHandle) {
    let st = update::check().await;
    *app.state::<AppState>().update.write().unwrap() = st.clone();
    let _ = app.emit("update-status", &st);
}

// ---- App wiring ----

/// Diagnostic/test override: redirect all app directories (including the
/// WebView2 user-data folder and `webview.log`) under one root. Used to run a
/// second instance or a sandboxed test without touching the user profile
/// (SPEC 22.5). Empty/unset means the normal per-user locations.
pub(crate) fn app_data_dir_override() -> Option<std::path::PathBuf> {
    std::env::var("KPT_APP_DATA_DIR")
        .ok()
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
}

fn apply_app_data_dir_override(context: &mut tauri::Context<tauri::Wry>) {
    if let Some(dir) = app_data_dir_override() {
        context.config_mut().app.app_directories_override =
            Some(tauri::utils::config::AppDirectoriesOverride::Root(dir));
    }
}

fn build_state() -> AppState {
    let data = settings::load();
    let theme = theme_watch::effective(&data.theme);
    AppState::new(data, theme)
}

fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        // Focus-loss: panel auto-hides (suppressed while settings is open),
        // menu closes immediately (SPEC 10.1 / 10.3)
        tauri::WindowEvent::Focused(false) => match window.label() {
            "main" => panel::on_main_blur(window.app_handle()),
            "menu" => {
                let _ = window.hide();
                recovery::note_ui_event(window.app_handle(), "menu", "hide");
                recovery::set_last_visible(window.app_handle(), "menu", false);
            }
            _ => {}
        },
        // Windows are single-instance reused: never destroy, only hide.
        // destroy() (recovery rebuild) bypasses CloseRequested on purpose.
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let st = window.app_handle().state::<AppState>();
            match window.label() {
                "settings" => st.settings_open.store(false, Ordering::SeqCst),
                "skills" => st.skills_open.store(false, Ordering::SeqCst),
                _ => {}
            }
            let _ = window.hide();
            recovery::note_ui_event(window.app_handle(), window.label(), "hide");
            recovery::set_last_visible(window.app_handle(), window.label(), false);
        }
        tauri::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            recovery::note_ui_event(
                window.app_handle(),
                window.label(),
                &format!("scale={scale_factor:.2}"),
            );
        }
        _ => {}
    }
}

fn invoke_handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_state,
        refresh_now,
        open_settings,
        close_settings,
        open_skills,
        close_skills,
        get_skills,
        save_settings,
        open_releases,
        open_console,
        finish_hide_panel,
        quit_app,
        menu_action,
        menu_height,
        start_drag,
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    apply_app_data_dir_override(&mut context);
    tauri::Builder::default()
        // Single instance first: a second launch exits immediately (SPEC 20)
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_opener::init())
        .manage(build_state())
        .invoke_handler(invoke_handlers())
        .on_menu_event(|app, event| {
            let id: &str = event.id().as_ref();
            if id == "kpt-fallback-quit" {
                app.exit(0);
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            disable_dwm_corner_rounding(app);
            // WebView2 health: subscribe before any background task so a
            // failure during startup is still queued for the coordinator.
            recovery::install_all(&handle);
            recovery::start(&handle);
            tray::build(app.handle())?;
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move { polling::run(h).await });
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move { check_and_emit(&h).await });
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move { theme_watch::watch(h).await });
            Ok(())
        })
        .on_window_event(on_window_event)
        .build(context)
        .expect("error while building tauri application")
        // Tauri exits when the last window is destroyed. A BrowserProcessExited
        // recovery destroys all four windows at once, so that implicit exit
        // must be prevented until the rebuild completes; explicit quits
        // (app.exit(code), code = Some) still exit normally.
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

/// Remove stale `--test-ui` WebView2 data directories from earlier runs.
/// A clean exit also tries to delete its own directory, but WebView2 can keep
/// files locked for ~10 s after the process exits; leftovers are swept here
/// once they are no longer plausibly owned by a running test instance.
fn cleanup_stale_test_dirs() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name
            .to_string_lossy()
            .starts_with("kimi-planbar-tray-testui-")
        {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(created) = meta.created() else {
            continue;
        };
        let stale = now
            .duration_since(created)
            .map(|age| age > std::time::Duration::from_secs(600))
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn remove_test_dir(path: &std::path::Path) {
    // WebView2 child processes may still hold files when the app returns from
    // the event loop; retry briefly before leaving the directory to the stale
    // sweep on the next run.
    for _ in 0..25 {
        if std::fs::remove_dir_all(path).is_ok() || !path.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// --test-ui: apply the current theme and construct all four windows to
/// validate resource resolution, printing one OK line per window (SPEC 19).
/// Runs WITHOUT the single-instance plugin so it works alongside a live GUI,
/// and with its own app-data root so the WebView2 user-data folder never
/// collides with a running instance (SPEC 22.5).
pub fn run_ui_test() {
    cleanup_stale_test_dirs();
    let test_root =
        std::env::temp_dir().join(format!("kimi-planbar-tray-testui-{}", std::process::id()));
    let mut context = tauri::generate_context!();
    // Keep the diagnostic log inside the same isolated root (SPEC 22.5).
    std::env::set_var("KPT_APP_DATA_DIR", &test_root);
    context.config_mut().app.app_directories_override = Some(
        tauri::utils::config::AppDirectoriesOverride::Root(test_root.clone()),
    );
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(build_state())
        .invoke_handler(invoke_handlers())
        .setup(|app| {
            let handle = app.handle().clone();
            recovery::install_all(&handle);
            recovery::start(&handle);
            for (label, name) in [
                ("main", "MainWindow"),
                ("settings", "SettingsWindow"),
                ("skills", "SkillsWindow"),
                ("menu", "TrayMenuWindow"),
            ] {
                match app.get_webview_window(label) {
                    Some(w) => {
                        let _ = w.show();
                        if label == "main" {
                            let _ = app.emit("panel-show", ());
                        }
                        println!("{name} OK");
                    }
                    None => println!("UI-FAIL: {name}: window was not created"),
                }
            }
            // Give the webviews a moment to actually load, then exit headless.
            // Also emit panel-show (after the page's listeners attach) and suppress
            // the focus-loss auto-hide so the panel stays visible for inspection.
            app.state::<AppState>()
                .settings_open
                .store(true, Ordering::SeqCst);
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2500));
                let _ = handle.emit("panel-show", ());
                // Lets the skills window render its list in --test-ui too.
                let _ = handle.emit("skills-show", ());
            });
            let exit_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(6000));
                // Graceful exit so app.run returns and the test data dir can
                // be removed; std::process::exit would skip that cleanup.
                exit_handle.exit(0);
            });
            Ok(())
        })
        .build(context);
    match result {
        Ok(app) => app.run(|_app, event| {
            // Same last-window guard as the GUI path: a test-triggered rebuild
            // must not let Tauri exit before the windows are recreated.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        }),
        Err(e) => println!("UI-FAIL: {e}"),
    }
    remove_test_dir(&test_root);
}
