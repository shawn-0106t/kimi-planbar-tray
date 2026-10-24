// Settings window (SPEC 10.2 + 13): 404x464 viewport, frameless/transparent/
// topmost, centered on show, custom draggable titlebar, theme radios, refresh
// interval pill radios, autostart checkbox, Save. Singleton reuse: the window
// is only ever hidden (never destroyed); the draft is backfilled from the
// persisted settings on every open (SPEC 13.2).
//
// The paint pass runs inside the settings viewport's own egui pass — app.rs
// does NOT run. Interactions are therefore pushed onto the shared WinBus
// action queue and the root pass is woken to execute them.

use std::sync::Arc;
use std::time::Instant;

use egui::{pos2, Align2, Color32, Id, Rect, Ui, Vec2};

use crate::app::WinAction;
use crate::app::WinBus;
use crate::ctx::AppCtx;
use crate::settings::SettingsData;

use super::controls;
use super::theme::{self, Theme};

/// Theme setting values ("system" | "light" | "dark"), SPEC 18.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    /// Unknown persisted values fall back to "system" (the rust/ frontend
    /// checks the radio that matches, else checks System default).
    pub fn from_config(value: &str) -> ThemeChoice {
        match value {
            "light" => ThemeChoice::Light,
            "dark" => ThemeChoice::Dark,
            _ => ThemeChoice::System,
        }
    }

    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];

    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "System default",
            ThemeChoice::Light => "Moonlit (light)",
            ThemeChoice::Dark => "Moondark (dark)",
        }
    }
}

/// The in-window draft state, shared between app.rs (backfill on open + Save
/// execution) and this paint pass (radio/pill/checkbox clicks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsDraft {
    pub theme: ThemeChoice,
    pub minutes: i64,
    pub auto_start: bool,
}

impl SettingsDraft {
    /// Backfill from the persisted settings (SPEC 13.2: every open). Values
    /// outside the SPEC's option sets fall back to the defaults.
    pub fn from_settings(s: &SettingsData) -> SettingsDraft {
        let minutes = if [1, 5, 10, 30].contains(&s.refresh_minutes) {
            s.refresh_minutes
        } else {
            5
        };
        SettingsDraft {
            theme: ThemeChoice::from_config(&s.theme),
            minutes,
            auto_start: s.auto_start,
        }
    }

    pub fn to_settings_data(self) -> SettingsData {
        SettingsData {
            theme: self.theme.as_str().to_string(),
            refresh_minutes: self.minutes,
            auto_start: self.auto_start,
        }
    }
}

pub const PILL_MINUTES: [i64; 4] = [1, 5, 10, 30];

/// Paint one settings pass and collect interactions.
pub fn paint(ui: &mut Ui, biz: &AppCtx, bus: &Arc<WinBus>) {
    let theme = Theme::from_name(&biz.state.effective_theme.read().unwrap().clone());
    let now = Instant::now();

    // SPEC 10.2: Margin=6 WPF -> 28px shadow fade room, CornerRadius=14,
    // shadow Blur 24 / Depth 2 / Opacity 0.25.
    let card = controls::window_card(ui, 28.0, 14, 24, 64, &theme);
    let content = card.shrink(16.0);

    let mut glow = bus.settings_glow.lock().unwrap();

    // ---- Titlebar (SPEC 13.1) ----------------------------------------------
    let logo = bus
        .icons
        .get()
        .expect("icons installed on the first root frame");
    if controls::titlebar(
        ui,
        content,
        "Kimi Planbar Tray Settings",
        "settings-titlebar",
        &logo.logo,
        &theme,
    ) {
        bus.push_action(ui.ctx(), WinAction::SettingsClose);
    }

    // ---- Body: StackPanel Margin=6,0,4,0 (SPEC 13.2) -----------------------
    let body_left = content.left() + 6.0;
    let body_right = content.right() - 4.0;
    let body_w = body_right - body_left;
    let mut y = content.top() + 22.0 + 16.0; // titlebar row + 16px margin

    let painter = ui.painter();
    let section_h = painter
        .layout_no_wrap("Theme".to_string(), theme::semibold(13.0), Color32::WHITE)
        .size()
        .y;
    painter.text(
        pos2(body_left, y),
        Align2::LEFT_TOP,
        "Theme",
        theme::semibold(13.0),
        theme.text_secondary,
    );
    y += section_h;

    // Theme radios: Margin=0,10,0,0 then 0,8,0,0; row height = the 18px circle.
    let draft_theme = bus.draft.lock().unwrap().theme;
    for choice in ThemeChoice::ALL {
        y += if choice == ThemeChoice::System {
            10.0
        } else {
            8.0
        };
        let row = Rect::from_min_size(pos2(body_left, y), Vec2::new(body_w, 18.0));
        let id = Id::unique(("settings-theme", choice.as_str()));
        if controls::theme_radio(ui, id, row, draft_theme == choice, choice.label(), &theme) {
            bus.draft.lock().unwrap().theme = choice;
        }
        y += 18.0;
    }

    // Refresh interval (Margin=0,20,0,0 title; pills Margin=0,10,0,0).
    y += 20.0;
    painter.text(
        pos2(body_left, y),
        Align2::LEFT_TOP,
        "Refresh interval",
        theme::semibold(13.0),
        theme.text_secondary,
    );
    y += section_h + 10.0;

    let draft_minutes = bus.draft.lock().unwrap().minutes;
    let mut pill_x = body_left;
    for minutes in PILL_MINUTES {
        let label = format!("{minutes} min");
        let size = controls::pill_size(painter, &label);
        let rect = Rect::from_min_size(pos2(pill_x, y), size);
        let id = Id::unique(("settings-pill", minutes));
        if controls::pill_radio(ui, id, rect, draft_minutes == minutes, &label, &theme) {
            bus.draft.lock().unwrap().minutes = minutes;
        }
        pill_x += size.x + 6.0; // Margin=0,0,6,0 between pills
    }
    y += controls::pill_size(painter, "30 min").y;

    // Autostart checkbox (Margin=0,22,0,0), row height = the 18px box.
    y += 22.0;
    let auto_start = bus.draft.lock().unwrap().auto_start;
    let row = Rect::from_min_size(pos2(body_left, y), Vec2::new(body_w, 18.0));
    if controls::theme_checkbox(
        ui,
        Id::unique("settings-autostart"),
        row,
        auto_start,
        "Launch at Windows startup",
        &theme,
    ) {
        bus.draft.lock().unwrap().auto_start = !auto_start;
    }
    y += 18.0;

    // Save (Margin=0,26,0,0): ActionButton, full width, padding 0,10.
    y += 26.0;
    let save_h = 10.0
        + painter
            .layout_no_wrap("Save".to_string(), theme::font(13.0), Color32::WHITE)
            .size()
            .y
        + 10.0;
    let save_rect = Rect::from_min_size(pos2(body_left, y), Vec2::new(body_w, save_h));
    if controls::action_button(
        ui,
        Id::unique("settings-save"),
        save_rect,
        "Save",
        &theme,
        &mut glow,
        now,
    ) {
        let draft = *bus.draft.lock().unwrap();
        bus.push_action(ui.ctx(), WinAction::SettingsSave(draft));
    }

    // Singleton guard (rust on_window_event CloseRequested): never destroy,
    // hide instead (✕ / Save reach us via the action queue).
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        ui.ctx()
            .send_viewport_cmd(egui::viewport::ViewportCommand::CancelClose);
        bus.push_action(ui.ctx(), WinAction::SettingsClose);
    }

    // Hover-glow bookkeeping for this viewport (requests its own repaints).
    glow.end_frame(ui.ctx(), now);
}
