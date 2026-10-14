// Phase 0 feasibility spike for the "egui edition" of Kimi Planbar Tray.
//
// Validates on a real Windows 11 machine:
//   - 4 frameless / transparent / always-on-top / skip-taskbar egui viewports
//     (root = main panel 424x520, settings 404x464, skills 404x520, menu 188x160)
//   - tray-icon crate: left-click toggle, right-click menu-at-cursor, hover events
//   - panel/menu positioning ports of rust/src-tauri/src/panel.rs
//     (SPI_GETWORKAREA, MonitorFromPoint + GetDpiForMonitor, edge clamp / flip)
//   - HWND post-processing via FindWindowW: DWM donotround + WS_EX_TOOLWINDOW
//   - focus-change logging (SPEC 15 auto-hide feasibility), drag-to-move (settings)
//   - CJK font fallback chain msyh.ttc -> simhei.ttf -> simsun.ttc
//   - RSS + idle CPU sampling
//
// Console subsystem on purpose (no #![windows_subsystem]) so println! works
// from both Git Bash pipes and terminals.
//
// CLI:
//   --renderer glow|wgpu   render backend (default glow)
//   --duration <secs>      auto-exit timer, 0 = run until killed (default 60)
//   --show-main            show the main panel 3s after start (auto-hides after --hold)
//   --show-all             show all 4 windows 5s after start (auto-hides after --hold)
//   --hold <secs>          how long --show-main/--show-all windows stay up (default 8)
//   --autohide             SPEC 15 focus-loss auto-hide of the main panel
//
// stdin commands (same code paths as the real tray handlers; used to test
// menu clamping at arbitrary cursor coordinates that cannot be clicked by hand):
//   toggle | show | hide | menu <physX> <physY> | showall | quit

use std::io::BufRead;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::viewport::{ViewportBuilder, ViewportCommand, ViewportId};

// ---------------------------------------------------------------------------
// SPEC constants (docs/SPEC.md sections 10 / 11, light "Moonlit" theme)
// ---------------------------------------------------------------------------

const COL_WINDOW_BG: egui::Color32 = egui::Color32::from_rgb(0xF3, 0xF4, 0xF6);
const COL_TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0x1F, 0x23, 0x29);
const COL_TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(0x6B, 0x72, 0x80);
const COL_TRACK: egui::Color32 = egui::Color32::from_rgb(0xE5, 0xE7, 0xEB);
const COL_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x88, 0xFF);
const COL_DARK_CARD: egui::Color32 = egui::Color32::from_rgb(0x23, 0x26, 0x2D);
const COL_DARK_TEXT: egui::Color32 = egui::Color32::from_rgb(0xF2, 0xF3, 0xF5);

// Logical window sizes (rust/ edition values, shadow fade room included).
const MAIN_W: f32 = 424.0;
const MAIN_H: f32 = 520.0;
const SETTINGS_W: f32 = 404.0;
const SETTINGS_H: f32 = 464.0;
const SKILLS_W: f32 = 404.0;
const SKILLS_H: f32 = 520.0;
const MENU_W: f32 = 188.0;
const MENU_H: f32 = 160.0; // spike fixed height; the real edition computes it per items
const CARD_INSET: f32 = 22.0; // shadow fade room around main/settings/skills cards
const MENU_CARD_INSET: f32 = 19.0;

// Unique window titles double as FindWindowW keys.
const TITLE_MAIN: &str = "KPT-spike-main";
const TITLE_SETTINGS: &str = "KPT-spike-settings";
const TITLE_SKILLS: &str = "KPT-spike-skills";
const TITLE_MENU: &str = "KPT-spike-menu";

// Which font faces to probe, in order. All are system paths (not personal).
const FONT_CANDIDATES: [(&str, &str); 3] = [
    ("msyh", "C:\\Windows\\Fonts\\msyh.ttc"),
    ("simhei", "C:\\Windows\\Fonts\\simhei.ttf"),
    ("simsun", "C:\\Windows\\Fonts\\simsun.ttc"),
];

// The CJK line painted into every card (tests CJK glyphs + U+00A5 + em dash).
const CJK_LINE: &str = "\u{52A0}\u{6CB9}\u{5305}\u{4F59}\u{989D} \u{A5}12.34 \u{2014} \u{4E94}\u{5C0F}\u{65F6}\u{7528}\u{91CF} 42%";

fn vp_main() -> ViewportId {
    ViewportId::ROOT
}
fn vp_settings() -> ViewportId {
    ViewportId::from_hash_of("settings")
}
fn vp_skills() -> ViewportId {
    ViewportId::from_hash_of("skills")
}
fn vp_menu() -> ViewportId {
    ViewportId::from_hash_of("menu")
}

// ---------------------------------------------------------------------------
// Shared state between the egui UI thread(s) and the tray/timeline/stdin threads
// ---------------------------------------------------------------------------

const N_WIN: usize = 4; // main, settings, skills, menu

#[derive(Default)]
struct Shared {
    visible: [bool; N_WIN],
    focus_last: [Option<bool>; N_WIN],
    dpi_logged: [bool; N_WIN],
    // Tray click -> first root repaint latency measurement.
    click_ts: Option<Instant>,
    latency_reported: bool,
}

type SharedArc = Arc<Mutex<Shared>>;

static CTX: OnceLock<egui::Context> = OnceLock::new();

fn wait_ctx() -> egui::Context {
    loop {
        if let Some(ctx) = CTX.get() {
            return ctx.clone();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

struct Args {
    renderer: String,
    duration_secs: u64,
    show_main: bool,
    show_all: bool,
    hold_secs: u64,
    show_delay: Option<u64>,
    autohide: bool,
}

impl Args {
    fn parse() -> Args {
        let mut a = Args {
            renderer: "glow".into(),
            duration_secs: 60,
            show_main: false,
            show_all: false,
            hold_secs: 8,
            show_delay: None,
            autohide: false,
        };
        let mut it = std::env::args().skip(1);
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--renderer" => {
                    if let Some(v) = it.next() {
                        a.renderer = v;
                    }
                }
                "--duration" => {
                    if let Some(v) = it.next() {
                        a.duration_secs = v.parse().unwrap_or(60);
                    }
                }
                "--show-main" => a.show_main = true,
                "--show-all" => a.show_all = true,
                "--hold" => {
                    if let Some(v) = it.next() {
                        a.hold_secs = v.parse().unwrap_or(8);
                    }
                }
                "--show-delay" => {
                    if let Some(v) = it.next() {
                        a.show_delay = v.parse().ok();
                    }
                }
                "--autohide" => a.autohide = true,
                other => println!("[ARGS] ignored: {}", other),
            }
        }
        a
    }

    fn clone_for_thread(&self) -> Args {
        Args {
            renderer: self.renderer.clone(),
            duration_secs: self.duration_secs,
            show_main: self.show_main,
            show_all: self.show_all,
            hold_secs: self.hold_secs,
            show_delay: self.show_delay,
            autohide: self.autohide,
        }
    }
}

// ---------------------------------------------------------------------------
// Win32 helpers (1:1 ports of the calls in rust/src-tauri/src/panel.rs)
// ---------------------------------------------------------------------------

/// Primary-monitor work area in physical px (SPI_GETWORKAREA; panel.rs).
fn work_area_phys() -> (f64, f64, f64, f64) {
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
/// (MonitorFromPoint + GetDpiForMonitor; panel.rs show_menu).
fn dpi_scale_at(px: f64, py: f64) -> f64 {
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

/// Primary monitor scale, read from the monitor at screen origin (panel.rs uses
/// the tauri primary_monitor(); a per-monitor-aware process can read it this way).
fn primary_scale_phys() -> f64 {
    dpi_scale_at(0.0, 0.0)
}

/// FindWindowW by exact window title (each viewport has a unique one).
fn find_hwnd(title: &str) -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        FindWindowW(PCWSTR::null(), PCWSTR::from_raw(wide.as_ptr()))
            .ok()
            .filter(|h: &HWND| !h.is_invalid())
    }
}

/// Per-window HWND post-processing (SPEC 10 + Win11 corners):
/// (a) DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND,
/// (b) add WS_EX_TOOLWINDOW (skip taskbar).
fn style_hwnd(title: &str) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DWM_WINDOW_CORNER_PREFERENCE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };
    let Some(hwnd) = find_hwnd(title) else {
        println!("[HWND] {} not found (not shown yet?)", title);
        return;
    };
    unsafe {
        let pref: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND;
        let r_dwm = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as isize);
        let ex2 = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        println!(
            "[HWND] {} hwnd={:?} donotround={} toolwindow={} exstyle {:#x}->{:#x}",
            title,
            hwnd,
            r_dwm.is_ok(),
            ex2 & WS_EX_TOOLWINDOW.0 as isize != 0,
            ex,
            ex2
        );
    }
}

/// Current process CPU time (kernel+user) in 100ns units, for idle-CPU sampling.
fn process_cpu_ticks() -> u64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    unsafe {
        let (mut c, mut e, mut k, mut u) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        if GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u).is_err() {
            return 0;
        }
        let f = |t: &FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
        f(&k) + f(&u)
    }
}

/// Current RSS (working set) in bytes.
fn process_rss_bytes() -> usize {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut pmc = PROCESS_MEMORY_COUNTERS::default();
        let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, cb).is_ok() {
            pmc.WorkingSetSize
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Tray icon: programmatic 32x32 blue ball, ported from rust/src-tauri/src/tray.rs
// (circle (1,1,30,30) #1A88FF + highlight ellipse white@90)
// ---------------------------------------------------------------------------

fn fallback_icon_rgba() -> Vec<u8> {
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
    px
}

// ---------------------------------------------------------------------------
// Positioning ports (panel.rs show_panel / show_menu)
// ---------------------------------------------------------------------------

/// panel.rs show_panel: bottom-right of the work area, 12px card margin, the
/// position shift compensated by the 22px card inset. Sends the OuterPosition
/// as LOGICAL points (egui-winit multiplies by the window's pixels_per_point,
/// which equals the primary scale for a window on the primary monitor).
fn position_main_panel(ctx: &egui::Context) {
    let s = primary_scale_phys();
    let (_, _, right, bottom) = work_area_phys();
    let phys_x = right - (MAIN_W as f64 + 12.0 - 22.0) * s;
    let phys_y = bottom - (MAIN_H as f64 + 12.0 - 22.0) * s;
    ctx.send_viewport_cmd_to(
        vp_main(),
        ViewportCommand::OuterPosition(egui::pos2((phys_x / s) as f32, (phys_y / s) as f32)),
    );
    println!(
        "[POS] main panel: physical=({:.0},{:.0}) logical=({:.1},{:.1}) scale={}",
        phys_x,
        phys_y,
        phys_x / s,
        phys_y / s,
        s
    );
    write_windows_json();
}

/// panel.rs show_menu: physical cursor px -> cursor-monitor DIP, clamp
/// horizontally, flip up near the bottom edge; position first, then show.
fn position_menu_at(ctx: &egui::Context, px: f64, py: f64) {
    let scale = dpi_scale_at(px, py);
    let s_prim = primary_scale_phys();
    let (_, _, wa_right, wa_bottom) = work_area_phys();
    let wa_right_dip = wa_right / s_prim;
    let wa_bottom_dip = wa_bottom / s_prim;
    let cx = px / scale;
    let cy = py / scale;
    // panel.rs legacy math: the menu card is MENU_W-38 x MENU_H-38 and the
    // window origin shifts by -19px on both axes so the visible card lands at
    // the same screen spot as the old 5px-margin layout.
    let legacy_w = MENU_W as f64 - 38.0;
    let legacy_h = MENU_H as f64 - 38.0;
    let left = cx.min(wa_right_dip - legacy_w - 8.0) - 19.0;
    let top = if cy + legacy_h + 24.0 > wa_bottom_dip {
        cy - legacy_h - 8.0
    } else {
        cy + 8.0
    } - 19.0;
    ctx.send_viewport_cmd_to(
        vp_menu(),
        ViewportCommand::OuterPosition(egui::pos2(left as f32, top as f32)),
    );
    println!(
        "[POS] menu: cursor phys=({:.0},{:.0}) monitor_scale={} work_right_dip={:.0} -> window logical=({:.1},{:.1}) {}x{}",
        px, py, scale, wa_right_dip, left, top, MENU_W, MENU_H
    );
}

/// --show-all helper: settings centered on the work area, skills to the left
/// of the main panel, so both are on screen at once for screenshots.
fn position_secondary_windows(ctx: &egui::Context) {
    let s = primary_scale_phys();
    let (l, t, r, b) = work_area_phys();
    let settings_x = (l + r) / 2.0 - SETTINGS_W as f64 * s / 2.0;
    let settings_y = (t + b) / 2.0 - SETTINGS_H as f64 * s / 2.0;
    let main_x = r - (MAIN_W as f64 + 12.0) * s;
    let main_y = b - (MAIN_H as f64 + 12.0) * s;
    let skills_x = main_x - SKILLS_W as f64 * s - 24.0 * s;
    ctx.send_viewport_cmd_to(
        vp_settings(),
        ViewportCommand::OuterPosition(egui::pos2(
            (settings_x / s) as f32,
            (settings_y / s) as f32,
        )),
    );
    ctx.send_viewport_cmd_to(
        vp_skills(),
        ViewportCommand::OuterPosition(egui::pos2((skills_x / s) as f32, (main_y / s) as f32)),
    );
    println!(
        "[POS] settings physical=({:.0},{:.0}) skills physical=({:.0},{:.0})",
        settings_x, settings_y, skills_x, main_y
    );
}

/// Physical rects of the windows for the screenshot/sampling scripts. The menu
/// rect is written as -1 (it moves per request; sampled via logs instead).
fn write_windows_json() {
    let s = primary_scale_phys();
    let (l, t, r, b) = work_area_phys();
    let main_x = r - (MAIN_W as f64 + 12.0) * s;
    let main_y = b - (MAIN_H as f64 + 12.0) * s;
    let settings_x = (l + r) / 2.0 - SETTINGS_W as f64 * s / 2.0;
    let settings_y = (t + b) / 2.0 - SETTINGS_H as f64 * s / 2.0;
    let skills_x = main_x - SKILLS_W as f64 * s - 24.0 * s;
    let windows = [
        win_json("main", main_x, main_y, MAIN_W, MAIN_H, s),
        win_json(
            "settings", settings_x, settings_y, SETTINGS_W, SETTINGS_H, s,
        ),
        win_json("skills", skills_x, main_y, SKILLS_W, SKILLS_H, s),
        win_json("menu", -1.0, -1.0, MENU_W, MENU_H, s),
    ]
    .join(",");
    let json = format!(
        "{{\"scale\":{s},\"windows\":{{{windows}}},\"work_area\":{{\"right\":{r},\"bottom\":{b}}}}}\n"
    );
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("spike-windows.json");
            let _ = std::fs::write(&p, json);
        }
    }
}

fn win_json(name: &str, x: f64, y: f64, w: f32, h: f32, s: f64) -> String {
    format!(
        "\"{name}\":{{\"x\":{:.0},\"y\":{:.0},\"w\":{:.0},\"h\":{:.0}}}",
        x,
        y,
        w as f64 * s,
        h as f64 * s
    )
}

// ---------------------------------------------------------------------------
// Tray event thread (real tray events) + stdin command thread (test driver)
// ---------------------------------------------------------------------------

fn toggle_main(ctx: &egui::Context, shared: &SharedArc, from_click: bool) {
    let show = {
        let mut s = shared.lock().unwrap();
        let show = !s.visible[0];
        if from_click {
            s.click_ts = Some(Instant::now());
            s.latency_reported = false;
        }
        s.visible[0] = show;
        show
    };
    if show {
        position_main_panel(ctx);
    }
    ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(show));
    ctx.request_repaint();
    println!(
        "[TRAY] toggle main -> {}",
        if show { "show" } else { "hide" }
    );
}

fn show_menu_at(ctx: &egui::Context, shared: &SharedArc, px: f64, py: f64) {
    position_menu_at(ctx, px, py);
    {
        let mut s = shared.lock().unwrap();
        s.visible[3] = true;
    }
    ctx.send_viewport_cmd_to(vp_menu(), ViewportCommand::Visible(true));
    ctx.request_repaint();
    // panel.rs: steal the foreground AFTER show, else the menu never gets focus.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(120));
        if let Some(hwnd) = find_hwnd(TITLE_MENU) {
            use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
            let ok = unsafe { SetForegroundWindow(hwnd) };
            println!("[HWND] menu SetForegroundWindow ok={}", ok.as_bool());
        }
    });
}

fn hide_all(ctx: &egui::Context, shared: &SharedArc) {
    let mut any = false;
    {
        let mut s = shared.lock().unwrap();
        for v in s.visible.iter_mut() {
            if *v {
                *v = false;
                any = true;
            }
        }
    }
    if any {
        for vp in [vp_main(), vp_settings(), vp_skills(), vp_menu()] {
            ctx.send_viewport_cmd_to(vp, ViewportCommand::Visible(false));
        }
        ctx.request_repaint();
        println!("[STATE] hid all windows");
    }
}

fn show_all(ctx: &egui::Context, shared: &SharedArc) {
    position_main_panel(ctx);
    position_secondary_windows(ctx);
    {
        let mut s = shared.lock().unwrap();
        for v in s.visible.iter_mut() {
            *v = true;
        }
    }
    for vp in [vp_main(), vp_settings(), vp_skills(), vp_menu()] {
        ctx.send_viewport_cmd_to(vp, ViewportCommand::Visible(true));
    }
    ctx.request_repaint();
    println!("[STATE] showed all windows");
}

fn tray_event_thread(shared: SharedArc) {
    let ctx = wait_ctx();
    let receiver = tray_icon::TrayIconEvent::receiver();
    let mut last_hover: Option<Instant> = None;
    loop {
        match receiver.recv() {
            Ok(event) => {
                // Stamp the event time as early as possible for latency numbers.
                let now = Instant::now();
                match event {
                    tray_icon::TrayIconEvent::Click {
                        button,
                        button_state,
                        position,
                        ..
                    } => {
                        // MouseUp semantics, like the tauri tray in tray.rs.
                        if button_state == tray_icon::MouseButtonState::Up {
                            match button {
                                tray_icon::MouseButton::Left => {
                                    println!(
                                        "[TRAY] Click Left Up at ({:.0},{:.0}) t={:?}",
                                        position.x, position.y, now
                                    );
                                    toggle_main(&ctx, &shared, true);
                                }
                                tray_icon::MouseButton::Right => {
                                    println!(
                                        "[TRAY] Click Right Up at ({:.0},{:.0})",
                                        position.x, position.y
                                    );
                                    show_menu_at(&ctx, &shared, position.x, position.y);
                                }
                                _ => {}
                            }
                        }
                    }
                    tray_icon::TrayIconEvent::Enter { .. }
                    | tray_icon::TrayIconEvent::Move { .. } => {
                        // SPEC 14 hover-to-refresh, throttled at 10s.
                        let throttled = last_hover
                            .map(|t| t.elapsed() < Duration::from_secs(10))
                            .unwrap_or(false);
                        if !throttled {
                            println!("[TRAY] Enter/Move (hover) event at t={:?}", now);
                            last_hover = Some(now);
                        }
                        ctx.request_repaint();
                    }
                    tray_icon::TrayIconEvent::Leave { .. } => {
                        println!("[TRAY] Leave event at t={:?}", now);
                    }
                    _ => {}
                }
            }
            Err(err) => {
                println!("[TRAY] event channel closed: {:?}", err);
                break;
            }
        }
    }
}

fn stdin_thread(shared: SharedArc) {
    let ctx = wait_ctx();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        println!("[STDIN] command: {}", line);
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("toggle") => toggle_main(&ctx, &shared, true),
            Some("show") => {
                position_main_panel(&ctx);
                shared.lock().unwrap().visible[0] = true;
                ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(true));
                ctx.request_repaint();
            }
            Some("hide") => {
                shared.lock().unwrap().visible[0] = false;
                ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(false));
                ctx.request_repaint();
            }
            Some("menu") => {
                let px: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let py: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                show_menu_at(&ctx, &shared, px, py);
            }
            Some("showall") => show_all(&ctx, &shared),
            Some("quit") => {
                ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Close);
                break;
            }
            _ => println!("[STDIN] unknown command"),
        }
    }
}

fn timeline_thread(shared: SharedArc, args: &Args) {
    let ctx = wait_ctx();
    if args.show_main || args.show_all {
        let delay = args.show_delay.unwrap_or(if args.show_all { 5 } else { 3 });
        std::thread::sleep(Duration::from_secs(delay));
        if args.show_all {
            show_all(&ctx, &shared);
            println!("[TL] all windows shown");
        } else {
            position_main_panel(&ctx);
            shared.lock().unwrap().visible[0] = true;
            ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(true));
            ctx.request_repaint();
            println!("[TL] main panel shown");
        }
        // hold 0 = stay visible until the process ends (screenshot runs).
        if args.hold_secs > 0 {
            std::thread::sleep(Duration::from_secs(args.hold_secs));
            hide_all(&ctx, &shared);
            println!("[TL] windows hidden again; idle measurement starts");
        }
    }
    // Idle CPU over two 10s samples while all windows are hidden, then RSS.
    std::thread::sleep(Duration::from_secs(10));
    let cpu1 = process_cpu_ticks();
    let rss1 = process_rss_bytes();
    std::thread::sleep(Duration::from_secs(10));
    let cpu2 = process_cpu_ticks();
    let rss2 = process_rss_bytes();
    println!(
        "[MEM] RSS before={:.1} MB after={:.1} MB",
        rss1 as f64 / 1048576.0,
        rss2 as f64 / 1048576.0
    );
    println!(
        "[CPU] process CPU time delta over 10s hidden: {:.3} s ({:.1}% of one core)",
        (cpu2 - cpu1) as f64 / 1e7,
        (cpu2 - cpu1) as f64 / 1e7 / 10.0 * 100.0
    );
}

fn duration_thread(secs: u64) {
    if secs == 0 {
        return;
    }
    let ctx = wait_ctx();
    std::thread::sleep(Duration::from_secs(secs));
    println!("[EXIT] --duration {} reached, closing", secs);
    ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Close);
    // Fallback: the Close command should end the event loop; if anything hangs,
    // exit hard a few seconds later so automated runs never block.
    std::thread::sleep(Duration::from_secs(5));
    println!("[EXIT] event loop still alive, forcing exit");
    std::process::exit(0);
}

/// Watches for each viewport's native window to appear (after show) and applies
/// the DWM + TOOLWINDOW workarounds once per show.
fn hwnd_watcher_thread(shared: SharedArc) {
    let titles = [TITLE_MAIN, TITLE_SETTINGS, TITLE_SKILLS, TITLE_MENU];
    let mut styled = [false; N_WIN];
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let vis = {
            let s = shared.lock().unwrap();
            s.visible
        };
        for i in 0..N_WIN {
            if !vis[i] {
                styled[i] = false; // re-apply on next show (idempotent ops)
                continue;
            }
            if styled[i] {
                continue;
            }
            if find_hwnd(titles[i]).is_some() {
                style_hwnd(titles[i]);
                styled[i] = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Font probe state machine: install a candidate, then verify on the NEXT frame
/// (set_fonts rebuilds the font set at the start of the following begin_pass,
/// so a same-frame has_glyph check would still see the old built-in fonts).
enum FontProbe {
    Pending,
    Await { idx: usize, name: &'static str },
    Done,
}

/// Install candidate `idx` (if readable) into the font fallback chain.
fn install_font_candidate(ctx: &egui::Context, idx: usize) -> Option<&'static str> {
    let (name, path) = FONT_CANDIDATES.get(idx)?;
    match std::fs::read(path) {
        Ok(bytes) => {
            let len = bytes.len();
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert(name.to_string(), egui::FontData::from_owned(bytes).into());
            // Fallback AFTER the built-in proportional fonts.
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(name.to_string());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(name.to_string());
            ctx.set_fonts(fonts);
            println!(
                "[FONT] installed candidate {} ({}, {} bytes); verify next frame",
                name, path, len
            );
            Some(name)
        }
        Err(err) => {
            println!("[FONT] {} unavailable: {}", path, err);
            None
        }
    }
}

/// Verify CJK coverage of the currently-installed font set.
fn font_covers_cjk(ctx: &egui::Context) -> (bool, bool) {
    let jia = ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(14.0), '\u{52A0}'));
    let yen = ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(14.0), '\u{A5}'));
    (jia, yen)
}

/// Try candidates starting at `start`; log FAILURE when the list is exhausted.
fn try_next_font_candidate(ctx: &egui::Context, probe: &mut FontProbe, start: usize) {
    for idx in start..FONT_CANDIDATES.len() {
        if let Some(name) = install_font_candidate(ctx, idx) {
            *probe = FontProbe::Await { idx, name };
            return;
        }
    }
    println!(
        "[FONT] FAILURE: no CJK candidate covered the test glyphs; \
         tofu expected (mitigation: bundle a Noto Sans SC subset)"
    );
    *probe = FontProbe::Done;
}

// ---------------------------------------------------------------------------
// Card painting
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Which {
    Main,
    Settings,
    Skills,
    Menu,
}

impl Which {
    fn index(self) -> usize {
        match self {
            Which::Main => 0,
            Which::Settings => 1,
            Which::Skills => 2,
            Which::Menu => 3,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Which::Main => TITLE_MAIN,
            Which::Settings => TITLE_SETTINGS,
            Which::Skills => TITLE_SKILLS,
            Which::Menu => TITLE_MENU,
        }
    }
    fn inset(self) -> f32 {
        match self {
            Which::Menu => MENU_CARD_INSET,
            _ => CARD_INSET,
        }
    }
}

/// Paint the placeholder card: soft shadow + rounded WindowBg rect + sample
/// content (SPEC 10/11; pixel fidelity is explicitly not a spike goal).
fn paint_card(ui: &mut egui::Ui, which: Which, shared: &SharedArc) {
    let idx = which.index();
    let full = ui.max_rect();

    // Focus transitions (SPEC 15/20 feasibility) and DPI logging.
    let focused = ui.ctx().input(|i| i.viewport().focused);
    let native_ppp = ui
        .ctx()
        .input(|i| i.viewport().native_pixels_per_point)
        .unwrap_or(0.0);
    let outer = ui.ctx().input(|i| i.viewport().outer_rect);
    {
        let mut s = shared.lock().unwrap();
        if s.focus_last[idx] != focused {
            println!(
                "[FOCUS] {} focused={:?} at {:?}",
                which.title(),
                focused,
                Instant::now()
            );
            s.focus_last[idx] = focused;
        }
        if !s.dpi_logged[idx] {
            let phys = outer.map(|r| {
                (
                    (r.min.x * native_ppp).round(),
                    (r.min.y * native_ppp).round(),
                    (r.width() * native_ppp).round(),
                    (r.height() * native_ppp).round(),
                )
            });
            println!(
                "[DPI] {} native_ppp={} egui_ppp={:.3} outer_logical={:?} outer_physical={:?}",
                which.title(),
                native_ppp,
                ui.ctx().pixels_per_point(),
                outer,
                phys
            );
            s.dpi_logged[idx] = true;
        }
    }

    // SPEC 10.1 shadow: BlurRadius=24, Depth=2, Opacity=0.25 -> alpha 64.
    let card = full.shrink(which.inset());
    let shadow = egui::epaint::Shadow {
        offset: [0, 2],
        blur: 24,
        spread: 2,
        color: egui::Color32::from_black_alpha(64),
    };
    ui.painter().add(shadow.as_shape(card, 14));
    // The window background itself is the rounded WindowBg rect (SPEC 10:
    // transparent window + rounded inner border).
    ui.painter()
        .rect_filled(card, egui::CornerRadius::same(14), COL_WINDOW_BG);

    // Sample content.
    let pad = 16.0;
    let mut y = card.min.y + pad;
    ui.painter().text(
        egui::pos2(card.min.x + pad, y),
        egui::Align2::LEFT_TOP,
        "Kimi Planbar Tray",
        egui::FontId::proportional(17.0),
        COL_TEXT_PRIMARY,
    );
    y += 30.0;
    ui.painter().text(
        egui::pos2(card.min.x + pad, y),
        egui::Align2::LEFT_TOP,
        "42%",
        egui::FontId::proportional(30.0),
        COL_TEXT_PRIMARY,
    );
    y += 46.0;
    // Progress bar: track #e5e7eb h=6 r=3, fill accent at 42% (SPEC 11.2).
    let track = egui::Rect::from_min_size(
        egui::pos2(card.min.x + pad, y),
        egui::vec2(card.width() - 2.0 * pad, 6.0),
    );
    ui.painter()
        .rect_filled(track, egui::CornerRadius::same(3), COL_TRACK);
    let fill = egui::Rect::from_min_size(track.min, egui::vec2(track.width() * 0.42, 6.0));
    ui.painter()
        .rect_filled(fill, egui::CornerRadius::same(3), COL_ACCENT);
    y += 22.0;
    ui.painter().text(
        egui::pos2(card.min.x + pad, y),
        egui::Align2::LEFT_TOP,
        CJK_LINE,
        egui::FontId::proportional(13.0),
        COL_TEXT_SECONDARY,
    );
    y += 26.0;
    // Small dark-card sample (SPEC 11 dark values) inside the light card.
    let dark = egui::Rect::from_min_size(
        egui::pos2(card.min.x + pad, y),
        egui::vec2((card.width() - 2.0 * pad).min(220.0), 34.0),
    );
    ui.painter()
        .rect_filled(dark, egui::CornerRadius::same(10), COL_DARK_CARD);
    ui.painter().text(
        egui::pos2(dark.min.x + 10.0, dark.center().y),
        egui::Align2::LEFT_CENTER,
        "Dark card #23262d",
        egui::FontId::proportional(12.0),
        COL_DARK_TEXT,
    );

    // Window title tag at the card's bottom-right (helps identify screenshots).
    ui.painter().text(
        egui::pos2(card.max.x - 10.0, card.max.y - 8.0),
        egui::Align2::RIGHT_BOTTOM,
        which.title(),
        egui::FontId::proportional(10.0),
        COL_TEXT_SECONDARY,
    );
}

// ---------------------------------------------------------------------------
// eframe App
// ---------------------------------------------------------------------------

struct SpikeApp {
    shared: SharedArc,
    autohide: bool,
    first_frame: bool,
    font_probe: FontProbe,
}

impl SpikeApp {
    fn new(shared: SharedArc, autohide: bool) -> SpikeApp {
        SpikeApp {
            shared,
            autohide,
            first_frame: true,
            font_probe: FontProbe::Pending,
        }
    }
}

// eframe 0.36 App trait: the root-viewport callback receives a full-viewport Ui
// (run_ui), not a Context+CentralPanel pair like <= 0.35.
impl eframe::App for SpikeApp {
    // Transparent clear so the per-pixel-alpha window shows the desktop behind.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.first_frame {
            ctx.set_visuals(egui::Visuals::light());
            // eframe force-shows the root viewport after the first frame
            // (post_rendering); re-hide immediately (the alpha-0 frame shows
            // nothing, so there is no visible flash).
            ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(false));
            println!("[APP] first frame rendered; root re-hidden");
            self.first_frame = false;
        }

        // Font probe: verify the candidate installed on the previous frame
        // (fonts rebuild at begin_pass), else advance to the next candidate.
        match self.font_probe {
            FontProbe::Pending => {
                if let Some(name) = install_font_candidate(&ctx, 0) {
                    self.font_probe = FontProbe::Await { idx: 0, name };
                } else {
                    try_next_font_candidate(&ctx, &mut self.font_probe, 1);
                }
            }
            FontProbe::Await { idx, name } => {
                let (jia, yen) = font_covers_cjk(&ctx);
                if jia && yen {
                    println!(
                        "[FONT] SUCCESS: {} covers CJK + yen; installed as fallback",
                        name
                    );
                    self.font_probe = FontProbe::Done;
                } else {
                    println!(
                        "[FONT] {} verified jia={} yen={} - not covering, trying next",
                        name, jia, yen
                    );
                    try_next_font_candidate(&ctx, &mut self.font_probe, idx + 1);
                }
            }
            FontProbe::Done => {}
        }

        // Latency: first root paint at/after the recorded tray click.
        {
            let mut s = self.shared.lock().unwrap();
            if let (Some(click), false) = (s.click_ts, s.latency_reported) {
                println!(
                    "[TRAY] click -> first repaint latency: {:?}",
                    Instant::now() - click
                );
                s.latency_reported = true;
            }
        }

        // SPEC 15 auto-hide probe: main visible + focus lost + no settings/skills
        // window open -> hide (only with --autohide).
        if self.autohide {
            let (main_visible, settings_open, skills_open, main_focus) = {
                let s = self.shared.lock().unwrap();
                (s.visible[0], s.visible[1], s.visible[2], s.focus_last[0])
            };
            if main_visible && !settings_open && !skills_open && main_focus == Some(false) {
                let was = {
                    let mut s = self.shared.lock().unwrap();
                    let was = s.visible[0];
                    s.visible[0] = false;
                    was
                };
                if was {
                    ctx.send_viewport_cmd_to(vp_main(), ViewportCommand::Visible(false));
                    println!("[APP] auto-hide: main panel lost focus -> hidden");
                }
            }
        }

        // Root viewport = main panel (the App::ui root Ui covers the window).
        paint_card(ui, Which::Main, &self.shared);

        // Deferred viewports. Visibility is managed exclusively through
        // ViewportCommand::Visible so command order (position before show) is
        // deterministic; builders stay visible(false).
        let shared = self.shared.clone();
        ctx.show_viewport_deferred(
            vp_settings(),
            window_builder(TITLE_SETTINGS, egui::vec2(SETTINGS_W, SETTINGS_H)),
            move |ui, _class| {
                settings_ui(ui, &shared);
            },
        );
        let shared = self.shared.clone();
        ctx.show_viewport_deferred(
            vp_skills(),
            window_builder(TITLE_SKILLS, egui::vec2(SKILLS_W, SKILLS_H)),
            move |ui, _class| {
                paint_card(ui, Which::Skills, &shared);
            },
        );
        let shared = self.shared.clone();
        ctx.show_viewport_deferred(
            vp_menu(),
            window_builder(TITLE_MENU, egui::vec2(MENU_W, MENU_H)),
            move |ui, _class| {
                paint_card(ui, Which::Menu, &shared);
                // SPEC 10.3: menu closes on focus loss.
                let focused = ui.ctx().input(|i| i.viewport().focused);
                let visible = shared.lock().unwrap().visible[Which::Menu.index()];
                if visible && focused == Some(false) {
                    let mut s = shared.lock().unwrap();
                    if s.visible[Which::Menu.index()] {
                        s.visible[Which::Menu.index()] = false;
                        drop(s);
                        ui.ctx()
                            .send_viewport_cmd_to(vp_menu(), ViewportCommand::Visible(false));
                        println!("[APP] menu closed on focus loss");
                    }
                }
            },
        );

        // No continuous animation requested: egui sleeps until events or an
        // explicit request_repaint, which is what keeps idle CPU near zero.
    }
}

fn window_builder(title: &str, size: egui::Vec2) -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title(title)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_visible(false)
        .with_taskbar(false)
        .with_inner_size(size)
}

/// Settings viewport: card + drag title bar (SPEC 10.2 DragMove equivalent).
fn settings_ui(ui: &mut egui::Ui, shared: &SharedArc) {
    paint_card(ui, Which::Settings, shared);
    let card = ui.max_rect().shrink(Which::Settings.inset());
    let bar = egui::Rect::from_min_size(
        egui::pos2(card.min.x + 16.0, card.min.y + 12.0),
        egui::vec2(card.width() - 32.0, 30.0),
    );
    let resp = ui.interact(bar, egui::Id::new("settings-titlebar"), egui::Sense::drag());
    ui.painter().text(
        bar.left_center(),
        egui::Align2::LEFT_CENTER,
        "Settings (drag this bar)",
        egui::FontId::proportional(12.0),
        COL_TEXT_SECONDARY,
    );
    if resp.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        println!("[APP] settings StartDrag requested");
    }
}

// ---------------------------------------------------------------------------
// Version banner (parse Cargo.lock once at startup; no hardcoded drift risk)
// ---------------------------------------------------------------------------

fn dep_versions() -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock"))
    else {
        return vec![];
    };
    let wanted = [
        "eframe",
        "egui",
        "egui-winit",
        "egui_glow",
        "egui-wgpu",
        "tray-icon",
        "winit",
        "windows",
        "epaint",
    ];
    let mut out = vec![];
    let mut in_package = false;
    let mut name = String::new();
    let mut version = String::new();
    for line in text.lines() {
        let t = line.trim();
        if t == "[[package]]" {
            if in_package && wanted.contains(&name.as_str()) {
                out.push((name.clone(), version.clone()));
            }
            in_package = true;
            name.clear();
            version.clear();
        } else if in_package {
            if let Some(v) = t.strip_prefix("name = ") {
                name = v.trim().trim_matches('"').to_string();
            } else if let Some(v) = t.strip_prefix("version = ") {
                version = v.trim().trim_matches('"').to_string();
            } else if t.is_empty() {
                if wanted.contains(&name.as_str()) {
                    out.push((name.clone(), version.clone()));
                }
                in_package = false;
                name.clear();
                version.clear();
            }
        }
    }
    if in_package && wanted.contains(&name.as_str()) {
        out.push((name, version));
    }
    out
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = Args::parse();
    println!("=== Kimi Planbar Tray egui edition - Phase 0 spike ===");
    println!("[VER] crate {}", env!("CARGO_PKG_VERSION"));
    for (name, version) in dep_versions() {
        println!("[VER] {} {}", name, version);
    }
    println!(
        "[CFG] renderer={} duration={}s show_main={} show_all={} hold={}s autohide={}",
        args.renderer,
        args.duration_secs,
        args.show_main,
        args.show_all,
        args.hold_secs,
        args.autohide
    );

    let shared: SharedArc = Arc::new(Mutex::new(Shared::default()));

    // Threads before run_native; they block on the OnceLock until the egui
    // Context exists.
    type NamedThread = (&'static str, Box<dyn FnOnce() + Send>);
    let threads: [NamedThread; 5] = [
        (
            "tray-events",
            Box::new({
                let shared = shared.clone();
                move || tray_event_thread(shared)
            }) as Box<dyn FnOnce() + Send>,
        ),
        (
            "stdin",
            Box::new({
                let shared = shared.clone();
                move || stdin_thread(shared)
            }),
        ),
        (
            "timeline",
            Box::new({
                let shared = shared.clone();
                let args = args.clone_for_thread();
                move || timeline_thread(shared, &args)
            }),
        ),
        (
            "duration",
            Box::new(move || duration_thread(args.duration_secs)),
        ),
        (
            "hwnd-watcher",
            Box::new({
                let shared = shared.clone();
                move || hwnd_watcher_thread(shared)
            }),
        ),
    ];
    for (name, body) in threads {
        if let Err(err) = std::thread::Builder::new().name(name.into()).spawn(body) {
            println!("[THREAD] failed to spawn {}: {:?}", name, err);
        }
    }

    // Tray icon (created on the main thread; winit's message pump dispatches
    // the tray-icon message window because it pumps the whole thread queue).
    let icon =
        tray_icon::Icon::from_rgba(fallback_icon_rgba(), 32, 32).expect("build tray icon rgba");
    let tray = tray_icon::TrayIconBuilder::new()
        .with_id("kpt-spike")
        .with_icon(icon)
        .with_tooltip("Kimi Planbar Tray")
        .build()
        .expect("build tray icon");
    println!("[TRAY] icon built (32x32 blue ball fallback port)");

    let renderer = match args.renderer.as_str() {
        "wgpu" => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    };

    let options = eframe::NativeOptions {
        renderer,
        viewport: window_builder(TITLE_MAIN, egui::vec2(MAIN_W, MAIN_H)),
        ..Default::default()
    };

    let shared2 = shared.clone();
    let autohide = args.autohide;
    let _tray_guard = tray; // keep the tray icon alive for the process lifetime
    let result = eframe::run_native(
        TITLE_MAIN,
        options,
        Box::new(move |cc: &eframe::CreationContext<'_>| {
            let _ = CTX.set(cc.egui_ctx.clone());
            Ok(Box::new(SpikeApp::new(shared2, autohide)) as Box<dyn eframe::App>)
        }),
    );
    println!("[EXIT] eframe returned: {:?}", result);
}
