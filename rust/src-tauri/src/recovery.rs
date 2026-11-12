// WebView2 failure detection, diagnostics and in-process window recovery
// (SPEC 22). ProcessFailed is the primary signal; a missing-window gate on
// every show path is the supplementary one. A failed window is hidden and
// destroyed, then rebuilt from the same tauri.conf.json WindowConfig with the
// same label, so AppState, the tray icon and the polling tasks keep running.

use crate::panel;
use crate::settings;
use crate::state::AppState;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2ProcessFailedEventArgs2, COREWEBVIEW2_PROCESS_FAILED_KIND,
    COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_PLUGIN_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
    COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED, COREWEBVIEW2_PROCESS_FAILED_REASON,
    COREWEBVIEW2_PROCESS_FAILED_REASON_CRASHED, COREWEBVIEW2_PROCESS_FAILED_REASON_LAUNCH_FAILED,
    COREWEBVIEW2_PROCESS_FAILED_REASON_OUT_OF_MEMORY,
    COREWEBVIEW2_PROCESS_FAILED_REASON_PROFILE_DELETED,
    COREWEBVIEW2_PROCESS_FAILED_REASON_TERMINATED, COREWEBVIEW2_PROCESS_FAILED_REASON_UNEXPECTED,
    COREWEBVIEW2_PROCESS_FAILED_REASON_UNRESPONSIVE,
};
use webview2_com::{take_pwstr, ProcessFailedEventHandler};
use windows::core::{Interface, PWSTR};

/// Synthetic kind used when a show path finds the window missing (a rebuild is
/// in flight or a previous rebuild failed).
const KIND_MISSING_WINDOW: i32 = -2;
/// Rebuild attempts are capped at 3 per sliding interval of this length
/// (SPEC 22.4).
const REBUILD_WINDOW: Duration = Duration::from_secs(30);
/// Reload-then-rebuild escalation distance for renderer failures (SPEC 22.3).
const RELOAD_ESCALATION: Duration = Duration::from_secs(120);
/// Upper bound for "the destroyed label is gone from the manager" polling.
const LABEL_REMOVAL_TIMEOUT: Duration = Duration::from_secs(3);
/// Upper bound for waiting on the rebuilt page's load event before showing it.
const PAGE_LOAD_TIMEOUT: Duration = Duration::from_secs(2);
const LOG_MAX_BYTES: u64 = 256 * 1024;
const LOG_KEEP_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct FailureEvent {
    pub label: String,
    pub generation: u64,
    pub kind: i32,
    pub reason: Option<i32>,
    pub exit_code: Option<i32>,
    pub description: Option<String>,
    pub at: Instant,
}

#[derive(Debug, Clone)]
struct UiEvent {
    at: Instant,
    label: String,
    what: String,
}

#[derive(Default)]
struct RecoveryInner {
    /// Per-label generation, bumped at the start of every rebuild; events from
    /// handlers installed for an older webview are stale and ignored.
    generations: HashMap<String, u64>,
    runtime_versions: HashMap<String, String>,
    /// Recent reload timestamps per label (renderer-failure escalation).
    reloads: HashMap<String, Vec<Instant>>,
    /// Rebuild start timestamps inside REBUILD_WINDOW (rate limiting).
    rebuilds: Vec<Instant>,
    /// Show requests deferred while the window was missing.
    pending_show: HashSet<String>,
    /// Last known visibility per label, maintained at every show/hide call
    /// site. A failure can precede the focus-loss hide, so the current native
    /// state is not a reliable restore snapshot.
    last_visible: HashMap<String, bool>,
    /// Labels whose current webview reported a failure and has not been
    /// rebuilt since. The native window may still exist as a dead shell.
    dead: HashSet<String>,
    /// Diagnostic ring buffer attached to every failure log line.
    recent_ui: VecDeque<UiEvent>,
    /// Whether the rate-limit cutoff was already logged.
    disabled_logged: bool,
}

pub struct RecoveryState {
    pub tx: tokio::sync::mpsc::UnboundedSender<FailureEvent>,
    rx: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<FailureEvent>>>,
    inner: Mutex<RecoveryInner>,
    pub recovering: AtomicBool,
    pub disabled: AtomicBool,
    pub fallback_menu: AtomicBool,
    pub started: Instant,
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryState {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        RecoveryState {
            tx,
            rx: Mutex::new(Some(rx)),
            inner: Mutex::new(RecoveryInner::default()),
            recovering: AtomicBool::new(false),
            disabled: AtomicBool::new(false),
            fallback_menu: AtomicBool::new(false),
            started: Instant::now(),
        }
    }

    fn take_receiver(&self) -> Option<tokio::sync::mpsc::UnboundedReceiver<FailureEvent>> {
        self.rx.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    ReloadThenRebuild,
    RebuildAll,
    Missing,
    LogOnly,
}

fn classify(kind: i32) -> Class {
    if kind == KIND_MISSING_WINDOW {
        return Class::Missing;
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED.0 {
        return Class::RebuildAll;
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED.0
        || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE.0
        || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED.0
    {
        return Class::ReloadThenRebuild;
    }
    Class::LogOnly
}

fn kind_name(kind: i32) -> &'static str {
    if kind == KIND_MISSING_WINDOW {
        return "WindowMissing";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED.0 {
        return "BrowserProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED.0 {
        return "RenderProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE.0 {
        return "RenderProcessUnresponsive";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED.0 {
        return "FrameRenderProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED.0 {
        return "GPUProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED.0 {
        return "PpapiBrokerProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_PLUGIN_PROCESS_EXITED.0 {
        return "PpapiPluginProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED.0 {
        return "SandboxHelperProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED.0 {
        return "UtilityProcessExited";
    }
    if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED.0 {
        return "UnknownProcessExited";
    }
    "Unrecognized"
}

fn reason_name(reason: Option<i32>) -> &'static str {
    match reason {
        None => "-",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_UNEXPECTED.0 => "Unexpected",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_UNRESPONSIVE.0 => "Unresponsive",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_TERMINATED.0 => "Terminated",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_CRASHED.0 => "Crashed",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_LAUNCH_FAILED.0 => "LaunchFailed",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_OUT_OF_MEMORY.0 => "OutOfMemory",
        Some(r) if r == COREWEBVIEW2_PROCESS_FAILED_REASON_PROFILE_DELETED.0 => "ProfileDeleted",
        Some(_) => "Unknown",
    }
}

/// Backoff before the Nth rebuild inside the rate-limit window; `None` once
/// the cap is reached.
fn backoff_after(attempts: usize) -> Option<Duration> {
    match attempts {
        0 => Some(Duration::from_secs(1)),
        1 => Some(Duration::from_secs(2)),
        2 => Some(Duration::from_secs(5)),
        _ => None,
    }
}

/// Byte offset of the first complete line at or after `len - keep`.
fn tail_from_line_boundary(bytes: &[u8], keep: usize) -> usize {
    if bytes.len() <= keep {
        return 0;
    }
    let start = bytes.len() - keep;
    if start == 0 || bytes[start - 1] == b'\n' {
        return start;
    }
    bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| start + p + 1)
        .unwrap_or(start)
}

static LOG_LOCK: Mutex<()> = Mutex::new(());

/// Append one line to `webview.log` next to settings.json. Never contains
/// credentials or quota data. Truncates the file to its tail past 256 KB so a
/// crash loop cannot grow it without bound.
pub(crate) fn log(msg: &str) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = crate::app_data_dir_override().unwrap_or_else(settings::config_dir);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("webview.log");
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > LOG_MAX_BYTES {
            if let Ok(bytes) = std::fs::read(&path) {
                let start = tail_from_line_boundary(&bytes, LOG_KEEP_BYTES);
                let _ = std::fs::write(&path, &bytes[start..]);
            }
        }
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(
            file,
            "{} {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            msg
        );
    }
}

fn window_info(app: &AppHandle, label: &str) -> (usize, bool, u32) {
    let Some(window) = app.get_webview_window(label) else {
        return (0, false, 0);
    };
    let visible = window.is_visible().unwrap_or(false);
    let Ok(hwnd) = window.hwnd() else {
        return (0, visible, 0);
    };
    let dpi = unsafe {
        windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(hwnd.0 as _))
    };
    (hwnd.0 as usize, visible, dpi)
}

fn log_event(app: &AppHandle, event: &FailureEvent, action: &str) {
    let st = app.state::<AppState>();
    let (runtime, recent) = {
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = inner
            .runtime_versions
            .get(&event.label)
            .cloned()
            .unwrap_or_else(|| "-".to_string());
        let recent = inner
            .recent_ui
            .iter()
            .map(|e| {
                format!(
                    "{}:{}@{}s",
                    e.label,
                    e.what,
                    e.at.saturating_duration_since(st.recovery.started)
                        .as_secs()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        (runtime, recent)
    };
    let (hwnd, visible, dpi) = window_info(app, &event.label);
    log(&format!(
        "webview2 label={} kind={}({}) reason={}({}) exit={:?} desc={:?} pid={} hwnd=0x{:X} visible={} dpi={} runtime={} uptime={}s action={} recent=[{}]",
        event.label,
        kind_name(event.kind),
        event.kind,
        reason_name(event.reason),
        event.reason.map(|r| r.to_string()).unwrap_or_else(|| "-".to_string()),
        event.exit_code,
        event.description.as_deref().unwrap_or("-"),
        std::process::id(),
        hwnd,
        visible,
        dpi,
        runtime,
        st.recovery.started.elapsed().as_secs(),
        action,
        recent
    ));
}

/// Record a show/hide/DPI/wake event for the diagnostic ring buffer.
pub fn note_ui_event(app: &AppHandle, label: &str, what: &str) {
    let st = app.state::<AppState>();
    let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
    inner.recent_ui.push_back(UiEvent {
        at: Instant::now(),
        label: label.to_string(),
        what: what.to_string(),
    });
    while inner.recent_ui.len() > 8 {
        inner.recent_ui.pop_front();
    }
}

/// Track the last known visibility of a window. Show/hide paths call this so a
/// rebuild restores the state from before the failure, not after a racing
/// focus-loss hide.
pub fn set_last_visible(app: &AppHandle, label: &str, visible: bool) {
    let st = app.state::<AppState>();
    st.recovery
        .inner
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .last_visible
        .insert(label.to_string(), visible);
}

/// Supplementary show gate: never interact with a missing or failed shell.
/// Defers the request until the rebuild finishes (`pending_show`) and, when no
/// recovery is running, asks the coordinator to rebuild the window.
pub fn gate_show(app: &AppHandle, label: &str) -> bool {
    let present = app.get_webview_window(label).is_some();
    let st = app.state::<AppState>();
    let dead = {
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.dead.contains(label)
    };
    if present && !dead {
        return true;
    }
    // The tray menu is the only path to Settings/Skills/Exit, so it must also
    // be able to trigger a rebuild even though it is never restored visible.
    let restore_after_rebuild = matches!(label, "main" | "settings" | "skills");
    if !restore_after_rebuild && label != "menu" {
        return false;
    }
    if st.recovery.disabled.load(Ordering::SeqCst) {
        // The webview menu is the only way to Exit; replace it lazily once its
        // webview is known dead (or missing) after auto-recovery was disabled.
        if label == "menu"
            && !st.recovery.fallback_menu.load(Ordering::SeqCst)
            && (!present || dead)
        {
            crate::install_fallback_tray_menu(app);
        }
        return false;
    }
    let generation = {
        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        if restore_after_rebuild {
            inner.pending_show.insert(label.to_string());
        }
        inner.generations.get(label).copied().unwrap_or(0)
    };
    if !st.recovery.recovering.load(Ordering::SeqCst) {
        let _ = st.recovery.tx.send(FailureEvent {
            label: label.to_string(),
            generation,
            kind: KIND_MISSING_WINDOW,
            reason: None,
            exit_code: None,
            description: None,
            at: Instant::now(),
        });
    }
    false
}

/// True once the native fallback tray menu (quit-only) replaced the webview
/// menu after auto-recovery was disabled.
pub fn fallback_menu_active(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .recovery
        .fallback_menu
        .load(Ordering::SeqCst)
}

/// Subscribe the window's CoreWebView2 to ProcessFailed. Safe from the main
/// thread (synchronous dispatch) and from background threads (proxied).
pub fn install(window: &WebviewWindow) {
    let label = window.label().to_string();
    let app = window.app_handle().clone();
    let generation = {
        let st = app.state::<AppState>();
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.generations.get(&label).copied().unwrap_or(0)
    };
    let sender = app.state::<AppState>().recovery.tx.clone();
    let version_app = app.clone();
    let version_label = label.clone();
    let handler_label = label.clone();
    let log_label = label.clone();
    let result = window.with_webview(move |platform| {
        let controller = platform.controller();
        let Ok(core) = (unsafe { controller.CoreWebView2() }) else {
            log(&format!(
                "webview2 label={log_label} CoreWebView2 unavailable"
            ));
            return;
        };
        let mut version = PWSTR::null();
        if unsafe { platform.environment().BrowserVersionString(&mut version) }.is_ok() {
            let value = take_pwstr(version);
            let st = version_app.state::<AppState>();
            st.recovery
                .inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .runtime_versions
                .insert(version_label, value);
        }
        let handler = ProcessFailedEventHandler::create(Box::new(move |_sender, args| {
            let Some(args) = args else {
                return Ok(());
            };
            let mut kind = -1i32;
            let mut kind_value = COREWEBVIEW2_PROCESS_FAILED_KIND(0);
            if unsafe { args.ProcessFailedKind(&mut kind_value) }.is_ok() {
                kind = kind_value.0;
            }
            let mut reason: Option<i32> = None;
            let mut exit_code: Option<i32> = None;
            let mut description: Option<String> = None;
            if let Ok(args2) = args.cast::<ICoreWebView2ProcessFailedEventArgs2>() {
                let mut value = COREWEBVIEW2_PROCESS_FAILED_REASON(0);
                if unsafe { args2.Reason(&mut value) }.is_ok() {
                    reason = Some(value.0);
                }
                let mut code = 0i32;
                if unsafe { args2.ExitCode(&mut code) }.is_ok() {
                    exit_code = Some(code);
                }
                let mut text = PWSTR::null();
                if unsafe { args2.ProcessDescription(&mut text) }.is_ok() {
                    description = Some(take_pwstr(text));
                }
            }
            let _ = sender.send(FailureEvent {
                label: handler_label.clone(),
                generation,
                kind,
                reason,
                exit_code,
                description,
                at: Instant::now(),
            });
            Ok(())
        }));
        let mut token = 0i64;
        unsafe {
            let _ = core.add_ProcessFailed(&handler, &mut token);
        }
    });
    if result.is_err() {
        log(&format!(
            "webview2 label={label} ProcessFailed install failed"
        ));
    }
}

/// Install the ProcessFailed handler on every live window (startup).
pub fn install_all(app: &AppHandle) {
    for window in app.webview_windows().values() {
        install(window);
    }
}

/// Spawn the single recovery coordinator (startup). The coordinator owns the
/// receiver for the whole process lifetime; AppState, the tray and polling are
/// never restarted.
pub fn start(app: &AppHandle) {
    let Some(mut rx) = app.state::<AppState>().recovery.take_receiver() else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(first) = rx.recv().await {
            let mut batch = vec![first];
            while let Ok(event) = rx.try_recv() {
                batch.push(event);
            }
            handle_batch(&app, batch).await;
        }
    });
}

async fn handle_batch(app: &AppHandle, batch: Vec<FailureEvent>) {
    let st = app.state::<AppState>();
    if st.recovery.disabled.load(Ordering::SeqCst) {
        for event in &batch {
            log_event(app, event, "ignored (auto-recovery disabled)");
        }
        return;
    }

    let mut rebuild_all = false;
    let mut rebuild_labels: Vec<String> = Vec::new();
    let mut reload_labels: Vec<String> = Vec::new();
    let mut reloaded_in_batch: HashSet<String> = HashSet::new();
    let mut actions: Vec<(FailureEvent, String)> = Vec::new();

    for event in batch {
        let stale = {
            let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.generations.get(&event.label).copied().unwrap_or(0) != event.generation
        };
        if stale {
            actions.push((event, "ignored (stale generation)".to_string()));
            continue;
        }
        let window_present = app.get_webview_window(&event.label).is_some();
        let class = classify(event.kind);
        {
            let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
            match class {
                Class::RebuildAll => {
                    for label in ["main", "settings", "skills", "menu"] {
                        inner.dead.insert(label.to_string());
                    }
                }
                Class::Missing | Class::ReloadThenRebuild => {
                    inner.dead.insert(event.label.clone());
                }
                Class::LogOnly => {}
            }
        }
        match class {
            Class::RebuildAll => {
                rebuild_all = true;
                actions.push((event, "request rebuild all windows".to_string()));
            }
            Class::Missing => {
                // A synthetic event from gate_show: the window is missing or a
                // known-dead shell, so it always needs a rebuild.
                if !rebuild_labels.contains(&event.label) {
                    rebuild_labels.push(event.label.clone());
                }
                actions.push((event, "request rebuild window".to_string()));
            }
            Class::ReloadThenRebuild => {
                if !window_present {
                    if !rebuild_labels.contains(&event.label) {
                        rebuild_labels.push(event.label.clone());
                    }
                    actions.push((event, "request rebuild window (already gone)".to_string()));
                    continue;
                }
                // One crash can emit several events for the same label (e.g.
                // RenderProcessExited + RenderProcessUnresponsive); only the
                // first one in a batch counts as the single allowed reload.
                if reloaded_in_batch.contains(&event.label) {
                    // The first event in this batch already reloaded the page;
                    // this duplicate must not leave the label marked dead or
                    // the next show would destroy and rebuild a working window.
                    {
                        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
                        inner.dead.remove(&event.label);
                    }
                    actions.push((event, "ignored (duplicate in batch)".to_string()));
                    continue;
                }
                let repeated = {
                    let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
                    let list = inner.reloads.entry(event.label.clone()).or_default();
                    list.retain(|t| t.elapsed() < RELOAD_ESCALATION);
                    !list.is_empty()
                };
                if repeated {
                    if !rebuild_labels.contains(&event.label) {
                        rebuild_labels.push(event.label.clone());
                    }
                    actions.push((event, "request rebuild window (repeat failure)".to_string()));
                } else {
                    {
                        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
                        inner
                            .reloads
                            .entry(event.label.clone())
                            .or_default()
                            .push(Instant::now());
                    }
                    reloaded_in_batch.insert(event.label.clone());
                    {
                        // The reload replaces the failed renderer; clear the
                        // dead flag optimistically (a repeat failure re-sets it
                        // and escalates to a rebuild).
                        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
                        inner.dead.remove(&event.label);
                    }
                    if !reload_labels.contains(&event.label) {
                        reload_labels.push(event.label.clone());
                    }
                    actions.push((event, "reload page".to_string()));
                }
            }
            Class::LogOnly => actions.push((event, "logged only".to_string())),
        }
    }

    for (event, action) in &actions {
        log_event(app, event, action);
    }

    if rebuild_all {
        let labels = app
            .config()
            .app
            .windows
            .iter()
            .map(|w| w.label.clone())
            .collect::<Vec<_>>();
        rebuild(app, labels).await;
        return;
    }

    for label in reload_labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.reload();
        }
    }
    if !rebuild_labels.is_empty() {
        rebuild(app, rebuild_labels).await;
    }
}

async fn rebuild(app: &AppHandle, labels: Vec<String>) {
    if labels.is_empty() {
        return;
    }
    let st = app.state::<AppState>();
    if st.recovery.recovering.swap(true, Ordering::SeqCst) {
        log(&format!("recovery skipped (already running): {labels:?}"));
        return;
    }

    let backoff = {
        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        inner
            .rebuilds
            .retain(|t| now.duration_since(*t) < REBUILD_WINDOW);
        match backoff_after(inner.rebuilds.len()) {
            Some(delay) => {
                inner.rebuilds.push(now);
                Some(delay)
            }
            None => None,
        }
    };
    let Some(backoff) = backoff else {
        let already_logged = {
            let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
            let seen = inner.disabled_logged;
            inner.disabled_logged = true;
            seen
        };
        if !already_logged {
            st.recovery.disabled.store(true, Ordering::SeqCst);
            log(&format!(
                "auto-recovery disabled: 3 rebuild attempts within {}s",
                REBUILD_WINDOW.as_secs()
            ));
            // Install the native quit-only menu only when the webview menu is
            // unusable; a transient crash storm on other windows must not take
            // the healthy Open/Refresh/Settings/Skills menu away permanently.
            let menu_unusable = labels.iter().any(|l| l == "menu")
                || app.get_webview_window("menu").is_none()
                || st
                    .recovery
                    .inner
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .dead
                    .contains("menu");
            if menu_unusable {
                crate::install_fallback_tray_menu(app);
            }
        }
        st.recovery.recovering.store(false, Ordering::SeqCst);
        return;
    };

    log(&format!(
        "recovery start labels={labels:?} backoff={}ms",
        backoff.as_millis()
    ));
    tokio::time::sleep(backoff).await;
    let started = Instant::now();
    let result = do_rebuild(app, &labels).await;
    match &result {
        Ok(()) => {
            log(&format!(
                "recovery ok labels={labels:?} elapsed={}ms",
                started.elapsed().as_millis()
            ));
            let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
            for label in &labels {
                inner.reloads.remove(label);
                inner.pending_show.remove(label);
            }
        }
        Err(error) => {
            log(&format!("recovery failed labels={labels:?}: {error}"));
            {
                let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
                for label in &labels {
                    inner.pending_show.remove(label);
                }
            }
            reconcile_after_failure(app, &labels);
        }
    }
    st.recovery.recovering.store(false, Ordering::SeqCst);

    // Deferred show requests for labels outside this round (e.g. Settings was
    // missing while only main was being rebuilt) must not be swallowed: queue
    // them again now that the coordinator is free.
    let mut extra: Vec<(String, u64)> = Vec::new();
    {
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        for label in &inner.pending_show {
            if !labels.contains(label) {
                extra.push((
                    label.clone(),
                    inner.generations.get(label).copied().unwrap_or(0),
                ));
            }
        }
    }
    for (label, generation) in extra {
        let _ = st.recovery.tx.send(FailureEvent {
            label,
            generation,
            kind: KIND_MISSING_WINDOW,
            reason: None,
            exit_code: None,
            description: None,
            at: Instant::now(),
        });
    }
}

/// After a failed round, reconcile the suppression flags and the visibility
/// cache with the windows that actually survived: a round can fail halfway
/// (some windows rebuilt, others not), leaving `settings_open` true over a
/// hidden window, which would disable the panel's focus-loss auto-hide.
fn reconcile_after_failure(app: &AppHandle, labels: &[String]) {
    let st = app.state::<AppState>();
    let mut visible_now: HashMap<String, bool> = HashMap::new();
    for label in labels {
        let visible = app
            .get_webview_window(label)
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        visible_now.insert(label.clone(), visible);
    }
    {
        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        for (label, visible) in &visible_now {
            inner.last_visible.insert(label.clone(), *visible);
        }
    }
    if labels.iter().any(|l| l == "settings")
        && !visible_now.get("settings").copied().unwrap_or(false)
    {
        st.settings_open.store(false, Ordering::SeqCst);
    }
    if labels.iter().any(|l| l == "skills") && !visible_now.get("skills").copied().unwrap_or(false)
    {
        st.skills_open.store(false, Ordering::SeqCst);
    }
    if labels.iter().any(|l| l == "main") && !visible_now.get("main").copied().unwrap_or(false) {
        st.panel_hiding.store(false, Ordering::SeqCst);
    }
}

async fn do_rebuild(app: &AppHandle, labels: &[String]) -> Result<(), String> {
    let st = app.state::<AppState>();
    let rebuilding_main = labels.iter().any(|l| l == "main");
    let panel_was_hiding = st.panel_hiding.load(Ordering::SeqCst);

    let last_visible = {
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.last_visible.clone()
    };
    let mut was_visible: HashMap<String, bool> = HashMap::new();
    for label in labels {
        // Prefer the state recorded at the last show/hide call over the live
        // native flag: a focus-loss hide can race the failure event and would
        // otherwise restore the wrong visibility.
        let visible = last_visible.get(label).copied().unwrap_or_else(|| {
            app.get_webview_window(label)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false)
        });
        was_visible.insert(label.clone(), visible);
    }
    if rebuilding_main {
        st.panel_hiding.store(false, Ordering::SeqCst);
    }

    for label in labels {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.hide();
        }
    }
    {
        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        for label in labels {
            *inner.generations.entry(label.clone()).or_insert(0) += 1;
        }
    }
    for label in labels {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.destroy();
        }
    }

    let deadline = Instant::now() + LABEL_REMOVAL_TIMEOUT;
    loop {
        if labels
            .iter()
            .all(|label| app.get_webview_window(label).is_none())
        {
            break;
        }
        if Instant::now() >= deadline {
            let remaining = labels
                .iter()
                .filter(|label| app.get_webview_window(label).is_some())
                .cloned()
                .collect::<Vec<_>>();
            return Err(format!("label removal timeout: {remaining:?}"));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let mut rebuilt: Vec<(String, WebviewWindow, tokio::sync::oneshot::Receiver<()>)> = Vec::new();
    for label in labels {
        let Some(config) = app
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == *label)
            .cloned()
        else {
            return Err(format!("no window config for {label}"));
        };
        let (load_tx, load_rx) = tokio::sync::oneshot::channel::<()>();
        let load_tx = std::sync::Arc::new(Mutex::new(Some(load_tx)));
        let load_tx2 = load_tx.clone();
        let builder = WebviewWindowBuilder::from_config(app, &config)
            .map_err(|e| format!("rebuild {label}: {e}"))?
            .on_page_load(move |_window, payload| {
                if matches!(payload.event(), PageLoadEvent::Finished) {
                    if let Some(tx) = load_tx2.lock().unwrap_or_else(|e| e.into_inner()).take() {
                        let _ = tx.send(());
                    }
                }
            });
        let window = builder
            .build()
            .map_err(|e| format!("rebuild {label}: {e}"))?;
        crate::apply_no_round(&window);
        install(&window);
        // The new webview is alive again; clear the dead mark before the
        // restore loop calls the gated show paths.
        st.recovery
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dead
            .remove(label);
        rebuilt.push((label.clone(), window, load_rx));
    }

    // Read pending shows after the build: a tray click during the rebuild must
    // still surface the window once it is back (SPEC 22.3).
    let pending = {
        let inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.pending_show.clone()
    };
    let mut shown: HashSet<String> = HashSet::new();
    for (label, _window, load_rx) in &mut rebuilt {
        let mut should_show = was_visible.get(label).copied().unwrap_or(false);
        if label == "main" && panel_was_hiding && !pending.contains(label) {
            should_show = false;
        }
        let should_show = should_show || pending.contains(label);
        if !should_show || label == "menu" {
            continue;
        }
        let _ = tokio::time::timeout(PAGE_LOAD_TIMEOUT, load_rx).await;
        match label.as_str() {
            // show_panel re-pins the size, positions against the primary work
            // area and emits panel-show after the page listener is attached.
            "main" => panel::show_panel(app),
            "settings" => crate::show_settings_window(app),
            "skills" => crate::show_skills_window(app),
            _ => {}
        }
        // Only count it as restored if the window still exists (a show helper
        // bails out when the window disappeared between build and show).
        if app.get_webview_window(label).is_some() {
            shown.insert(label.clone());
        }
    }

    // Reconcile the suppression flags and the visibility cache with what was
    // actually restored: a flag left true over a hidden window would suppress
    // the main panel's focus-loss auto-hide forever.
    {
        let mut inner = st.recovery.inner.lock().unwrap_or_else(|e| e.into_inner());
        for (label, _, _) in &rebuilt {
            if !shown.contains(label) {
                inner.last_visible.insert(label.clone(), false);
            }
        }
    }
    if labels.iter().any(|l| l == "settings") && !shown.contains("settings") {
        st.settings_open.store(false, Ordering::SeqCst);
    }
    if labels.iter().any(|l| l == "skills") && !shown.contains("skills") {
        st.skills_open.store(false, Ordering::SeqCst);
    }
    if rebuilding_main && !shown.contains("main") {
        st.panel_hiding.store(false, Ordering::SeqCst);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_covers_every_kind_bucket() {
        assert_eq!(classify(KIND_MISSING_WINDOW), Class::Missing);
        assert_eq!(
            classify(COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED.0),
            Class::RebuildAll
        );
        for kind in [
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED.0,
        ] {
            assert_eq!(classify(kind), Class::ReloadThenRebuild);
        }
        for kind in [
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_PLUGIN_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED.0,
            COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED.0,
        ] {
            assert_eq!(classify(kind), Class::LogOnly);
        }
    }

    #[test]
    fn backoff_is_capped_after_three_attempts() {
        assert_eq!(backoff_after(0), Some(Duration::from_secs(1)));
        assert_eq!(backoff_after(1), Some(Duration::from_secs(2)));
        assert_eq!(backoff_after(2), Some(Duration::from_secs(5)));
        assert_eq!(backoff_after(3), None);
    }

    #[test]
    fn tail_trimming_starts_at_a_line_boundary() {
        let bytes = b"alpha\nbeta\ngamma\n";
        assert_eq!(tail_from_line_boundary(bytes, 100), 0);
        assert_eq!(&bytes[tail_from_line_boundary(bytes, 6)..], b"gamma\n");
        assert_eq!(&bytes[tail_from_line_boundary(bytes, 8)..], b"gamma\n");
    }
}
