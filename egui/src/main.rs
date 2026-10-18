// Entry point of the egui edition. Self-check args (--test-fetch /
// --test-update / --test-ui) run BEFORE the single-instance check, mirroring
// rust/src-tauri/src/main.rs (SPEC 19). Startup order follows SPEC 20:
// settings -> services -> (self-check branch) -> single instance -> theme ->
// tray -> polling (2s first refresh) -> background version check.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use kimi_planbar_tray_egui::app::{App, UiCmd, UiShared, WinBus};
use kimi_planbar_tray_egui::ctx::{AppCtx, AppEvent};
use kimi_planbar_tray_egui::{app, panel_pos, quota, theme_watch, tray, update, windows};
use tokio::sync::mpsc::UnboundedReceiver;

// Named mutex identical to the Rust/Qt/WPF editions (SPEC 20): guarantees
// mutual exclusion across all four editions. Do not rename.
fn acquire_single_instance_mutex() -> bool {
    // `::windows` (not `windows`): the lib's module `windows` is imported in
    // this scope and would shadow the extern crate.
    use ::windows::core::w;
    use ::windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use ::windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        match CreateMutexW(None, true, w!("KimiPlanbarTray.SingleInstance")) {
            Ok(h) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    return false;
                }
                // Leak the handle on purpose: it must stay open until process
                // exit, or the mutex would be released early.
                let _keep_alive: &'static _ = Box::leak(Box::new(h));
                true
            }
            Err(_) => true, // mutex creation failure must not block startup
        }
    }
}

fn dotnet_bool(b: bool) -> &'static str {
    // C# string interpolation prints True/False
    if b {
        "True"
    } else {
        "False"
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    rt.block_on(fut)
}

// ---- GUI wiring (SPEC 20 startup order) -----------------------------------

/// Shared UI handle for the console ctrl handler (runs on its own thread).
static UI_SHARED: OnceLock<Arc<UiShared>> = OnceLock::new();

/// Clean exit on console control events (Ctrl+C, console close, logoff) by
/// routing through the normal Quit path, so the tray icon is removed. A hard
/// TerminateProcess (e.g. `kill -9` semantics) cannot be intercepted on
/// Windows; every graceful path lands here.
unsafe extern "system" fn console_ctrl_handler(_ctrl: u32) -> ::windows::core::BOOL {
    match UI_SHARED.get() {
        Some(ui) => {
            ui.push(UiCmd::Quit);
            ::windows::core::BOOL::from(true)
        }
        None => ::windows::core::BOOL::from(false),
    }
}

/// Owns the tokio runtime on a background thread and keeps it parked for the
/// process lifetime. Runs the refresh loop (2s first refresh), the system
/// theme watcher, and the startup version check (SPEC 20); the runtime Handle
/// is published through `handle_slot` for UI-triggered async work.
fn spawn_runtime(ctx: AppCtx, handle_slot: Arc<OnceLock<tokio::runtime::Handle>>) {
    // Thread-spawn failure means no polling / theme follow / version check
    // while the UI keeps running — never silently invisible. The message
    // carries only the OS error (no credentials or paths).
    match std::thread::Builder::new()
        .name("tokio-runtime".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            let _ = handle_slot.set(rt.handle().clone());
            rt.block_on(async move {
                tokio::spawn(kimi_planbar_tray_egui::polling::run(ctx.clone()));
                tokio::spawn(theme_watch::watch(ctx.clone()));
                let c = ctx.clone();
                tokio::spawn(async move { kimi_planbar_tray_egui::check_and_emit(&c).await });
                // Park forever: the spawned services must outlive this future.
                std::future::pending::<()>().await;
            });
        }) {
        Ok(_) => {}
        Err(e) => eprintln!("egui edition: failed to spawn runtime thread: {e}"),
    }
}

/// Consumes the AppEvent bus off the UI thread. Events carry no payload the
/// UI needs directly (AppState holds the data) — they only wake the loop; a
/// quota update also refreshes the tray tooltip.
fn spawn_event_drain(mut events: UnboundedReceiver<AppEvent>, ui: Arc<UiShared>) {
    std::thread::Builder::new()
        .name("event-drain".into())
        .spawn(move || loop {
            match events.blocking_recv() {
                Some(AppEvent::QuotaUpdated(_)) => ui.push(UiCmd::UpdateTooltip),
                Some(_) => {
                    // Theme flips (and any other wake) must repaint every
                    // visible window: a plain request_repaint() wakes only the
                    // ROOT viewport, so the settings / skills / menu windows
                    // would keep the stale palette until their next
                    // interaction (same four-window sweep as save_settings).
                    if let Some(ctx) = ui.egui_ctx.get() {
                        ctx.request_repaint();
                        ctx.request_repaint_of(panel_pos::vp_settings());
                        ctx.request_repaint_of(panel_pos::vp_skills());
                        ctx.request_repaint_of(panel_pos::vp_menu());
                    }
                }
                None => break, // bus closed: shutting down
            }
        })
        .ok();
}

fn run_app(test_ui: bool) {
    // kpt-windows.json is a screenshot-harness diagnostic (written next to
    // the exe): opt-in only, so normal users never get the file. --test-ui
    // (the harness launch mode) or KPT_WRITE_RECTS=1 enables it.
    panel_pos::set_rects_enabled(test_ui || std::env::var("KPT_WRITE_RECTS").as_deref() == Ok("1"));
    let (biz, events_rx) = kimi_planbar_tray_egui::build_state();

    let ui_shared = Arc::new(UiShared::default());
    let _ = UI_SHARED.set(ui_shared.clone());

    let runtime_slot: Arc<OnceLock<tokio::runtime::Handle>> = Arc::new(OnceLock::new());
    spawn_runtime(biz.clone(), runtime_slot.clone());
    spawn_event_drain(events_rx, ui_shared.clone());

    windows::spawn_watcher();
    // WH_CBT hook: styles each viewport window inside CreateWindowEx (before
    // it can ever be shown), closing the first-show caption race. Must be
    // installed on THIS thread before run_native — eframe creates every
    // viewport window on the main/event-loop thread (see windows.rs). If the
    // hook cannot be installed, engage the spec's A′ fallback (10ms polling
    // for the first 3s) so the race is shrunk instead of silently lost.
    if !windows::install_cbt_hook() {
        windows::spawn_fast_startup_watcher();
    }

    // The tray icon must be created on the main thread: winit's message pump
    // (PeekMessageW over the whole thread queue) dispatches the tray-icon
    // message window while eframe runs (Phase 0 finding).
    let tray = tray::build(&biz, &ui_shared, &runtime_slot).expect("build tray icon");

    // `::windows` (not `windows`): the lib's module `windows` is imported in
    // this scope and would shadow the extern crate.
    unsafe {
        let _ = ::windows::Win32::System::Console::SetConsoleCtrlHandler(
            Some(console_ctrl_handler),
            true,
        );
    }

    if test_ui {
        // SPEC 19: exit ~6s after start; hard-exit fallback keeps automated
        // runs from blocking if the event loop refuses to close.
        std::thread::Builder::new()
            .name("test-ui-timer".into())
            .spawn(|| {
                std::thread::sleep(Duration::from_secs(6));
                if let Some(ui) = UI_SHARED.get() {
                    ui.push(UiCmd::Quit);
                }
                std::thread::sleep(Duration::from_secs(5));
                std::process::exit(0);
            })
            .ok();
    }

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::viewport::ViewportBuilder::default()
            .with_title(app::MAIN_WINDOW_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_visible(false)
            .with_taskbar(false)
            .with_inner_size(egui::vec2(panel_pos::PANEL_W, panel_pos::PANEL_H)),
        ..Default::default()
    };

    let ui_for_app = ui_shared.clone();
    let win_bus = Arc::new(WinBus::new());
    let result = eframe::run_native(
        app::MAIN_WINDOW_TITLE,
        options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(
                cc,
                biz,
                ui_for_app,
                win_bus,
                runtime_slot,
                Some(tray),
                test_ui,
            )) as Box<dyn eframe::App>)
        }),
    );
    // eframe dropped the App (and with it the tray icon). Mutex handle is
    // intentionally leaked for the process lifetime (SPEC 20).
    if let Err(e) = result {
        eprintln!("egui edition: eframe exited with error: {e}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Headless quota self-check: fetch once, print indented JSON, exit
    if args.iter().any(|a| a == "--test-fetch") {
        let r = block_on(quota::fetch());
        let line = match serde_json::to_string_pretty(&r) {
            Ok(j) => j,
            Err(e) => format!("serialize error: {e}"),
        };
        kimi_planbar_tray_egui::print_stdout(&format!("{line}\n"));
        return;
    }

    // Headless update-check self-check: single-line summary, exit
    if args.iter().any(|a| a == "--test-update") {
        let st = block_on(update::check());
        kimi_planbar_tray_egui::print_stdout(&format!(
            "local={} latest={} updateAvailable={} checkFailed={}\n",
            st.local_version.as_deref().unwrap_or(""),
            st.latest_version.as_deref().unwrap_or(""),
            dotnet_bool(st.update_available),
            dotnet_bool(st.check_failed),
        ));
        return;
    }

    // Headless UI self-check (SPEC 19): real window construction + theme,
    // prints all four OK lines (MainWindow / SettingsWindow / SkillsWindow /
    // TrayMenuWindow), auto-exits after ~6s. Runs WITHOUT the single-instance
    // mutex.
    if args.iter().any(|a| a == "--test-ui") {
        run_app(true);
        return;
    }

    if !acquire_single_instance_mutex() {
        return; // another instance (Rust, Qt, or WPF edition) is already running
    }

    run_app(false);
}
