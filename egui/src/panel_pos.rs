// Main-panel + secondary-window positioning, ports of rust/src-tauri/src/
// panel.rs show_panel / show_menu and lib.rs open_settings / open_skills
// (SPEC 10.1 / 10.2 / 10.3 + 20): bottom-right of the primary work area for
// the panel, CenterScreen for settings/skills, cursor-at with edge clamp +
// bottom flip for the tray menu.
//
// egui gotcha (Phase 0 spike): ViewportCommand::OuterPosition takes LOGICAL
// points — egui-winit multiplies by the window's pixels_per_point. On a
// single-scale setup that is the primary scale, so dividing the physical
// target by the primary scale used to be exact; on mixed-DPI multi-monitor
// the window's scale follows the monitor the WINDOW sits on (see
// window_dpi_scale) and every positioning entry point now converts through
// GetDpiForWindow of its own native window — the same physical landing as
// rust/'s PhysicalPosition calls. See docs/archive/EGUI-MIGRATION.md section 2.

use egui::viewport::{ViewportCommand, ViewportId};
use egui::{Context, Vec2};

use std::sync::{LazyLock, Mutex};
/// Logical window sizes; the panel/settings/skills include the 28px shadow
/// fade room on every side (visible cards 368x464 / 348x408 / 348x464, SPEC
/// 10.1 / 10.2 / 21.1). The menu keeps the rust/ 188px width and 24px inset.
pub const PANEL_W: f32 = 424.0;
pub const PANEL_H: f32 = 520.0;
/// Shadow fade room per side (rust/ grew the window margin 6 -> 28).
pub const CARD_INSET: f32 = 28.0;
pub const SETTINGS_W: f32 = 404.0;
pub const SETTINGS_H: f32 = 464.0;
pub const SKILLS_W: f32 = 404.0;
pub const SKILLS_H: f32 = 520.0;
/// Window width, rust/ conf (the WPF menu was 150 + 2*19px margin growth).
pub const MENU_W: f32 = 188.0;
/// Shadow fade room per side (margin 5 -> 24 in rust/).
pub const MENU_INSET: f32 = 24.0;
/// Builder height (rust/ conf guess); the real height is computed per open.
pub const MENU_H_INIT: f32 = 160.0;

/// Viewport ids of the secondary windows (root = the main panel).
pub fn vp_settings() -> ViewportId {
    ViewportId::from_hash_of("settings")
}
pub fn vp_skills() -> ViewportId {
    ViewportId::from_hash_of("skills")
}
pub fn vp_menu() -> ViewportId {
    ViewportId::from_hash_of("menu")
}

// ---- Diagnostics json (local screenshot harness; gitignored target/) -------

/// Whether record_rect may write kpt-windows.json next to the exe. Opt-in
/// only (--test-ui or KPT_WRITE_RECTS=1, set once from main.rs run_app): a
/// normal user must never get the diagnostic file, and a read-only exe
/// directory would swallow the write attempt on every window open.
static RECTS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_rects_enabled(on: bool) {
    RECTS_ENABLED.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[derive(Clone, Copy)]
pub struct WinRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Default)]
struct WinRects {
    main: Option<WinRect>,
    settings: Option<WinRect>,
    skills: Option<WinRect>,
    menu: Option<WinRect>,
}

static RECTS: LazyLock<Mutex<WinRects>> = LazyLock::new(|| Mutex::new(WinRects::default()));

fn record_rect(key: &str, rect: WinRect) {
    if !RECTS_ENABLED.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if let Ok(mut r) = RECTS.lock() {
        match key {
            "main" => r.main = Some(rect),
            "settings" => r.settings = Some(rect),
            "skills" => r.skills = Some(rect),
            "menu" => r.menu = Some(rect),
            _ => {}
        }
        write_rects_json(&r);
    }
}

fn write_rects_json(rects: &WinRects) {
    let s = primary_scale();
    let entry = |r: Option<WinRect>| match r {
        None => "-1".to_string(),
        Some(r) => format!(
            "{{\"x\":{:.0},\"y\":{:.0},\"w\":{:.0},\"h\":{:.0}}}",
            r.x, r.y, r.w, r.h
        ),
    };
    let json = format!(
        "{{\"scale\":{},\"main\":{},\"settings\":{},\"skills\":{},\"menu\":{}}}\n",
        s,
        entry(rects.main),
        entry(rects.settings),
        entry(rects.skills),
        entry(rects.menu),
    );
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::fs::write(dir.join("kpt-windows.json"), json);
        }
    }
}

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

/// Current DPI scale of the viewport's native window (GetDpiForWindow).
///
/// `ViewportCommand::OuterPosition` takes LOGICAL points and egui-winit
/// multiplies them by the window's own pixels_per_point — the scale of the
/// monitor the window is currently on, NOT the monitor being positioned for.
/// Mixed-DPI multi-monitor therefore needs the physical target divided by the
/// WINDOW's scale (the SPEC 20 launcher-monitor trap family; rust/ sidesteps
/// it with `PhysicalPosition`). `def` covers the window not existing yet:
/// before the first frame there is no HWND, so which monitor the window will
/// land on is unknowable (CW_USEDEFAULT opens on the LAUNCHER's monitor, not
/// necessarily the primary) and the primary scale is only a guess — a
/// --test-ui first-open started from a different-DPI monitor will be off by
/// the scale ratio (known-untested, checklist U-2). Normal user flows open
/// windows long after their HWNDs exist, so the conversion stays exact.
fn window_dpi_scale(title: &str, def: f64) -> f64 {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    let Some(hwnd) = crate::windows::find_hwnd(title) else {
        return def;
    };
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        def
    } else {
        dpi as f64 / 96.0
    }
}

/// ShowNearTray: pin the logical size (DPI guard, SPEC 20), place the window
/// bottom-right, then show and focus — in that command order (viewport
/// commands execute in enqueue order). `focus` skips the Focus command and
/// the delayed foreground steal for --test-ui (the rust run_ui_test only
/// calls show(), it never activates; SPEC 19).
pub fn show_panel(ctx: &Context, focus: bool) {
    let s = primary_scale();
    let (_, _, right, bottom) = work_area_phys();
    // +22 compensates the margin growth (6 -> 28, shadow fade room): the
    // window shifts 22px left/up so the visible card position is unchanged.
    let phys_x = right - (PANEL_W as f64 + 12.0 - 22.0) * s;
    let phys_y = bottom - (PANEL_H as f64 + 12.0 - 22.0) * s;
    let win_s = window_dpi_scale(crate::app::MAIN_WINDOW_TITLE, s);
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(PANEL_W, PANEL_H)));
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
        (phys_x / win_s) as f32,
        (phys_y / win_s) as f32,
    )));
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    if focus {
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        // The queued Focus command does not reliably take the foreground on
        // Win11 (see windows.rs): back it up with the spike's delayed steal,
        // retried a few times since the shell grant window varies.
        crate::windows::spawn_focus_steal(crate::app::MAIN_WINDOW_TITLE, &[60, 200, 450]);
    }
    record_rect(
        "main",
        WinRect {
            x: phys_x,
            y: phys_y,
            w: PANEL_W as f64 * s,
            h: PANEL_H as f64 * s,
        },
    );
}

/// CenterScreen (SPEC 10.2 / 21.1): centered on the primary work area, size
/// pinned on every open (DPI guard, same as show_panel). `focus` steals the
/// foreground with the usual delayed retry; --test-ui passes false (the rust
/// run_ui_test only calls show(), it never activates). `recenter` matches the
/// singleton semantics the other editions have: the WPF window centers at
/// construction (WindowStartupLocation) and rust/ at conf `center: true` —
/// both only ONCE; later opens show the window where it was (a user drag
/// survives). So the first open centers, later opens only re-pin the size.
pub fn show_centered(ctx: &Context, vp: ViewportId, w: f32, h: f32, focus: bool, recenter: bool) {
    let s = primary_scale();
    let (left, top, right, bottom) = work_area_phys();
    let phys_x = (left + right) / 2.0 - w as f64 * s / 2.0;
    let phys_y = (top + bottom) / 2.0 - h as f64 * s / 2.0;
    ctx.send_viewport_cmd_to(vp, ViewportCommand::InnerSize(egui::vec2(w, h)));
    if recenter {
        // The window may have been dragged to another monitor (its DPI
        // follows); divide by the window's own scale so the physical landing
        // is exact. Only the recenter branch needs the HWND query.
        let title = if vp == vp_settings() {
            crate::windows::SETTINGS_TITLE
        } else {
            crate::windows::SKILLS_TITLE
        };
        let win_s = window_dpi_scale(title, s);
        ctx.send_viewport_cmd_to(
            vp,
            ViewportCommand::OuterPosition(egui::pos2(
                (phys_x / win_s) as f32,
                (phys_y / win_s) as f32,
            )),
        );
    }
    ctx.send_viewport_cmd_to(vp, ViewportCommand::Visible(true));
    if focus {
        ctx.send_viewport_cmd_to(vp, ViewportCommand::Focus);
    }
    if recenter {
        record_rect(
            if vp == vp_settings() {
                "settings"
            } else {
                "skills"
            },
            WinRect {
                x: phys_x,
                y: phys_y,
                w: w as f64 * s,
                h: h as f64 * s,
            },
        );
    }
}

/// Content height of the menu window (WPF SizeToContent equivalent, SPEC
/// 10.3): 5 items + card padding 4 + 2*24px shadow room, measured from the
/// egui font set — the rust/ edition's frontend-reported height becomes a
/// direct measurement.
pub fn menu_window_height(ctx: &Context) -> f32 {
    let item_h = menu_item_height(ctx);
    let card = 4.0 + item_h * 5.0 + 4.0;
    card + 2.0 * MENU_INSET
}

/// One MenuItemButton: padding 14,9 around the 13px text line.
pub fn menu_item_height(ctx: &Context) -> f32 {
    let text_h = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            "Refresh".to_string(),
            crate::ui::theme::font(13.0),
            egui::Color32::WHITE,
        )
        .size()
        .y
    });
    9.0 + text_h + 9.0
}

/// Pure ShowAtCursor math (SPEC 10.3, the rust/ panel.rs legacy layout):
/// physical cursor px -> cursor-monitor DIP, clamp horizontally against the
/// work-area right edge, flip up near the bottom edge; both axes then shift by
/// -19px so the visible card lands where the old 150px/5px-margin layout put
/// it. Returns the logical window origin. Unit-tested against hand-computed
/// cases (see tests below).
fn menu_position(
    px: f64,
    py: f64,
    cursor_scale: f64,
    wa_right_dip: f64,
    wa_bottom_dip: f64,
    window_h: f32,
) -> (f64, f64) {
    let cx = px / cursor_scale;
    let cy = py / cursor_scale;
    // The menu window gained 19px of transparent margin per side (5 -> 24,
    // shadow fade room). Compute the position exactly as the old 150px /
    // 5px-margin layout did (legacy_w / legacy_h), then shift the window
    // origin -19px on both axes so the visible card lands on the same screen
    // spot as before, including the bottom-edge flip-up branch.
    let legacy_w = MENU_W as f64 - 38.0; // 150: window grew 2*19px
    let legacy_h = window_h as f64 - 38.0; // the height grew by 2*19px as well
    let left = cx.min(wa_right_dip - legacy_w - 8.0) - 19.0;
    let top = if cy + legacy_h + 24.0 > wa_bottom_dip {
        cy - legacy_h - 8.0
    } else {
        cy + 8.0
    } - 19.0;
    (left, top)
}

/// ShowAtCursor (SPEC 10.3), a port of rust panel.rs show_menu: measure the
/// cursor monitor, position (clamp + flip), then size -> position -> show in
/// that command order. `h` is the full window height (menu_window_height).
pub fn show_menu_at(ctx: &Context, px: f64, py: f64, h: f32) {
    let s_prim = primary_scale();
    let (_, _, wa_right, wa_bottom) = work_area_phys();
    let wa_right_dip = wa_right / s_prim;
    let wa_bottom_dip = wa_bottom / s_prim;
    let scale = dpi_scale_at(px, py);
    let (left, top) = menu_position(px, py, scale, wa_right_dip, wa_bottom_dip, h);
    // rust/ lands the menu at PHYSICAL (left*scale, top*scale)
    // (set_position(PhysicalPosition)); convert to the logical points
    // OuterPosition expects with the MENU WINDOW's own scale (mixed-DPI:
    // the window may still sit on the primary while the cursor is on a
    // 125% secondary — the raw DIP would land scaled by the wrong monitor).
    let phys_x = left * scale;
    let phys_y = top * scale;
    let win_s = window_dpi_scale(crate::windows::MENU_TITLE, s_prim);
    // SizeToContent equivalent: apply the measured height so the card never
    // overflows the window edge. Commands run in enqueue order: size,
    // position, then show (the builder stays visible(false)).
    let vp = vp_menu();
    ctx.send_viewport_cmd_to(vp, ViewportCommand::InnerSize(Vec2::new(MENU_W, h)));
    ctx.send_viewport_cmd_to(
        vp,
        ViewportCommand::OuterPosition(egui::pos2(
            (phys_x / win_s) as f32,
            (phys_y / win_s) as f32,
        )),
    );
    ctx.send_viewport_cmd_to(vp, ViewportCommand::Visible(true));
    record_rect(
        "menu",
        WinRect {
            x: phys_x,
            y: phys_y,
            // Sizes follow the cursor monitor's scale — same monitor the
            // x/y origins are physical values of (json stays one frame of
            // reference).
            w: MENU_W as f64 * scale,
            h: h as f64 * scale,
        },
    );
}

/// Current cursor position in physical px (rust/ uses tauri cursor_position;
/// the tray events carry the same coordinates).
pub fn cursor_pos_phys() -> (f64, f64) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    let _ = unsafe { GetCursorPos(&mut p) };
    (p.x as f64, p.y as f64)
}

#[cfg(test)]
mod tests {
    use super::menu_position;

    /// Synthetic 225% single-monitor numbers (1280x800 DIP, taskbar 48 ->
    /// work area bottom 752). `h` = window height incl. 2*24 shadow room.
    const SCALE: f64 = 2.25;
    const WA_RIGHT: f64 = 1280.0;
    const WA_BOTTOM: f64 = 752.0;

    fn pos(px: f64, py: f64, h: f32) -> (f64, f64) {
        menu_position(px, py, SCALE, WA_RIGHT, WA_BOTTOM, h)
    }

    #[test]
    fn menu_mid_screen_no_clamp_no_flip() {
        // 1440 phys = 640 DIP, well away from every edge.
        let (left, top) = pos(1440.0, 400.0, 229.0);
        assert!((left - 621.0).abs() < 0.05, "{left}");
        assert!((top - 166.78).abs() < 0.05, "{top}"); // 400/2.25 + 8 - 19
    }

    #[test]
    fn menu_right_edge_clamped() {
        // 2870 phys = 1275.6 DIP: clamps to wa_right - 150 - 8 = 1122, then
        // -19 -> 1103 (the Phase 0 spike measured exactly this value).
        let (left, _) = pos(2870.0, 400.0, 160.0);
        assert!((left - 1103.0).abs() < 0.05, "{left}");
    }

    #[test]
    fn menu_bottom_edge_flips_up() {
        // 1690 phys = 751.1 DIP with h=160 (legacy 122): 751.1+122+24 > 752
        // -> top = 751.1 - 122 - 8 - 19 = 602.1 (spike-measured value).
        let (_, top) = pos(2870.0, 1690.0, 160.0);
        assert!((top - 602.1).abs() < 0.05, "{top}");
    }

    #[test]
    fn menu_flip_boundary_is_strict() {
        // Exactly at the threshold (cy + legacy_h + 24 == wa_bottom): no flip,
        // the WPF branch is a strict greater-than.
        let cy = WA_BOTTOM - (160.0 - 38.0) - 24.0; // 606
        let (_, top) = pos(720.0 * SCALE, cy * SCALE, 160.0);
        assert!((top - (cy + 8.0 - 19.0)).abs() < 0.05, "{top}");
    }

    #[test]
    fn menu_top_left_matches_spike_sample() {
        // (200,200) phys -> (69.9, 77.9): the spike's second documented case.
        let (left, top) = pos(200.0, 200.0, 160.0);
        assert!((left - 69.9).abs() < 0.05, "{left}");
        assert!((top - 77.9).abs() < 0.05, "{top}");
    }
}
