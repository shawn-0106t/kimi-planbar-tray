// Per-window HWND post-processing, ported from the Phase 0 spike (SPEC 10 +
// Win11 corners): (a) DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND —
// the panel paints its own 14px radius on a transparent window; (b) add
// WS_EX_TOOLWINDOW as a belt-and-braces skip-taskbar guarantee; (c) disable
// non-client rendering — the windows keep WS_CAPTION for winit's
// undecorated-shadow, and since Win11 26H2 (build 26300) DWM draws that
// caption as a full white title bar with a close button over the transparent
// content (earlier builds honored winit's WM_NCCALCSIZE collapse and showed
// at most a 1-2px hairline); (d) subclass the wndproc to fully collapse the
// non-client area — winit's undecorated-shadow hack (WM_NCCALCSIZE top+=1)
// leaves a 1px gray strip at the window's top edge, and the egui UI paints
// its own shadow anyway (SPEC 10), so the DWM shadow the hack preserves is
// redundant. (c) and (d) together remove the whole NC frame.
//
// A watcher thread polls find_hwnd (title + owning-PID match) for each
// viewport's unique title and styles each native window as soon as its HWND
// exists — not on first show — so even the very first show of a window is
// already frameless (the styling would otherwise lag the show by up to one
// poll interval and flash the native frame). It also re-styles on every
// hidden→visible transition: winit's apply_diff rewrites GWL_STYLE /
// GWL_EXSTYLE from its own flag set on every visibility flip (wiping our
// WS_EX_TOOLWINDOW), and an unowned visible window without WS_EX_TOOLWINDOW
// / WS_EX_APPWINDOW would show up on the taskbar. The DWM attributes and the
// wndproc subclass survive the rewrite; the re-apply is idempotent. A new
// HWND (viewport recreation) is detected and re-styled.

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

/// Original wndproc per subclassed HWND (the windows live for the whole
/// process lifetime — singletons that are only ever hidden — so the map is
/// write-once and never needs cleanup).
static OLD_WNDPROCS: std::sync::Mutex<Vec<(isize, isize)>> = std::sync::Mutex::new(Vec::new());

// TEMP trace instrumentation (KPT_TRACE): per-HWND message counters drained
// once per second by app.rs into scripts/diagnostics/debug_trace.log.
pub static TRACE_MOUSEMOVE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TRACE_LBUTTON: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TRACE_NCACTIVATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TRACE_ANY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Subclass proc: winit's undecorated-shadow hack answers WM_NCCALCSIZE with
/// `rgrc[0].top += 1`, leaving a 1px non-client band at the window's top edge
/// that DWM paints as a gray strip (visible on light desktops). The egui UI
/// paints its own shadow (SPEC 10), so the hack buys nothing — fully collapse
/// the non-client area instead. Client height is unchanged (winit also does
/// `bottom += 1`), the area just shifts up by 1px.
///
/// WM_NCPAINT is swallowed too: with DWM non-client rendering disabled
/// (DWMNCRP_DISABLED, style_hwnd), DefWindowProc falls back to GDI-painting
/// the caption ITSELF, and that paint lands in the GL surface as a one-frame
/// white title bar until the next present overwrites it (seen when the
/// settings window steals the panel's foreground).
///
/// WM_NCACTIVATE needs both sides of that coin. It must keep flowing to
/// winit — it is the ONLY message that sets winit's `is_active`, and without
/// it `has_active_focus` (is_active && is_focused, window_state.rs) never
/// turns true, so egui never sees Focused(true) and the focus-loss auto-hide
/// latch (panel_was_focused) breaks. But winit's handler ends with
/// `ProcResult::DefWindowProc` (event_loop.rs), and DefWindowProc answers
/// WM_NCACTIVATE by redrawing the caption directly (NOT via WM_NCPAINT, so
/// swallowing WM_NCPAINT does not stop it). The fix: strip WS_CAPTION for
/// just the duration of the forwarded call so DefWindowProc finds no caption
/// to draw, then restore it. No SWP_FRAMECHANGED is issued, so the temporary
/// style change triggers no non-client recalc and no visible effect; winit
/// ignores WM_STYLECHANGING/WM_STYLECHANGED, and its apply_diff rewrites
/// GWL_STYLE from its own flag set on the next visibility flip regardless.
/// Everything else forwards to the original winit proc.
unsafe extern "system" fn nc_collapse_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WM_NCACTIVATE,
        WM_NCCALCSIZE, WM_NCPAINT, WS_CAPTION,
    };
    if msg == WM_NCCALCSIZE && wparam.0 != 0 {
        // wparam=TRUE: rgrc[0] already holds the proposed (full) window rect;
        // returning 0 without touching it accepts it as the client area.
        return windows::Win32::Foundation::LRESULT(0);
    }
    if msg == WM_NCPAINT {
        return windows::Win32::Foundation::LRESULT(0);
    }
    let forward = |msg: u32,
                   wparam: windows::Win32::Foundation::WPARAM,
                   lparam: windows::Win32::Foundation::LPARAM| {
        let old = OLD_WNDPROCS
            .lock()
            .unwrap()
            .iter()
            .find(|(h, _)| *h == hwnd.0 as isize)
            .map(|(_, p)| *p);
        match old {
            Some(p) => unsafe {
                CallWindowProcW(
                    Some(std::mem::transmute::<
                        isize,
                        unsafe extern "system" fn(
                            HWND,
                            u32,
                            windows::Win32::Foundation::WPARAM,
                            windows::Win32::Foundation::LPARAM,
                        )
                            -> windows::Win32::Foundation::LRESULT,
                    >(p)),
                    hwnd,
                    msg,
                    wparam,
                    lparam,
                )
            },
            None => unsafe {
                windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
            },
        }
    };
    // TEMP trace counters (KPT_TRACE) — drained by app.rs once per 500ms.
    use std::sync::atomic::Ordering as AOrd;
    TRACE_ANY.fetch_add(1, AOrd::Relaxed);
    match msg {
        0x0200 => {
            TRACE_MOUSEMOVE.fetch_add(1, AOrd::Relaxed);
        }
        0x0201 => {
            TRACE_LBUTTON.fetch_add(1, AOrd::Relaxed);
        }
        0x0086 => {
            TRACE_NCACTIVATE.fetch_add(1, AOrd::Relaxed);
        }
        _ => {}
    }
    if msg == WM_NCACTIVATE {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_CAPTION.0 as isize));
            let result = forward(msg, wparam, lparam);
            SetWindowLongPtrW(hwnd, GWL_STYLE, style);
            return result;
        }
    }
    forward(msg, wparam, lparam)
}

/// Apply the DWM + TOOLWINDOW workarounds to the window with `title`.
/// All errors are silently swallowed (styling is cosmetic hardening).
fn style_hwnd(title: &str) {
    use windows::core::w;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetPropW, GetWindowLongPtrW, SetPropW, SetWindowLongPtrW, SetWindowPos, GWLP_WNDPROC,
        GWL_EXSTYLE, SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        WS_EX_TOOLWINDOW,
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
        // Keep the WS_CAPTION style but stop DWM from drawing the non-client
        // frame — on Win11 26H2 the caption renders as a visible white title
        // bar over the transparent content.
        let ncrp = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &ncrp as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&ncrp) as u32,
        );
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as isize);
        // Subclass once per HWND (marker property), then force a non-client
        // recalc so the client area returns to the full window rect.
        if GetPropW(hwnd, w!("KPT_NC_SUBCLASSED")).0.is_null() {
            let old = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            OLD_WNDPROCS.lock().unwrap().push((hwnd.0 as isize, old));
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, nc_collapse_proc as *const () as isize);
            let _ = SetPropW(
                hwnd,
                w!("KPT_NC_SUBCLASSED"),
                // The value just needs to be non-null; reuse the HWND.
                Some(windows::Win32::Foundation::HANDLE(hwnd.0)),
            );
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SET_WINDOW_POS_FLAGS(
                    SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOZORDER.0 | SWP_FRAMECHANGED.0,
                ),
            );
        }
    }
}

/// Spawn the watcher thread (one per process). Styles each window as soon as
/// its HWND exists (not on first show), so even the very first show is
/// frameless, and re-styles on every hidden→visible transition because
/// winit's apply_diff rewrites GWL_STYLE/GWL_EXSTYLE from its own flag set on
/// every visibility flip (wiping our WS_EX_TOOLWINDOW) and re-issues
/// WM_NCCALCSIZE via SWP_FRAMECHANGED. The DWM attributes and the wndproc
/// subclass survive; re-applying is idempotent.
pub fn spawn_watcher() {
    use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
    std::thread::Builder::new()
        .name("hwnd-watcher".into())
        .spawn(move || {
            let mut state: Vec<(Option<isize>, bool)> = vec![(None, false); WINDOW_TITLES.len()];
            loop {
                std::thread::sleep(Duration::from_millis(200));
                for (i, title) in WINDOW_TITLES.iter().enumerate() {
                    let hwnd = find_hwnd(title);
                    let key = hwnd.map(|h| h.0 as isize);
                    let visible = hwnd.is_some_and(|h| unsafe { IsWindowVisible(h).as_bool() });
                    let (styled_hwnd, was_visible) = state[i];
                    if key.is_some() && (key != styled_hwnd || (visible && !was_visible)) {
                        style_hwnd(title);
                        state[i].0 = key;
                    } else if key.is_none() {
                        state[i].0 = None; // HWND gone: re-style on recreation
                    }
                    state[i].1 = visible;
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
