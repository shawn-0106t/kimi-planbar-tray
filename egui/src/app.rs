// The eframe application: owns the panel state machine, consumes the AppEvent
// bus and cross-thread UI commands, and drives the SPEC 15 animations. The
// root viewport is the main panel; the settings / skills / tray-menu windows
// (Phase 3) are deferred viewports registered every pass.
//
// Two execution realms have to be kept apart (eframe runs one egui pass per
// viewport):
//   * the ROOT pass runs App::ui — the panel state machine, the UiCmd /
//     WinAction queues, theme sync, viewport (re-)registration;
//   * each child viewport runs only its registered callback — app.rs does NOT
//     run. Child interactions therefore push WinAction values onto the shared
//     WinBus queue and wake the root with request_repaint_of(ROOT).
//
// Business data is read from the shared AppState each frame (the Phase 1 core
// keeps it current); the tray icon lives here because tray_icon::TrayIcon is
// not Send and must stay on the main thread.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::viewport::{ViewportBuilder, ViewportCommand, ViewportId};

use crate::ctx::AppCtx;
use crate::icons::Icons;
use crate::ui::panel::{self, PanelState};
use crate::ui::settings_win::SettingsDraft;
use crate::ui::theme::{FontProbe, Theme};
use crate::ui::widgets::GlowTracker;

/// Root viewport title — doubles as the find_hwnd key (windows.rs).
pub const MAIN_WINDOW_TITLE: &str = crate::windows::MAIN_TITLE;

/// Commands from non-UI threads (tray events, console ctrl handler).
pub enum UiCmd {
    TogglePanel,
    UpdateTooltip,
    /// Tray right-click (SPEC 10.3): position the menu at this physical
    /// cursor point. Executed in the root pass (viewport commands issued from
    /// a thread have no pass to ride on).
    ShowMenu {
        px: f64,
        py: f64,
    },
    Quit,
}

/// Actions collected inside the secondary viewports (settings / skills /
/// menu). Their passes cannot touch App state, so they queue here and the
/// root pass executes (same split as the rust/ edition's invoke commands).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WinAction {
    /// Settings Save (SPEC 13.2): persist -> autostart -> theme -> reschedule
    /// -> close. Carries the draft as edited in the window.
    SettingsSave(SettingsDraft),
    SettingsClose,
    SkillsClose,
    /// Skills window Refresh button: force rescan (SPEC 21.2).
    SkillsRefresh,
    /// Tray menu items (SPEC 14): the menu window is already hidden by the
    /// time these run (menu_action semantics in rust/lib.rs).
    MenuOpen,
    MenuRefresh,
    MenuSettings,
    MenuSkills,
    MenuQuit,
}

/// Wake the UI thread, with a delayed retry. eframe 0.36.2 drops a
/// `UserEvent::RequestRepaint` as "outdated" when two passes of the target
/// viewport completed between the repaint callback and the event being
/// processed (run.rs, egui issue #7776-era scheduling). A request issued
/// right as a pass burst ends can be the LAST wake source — dropped, it
/// leaves the queued command undrained until some unrelated event arrives
/// (observed as a lost tray TogglePanel). The retry fires only when passes
/// have run since the first request (egui's repaint-delay gate suppresses it
/// otherwise), so it costs nothing in the common case.
fn wake_with_retry(ctx: &egui::Context, vp: ViewportId) {
    ctx.request_repaint_of(vp);
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("wake-retry".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            ctx.request_repaint_of(vp);
        })
        .ok();
}

/// Shared handle for other threads to talk to the UI thread. The egui Context
/// lands here once the app is constructed; pushing a command wakes the loop.
#[derive(Default)]
pub struct UiShared {
    pub egui_ctx: OnceLock<egui::Context>,
    pub cmds: Mutex<VecDeque<UiCmd>>,
}

impl UiShared {
    pub fn push(&self, cmd: UiCmd) {
        self.cmds.lock().unwrap().push_back(cmd);
        if let Some(ctx) = self.egui_ctx.get() {
            // Outside a pass this targets the root viewport, which is what
            // drains the queues.
            wake_with_retry(ctx, ViewportId::ROOT);
        }
    }

    fn drain(&self) -> VecDeque<UiCmd> {
        std::mem::take(&mut self.cmds.lock().unwrap())
    }
}

/// Cross-pass state for the secondary windows. Everything here is
/// Send+Sync because the deferred-viewport callbacks (which run without
/// app.rs) capture an Arc of it.
pub struct WinBus {
    /// Installed on the first root frame (textures are context-global).
    pub icons: OnceLock<Icons>,
    /// Settings draft: written on open (backfill) by the root pass, mutated
    /// by radio/pill/checkbox clicks in the settings pass.
    pub draft: Mutex<SettingsDraft>,
    /// Menu window open flag (mirrors the Visible state; also read by the
    /// --test-ui harness scripts via kpt-windows.json).
    pub menu_open: AtomicBool,
    /// Real-focus latch for the menu (same never-focused trap as the panel,
    /// Phase 2 gotcha 5).
    pub menu_was_focused: AtomicBool,
    /// Actions queued by the child viewports, executed by the root pass.
    pub actions: Mutex<VecDeque<WinAction>>,
    /// Per-viewport hover-glow state (SPEC 15.3); one each so glow progress
    /// bookkeeping stays inside the pass that owns the widgets.
    pub settings_glow: Mutex<GlowTracker>,
    pub skills_glow: Mutex<GlowTracker>,
    pub menu_glow: Mutex<GlowTracker>,
}

impl Default for WinBus {
    fn default() -> WinBus {
        WinBus {
            icons: OnceLock::new(),
            draft: Mutex::new(SettingsDraft {
                theme: crate::ui::settings_win::ThemeChoice::System,
                minutes: 5,
                auto_start: false,
            }),
            menu_open: AtomicBool::new(false),
            menu_was_focused: AtomicBool::new(false),
            actions: Mutex::new(VecDeque::new()),
            settings_glow: Mutex::new(GlowTracker::default()),
            skills_glow: Mutex::new(GlowTracker::default()),
            menu_glow: Mutex::new(GlowTracker::default()),
        }
    }
}

impl WinBus {
    pub fn new() -> WinBus {
        WinBus::default()
    }

    /// Queue an action from a child viewport and wake the root pass (each
    /// viewport repaints independently — the root would stay asleep).
    pub fn push_action(&self, ctx: &egui::Context, action: WinAction) {
        self.actions.lock().unwrap().push_back(action);
        wake_with_retry(ctx, ViewportId::ROOT);
    }

    fn drain(&self) -> VecDeque<WinAction> {
        std::mem::take(&mut self.actions.lock().unwrap())
    }
}

pub struct App {
    pub biz: AppCtx,
    pub ui: Arc<UiShared>,
    pub win: Arc<WinBus>,
    /// Tokio handle for UI-triggered async work (manual refresh, skills scan).
    pub runtime: Arc<OnceLock<tokio::runtime::Handle>>,
    /// Main-thread-only tray handle (Rc inside tray-icon).
    pub tray: Option<crate::tray::Tray>,
    pub icons: Option<Icons>,
    pub panel_state: PanelState,
    pub glow: GlowTracker,
    pub font_probe: FontProbe,
    pub last_theme: String,
    pub first_frame: bool,
    /// True once the panel actually held focus since the current show. The
    /// focus-loss auto-hide must fire only on a REAL focus loss — egui reports
    /// `focused == Some(false)` for a shown-but-never-focused window, while
    /// tauri's Focused(false) event only follows a Focused(true) (SPEC 15).
    pub panel_was_focused: bool,
    /// Singleton-placement latches (SPEC 10.2 / 21.1): the settings/skills
    /// windows center on the primary work area on their FIRST open only —
    /// later opens show them where they were, like the WPF WindowStartupLocation
    /// / tauri conf `center: true` semantics (a user drag survives reopen).
    /// Sizes are re-pinned on every open either way (DPI guard).
    pub settings_placed: bool,
    pub skills_placed: bool,
    /// --test-ui mode: show all four windows headless-style, print the SPEC 19
    /// OK lines, and suppress the focus-loss auto-hide (programmatic shows
    /// never get focus, SPEC 20).
    pub test_ui: bool,
    /// Set by the app's own Quit paths (Exit button / tray-menu Exit /
    /// console ctrl) right before they send `ViewportCommand::Close`. The
    /// root close guard below cannot distinguish that Close event from an
    /// OS WM_CLOSE (egui-winit pushes the same ViewportEvent::Close for
    /// both), so the flag is the only way to let the explicit exits through
    /// while cancelling Alt+F4 (SPEC 20).
    pub quitting: bool,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        biz: AppCtx,
        ui: Arc<UiShared>,
        win: Arc<WinBus>,
        runtime: Arc<OnceLock<tokio::runtime::Handle>>,
        tray: Option<crate::tray::Tray>,
        test_ui: bool,
    ) -> App {
        let _ = cc;
        let last_theme = biz.state.effective_theme.read().unwrap().clone();
        // Fixed-geometry tray app: egui's keyboard zoom (Ctrl+= / Ctrl+- /
        // Ctrl+0) would scale pixels_per_point above the native DPI, and every
        // OuterPosition landing divides by the native scale only (see
        // panel_pos::window_dpi_scale) — the windows would drift by the zoom
        // factor after a shortcut. Clear the built-in Ctrl/Cmd+Q quit
        // shortcut too: it would surface as the same close event the root
        // guard cancels (hide, SPEC 20) while no other edition has that key.
        // Both must be set before the first pass.
        cc.egui_ctx.options_mut(|o| {
            o.zoom_with_keyboard = false;
            o.quit_shortcuts = vec![];
        });
        // Install fonts before the first frame so the named weight families
        // are bound when the panel first paints (an unbound family panics).
        let font_probe = FontProbe::new(&cc.egui_ctx);
        App {
            biz,
            ui,
            win,
            runtime,
            tray,
            icons: None,
            panel_state: PanelState::Hidden,
            glow: GlowTracker::default(),
            font_probe,
            last_theme,
            first_frame: true,
            panel_was_focused: false,
            settings_placed: false,
            skills_placed: false,
            test_ui,
            quitting: false,
        }
    }

    fn ctx(&self) -> egui::Context {
        self.ui.egui_ctx.get().expect("egui ctx").clone()
    }

    // ---- Panel transitions (port of rust/src-tauri/src/panel.rs) ----------

    fn show_panel(&mut self, focus: bool) {
        let ctx = self.ctx();
        #[cfg(debug_assertions)]
        eprintln!("[ui] show_panel");
        crate::panel_pos::show_panel(&ctx, focus);
        self.biz.state.panel_hiding.store(false, Ordering::SeqCst);
        self.panel_was_focused = false;
        self.panel_state = PanelState::Showing { t0: Instant::now() };
        ctx.request_repaint();
    }

    /// Ask the panel to play the slide-out (re-entry guarded, SPEC 15.2).
    fn start_hide(&mut self) {
        #[cfg(debug_assertions)]
        eprintln!("[ui] start_hide (was {:?})", self.panel_state);
        if matches!(
            self.panel_state,
            PanelState::Hiding { .. } | PanelState::Hidden
        ) {
            return;
        }
        self.biz.state.panel_hiding.store(true, Ordering::SeqCst);
        self.panel_state = PanelState::Hiding { t0: Instant::now() };
        if let Some(ctx) = self.ui.egui_ctx.get() {
            ctx.request_repaint();
        }
    }

    /// Called when the fade-out finished: actually hide + stamp last_hide for
    /// the 300ms tray-click guard (SPEC 14 / 15.2). The state machine doubles
    /// as the rust edition's hard guard: a show since the hide started
    /// replaces the Hiding state, so a stale finish can never fire.
    fn finish_hide(&mut self) {
        if !matches!(self.panel_state, PanelState::Hiding { .. }) {
            return;
        }
        if let Some(ctx) = self.ui.egui_ctx.get() {
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        self.biz.state.panel_hiding.store(false, Ordering::SeqCst);
        *self.biz.state.last_hide.lock().unwrap() = Some(Instant::now());
        self.panel_state = PanelState::Hidden;
    }

    /// TogglePopup (SPEC 14): tray left-clicks within 300ms of an auto-hide
    /// are ignored (same click caused the focus-loss hide first). `from_tray`
    /// selects the guard — the tray-menu "Open" item calls with false, exactly
    /// like rust menu_action("open") -> toggle_panel(from_tray=false).
    fn toggle_panel(&mut self, from_tray: bool) {
        if from_tray {
            let last_hide = *self.biz.state.last_hide.lock().unwrap();
            if let Some(t) = last_hide {
                if t.elapsed() < Duration::from_millis(300) {
                    return;
                }
            }
        }
        if self.panel_state.is_visible() {
            self.start_hide();
        } else {
            self.show_panel(true);
        }
    }

    // ---- Actions -----------------------------------------------------------

    fn update_tooltip(&mut self) {
        if let Some(tray) = &self.tray {
            let state = &self.biz.state;
            crate::tray::update_tooltip(&tray.icon, state);
        }
    }

    /// Panel Refresh button / tray-menu "Refresh": SafeRefresh + CheckAsync
    /// with the 2s manual refresh debounce (SPEC 12.7 / 14).
    fn refresh_now(&mut self) {
        {
            let mut last = self.biz.state.last_manual_refresh.lock().unwrap();
            let now = Instant::now();
            if last.is_some_and(|t| now.duration_since(t) < Duration::from_secs(2)) {
                return;
            }
            *last = Some(now);
        }
        let Some(handle) = self.runtime.get() else {
            return;
        };
        let c1 = self.biz.clone();
        handle.spawn(async move {
            crate::polling::safe_refresh(&c1).await;
        });
        let c2 = self.biz.clone();
        handle.spawn(async move {
            crate::check_and_emit(&c2).await;
        });
    }

    // ---- Secondary windows (SPEC 10.2 / 10.3 / 13 / 21) --------------------

    /// ShowSettings: flag up, backfill the draft, center + show + focus. The
    /// window itself is a persistent hidden viewport (singleton reuse).
    fn open_settings(&mut self, focus: bool) {
        self.biz.state.settings_open.store(true, Ordering::SeqCst);
        *self.win.draft.lock().unwrap() =
            SettingsDraft::from_settings(&self.biz.state.settings.read().unwrap());
        let ctx = self.ctx();
        crate::panel_pos::show_centered(
            &ctx,
            crate::panel_pos::vp_settings(),
            crate::panel_pos::SETTINGS_W,
            crate::panel_pos::SETTINGS_H,
            focus,
            !self.settings_placed,
        );
        self.settings_placed = true;
        if focus {
            // A user gesture (panel button / tray menu click) granted our
            // process foreground rights; back the queued Focus up with the
            // delayed steal (Phase 2 gotcha 4).
            crate::windows::spawn_focus_steal(crate::windows::SETTINGS_TITLE, &[60, 200, 450]);
        }
        ctx.request_repaint();
    }

    fn close_settings(&mut self) {
        self.biz.state.settings_open.store(false, Ordering::SeqCst);
        let ctx = self.ctx();
        ctx.send_viewport_cmd_to(
            crate::panel_pos::vp_settings(),
            ViewportCommand::Visible(false),
        );
        ctx.request_repaint();
    }

    /// ShowSkills (SPEC 21): same reuse pattern as settings; the scan is lazy
    /// — first open kicks off one blocking scan, later opens just reuse the
    /// cache (zero background cost, SPEC 21.2).
    fn open_skills(&mut self, focus: bool) {
        self.biz.state.skills_open.store(true, Ordering::SeqCst);
        self.scan_skills_if_needed();
        let ctx = self.ctx();
        crate::panel_pos::show_centered(
            &ctx,
            crate::panel_pos::vp_skills(),
            crate::panel_pos::SKILLS_W,
            crate::panel_pos::SKILLS_H,
            focus,
            !self.skills_placed,
        );
        self.skills_placed = true;
        if focus {
            crate::windows::spawn_focus_steal(crate::windows::SKILLS_TITLE, &[60, 200, 450]);
        }
        ctx.request_repaint();
    }

    fn close_skills(&mut self) {
        self.biz.state.skills_open.store(false, Ordering::SeqCst);
        let ctx = self.ctx();
        ctx.send_viewport_cmd_to(
            crate::panel_pos::vp_skills(),
            ViewportCommand::Visible(false),
        );
        ctx.request_repaint();
    }

    /// One-shot scan into the cache (first open; SPEC 21.2 "scan once on
    /// first open, cache"). Runs on the shared tokio runtime; the skills pass
    /// shows "Loading..." until the cache fills, then repaints that viewport.
    fn scan_skills_if_needed(&mut self) {
        if !self.biz.state.skills_cache.read().unwrap().is_none() {
            return;
        }
        let Some(handle) = self.runtime.get() else {
            // No runtime (should not happen): fall back to a synchronous scan.
            let list = crate::skills::scan();
            *self.biz.state.skills_cache.write().unwrap() = Some(list);
            return;
        };
        let state = self.biz.state.clone();
        let ctx = self.ctx();
        handle.spawn_blocking(move || {
            let list = crate::skills::scan();
            *state.skills_cache.write().unwrap() = Some(list);
            ctx.request_repaint_of(crate::panel_pos::vp_skills());
        });
    }

    /// Force rescan (Skills window Refresh button, SPEC 21.2): the old cache
    /// keeps rendering until the fresh list lands (rust/ get_skills(refresh)
    /// re-renders when the invoke resolves).
    fn refresh_skills(&mut self) {
        let Some(handle) = self.runtime.get() else {
            return;
        };
        let state = self.biz.state.clone();
        let ctx = self.ctx();
        handle.spawn_blocking(move || {
            let list = crate::skills::scan();
            *state.skills_cache.write().unwrap() = Some(list);
            ctx.request_repaint_of(crate::panel_pos::vp_skills());
        });
    }

    /// Tray right-click (SPEC 10.3): reposition the menu at the cursor and
    /// show it (the rust edition closes the old instance and creates a new
    /// one; our singleton just moves). `focus` skips the foreground steal for
    /// --test-ui (run_ui_test never activates windows).
    fn open_menu_at(&mut self, px: f64, py: f64, focus: bool) {
        let ctx = self.ctx();
        self.win.menu_was_focused.store(false, Ordering::SeqCst);
        self.win.menu_open.store(true, Ordering::SeqCst);
        let h = crate::panel_pos::menu_window_height(&ctx);
        *self.biz.state.menu_height.lock().unwrap() = h as f64;
        crate::panel_pos::show_menu_at(&ctx, px, py, h);
        if focus {
            // panel.rs: steal the foreground AFTER show, else the menu never
            // gets focus and would never close on focus loss.
            crate::windows::spawn_focus_steal(crate::windows::MENU_TITLE, &[60, 200, 450]);
        }
        ctx.request_repaint();
    }

    fn hide_menu(&mut self) {
        self.win.menu_open.store(false, Ordering::SeqCst);
        self.win.menu_was_focused.store(false, Ordering::SeqCst);
        let ctx = self.ctx();
        ctx.send_viewport_cmd_to(crate::panel_pos::vp_menu(), ViewportCommand::Visible(false));
    }

    /// SPEC 13.2 save chain: persist -> ApplyAutoStart -> Theme.Apply ->
    /// Quota.Reschedule -> Close.
    fn save_settings(&mut self, draft: SettingsDraft) {
        {
            let mut cur = self.biz.state.settings.write().unwrap();
            cur.theme = draft.theme.as_str().to_string();
            // Clamp at the boundary like rust save_settings (IPC clamp).
            cur.refresh_minutes = draft.minutes.clamp(1, 30);
            cur.auto_start = draft.auto_start;
            crate::settings::save(&cur);
            crate::settings::apply_auto_start(&cur);
        }
        // Live theme apply (incl. the "system" re-evaluation path)...
        crate::theme_watch::apply_now(&self.biz);
        // ...and wake the root pass so every window picks the new theme up at
        // once (apply_now only emits; the repaint happens here).
        {
            let ctx = self.ctx();
            ctx.request_repaint();
            ctx.request_repaint_of(crate::panel_pos::vp_settings());
            ctx.request_repaint_of(crate::panel_pos::vp_skills());
            ctx.request_repaint_of(crate::panel_pos::vp_menu());
        }
        self.biz.state.reschedule.notify_one();
        self.close_settings();
    }

    fn handle_actions(&mut self, actions: Vec<panel::PanelAction>) {
        for action in actions {
            match action {
                panel::PanelAction::OpenReleases => {
                    crate::windows::open_url(crate::RELEASES_URL);
                }
                panel::PanelAction::OpenConsole => {
                    crate::windows::open_url(crate::CONSOLE_URL);
                }
                panel::PanelAction::Refresh => self.refresh_now(),
                // Settings/Skills buttons (SPEC 12.7): the panel suppresses
                // its focus-loss auto-hide while either window is open.
                panel::PanelAction::OpenSettings => self.open_settings(true),
                panel::PanelAction::OpenSkills => self.open_skills(true),
                panel::PanelAction::Quit => {
                    // Explicit exit: raise the flag BEFORE the Close command so
                    // the root close guard stands down (SPEC 20). self.ctx()
                    // panics if the ctx is unset instead of leaving the flag
                    // latched without a Close (which would let a later Alt+F4
                    // exit for real).
                    self.quitting = true;
                    self.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn execute_win_action(&mut self, action: WinAction) {
        #[cfg(debug_assertions)]
        eprintln!("[ui] win action {action:?}");
        match action {
            WinAction::SettingsSave(draft) => self.save_settings(draft),
            WinAction::SettingsClose => self.close_settings(),
            WinAction::SkillsClose => self.close_skills(),
            WinAction::SkillsRefresh => self.refresh_skills(),
            WinAction::MenuOpen => {
                self.hide_menu();
                self.toggle_panel(false);
            }
            WinAction::MenuRefresh => self.refresh_now(),
            WinAction::MenuSettings => {
                self.hide_menu();
                self.open_settings(true);
            }
            WinAction::MenuSkills => {
                self.hide_menu();
                self.open_skills(true);
            }
            WinAction::MenuQuit => {
                self.hide_menu();
                // Same explicit-exit bypass as PanelAction::Quit (SPEC 20):
                // self.ctx() never leaves the flag latched without a Close.
                self.quitting = true;
                self.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    fn secondary_viewport_builder(title: &str, size: egui::Vec2) -> ViewportBuilder {
        // Same SPEC 10 chrome as the panel: frameless, transparent, topmost,
        // no taskbar, fixed size, created hidden (visibility is command-driven
        // so "position before show" ordering is deterministic).
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

    /// Register the three deferred viewports (every root pass — unregistering
    /// would destroy the native windows and break singleton reuse; hiding is
    /// done with Visible commands).
    fn register_viewports(&mut self, ctx: &egui::Context) {
        let biz = self.biz.clone();
        let win = self.win.clone();
        ctx.show_viewport_deferred(
            crate::panel_pos::vp_settings(),
            Self::secondary_viewport_builder(
                crate::windows::SETTINGS_TITLE,
                egui::vec2(crate::panel_pos::SETTINGS_W, crate::panel_pos::SETTINGS_H),
            ),
            move |ui, _class| {
                crate::ui::settings_win::paint(ui, &biz, &win);
            },
        );
        let biz = self.biz.clone();
        let win = self.win.clone();
        ctx.show_viewport_deferred(
            crate::panel_pos::vp_skills(),
            Self::secondary_viewport_builder(
                crate::windows::SKILLS_TITLE,
                egui::vec2(crate::panel_pos::SKILLS_W, crate::panel_pos::SKILLS_H),
            ),
            move |ui, _class| {
                crate::ui::skills_win::paint(ui, &biz, &win);
            },
        );
        let biz = self.biz.clone();
        let win = self.win.clone();
        ctx.show_viewport_deferred(
            crate::panel_pos::vp_menu(),
            Self::secondary_viewport_builder(
                crate::windows::MENU_TITLE,
                egui::vec2(crate::panel_pos::MENU_W, crate::panel_pos::MENU_H_INIT),
            ),
            move |ui, _class| {
                crate::ui::menu_win::paint(ui, &biz, &win);
            },
        );
    }
}

impl eframe::App for App {
    // Transparent clear so the per-pixel-alpha window shows the desktop behind.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();

        if self.first_frame {
            self.first_frame = false;
            let _ = self.ui.egui_ctx.set(ctx.clone());
            ctx.set_visuals(egui::Visuals::light()); // colors come from Theme
                                                     // eframe force-shows the root viewport after the first frame
                                                     // (anti-flash); re-hide immediately — the first frame is fully
                                                     // transparent, so there is no visible flash.
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            let icons = Icons::load(&ctx);
            let _ = self.win.icons.set(icons.clone());
            self.icons = Some(icons);
            if self.test_ui {
                // SPEC 19: construct + show all four windows and report, then
                // exit (~6s timer in main.rs). Matches rust run_ui_test: show
                // only, no focus stealing; settings_open suppresses the panel
                // auto-hide; the skills list is loaded so it renders too.
                self.show_panel(false);
                crate::print_stdout("MainWindow OK\n");
                self.open_settings(false);
                crate::print_stdout("SettingsWindow OK\n");
                self.open_skills(false);
                crate::print_stdout("SkillsWindow OK\n");
                let (px, py) = crate::panel_pos::cursor_pos_phys();
                self.open_menu_at(px, py, false);
                crate::print_stdout("TrayMenuWindow OK\n");
            }
        }

        // Fonts: install/verify one step per frame (set_fonts takes effect at
        // the NEXT begin_pass — a same-frame coverage check would misreport).
        self.font_probe.step(&ctx);

        // (The AppEvent bus is consumed by the drain thread, which pushes
        // commands / repaints here; AppState always carries the live data.)

        // Cross-thread UI commands (tray clicks, console ctrl handler).
        for cmd in self.ui.drain() {
            #[cfg(debug_assertions)]
            if matches!(cmd, UiCmd::TogglePanel) {
                eprintln!("[ui] drain TogglePanel, state={:?}", self.panel_state);
            }
            match cmd {
                UiCmd::TogglePanel => self.toggle_panel(true),
                UiCmd::UpdateTooltip => self.update_tooltip(),
                UiCmd::ShowMenu { px, py } => self.open_menu_at(px, py, true),
                UiCmd::Quit => {
                    // Console ctrl handler / --test-ui timer: explicit exit.
                    self.quitting = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        // Actions queued by the secondary viewports (their passes cannot touch
        // App state — see the module comment).
        for action in self.win.drain() {
            self.execute_win_action(action);
        }

        // Theme sync: ThemeChanged events already wake us; pick up the value.
        {
            let name = self.biz.state.effective_theme.read().unwrap().clone();
            if name != self.last_theme {
                self.last_theme = name;
                ctx.request_repaint();
            }
        }
        let theme = Theme::from_name(&self.last_theme);

        // Panel state machine: advance animations, apply the focus-loss
        // auto-hide (SPEC 15 / 20), and schedule repaints while animating.
        match self.panel_state {
            PanelState::Showing { t0 } => {
                if now.duration_since(t0).as_secs_f32() >= panel::ENTER_MOVE_SECS {
                    self.panel_state = PanelState::Shown;
                }
            }
            PanelState::Hiding { t0 }
                if now.duration_since(t0).as_secs_f32() >= panel::EXIT_DONE_SECS =>
            {
                self.finish_hide();
            }
            _ => {}
        }
        // Root close guard (SPEC 20: closing a window never exits the
        // process — the tray app keeps running hidden). Alt+F4 / WM_CLOSE on
        // the focused panel surfaces here as a root close request: cancel it
        // and play the normal hide. The app's own Quit paths arrive through
        // the very same ViewportEvent::Close, so they raise `quitting` first
        // and this guard stands down to let eframe exit for real.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.start_hide();
        }
        let focused = ctx.input(|i| i.viewport().focused);
        if focused == Some(true) {
            self.panel_was_focused = true;
        }
        #[cfg(debug_assertions)]
        if self.panel_state.is_visible() {
            let (lp, ip) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.interact_pos()));
            eprintln!("[ui] pointer latest={lp:?} interact={ip:?} focused={focused:?}");
        }
        if self.panel_state.is_visible()
            && !matches!(self.panel_state, PanelState::Hiding { .. })
            && self.panel_was_focused
            && !self.biz.state.settings_open.load(Ordering::SeqCst)
            && !self.biz.state.skills_open.load(Ordering::SeqCst)
            && !self.test_ui
            && focused == Some(false)
        {
            self.start_hide();
        }
        if let Some(remaining) = self.panel_state.animating_for(now) {
            ctx.request_repaint_after(remaining.min(Duration::from_millis(16)));
        }

        // Deferred viewports: registered every pass (singleton windows).
        self.register_viewports(&ctx);

        // Paint (skipped entirely while hidden: zero cost when idle).
        if self.panel_state != PanelState::Hidden {
            let (alpha, y_off) = self.panel_state.appearance(now);
            let quota = self.biz.state.last_quota.read().unwrap().clone();
            let update = self.biz.state.update.read().unwrap().clone();
            let params = panel::PanelPaint {
                theme: &theme,
                alpha,
                y_off,
                icons: self.icons.as_ref().expect("icons loaded on first frame"),
            };
            let actions = panel::paint(ui, &params, quota.as_ref(), &update, &mut self.glow, now);
            self.handle_actions(actions);

            // TEMP Phase 2 bring-up: visualize the console-button glow state
            // as a magenta bar at the panel's top-left (length = internal p).
            #[cfg(debug_assertions)]
            if std::env::var("KPT_GLOW_DEBUG").is_ok() {
                let marker = self
                    .glow
                    .peek(egui::Id::new("panel-btn-console"))
                    .unwrap_or(0.0);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(ui.max_rect().left() + 4.0, ui.max_rect().top() + 2.0),
                        egui::Vec2::new(marker * 100.0, 3.0),
                    ),
                    egui::CornerRadius::same(1),
                    egui::Color32::from_rgb(255, 0, 255),
                );
            }
        }

        // Hover-glow bookkeeping: forget faded widgets, keep the frame loop
        // running only while a transition is in flight.
        self.glow.end_frame(&ctx, now);
    }
}
