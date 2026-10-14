// The eframe application (Phase 2): owns the panel state machine, consumes
// the AppEvent bus and cross-thread UI commands, and drives the SPEC 15
// animations. Business data is read from the shared AppState each frame (the
// Phase 1 core keeps it current); the tray icon lives here because
// tray_icon::TrayIcon is not Send and must stay on the main thread.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::viewport::ViewportCommand;

use crate::ctx::AppCtx;
use crate::icons::Icons;
use crate::ui::panel::{self, PanelState};
use crate::ui::theme::{FontProbe, Theme};
use crate::ui::widgets::GlowTracker;

/// Root viewport title — doubles as the FindWindowW key (windows.rs).
pub const MAIN_WINDOW_TITLE: &str = crate::windows::MAIN_TITLE;

/// Commands from non-UI threads (tray events, console ctrl handler).
pub enum UiCmd {
    TogglePanel,
    UpdateTooltip,
    Quit,
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
            ctx.request_repaint();
        }
    }

    fn drain(&self) -> VecDeque<UiCmd> {
        std::mem::take(&mut self.cmds.lock().unwrap())
    }
}

pub struct App {
    pub biz: AppCtx,
    pub ui: Arc<UiShared>,
    /// Tokio handle for UI-triggered async work (manual refresh).
    pub runtime: Arc<OnceLock<tokio::runtime::Handle>>,
    /// Main-thread-only tray handle (Rc inside tray-icon).
    pub tray: Option<crate::tray::Tray>,
    pub icons: Option<Icons>,
    pub panel_state: PanelState,
    pub glow: GlowTracker,
    pub font_probe: FontProbe,
    pub last_theme: String,
    pub visibility: crate::windows::VisibilityFlags,
    pub first_frame: bool,
    /// True once the panel actually held focus since the current show. The
    /// focus-loss auto-hide must fire only on a REAL focus loss — egui reports
    /// `focused == Some(false)` for a shown-but-never-focused window, while
    /// tauri's Focused(false) event only follows a Focused(true) (SPEC 15).
    pub panel_was_focused: bool,
    /// --test-ui mode: show the panel headless-style and suppress the
    /// focus-loss auto-hide (programmatic shows never get focus, SPEC 20).
    pub test_ui: bool,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        biz: AppCtx,
        ui: Arc<UiShared>,
        runtime: Arc<OnceLock<tokio::runtime::Handle>>,
        tray: Option<crate::tray::Tray>,
        visibility: crate::windows::VisibilityFlags,
        test_ui: bool,
    ) -> App {
        let _ = cc;
        let last_theme = biz.state.effective_theme.read().unwrap().clone();
        // Install fonts before the first frame so the named weight families
        // are bound when the panel first paints (an unbound family panics).
        let font_probe = FontProbe::new(&cc.egui_ctx);
        App {
            biz,
            ui,
            runtime,
            tray,
            icons: None,
            panel_state: PanelState::Hidden,
            glow: GlowTracker::default(),
            font_probe,
            last_theme,
            visibility,
            first_frame: true,
            panel_was_focused: false,
            test_ui,
        }
    }

    // ---- Panel transitions (port of rust/src-tauri/src/panel.rs) ----------

    fn show_panel(&mut self) {
        let ctx = self.ui.egui_ctx.get().expect("egui ctx").clone();
        #[cfg(debug_assertions)]
        eprintln!("[ui] show_panel");
        crate::panel_pos::show_panel(&ctx);
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
    /// are ignored (same click caused the focus-loss hide first).
    fn toggle_panel(&mut self) {
        let last_hide = *self.biz.state.last_hide.lock().unwrap();
        if let Some(t) = last_hide {
            if t.elapsed() < Duration::from_millis(300) {
                return;
            }
        }
        if self.panel_state.is_visible() {
            self.start_hide();
        } else {
            self.show_panel();
        }
    }

    // ---- Actions -----------------------------------------------------------

    fn update_tooltip(&mut self) {
        if let Some(tray) = &self.tray {
            let state = &self.biz.state;
            crate::tray::update_tooltip(&tray.icon, state);
        }
    }

    /// Panel Refresh button: SafeRefresh + CheckAsync with the 2s manual
    /// refresh debounce (SPEC 12.7).
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
                // TODO(Phase 3): open the settings window (SPEC 17) and the
                // skills window (SPEC 21); both suppress the panel's
                // focus-loss auto-hide while open.
                panel::PanelAction::OpenSettings | panel::PanelAction::OpenSkills => {}
                panel::PanelAction::Quit => {
                    if let Some(ctx) = self.ui.egui_ctx.get() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
        }
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
            self.icons = Some(Icons::load(&ctx));
            if self.test_ui {
                // Headless-style UI self-check: construct + show the panel and
                // report; main.rs schedules the ~6s auto-exit.
                self.show_panel();
                crate::print_stdout("MainWindow OK\n");
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
                UiCmd::TogglePanel => self.toggle_panel(),
                UiCmd::UpdateTooltip => self.update_tooltip(),
                UiCmd::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
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
        // HWND styling watcher tracks panel visibility.
        if let Ok(mut flags) = self.visibility.lock() {
            if let Some(f) = flags.first_mut() {
                *f = self.panel_state.is_visible();
            }
        }

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
