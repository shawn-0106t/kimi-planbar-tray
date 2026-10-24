// Skills window (SPEC 21): 404x520 viewport, same chrome pattern as settings,
// read-only grouped skill list. Data comes from the AppState skills cache —
// the scan itself is triggered by app.rs on open (once) or on Refresh.
//
// Everything renders through plain painter text (the textContent-only rule:
// skill names/descriptions are external input). Long names ellipsize on one
// line and descriptions clamp to two rows (qt clamplabel equivalents), with
// the full description on the card hover tooltip.

use std::sync::Arc;
use std::time::Instant;

use egui::{pos2, Align2, Color32, CornerRadius, Id, Rect, ScrollArea, Sense, Ui, Vec2};

use crate::app::WinAction;
use crate::app::WinBus;
use crate::ctx::AppCtx;
use crate::skills::SkillInfo;

use super::controls;
use super::theme::{self, Theme};

/// Paint one skills pass.
pub fn paint(ui: &mut Ui, biz: &AppCtx, bus: &Arc<WinBus>) {
    let theme = Theme::from_name(&biz.state.effective_theme.read().unwrap().clone());
    let now = Instant::now();

    let card = controls::window_card(ui, 28.0, 14, 24, 64, &theme);
    let content = card.shrink(16.0);

    let mut glow = bus.skills_glow.lock().unwrap();

    // ---- Titlebar (SPEC 21.1) ----------------------------------------------
    let logo = bus
        .icons
        .get()
        .expect("icons installed on the first root frame");
    if controls::titlebar(
        ui,
        content,
        "Kimi Skills",
        "skills-titlebar",
        &logo.logo,
        &theme,
    ) {
        bus.push_action(ui.ctx(), WinAction::SkillsClose);
    }

    // ---- Summary row (SPEC 21.3): "N skills" left, Refresh right ------------
    let painter = ui.painter().clone();
    let skills = biz.state.skills_cache.read().unwrap().clone();
    let summary_text = match &skills {
        None => "Loading\u{2026}".to_string(), // first-open scan in flight
        Some(v) if v.is_empty() => "No skills found".to_string(),
        Some(v) => format!("{} skills", v.len()),
    };
    let summary_y = content.top() + 22.0 + 16.0;
    let btn_size = controls::btn_mini_size(&painter, "Refresh");
    let summary_left = content.left() + 6.0;
    let summary_right = content.right() - 4.0;
    let btn_rect = Rect::from_min_size(pos2(summary_right - btn_size.x, summary_y), btn_size);
    if controls::btn_mini(
        ui,
        Id::unique("skills-refresh"),
        btn_rect,
        "Refresh",
        &theme,
        &mut glow,
        now,
    ) {
        bus.push_action(ui.ctx(), WinAction::SkillsRefresh);
    }
    painter.text(
        pos2(summary_left, btn_rect.center().y),
        Align2::LEFT_CENTER,
        summary_text,
        theme::font(12.0),
        theme.text_secondary,
    );

    // ---- Grouped scrollable list (SPEC 21.3) --------------------------------
    let list_rect = Rect::from_min_max(
        pos2(summary_left, btn_rect.bottom() + 10.0),
        pos2(summary_right, content.bottom()),
    );
    let mut list_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(list_rect)
            .id_salt("skills-list-area"),
    );
    // Themed subtle scrollbar: 8px, progress_track handle, transparent track
    // (rust/ skills.css ::-webkit-scrollbar rules). Non-floating handles paint
    // with the WidgetVisuals bg_fill; the track with extreme_bg_color.
    {
        let style = list_ui.style_mut();
        style.spacing.scroll.floating = false;
        style.spacing.scroll.foreground_color = false;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.bar_outer_margin = 0.0;
        style.visuals.extreme_bg_color = Color32::TRANSPARENT; // track
        for w in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            w.bg_fill = theme.progress_track; // handle
            w.corner_radius = CornerRadius::same(4);
        }
    }

    ScrollArea::vertical()
        .id_salt("skills-scroll")
        .auto_shrink([false, false])
        .show(&mut list_ui, |ui| {
            // The closure's painter carries the scroll transform — content is
            // painted in unscrolled coordinates and egui shifts it.
            let painter = ui.painter().clone();
            let inner_w = ui.max_rect().width();
            let top = ui.max_rect().left_top();
            let mut y = top.y;
            if let Some(list) = &skills {
                let mut last_source = String::new();
                let group_h = painter
                    .layout_no_wrap("Ag".to_string(), theme::semibold(12.0), Color32::WHITE)
                    .size()
                    .y;
                for (i, skill) in list.iter().enumerate() {
                    if skill.source != last_source {
                        last_source = skill.source.clone();
                        // group-title: margin 12,2,6,2 (first: 2,2,6,2)
                        y += if y == top.y { 2.0 } else { 12.0 };
                        painter.text(
                            pos2(top.x + 2.0, y),
                            Align2::LEFT_TOP,
                            &skill.source,
                            theme::semibold(12.0),
                            theme.text_secondary,
                        );
                        y += group_h + 6.0;
                    }
                    y = paint_skill_card(ui, &painter, i, pos2(top.x, y), inner_w, skill, &theme);
                }
            }
            ui.set_min_width(inner_w);
            ui.set_min_height(y - top.y);
        });

    // Singleton guard (same as settings: hide, never destroy).
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        ui.ctx()
            .send_viewport_cmd(egui::viewport::ViewportCommand::CancelClose);
        bus.push_action(ui.ctx(), WinAction::SkillsClose);
    }

    glow.end_frame(ui.ctx(), now);
}

/// One skill card (SPEC 21.3): radius 10, padding 10x12, 6px gap below; name
/// 13px semibold single-line ellipsis, description 12px clamped to 2 rows,
/// full description on hover tooltip. Returns the next y.
fn paint_skill_card(
    ui: &Ui,
    painter: &egui::Painter,
    index: usize,
    at: egui::Pos2,
    width: f32,
    skill: &SkillInfo,
    theme: &Theme,
) -> f32 {
    let pad_x = 12.0;
    let pad_y = 10.0;
    let inner_w = width - 2.0 * pad_x;
    let name_h = painter
        .layout_no_wrap("Ag".to_string(), theme::semibold(13.0), Color32::WHITE)
        .size()
        .y;
    let desc = if skill.description.is_empty() {
        "(no description)".to_string()
    } else {
        controls::clamp_rows(ui.ctx(), &skill.description, &theme::font(12.0), inner_w, 2)
    };
    let desc_gal = painter.layout(desc, theme::font(12.0), theme.text_secondary, inner_w);
    let desc_h = desc_gal.size().y;
    let card_h = pad_y + name_h + 4.0 + desc_h + pad_y;
    let card_rect = Rect::from_min_size(at, Vec2::new(width, card_h));

    painter.rect_filled(card_rect, CornerRadius::same(10), theme.card_bg);

    let name =
        controls::ellipsize_single_line(ui.ctx(), &skill.name, &theme::semibold(13.0), inner_w);
    painter.text(
        pos2(at.x + pad_x, at.y + pad_y),
        Align2::LEFT_TOP,
        name,
        theme::semibold(13.0),
        theme.text_primary,
    );
    painter.galley(
        pos2(at.x + pad_x, at.y + pad_y + name_h + 4.0),
        desc_gal,
        theme.text_secondary,
    );

    // Card hover tooltip carries the full (unclamped) description.
    let resp = ui.interact(
        card_rect,
        Id::unique(("skills-card", index)),
        Sense::hover(),
    );
    if !skill.description.is_empty() {
        resp.on_hover_text(skill.description.clone());
    }

    card_rect.bottom() + 6.0 // margin-bottom 6
}
