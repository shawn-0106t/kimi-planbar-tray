// Refresh scheduling, 1:1 port of QuotaService.SafeRefresh/Reschedule (SPEC 16.5):
// first refresh 2s after start, period = max(1, RefreshMinutes), failure keeps
// last-known-good data and retries fast after 30s.

use crate::ctx::{AppCtx, AppEvent};
use crate::quota::{self, QuotaResult};
use std::time::Duration;

pub async fn safe_refresh(ctx: &AppCtx) -> QuotaResult {
    let mut r = quota::fetch().await;
    let st = &ctx.state;
    if r.error.is_some() {
        let last = st.last_quota.read().unwrap().clone();
        if let Some(l) = &last {
            r.fill_missing_from(l);
        }
    }
    *st.last_quota.write().unwrap() = Some(r.clone());
    // Replaces the Tauri "quota-updated" emit + the tray::update_tooltip nudge:
    // the Phase 2 GUI/tray consumes this event and refreshes the tooltip.
    let _ = ctx.events.send(AppEvent::QuotaUpdated(r.clone()));
    // Every refresh (scheduled, manual, or hover) moves the next scheduled
    // tick: 30s after a failure, one period after a success (SPEC 16.5 step 3,
    // mirrors _timer.Change in QuotaService.SafeRefresh)
    let mins = st.settings.read().unwrap().refresh_minutes.max(1) as u64;
    let delay = if r.error.is_some() {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(mins.saturating_mul(60))
    };
    *st.retime_delay.lock().unwrap() = Some(delay);
    st.retime.notify_one();
    r
}

/// Polling loop. `reschedule` notify restarts the cycle with the 2s first
/// delay (mirrors QuotaService.Reschedule's timer.Change(2s, period)).
pub async fn run(ctx: AppCtx) {
    let mut next = Duration::from_secs(2);
    loop {
        let reschedule = ctx.state.reschedule.clone();
        let retime = ctx.state.retime.clone();
        tokio::select! {
            _ = tokio::time::sleep(next) => {}
            _ = reschedule.notified() => {
                // Drain any stale retime hint queued before this reschedule:
                // the notify permit survives `continue`, and a leftover delay
                // would otherwise overwrite the 2s first-refresh tick.
                let _ = ctx.state.retime_delay.lock().unwrap().take();
                next = Duration::from_secs(2);
                continue;
            }
            _ = retime.notified() => {
                let hint = ctx.state.retime_delay.lock().unwrap().take();
                if let Some(d) = hint {
                    next = d;
                }
                continue;
            }
        }
        let r = safe_refresh(&ctx).await;
        next = if r.error.is_some() {
            Duration::from_secs(30) // fast retry after failure
        } else {
            let mins = ctx.state.settings.read().unwrap().refresh_minutes.max(1) as u64;
            Duration::from_secs(mins.saturating_mul(60))
        };
    }
}
