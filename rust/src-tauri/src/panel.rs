// Panel + menu window positioning and focus behavior (UI-SPEC sections 1, 5, 6).
// The slide/fade animation itself runs in frontend CSS (SPEC 6); the backend
// only emits panel-show / panel-hide and hides the window when told to.

use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};

// Window sizes include the transparent shadow fade room around the card:
// main/settings grew by 2*22px (margin 6 -> 28), menu by 2*19px (margin 5 -> 24).
const PANEL_W: f64 = 424.0;
const PANEL_H: f64 = 512.0;
const MENU_W: f64 = 188.0;

/// Primary-monitor work area in DIP (matches WPF SystemParameters.WorkArea).
fn work_area_dip(app: &AppHandle) -> (f64, f64, f64, f64) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SPI_GETWORKAREA,
    };
    let mut rect = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    let scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let s = if scale > 0.0 { scale } else { 1.0 };
    (
        rect.left as f64 / s,
        rect.top as f64 / s,
        rect.right as f64 / s,
        rect.bottom as f64 / s,
    )
}

/// ShowNearTray: bottom-right of the work area, 12px margin (SPEC 1.1).
pub fn show_panel(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    // Defensive re-size: tao can inflate a shown window's logical size when
    // the display scale changes (WM_DPICHANGED computed with a stale scale
    // factor). Pinning the size on every show keeps the panel at 424x512.
    let _ = w.set_size(LogicalSize::new(PANEL_W, PANEL_H));
    let (_, _, right, bottom) = work_area_dip(app);
    // +22 compensates the margin growth (6 -> 28, shadow fade room): the window
    // shifts 22px left/up so the card's visible position is unchanged (card
    // right/bottom edges still sit 18px off the work-area corner).
    let _ = w.set_position(LogicalPosition::new(
        right - PANEL_W - 12.0 + 22.0,
        bottom - PANEL_H - 12.0 + 22.0,
    ));
    app.state::<AppState>()
        .panel_hiding
        .store(false, Ordering::SeqCst);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit("panel-show", ());
}

/// Ask the frontend to play the slide-out animation (re-entry guarded).
pub fn start_hide(app: &AppHandle) {
    let st = app.state::<AppState>();
    if st.panel_hiding.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = app.emit("panel-hide", ());
}

/// Called by the frontend after the fade-out finished: actually hide + stamp
/// last_hide for the 300ms tray-click guard (SPEC 5 / 6.2).
/// Hard guard: if a panel-show happened after the hide started, the pending
/// finish_hide is stale and must not hide the freshly shown window.
pub fn finish_hide(app: &AppHandle) {
    let st = app.state::<AppState>();
    if !st.panel_hiding.swap(false, Ordering::SeqCst) {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    *st.last_hide.lock().unwrap() = Some(Instant::now());
}

/// TogglePopup: visible -> hide, hidden -> show. Tray left-clicks within 300ms
/// of an auto-hide are ignored (same click caused the focus-loss hide first).
pub fn toggle_panel(app: &AppHandle, from_tray: bool) {
    if from_tray {
        let last_hide = {
            let st = app.state::<AppState>();
            let v = *st.last_hide.lock().unwrap();
            v
        };
        if let Some(t) = last_hide {
            if t.elapsed() < Duration::from_millis(300) {
                return;
            }
        }
    }
    let visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    if visible {
        start_hide(app);
    } else {
        show_panel(app);
    }
}

/// Deactivated auto-hide, suppressed while the settings window is open (SPEC 11).
pub fn on_main_blur(app: &AppHandle) {
    let st = app.state::<AppState>();
    if st.settings_open.load(Ordering::SeqCst) {
        return;
    }
    let visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    if visible {
        start_hide(app);
    }
}

/// ShowAtCursor (SPEC 1.3): physical cursor px -> DIP, clamp horizontally,
/// flip up near the bottom edge, then steal the foreground so the menu
/// actually receives focus (otherwise Deactivated closes it at once).
pub fn show_menu(app: &AppHandle, px: f64, py: f64) {
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

    let Some(w) = app.get_webview_window("menu") else {
        return;
    };
    let hwnd: Option<HWND> = w
        .hwnd()
        .ok()
        .map(|h| HWND(h.0 as *mut core::ffi::c_void));
    // Mixed-DPI multi-monitor: the px->DIP conversion must use the scale of
    // the monitor UNDER THE CURSOR, not the menu window's (hidden windows sit
    // on the primary monitor, so GetDpiForWindow would use the wrong scale).
    let mut scale = 1.0f64;
    unsafe {
        let mon = MonitorFromPoint(
            POINT {
                x: px as i32,
                y: py as i32,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        let (mut dx, mut dy) = (0u32, 0u32);
        if GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
            let s = dx as f64 / 96.0;
            if s > 0.0 {
                scale = s;
            }
        }
    }
    let cx = px / scale;
    let cy = py / scale;
    let (_, _, wa_right, wa_bottom) = work_area_dip(app);
    let h = {
        let st = app.state::<AppState>();
        let v = *st.menu_height.lock().unwrap();
        v
    };
    // The menu window gained 19px of transparent margin per side (5 -> 24,
    // shadow fade room) and the frontend now reports height + 2*24 instead of
    // + 2*5. Compute the position exactly as the old 150px/5px-margin layout
    // did (legacy_w / legacy_h), then shift the window origin -19px on both
    // axes so the visible card lands on the same screen spot as before,
    // including the bottom-edge flip-up branch.
    let legacy_w = MENU_W - 38.0; // 150: window grew 2*19px
    let legacy_h = h - 38.0; // reported height grew by 2*19px as well
    let left = cx.min(wa_right - legacy_w - 8.0) - 19.0;
    let top = if cy + legacy_h + 24.0 > wa_bottom {
        cy - legacy_h - 8.0
    } else {
        cy + 8.0
    } - 19.0;
    // SizeToContent equivalent: the frontend-reported height (content + 2*24px
    // shadow fade room) is the real window height; the conf height is only the
    // initial guess. Apply it so the card never overflows the window edge.
    let _ = w.set_size(LogicalSize::new(MENU_W, h));
    let _ = w.set_position(LogicalPosition::new(left, top));
    let _ = w.show();
    let _ = w.set_focus();
    if let Some(h) = hwnd {
        unsafe {
            let _ = SetForegroundWindow(h);
        }
    }
}
