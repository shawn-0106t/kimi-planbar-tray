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
// Caption hardening (docs/EGUI-CAPTION-FLASH-FIX.md): with DWM NC rendering
// disabled, DefWindowProc falls back to GDI-painting the caption itself on
// WM_NCACTIVATE (and other themed-paint paths), and that paint lands in the
// window's redirection surface — the next present shows it as a one-frame
// white title bar. The subclass therefore (e) swallows the undocumented
// WM_NCUAHDRAWCAPTION/WM_NCUAHDRAWFRAME themed-paint requests and (f) strips
// WS_CAPTION again on every WM_STYLECHANGED (winit re-adds the bit on each
// visibility flip), so DefWindowProc never has a caption to draw no matter
// which message it comes in through. The WM_NCACTIVATE temporary strip stays
// as the belt-and-braces for the first activation after subclassing.
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

/// Re-arm the winit pointer state after a programmatic show. egui-winit only
/// learns the pointer position from `CursorMoved` events, and drops all
/// mouse-button input while none is known (`on_mouse_button_input` bails when
/// `pointer_pos_in_points` is None). Hiding a window under the cursor fires
/// WM_MOUSELEAVE (egui pointer -> None) but winit keeps `mouse.last_position`
/// across the hide/show cycle, and a later cursor move that lands on that
/// exact stale pixel emits no `CursorMoved` (position unchanged) — so a
/// re-shown window is click-dead until the cursor moves to a NEW position
/// over it. Post two synthetic WM_MOUSEMOVEs carrying the current cursor
/// position (the second nudged 1px to break any tie with the stale pixel) so
/// `last_position` is refreshed on every show; real input then flows again.
pub fn spawn_pointer_rearm(title: &'static str) {
    std::thread::Builder::new()
        .name("pointer-rearm".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            let Some(hwnd) = find_hwnd(title) else {
                return;
            };
            use windows::Win32::Foundation::POINT;
            use windows::Win32::Graphics::Gdi::ScreenToClient;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetCursorPos, PostMessageW, WM_MOUSEMOVE,
            };
            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt) }.is_err() {
                return;
            }
            let mut client = pt;
            let _ = unsafe { ScreenToClient(hwnd, &mut client) };
            let pack = |x: i32, y: i32| {
                windows::Win32::Foundation::LPARAM(
                    (((y as u16 as u32) << 16) | (x as u16 as u32)) as isize,
                )
            };
            // Two moves: exact position, then +1px — one of them always
            // differs from any stale last_position.
            for (x, y) in [(client.x, client.y), (client.x + 1, client.y + 1)] {
                let _ = unsafe {
                    PostMessageW(
                        Some(hwnd),
                        WM_MOUSEMOVE,
                        windows::Win32::Foundation::WPARAM(0),
                        pack(x, y),
                    )
                };
            }
        })
        .ok(); // a failed spawn only costs the re-arm
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

/// Undocumented themed-NC paint requests (UxTheme internals): some caption /
/// frame drawing flows through these instead of WM_NCPAINT. Swallowing them
/// has no side effect and closes one more GDI-caption draw path.
const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
const WM_NCUAHDRAWFRAME: u32 = 0x00AF;

/// Re-assert NC suppression on `hwnd` while the window is not (yet) visible:
/// DWM must keep rendering no non-client area (on Win11 26H2 it paints the
/// retained WS_CAPTION as a full native title bar) and the caption style bit
/// is stripped so DefWindowProc has no caption to draw. No SWP_FRAMECHANGED
/// is issued — the subclass already collapses the whole NC area and the
/// window geometry never depends on the caption bit; skipping the recalc
/// avoids repaint churn.
fn apply_nc_suppression(hwnd: HWND) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_CAPTION,
    };
    unsafe {
        let ncrp = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &ncrp as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&ncrp) as u32,
        );
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if style & (WS_CAPTION.0 as isize) != 0 {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_CAPTION.0 as isize));
        }
    }
}

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
///
/// On top of that (caption hardening, docs/EGUI-CAPTION-FLASH-FIX.md): the
/// undocumented WM_NCUAHDRAWCAPTION/WM_NCUAHDRAWFRAME themed-paint requests
/// are swallowed, and WM_STYLECHANGED (winit re-adding WS_CAPTION on every
/// visibility flip) strips the caption bit again immediately, so no caption
/// can be painted from any message path. The WM_NCACTIVATE strip stays as
/// the guard for the very first activation right after subclassing.
/// Everything else forwards to the original winit proc.
unsafe extern "system" fn nc_collapse_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, STYLESTRUCT,
        WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WM_SHOWWINDOW, WM_STYLECHANGED, WS_CAPTION,
    };
    if msg == WM_NCCALCSIZE && wparam.0 != 0 {
        // wparam=TRUE: rgrc[0] already holds the proposed (full) window rect;
        // returning 0 without touching it accepts it as the client area.
        return windows::Win32::Foundation::LRESULT(0);
    }
    if msg == WM_NCPAINT {
        return windows::Win32::Foundation::LRESULT(0);
    }
    if msg == WM_NCUAHDRAWCAPTION || msg == WM_NCUAHDRAWFRAME {
        // Themed caption/frame paint requests (see the constants above):
        // swallow so no GDI caption can land in the redirection surface.
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
    if msg == WM_SHOWWINDOW && wparam.0 != 0 {
        // Being shown (sent BEFORE the window becomes visible): re-assert the
        // DWM NC-suppression attribute and the caption-free style. This only
        // reaches windows that ALREADY carry our subclass — it covers the
        // case where an outside force reset the DWM attribute or re-added
        // the caption bit between styling and this show (winit's apply_diff,
        // a theme change); a window that missed the CBT hook AND the watcher
        // has no subclass yet and cannot run this branch (that residual risk
        // is what the A′ fast-startup watcher covers when the CBT hook could
        // not be installed at all). winit does not handle this message — it
        // forwards to DefWindowProc, which draws nothing once there is no
        // caption bit. Falls through to the normal forward.
        apply_nc_suppression(hwnd);
    }
    if msg == WM_STYLECHANGED && wparam.0 == GWL_STYLE.0 as usize && lparam.0 != 0 {
        // winit re-adds WS_CAPTION on every visibility flip (apply_diff
        // rewrites GWL_STYLE from its own flag set); strip it again right
        // here so the window never carries a caption bit while visible —
        // DefWindowProc then has no caption to paint from ANY message path
        // (WM_NCACTIVATE, WM_SETTEXT, theme changes, ...). Recursion is
        // self-limiting: the nested WM_STYLECHANGED from this write already
        // sees a caption-free styleNew. winit ignores WM_STYLECHANGING /
        // WM_STYLECHANGED, so there is no fight; no SWP_FRAMECHANGED is
        // issued (the NC area is fully collapsed — a recalc would only add
        // repaint churn).
        let ss = &mut *(lparam.0 as *mut STYLESTRUCT);
        if ss.styleNew & WS_CAPTION.0 != 0 {
            // Rewrite the snapshot we forward, too: the real style no longer
            // has the caption, so the forwarded message should not describe
            // one (winit ignores this message today; keep the data honest
            // for any future version that does not).
            ss.styleNew &= !WS_CAPTION.0;
            let cur = GetWindowLongPtrW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWL_STYLE, cur & !(WS_CAPTION.0 as isize));
        }
        // fall through: the (possibly rewritten) message still forwards.
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

/// Apply the DWM + TOOLWINDOW + NC-collapse workarounds to one native
/// window. `during_creation` marks the CBT `HCBT_CREATEWND` context and skips
/// the SWP_FRAMECHANGED: the subclass is installed right here, so the
/// window's FIRST WM_NCCALCSIZE (which arrives later inside CreateWindowEx)
/// is answered by this proc with the full-window client rect — issuing a
/// frame recalc now would only pump messages into a half-created window
/// (winit's wndproc still runs its `userdata == 0` DefWindowProc fallback
/// before WM_NCCREATE). All errors are silently swallowed (styling is
/// cosmetic hardening).
fn style_hwnd_hwnd(hwnd: HWND, during_creation: bool) {
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
            if !during_creation {
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
}

/// Apply the DWM + TOOLWINDOW workarounds to the window with `title`
/// (post-creation paths: the watcher, `App::new`, the three show paths).
/// All errors are silently swallowed (styling is cosmetic hardening).
pub fn style_hwnd(title: &str) {
    let Some(hwnd) = find_hwnd(title) else {
        return;
    };
    style_hwnd_hwnd(hwnd, false);
}

/// Handle of the thread-level WH_CBT hook. Never unhooked: the hook must
/// cover window creation for the whole process lifetime, and the hook proc
/// is trivial for non-matching windows.
static CBT_HOOK: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Does this (NUL-terminated-by-callback) window name match one of our four
/// known viewport titles? `PCWSTR::as_wide` scans up to the NUL, so compare
/// against the title's UTF-16 units without a terminator. The wide table is
/// cached (the hook runs on every window creation on this thread).
fn title_matches_any(name: windows::core::PCWSTR) -> bool {
    if name.0.is_null() {
        return false;
    }
    static WIDE_TITLES: std::sync::OnceLock<Vec<Vec<u16>>> = std::sync::OnceLock::new();
    let table = WIDE_TITLES.get_or_init(|| {
        WINDOW_TITLES
            .iter()
            .map(|t| t.encode_utf16().collect::<Vec<u16>>())
            .collect()
    });
    let wide = unsafe { name.as_wide() };
    table.iter().any(|units| wide == units.as_slice())
}

/// CBT hook proc: fires synchronously inside CreateWindowEx (main thread) for
/// every window this thread creates. For our four viewport titles this is the
/// only point that is guaranteed to run BEFORE the window can ever be shown,
/// closing the first-show race (P1: HWND created and shown within one 200ms
/// watcher tick painted a full native caption on Win11 26H2). Styling here is
/// safe: the HWND exists (created, not yet announced), DWM attributes and
/// style/proc changes apply, and the subclass intercepts the window's very
/// first WM_NCCALCSIZE so no SWP_FRAMECHANGED is needed (see
/// style_hwnd_hwnd). Non-matching windows (tray message window, IME, ...)
/// fall through at zero cost.
unsafe extern "system" fn cbt_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CBT_CREATEWNDW, HCBT_CREATEWND};
    if code as u32 == HCBT_CREATEWND {
        let cbt = &*(lparam.0 as *const CBT_CREATEWNDW);
        if !cbt.lpcs.is_null() {
            let name = (*cbt.lpcs).lpszName;
            if title_matches_any(name) {
                let hwnd = HWND(wparam.0 as *mut core::ffi::c_void);
                style_hwnd_hwnd(hwnd, true);
            }
        }
    }
    windows::Win32::UI::WindowsAndMessaging::CallNextHookEx(None, code, wparam, lparam)
}

/// Install the thread-level WH_CBT hook. MUST be called on the thread that
/// creates the windows (the main / winit event-loop thread — eframe builds
/// every viewport window on it, glow_integration.rs handle_viewport_output)
/// and BEFORE `eframe::run_native`. A thread hook only intercepts windows
/// created by its own thread; installed anywhere else it is silently useless.
/// The handle is kept in a static and never unhooked (process lifetime).
/// Returns `true` when the hook is installed; `false` means the first-show
/// guarantee is lost and the caller should fall back to A′ (see
/// `spawn_fast_startup_watcher`).
pub fn install_cbt_hook() -> bool {
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_CBT};
    let tid = unsafe { GetCurrentThreadId() };
    let hook = unsafe { SetWindowsHookExW(WH_CBT, Some(cbt_proc), None, tid) };
    match hook {
        Ok(h) => {
            CBT_HOOK.store(h.0 as isize, std::sync::atomic::Ordering::SeqCst);
            true
        }
        // A failed hook only costs the first-show guarantee; the caller
        // engages the A′ fallback and the regular watcher stays (errors are
        // not popped up — error-handling baseline).
        Err(_) => false,
    }
}

/// A′ fallback (docs/EGUI-CAPTION-FLASH-FIX.md §3.1): when the CBT hook could
/// not be installed, poll the four titles at 10ms for the first 3 seconds so
/// a window created AND shown in that window is styled before its first
/// paint. Not a hard guarantee (a show can still land between polls), but it
/// shrinks the race by ~20x vs the 200ms watcher, which takes over after 3s.
/// Styling is idempotent, and windows that do not exist yet are a cheap
/// find_hwnd miss.
pub fn spawn_fast_startup_watcher() {
    std::thread::Builder::new()
        .name("hwnd-fast-startup".into())
        .spawn(move || {
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs(3) {
                for title in WINDOW_TITLES {
                    style_hwnd(title);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
        .ok(); // a failed spawn only costs the styling hardening
}

/// Spawn the watcher thread (one per process). Safety net for styling that
/// the CBT hook / show-path calls cannot guarantee: winit's apply_diff
/// rewrites GWL_STYLE/GWL_EXSTYLE from its own flag set on every visibility
/// flip (wiping our WS_EX_TOOLWINDOW) and re-issues WM_NCCALCSIZE via
/// SWP_FRAMECHANGED, so each window is re-styled as soon as its HWND exists
/// and on every hidden→visible transition. The DWM attributes and the
/// wndproc subclass survive; re-applying is idempotent. (Caption
/// suppression itself no longer depends on this polling: the CBT hook
/// styles at creation and the subclass strips WS_CAPTION on every
/// WM_STYLECHANGED — this thread only keeps WS_EX_TOOLWINDOW reapplied and
/// catches anything exotic, e.g. a re-created HWND on a foreign thread.)
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
