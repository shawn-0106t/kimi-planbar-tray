// Tray menu window (SPEC 10.3 + 14): 188-logical-wide viewport, height =
// content + 2*24px shadow room (computed in Rust, see panel_pos
// ::menu_window_height — the rust/ edition's frontend height-report becomes a
// direct text measurement). Five MenuItemButton items; closes on focus loss
// (deferred command = the SPEC 10.3 delayed-safe form); menu item clicks hide
// the menu first, then dispatch through the WinBus (app.rs executes).
//
// Positioning (cursor clamp + bottom flip + the -19px legacy margin
// compensation) lives in panel_pos::show_menu_at — a port of rust panel.rs
// show_menu; the UiCmd::ShowMenu path runs it in the root pass.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use egui::{pos2, Id, Rect, Ui, Vec2};

use crate::app::WinAction;
use crate::app::WinBus;
use crate::ctx::AppCtx;

use super::controls;
use super::theme::Theme;
use super::widgets::GlowTracker;

/// The five menu items in SPEC order ("Skills" sits between Settings and Exit).
const ITEMS: [(&str, WinAction); 5] = [
    ("Open", WinAction::MenuOpen),
    ("Refresh", WinAction::MenuRefresh),
    ("Settings", WinAction::MenuSettings),
    ("Skills", WinAction::MenuSkills),
    ("Exit", WinAction::MenuQuit),
];

/// Paint one menu pass.
pub fn paint(ui: &mut Ui, biz: &AppCtx, bus: &Arc<WinBus>) {
    let theme = Theme::from_name(&biz.state.effective_theme.read().unwrap().clone());
    let now = Instant::now();

    // SPEC 10.3: Margin=5 WPF -> 24px shadow fade room, CornerRadius=12,
    // Padding=4, shadow Blur 20 / Depth 2 / Opacity 0.3.
    let card = controls::window_card(ui, 24.0, 12, 20, 77, &theme);

    let mut glow: std::sync::MutexGuard<'_, GlowTracker> = bus.menu_glow.lock().unwrap();
    let item_h = controls::menu_item_height(ui.painter());
    let item_w = card.width() - 8.0; // card padding 4 per side
    let mut y = card.top() + 4.0;
    for (i, (label, action)) in ITEMS.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(card.left() + 4.0, y), Vec2::new(item_w, item_h));
        if controls::menu_item(
            ui,
            Id::new(("menu-item", i)),
            rect,
            label,
            &theme,
            &mut glow,
            now,
        ) {
            // Menu items close the menu first, then act (SPEC 14). The hide is
            // a queued command; the action reaches app.rs via the wake below.
            bus.menu_open.store(false, Ordering::SeqCst);
            ui.ctx().send_viewport_cmd_to(
                crate::panel_pos::vp_menu(),
                egui::viewport::ViewportCommand::Visible(false),
            );
            bus.push_action(ui.ctx(), *action);
        }
        y += item_h;
    }

    // SPEC 10.3: close on focus loss. The Visible(false) command is executed
    // after this pass, which is the delayed-safe form (the WPF edition queued
    // Close via Dispatcher.BeginInvoke to dodge a synchronous re-entrancy
    // crash; egui commands are always deferred, so there is no crash window).
    // The was-focused latch guards against the never-focused reporting trap
    // (Phase 2 gotcha 5: a shown-but-never-focused window reads Some(false)).
    let focused = ui.ctx().input(|i| i.viewport().focused);
    if focused == Some(true) {
        bus.menu_was_focused.store(true, Ordering::SeqCst);
    }
    if bus.menu_open.load(Ordering::SeqCst)
        && bus.menu_was_focused.load(Ordering::SeqCst)
        && focused == Some(false)
    {
        bus.menu_open.store(false, Ordering::SeqCst);
        bus.menu_was_focused.store(false, Ordering::SeqCst);
        ui.ctx().send_viewport_cmd_to(
            crate::panel_pos::vp_menu(),
            egui::viewport::ViewportCommand::Visible(false),
        );
    }

    // Alt+F4 on a focused menu: cancel the close, hide instead (singleton).
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        ui.ctx()
            .send_viewport_cmd(egui::viewport::ViewportCommand::CancelClose);
        bus.menu_open.store(false, Ordering::SeqCst);
        ui.ctx().send_viewport_cmd_to(
            crate::panel_pos::vp_menu(),
            egui::viewport::ViewportCommand::Visible(false),
        );
    }

    glow.end_frame(ui.ctx(), now);
}
