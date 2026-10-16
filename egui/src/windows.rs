// Per-window HWND post-processing, ported from the Phase 0 spike (SPEC 10 +
// Win11 corners): (a) DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND —
// the panel paints its own 14px radius on a transparent window; (b) add
// WS_EX_TOOLWINDOW as a belt-and-braces skip-taskbar guarantee.
//
// A watcher thread polls find_hwnd (title + owning-PID match) for each
// viewport's unique title while
// that viewport is flagged visible, and styles each native window once per
// show (the ops are idempotent; the HWND persists across hide/show).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::Win32::Foundation::HWND;

/// Steal the foreground shortly after a programmatic show. After a tray click
/// the shell grants foreground rights to the tray-icon owner for a short
/// window; a direct SetForegroundWindow in that window takes it (the Phase 0
/// spike validated this path for the menu window). The queued egui
/// ViewportCommand::Focus alone does not reliably land on Win11 (observed in
/// Phase 2: the panel stayed focused=false after a real tray click), and the
/// grant window is timing-dependent, so retry a few times.
pub fn spawn_focus_steal(title: &'static str, delays_ms: &[u64]) {
    let delays: Vec<u64> = delays_ms.to_vec();
    std::thread::Builder::new()
        .name("focus-steal".into())
        .spawn(move || {
            for delay in delays {
                std::thread::sleep(Duration::from_millis(delay));
                if let Some(hwnd) = find_hwnd(title) {
                    let ok = unsafe {
                        windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd)
                    };
                    if ok.as_bool() {
                        return; // got the foreground
                    }
                }
            }
        })
        .ok(); // a failed spawn only costs the focus fix
}

use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextLengthW, GetWindowTextW};

/// Unique window titles double as find_hwnd keys. All four viewports are
/// registered (the panel is the root; Phase 3 adds settings/skills/menu).
pub const MAIN_TITLE: &str = "Kimi Planbar Tray Main Panel";
pub const SETTINGS_TITLE: &str = "Kimi Planbar Tray Settings";
pub const SKILLS_TITLE: &str = "Kimi Skills";
pub const MENU_TITLE: &str = "Kimi Planbar Tray Menu";

pub const WINDOW_TITLES: &[&str] = &[MAIN_TITLE, SETTINGS_TITLE, SKILLS_TITLE, MENU_TITLE];

/// Shared visibility flags, one per entry of WINDOW_TITLES, maintained by the
/// panel state machine in app.rs.
pub type VisibilityFlags = Arc<Mutex<Vec<bool>>>;

pub fn new_visibility_flags() -> VisibilityFlags {
    Arc::new(Mutex::new(vec![false; WINDOW_TITLES.len()]))
}

/// Find our own top-level window by unique title. The PID filter matters
/// because --test-* runs WITHOUT the single-instance mutex (SPEC 19): while a
/// GUI instance is live, another edition's same-titled window (the four
/// titles are shared across editions) would otherwise be styled /
/// foregrounded / DPI-queried by mistake. EnumWindows (not FindWindowW, which
/// returns only the first same-titled hit in Z order) so a foreign window
/// sorting earlier does not hide our own: keep enumerating until the title
/// AND the owning PID both match.
pub fn find_hwnd(title: &str) -> Option<HWND> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    struct State<'a> {
        title: &'a [u16], // including the trailing NUL
        pid: u32,
        hit: &'a mut Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut State);
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == state.pid {
            let len = unsafe { GetWindowTextLengthW(hwnd) };
            if len as usize + 1 == state.title.len() {
                let mut buf = vec![0u16; len as usize + 1];
                let copied = unsafe { GetWindowTextW(hwnd, &mut buf) } as usize;
                if copied + 1 == state.title.len() && buf == state.title {
                    *state.hit = Some(hwnd);
                    return BOOL::from(false); // found — stop enumerating
                }
            }
        }
        BOOL::from(true)
    }

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hit = None;
    let pid = unsafe { GetCurrentProcessId() };
    let mut state = State {
        title: &wide,
        pid,
        hit: &mut hit,
    };
    // EnumWindows yields Err when the callback stops early — that is the
    // success case here. All failures are silently swallowed (callers treat
    // a missing HWND as a no-op).
    let _ = unsafe {
        EnumWindows(
            Some(enum_proc),
            LPARAM(&mut state as *mut State as *mut _ as isize),
        )
    };
    hit
}

/// Apply the DWM + TOOLWINDOW workarounds to the window with `title`.
/// All errors are silently swallowed (styling is cosmetic hardening).
fn style_hwnd(title: &str) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };
    let Some(hwnd) = find_hwnd(title) else {
        return;
    };
    unsafe {
        let pref = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&pref) as u32,
        );
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as isize);
    }
}

/// Spawn the watcher thread (one per process); watches the visibility flags
/// maintained by the panel state machine.
pub fn spawn_watcher(visible: VisibilityFlags) {
    std::thread::Builder::new()
        .name("hwnd-watcher".into())
        .spawn(move || {
            let mut styled = vec![false; WINDOW_TITLES.len()];
            loop {
                std::thread::sleep(Duration::from_millis(200));
                let vis = visible.lock().unwrap().clone();
                for (i, title) in WINDOW_TITLES.iter().enumerate() {
                    let Some(v) = vis.get(i) else { continue };
                    if !*v {
                        styled[i] = false; // re-apply on next show (idempotent ops)
                        continue;
                    }
                    if styled[i] {
                        continue;
                    }
                    if find_hwnd(title).is_some() {
                        style_hwnd(title);
                        styled[i] = true;
                    }
                }
            }
        })
        .ok(); // a failed spawn only costs the styling hardening
}

/// Open a URL with the default browser (ShellExecuteW). Errors silently
/// swallowed (SPEC 12.6 / 12.7 baseline).
pub fn open_url(url: &str) {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let target = HSTRING::from(url);
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &target,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
