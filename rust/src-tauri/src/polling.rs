// Refresh scheduling, 1:1 port of QuotaService.SafeRefresh/Reschedule (UI-SPEC 7.5):
// first refresh 2s after start, period = max(1, RefreshMinutes), failure keeps
// last-known-good data and retries fast after 30s.

use crate::quota::{self, QuotaResult};
use crate::state::AppState;
use crate::tray;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub async fn safe_refresh(app: &AppHandle) -> QuotaResult {
    let mut r = quota::fetch().await;
    let st = app.state::<AppState>();
    if r.error.is_some() {
        let last = st.last_quota.read().unwrap().clone();
        if let Some(l) = &last {
            r.fill_missing_from(l);
        }
    }
    *st.last_quota.write().unwrap() = Some(r.clone());
    let _ = app.emit("quota-updated", &r);
    tray::update_tooltip(app);
    // Every refresh (scheduled, manual, or hover) moves the next scheduled
    // tick: 30s after a failure, one period after a success (SPEC 7.5 step 3,
    // mirrors _timer.Change in QuotaService.SafeRefresh)
    let mins = st.settings.read().unwrap().refresh_minutes.max(1) as u64;
    let delay = if r.error.is_some() {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(mins * 60)
    };
    *st.retime_delay.lock().unwrap() = Some(delay);
    st.retime.notify_one();
    r
}

/// Polling loop. `reschedule` notify restarts the cycle with the 2s first
/// delay (mirrors QuotaService.Reschedule's timer.Change(2s, period)).
pub async fn run(app: AppHandle) {
    let mut next = Duration::from_secs(2);
    loop {
        let reschedule = app.state::<AppState>().reschedule.clone();
        let retime = app.state::<AppState>().retime.clone();
        tokio::select! {
            _ = tokio::time::sleep(next) => {}
            _ = reschedule.notified() => {
                next = Duration::from_secs(2);
                continue;
            }
            _ = retime.notified() => {
                let hint = app.state::<AppState>().retime_delay.lock().unwrap().take();
                if let Some(d) = hint {
                    next = d;
                }
                continue;
            }
        }
        let r = safe_refresh(&app).await;
        next = if r.error.is_some() {
            Duration::from_secs(30) // fast retry after failure
        } else {
            let mins = app
                .state::<AppState>()
                .settings
                .read()
                .unwrap()
                .refresh_minutes
                .max(1) as u64;
            Duration::from_secs(mins * 60)
        };
    }
}
