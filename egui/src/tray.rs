// Tray icon, 1:1 port of rust/src-tauri/src/tray.rs (SPEC section 14):
// static icon, left-click toggles the panel, hover (Enter/Move) refresh
// throttled at 10s, tooltip text only. Right-click forwards the cursor
// position to the root pass, which positions + shows the menu viewport.
//
// tray_icon::TrayIcon is not Send (Rc inside) and must be built on the main
// thread — the same thread that runs the eframe event loop, which is what
// dispatches the tray message-window callbacks through winit's message pump.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::ctx::AppCtx;
use crate::polling;
use crate::quota::{QuotaResult, QuotaSegment};
use crate::state::AppState;

use crate::app::{UiCmd, UiShared};

pub struct Tray {
    pub icon: TrayIcon,
}

/// Tooltip: "Kimi Planbar Tray  5h {x}% · week {y}%" (+ " (update failed)" on
/// error); "Kimi Planbar Tray" alone before any data (SPEC 14).
pub fn tooltip_text(last: Option<&QuotaResult>) -> String {
    match last {
        None => "Kimi Planbar Tray".to_string(),
        Some(l) => {
            let mut t = format!(
                "Kimi Planbar Tray  5h {} · week {}",
                pct(&l.five_hour),
                pct(&l.week)
            );
            if l.error.is_some() {
                t.push_str(" (update failed)");
            }
            t
        }
    }
}

fn pct(seg: &Option<QuotaSegment>) -> String {
    match seg {
        Some(s) => format!("{}%", s.percent.round() as i64),
        None => "?".to_string(),
    }
}

/// Build the tray icon on the main thread and spawn its event thread.
/// `rt` carries the tokio Handle used to run hover refreshes.
pub fn build(
    ctx: &AppCtx,
    ui: &Arc<UiShared>,
    rt: &Arc<OnceLock<tokio::runtime::Handle>>,
) -> Result<Tray, tray_icon::Error> {
    let icon = crate::icons::tray_icon_from_logo()
        .or_else(crate::icons::fallback_tray_icon)
        .expect("build a tray icon bitmap");
    let tray = TrayIconBuilder::new()
        .with_id("tray")
        .with_icon(icon)
        .with_tooltip(tooltip_text(None))
        .build()?;

    std::thread::Builder::new()
        .name("tray-events".into())
        .spawn({
            let ctx = ctx.clone();
            let ui = ui.clone();
            let rt = rt.clone();
            move || event_thread(ctx, ui, rt)
        })
        .ok(); // a failed spawn only costs tray interactivity

    Ok(Tray { icon: tray })
}

fn event_thread(ctx: AppCtx, ui: Arc<UiShared>, rt: Arc<OnceLock<tokio::runtime::Handle>>) {
    let receiver = TrayIconEvent::receiver();
    while let Ok(event) = receiver.recv() {
        match event {
            // MouseUp semantics (not click): more reliable for tray icons.
            TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                position,
                ..
            } => match button {
                MouseButton::Left => ui.push(UiCmd::TogglePanel),
                // Right-click opens the tray menu window at the cursor
                // (SPEC 10.3 / 14): position + show happen in the root pass
                // (the tray thread only carries physical cursor coordinates).
                MouseButton::Right => {
                    let (px, py) = (position.x, position.y);
                    ui.push(UiCmd::ShowMenu { px, py });
                }
                _ => {}
            },
            // Hover-to-refresh, throttled at 10s (SPEC 14).
            TrayIconEvent::Enter { .. } | TrayIconEvent::Move { .. } => on_hover(&ctx, &rt),
            _ => {}
        }
    }
}

fn on_hover(ctx: &AppCtx, rt: &OnceLock<tokio::runtime::Handle>) {
    // Stamp last_hover under the lock, bail inside the 10s window (SPEC 14).
    {
        let mut last = ctx.state.last_hover.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(10) {
                return;
            }
        }
        *last = Some(std::time::Instant::now());
    }
    // SafeRefresh is async: run it on the shared tokio runtime.
    if let Some(handle) = rt.get() {
        let ctx = ctx.clone();
        handle.spawn(async move {
            polling::safe_refresh(&ctx).await;
        });
    }
}

/// Recompute the tooltip from the last quota result (called when a
/// QuotaUpdated event reaches the UI thread — port of tray::update_tooltip).
pub fn update_tooltip(tray: &TrayIcon, state: &AppState) {
    let last = state.last_quota.read().unwrap().clone();
    let _ = tray.set_tooltip(Some(tooltip_text(last.as_ref())));
}
