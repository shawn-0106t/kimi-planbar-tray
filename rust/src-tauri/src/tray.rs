// Tray icon, 1:1 port of TrayManager (UI-SPEC section 5):
// static icon, left-click toggles the panel, right-click opens the menu window
// at the cursor, hover (Enter/Move) prefetch-throttled at 10s, tooltip text only.

use crate::panel;
use crate::polling;
use crate::quota::{QuotaResult, QuotaSegment};
use crate::state::AppState;
use std::time::{Duration, Instant};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

/// Programmatic 32x32 blue ball, drawn exactly like the WPF fallback
/// (circle (1,1,30,30) #1A88FF + highlight ellipse (7,5,10,7) white@90).
fn fallback_icon() -> tauri::image::Image<'static> {
    const S: usize = 32;
    let mut px = vec![0u8; S * S * 4];
    for y in 0..S {
        for x in 0..S {
            let dx = x as f64 + 0.5 - 16.0;
            let dy = y as f64 + 0.5 - 16.0;
            if dx * dx + dy * dy <= 15.0 * 15.0 {
                let i = (y * S + x) * 4;
                px[i] = 0x1A;
                px[i + 1] = 0x88;
                px[i + 2] = 0xFF;
                px[i + 3] = 0xFF;
            }
        }
    }
    let a = 90.0 / 255.0;
    for y in 0..S {
        for x in 0..S {
            let dx = (x as f64 + 0.5 - 12.0) / 5.0;
            let dy = (y as f64 + 0.5 - 8.5) / 3.5;
            if dx * dx + dy * dy <= 1.0 {
                let i = (y * S + x) * 4;
                if px[i + 3] > 0 {
                    px[i] = (255.0 * a + px[i] as f64 * (1.0 - a)) as u8;
                    px[i + 1] = (255.0 * a + px[i + 1] as f64 * (1.0 - a)) as u8;
                    px[i + 2] = (255.0 * a + px[i + 2] as f64 * (1.0 - a)) as u8;
                }
            }
        }
    }
    tauri::image::Image::new_owned(px, S as u32, S as u32)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let icon = tauri::image::Image::from_bytes(ICON_PNG).unwrap_or_else(|_| fallback_icon());
    TrayIconBuilder::with_id("tray")
        .icon(icon)
        .tooltip("Kimi Planbar Tray")
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                // MouseUp semantics (not click): more reliable for tray icons
                TrayIconEvent::Click {
                    button,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } => match button {
                    MouseButton::Left => panel::toggle_panel(app, true),
                    MouseButton::Right => panel::show_menu(app, position.x, position.y),
                    _ => {}
                },
                // Hover-to-refresh, throttled at 10s (SPEC 5)
                TrayIconEvent::Enter { .. } | TrayIconEvent::Move { .. } => on_hover(app),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

fn on_hover(app: &AppHandle) {
    let st = app.state::<AppState>();
    {
        let mut last = st.last_hover.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(10) {
                return;
            }
        }
        *last = Some(Instant::now());
    }
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        polling::safe_refresh(&app2).await;
    });
}

/// Tooltip: "Kimi Planbar Tray  5h X% · week Y%" (+ "（更新失败）" on error).
pub fn update_tooltip(app: &AppHandle) {
    let st = app.state::<AppState>();
    let last: Option<QuotaResult> = st.last_quota.read().unwrap().clone();
    let text = match &last {
        None => "Kimi Planbar Tray".to_string(),
        Some(l) => {
            let mut t = format!(
                "Kimi Planbar Tray  5h {} · week {}",
                pct(&l.five_hour),
                pct(&l.week)
            );
            if l.error.is_some() {
                t.push_str("（更新失败）");
            }
            t
        }
    };
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_tooltip(Some(text));
    }
}

fn pct(seg: &Option<QuotaSegment>) -> String {
    match seg {
        Some(s) => format!("{}%", s.percent.round() as i64),
        None => "?".to_string(),
    }
}
