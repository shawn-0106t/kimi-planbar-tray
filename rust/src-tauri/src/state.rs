use crate::quota::QuotaResult;
use crate::settings::SettingsData;
use crate::skills::SkillInfo;
use crate::update::UpdateStatus;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Shared application state (managed by Tauri).
pub struct AppState {
    pub settings: RwLock<SettingsData>,
    pub last_quota: RwLock<Option<QuotaResult>>,
    pub update: RwLock<UpdateStatus>,
    /// Effective theme after resolving "system": "light" | "dark"
    pub effective_theme: RwLock<String>,
    /// Timestamp of the last auto-hide, for the 300ms tray-click re-entry guard
    pub last_hide: Mutex<Option<Instant>>,
    /// Timestamp of the last hover-triggered refresh (10s throttle)
    pub last_hover: Mutex<Option<Instant>>,
    pub panel_hiding: AtomicBool,
    /// While the settings window is open, panel focus-loss hide is suppressed
    pub settings_open: AtomicBool,
    /// While the skills window is open, panel focus-loss hide is suppressed
    pub skills_open: AtomicBool,
    /// Lazy one-shot cache for the skills window (no background scanning)
    pub skills_cache: RwLock<Option<Vec<SkillInfo>>>,
    /// Content-measured menu height in DIP (WPF SizeToContent equivalent)
    pub menu_height: Mutex<f64>,
    /// Fired when the polling schedule must restart (settings saved)
    pub reschedule: Arc<Notify>,
    /// Fired by every safe_refresh (manual, hover, or scheduled) to move the
    /// next scheduled tick to now+delay — mirrors QuotaService's per-call
    /// _timer.Change(error ? 30s : period, period) (SPEC 16.5 step 3)
    pub retime: Arc<Notify>,
    pub retime_delay: Mutex<Option<Duration>>,
}

impl AppState {
    pub fn new(settings: SettingsData, effective_theme: String) -> Self {
        AppState {
            settings: RwLock::new(settings),
            last_quota: RwLock::new(None),
            update: RwLock::new(UpdateStatus::default()),
            effective_theme: RwLock::new(effective_theme),
            last_hide: Mutex::new(None),
            last_hover: Mutex::new(None),
            panel_hiding: AtomicBool::new(false),
            settings_open: AtomicBool::new(false),
            skills_open: AtomicBool::new(false),
            skills_cache: RwLock::new(None),
            menu_height: Mutex::new(160.0),
            reschedule: Arc::new(Notify::new()),
            retime: Arc::new(Notify::new()),
            retime_delay: Mutex::new(None),
        }
    }
}
