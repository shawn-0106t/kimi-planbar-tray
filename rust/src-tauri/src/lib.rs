// Kimi Planbar Tray — Tauri 2 (Rust) port of the WPF reference implementation.
// Startup order mirrors App.OnStartup (UI-SPEC section 11); self-check command
// line args are handled in main.rs BEFORE the single-instance check.

pub mod credentials;
pub mod panel;
pub mod polling;
pub mod quota;
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

/// Opt every window out of Windows 11 DWM auto corner rounding. The CSS draws
/// its own 14px/12px radius on a transparent window; DWM would otherwise clip
/// the window corners at ~8px and fight the painted radius.
fn disable_dwm_corner_rounding(app: &tauri::App) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    for w in app.webview_windows().values() {
        if let Ok(hwnd) = w.hwnd() {
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

/// Panel "⟳ 刷新" / menu "刷新": SafeRefresh + CheckAsync (SPEC 3.7 / 5).
#[tauri::command]
fn refresh_now(app: AppHandle) {
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
    // frontend to backfill the current settings every time it opens (SPEC 4.2)
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
}

/// Skills window: same reuse pattern as settings (SPEC 12). The scan itself
/// is lazy — it only runs when the frontend calls get_skills.
#[tauri::command]
fn open_skills(app: AppHandle) {
    app.state::<AppState>()
        .skills_open
        .store(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("skills") {
        // Same DPI-inflation guard as show_panel: pin the size on every open.
        let _ = w.set_size(tauri::LogicalSize::new(404.0, 520.0));
        let _ = w.show();
        let _ = w.set_focus();
    }
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
}

/// Cached skill list; `refresh` forces a rescan. Read-only (SPEC 12.2).
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

/// Save -> persist -> ApplyAutoStart -> apply theme -> Reschedule -> close (SPEC 4.2).
#[tauri::command]
fn save_settings(app: AppHandle, settings: SaveSettingsArgs) {
    {
        let st = app.state::<AppState>();
        let mut cur = st.settings.write().unwrap();
        cur.theme = settings.theme;
        cur.refresh_minutes = settings.refresh_minutes;
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
fn finish_hide_panel(app: AppHandle) {
    panel::finish_hide(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Tray menu items (SPEC 5): 打开/刷新/设置/退出; menu window closes first.
#[tauri::command]
fn menu_action(app: AppHandle, action: String) {
    if let Some(w) = app.get_webview_window("menu") {
        let _ = w.hide();
    }
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

fn build_state() -> AppState {
    let data = settings::load();
    let theme = theme_watch::effective(&data.theme);
    AppState::new(data, theme)
}

fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        // Focus-loss: panel auto-hides (suppressed while settings is open),
        // menu closes immediately (SPEC 1.1 / 1.3)
        tauri::WindowEvent::Focused(false) => match window.label() {
            "main" => panel::on_main_blur(&window.app_handle()),
            "menu" => {
                let _ = window.hide();
            }
            _ => {}
        },
        // Windows are single-instance reused: never destroy, only hide
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let st = window.app_handle().state::<AppState>();
            match window.label() {
                "settings" => st.settings_open.store(false, Ordering::SeqCst),
                "skills" => st.skills_open.store(false, Ordering::SeqCst),
                _ => {}
            }
            let _ = window.hide();
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
        finish_hide_panel,
        quit_app,
        menu_action,
        menu_height,
        start_drag,
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single instance first: a second launch exits immediately (SPEC 11)
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_opener::init())
        .manage(build_state())
        .invoke_handler(invoke_handlers())
        .setup(|app| {
            disable_dwm_corner_rounding(app);
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// --test-ui: apply the current theme and construct all four windows to
/// validate resource resolution, printing one OK line per window (SPEC 10).
/// Runs WITHOUT the single-instance plugin so it works alongside a live GUI.
pub fn run_ui_test() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(build_state())
        .invoke_handler(invoke_handlers())
        .setup(|app| {
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
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(6000));
                std::process::exit(0);
            });
            Ok(())
        })
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|_, _| {}),
        Err(e) => println!("UI-FAIL: {e}"),
    }
}
