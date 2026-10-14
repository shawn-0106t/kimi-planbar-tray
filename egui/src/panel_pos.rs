// Main-panel positioning, port of rust/src-tauri/src/panel.rs show_panel
// (SPEC 10.1 + 20): bottom-right of the primary work area with a 12px card
// margin, shifted by the 22px shadow-fade-room compensation so the card lands
// 18px off the corner.
//
// egui gotcha (Phase 0 spike): ViewportCommand::OuterPosition takes LOGICAL
// points — egui-winit multiplies by the window's pixels_per_point (the primary
// scale on a single-monitor setup). The target is computed in PHYSICAL pixels
// (SPI_GETWORKAREA) and divided back down; see docs/EGUI-MIGRATION.md section 2.

use egui::viewport::ViewportCommand;
use egui::Context;

/// Logical window size; includes the 28px shadow fade room on every side
/// (the visible card is 424-2*28 x 520-2*28 = 368x464, SPEC 10.1).
pub const PANEL_W: f32 = 424.0;
pub const PANEL_H: f32 = 520.0;
/// Shadow fade room per side (rust/ grew the window margin 6 -> 28).
pub const CARD_INSET: f32 = 28.0;

/// Primary-monitor work area in physical px (SPI_GETWORKAREA).
pub fn work_area_phys() -> (f64, f64, f64, f64) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
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
    (
        rect.left as f64,
        rect.top as f64,
        rect.right as f64,
        rect.bottom as f64,
    )
}

/// Scale of the monitor containing the given physical point
/// (MonitorFromPoint + GetDpiForMonitor).
pub fn dpi_scale_at(px: f64, py: f64) -> f64 {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
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
    scale
}

/// Primary monitor scale, read from the monitor at screen origin (panel.rs
/// uses the tauri primary_monitor(); a per-monitor-aware process reads it this
/// way — same as the Phase 0 spike).
pub fn primary_scale() -> f64 {
    dpi_scale_at(0.0, 0.0)
}

/// ShowNearTray: pin the logical size (DPI guard, SPEC 20), place the window
/// bottom-right, then show and focus — in that command order (viewport
/// commands execute in enqueue order).
pub fn show_panel(ctx: &Context) {
    let s = primary_scale();
    let (_, _, right, bottom) = work_area_phys();
    // +22 compensates the margin growth (6 -> 28, shadow fade room): the
    // window shifts 22px left/up so the visible card position is unchanged.
    let phys_x = right - (PANEL_W as f64 + 12.0 - 22.0) * s;
    let phys_y = bottom - (PANEL_H as f64 + 12.0 - 22.0) * s;
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(PANEL_W, PANEL_H)));
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
        (phys_x / s) as f32,
        (phys_y / s) as f32,
    )));
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    // The queued Focus command does not reliably take the foreground on Win11
    // (see windows.rs): back it up with the spike's delayed steal, retried a
    // few times since the shell grant window varies.
    crate::windows::spawn_focus_steal(crate::app::MAIN_WINDOW_TITLE, &[60, 200, 450]);
    write_window_rect_json();
}

/// Physical rect of the main window, for the local screenshot/sampling
/// harness. Written next to the exe (gitignored target/ dir); errors are
/// silently swallowed — this is diagnostics-only.
fn write_window_rect_json() {
    let s = primary_scale();
    let (_, _, right, bottom) = work_area_phys();
    // Same math as the OuterPosition above: the real window rect (physical).
    let x = right - (PANEL_W as f64 + 12.0 - 22.0) * s;
    let y = bottom - (PANEL_H as f64 + 12.0 - 22.0) * s;
    let json = format!(
        "{{\"scale\":{},\"main\":{{\"x\":{:.0},\"y\":{:.0},\"w\":{:.0},\"h\":{:.0}}}}}\n",
        s,
        x,
        y,
        PANEL_W as f64 * s,
        PANEL_H as f64 * s
    );
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::fs::write(dir.join("kpt-windows.json"), json);
        }
    }
}
