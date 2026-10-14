// Shared controls for the secondary windows (SPEC 13.3 + 10.3 + 15.3):
// titlebar (drag + chrome close), ThemeRadio, PillRadio, ThemeCheckBox,
// ActionButton, MenuItemButton and the skills btn-mini. Geometry follows the
// shared.xaml styles 1:1; the hover glow applies only to the controls SPEC 15.3
// lists (ActionButton, menu items, skills Refresh) — radios / checkbox / close
// switch hover state instantly.
//
// The ✕ / ✓ / refresh glyphs are drawn as primitives instead of text: no
// dependency on a system font carrying U+2715 / U+2713 / U+27F3, and pixel
// identical on every machine.

use std::time::Instant;

use egui::{
    epaint::Shadow, pos2, Align2, Color32, Context, CornerRadius, CursorIcon, FontId, Id, Pos2,
    Rect, Sense, Stroke, Ui, Vec2,
};

use super::theme::{self, Theme};
use super::widgets::{hover_glow, GlowTracker};

// ---- Window chrome ---------------------------------------------------------

/// Paint the window shadow + rounded WindowBg card (SPEC 10.2 / 21.1) and
/// return the card rect. `inset` is the shadow fade room (28 for
/// settings/skills, 24 for the menu).
pub fn window_card(
    ui: &Ui,
    inset: f32,
    radius: u8,
    shadow_blur: u8,
    shadow_alpha: u8,
    theme: &Theme,
) -> Rect {
    let card = ui.max_rect().shrink(inset);
    let shadow = Shadow {
        offset: [0, 2],
        blur: shadow_blur,
        spread: 2,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    ui.painter()
        .add(shadow.as_shape(card, CornerRadius::same(radius)));
    ui.painter()
        .rect_filled(card, CornerRadius::same(radius), theme.window_bg);
    card
}

/// Draw the settings/skills titlebar (SPEC 13.1 / 21.1): logo 18x18 + 15px
/// semibold title + 28px chrome close button; the row up to the close button
/// starts a window drag (DragMove equivalent). Returns true when ✕ was clicked.
pub fn titlebar(
    ui: &Ui,
    content: Rect,
    title: &str,
    id_prefix: &str,
    logo: &egui::TextureHandle,
    theme: &Theme,
) -> bool {
    let painter = ui.painter();
    let title_gal =
        painter.layout_no_wrap(title.to_string(), theme::semibold(15.0), theme.text_primary);
    let row_h = 18.0f32.max(title_gal.size().y).max(22.0);
    let row = Rect::from_min_size(
        content.left_top() + Vec2::new(2.0, 0.0),
        Vec2::new(content.width() - 4.0, row_h),
    );

    let logo_rect =
        Rect::from_center_size(pos2(row.left() + 9.0, row.center().y), Vec2::splat(18.0));
    painter.image(
        logo.id(),
        logo_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.galley(
        pos2(
            row.left() + 18.0 + 10.0,
            row.center().y - title_gal.size().y * 0.5,
        ),
        title_gal,
        theme.text_primary,
    );

    // ChromeCloseButton: Width=28, hover bg radius 6, secondary -> primary.
    let close_rect = Rect::from_min_size(
        pos2(row.right() - 28.0, row.center().y - 11.0),
        Vec2::new(28.0, 22.0),
    );
    let close_id = Id::new((id_prefix, "close"));
    let close = ui.interact(close_rect, close_id, Sense::click());
    if close.hovered() {
        painter.rect_filled(close_rect, CornerRadius::same(6), theme.button_hover);
    }
    let cross_col = if close.hovered() {
        theme.text_primary
    } else {
        theme.text_secondary
    };
    draw_x(painter, close_rect.center(), 4.0, cross_col, 1.4);
    if close.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    // Drag region: the row minus the close button (SPEC 13.1 whole-row drag).
    let drag_rect = Rect::from_min_max(row.min, pos2(close_rect.left() - 2.0, row.max.y));
    let drag = ui.interact(drag_rect, Id::new((id_prefix, "drag")), Sense::drag());
    if drag.drag_started() {
        ui.ctx()
            .send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
    }
    close.clicked()
}

/// Draw an ✕ glyph as two diagonal strokes (font-independent ChromeCloseButton).
pub fn draw_x(painter: &egui::Painter, center: Pos2, half: f32, color: Color32, width: f32) {
    let stroke = Stroke::new(width, color);
    painter.line_segment(
        [
            pos2(center.x - half, center.y - half),
            pos2(center.x + half, center.y + half),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x - half, center.y + half),
            pos2(center.x + half, center.y - half),
        ],
        stroke,
    );
}

/// Draw a ✓ glyph as a polyline (ThemeCheckBox checked state).
fn draw_check(painter: &egui::Painter, box_rect: Rect, color: Color32) {
    let p1 = pos2(box_rect.left() + 4.0, box_rect.center().y + 1.0);
    let p2 = pos2(box_rect.center().x - 0.5, box_rect.bottom() - 5.0);
    let p3 = pos2(box_rect.right() - 4.0, box_rect.top() + 5.0);
    painter.add(egui::Shape::line(vec![p1, p2, p3], Stroke::new(2.0, color)));
}

// ---- Settings controls (SPEC 13.3) -----------------------------------------

/// ThemeRadio: 18x18 circle, 1.5px secondary stroke; selected shows an 8x8
/// accent dot; hover flips the TEXT to accent (circle stays). Returns clicked.
pub fn theme_radio(ui: &Ui, id: Id, row: Rect, selected: bool, label: &str, theme: &Theme) -> bool {
    let painter = ui.painter();
    let resp = ui.interact(row, id, Sense::click());
    let c = pos2(row.left() + 9.0, row.center().y);
    painter.circle_stroke(c, 9.0, Stroke::new(1.5, theme.text_secondary));
    if selected {
        painter.circle_filled(c, 4.0, theme.accent);
    }
    let text_col = if resp.hovered() {
        theme.accent
    } else {
        theme.text_primary
    };
    painter.text(
        pos2(row.left() + 18.0 + 8.0, row.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font(13.0),
        text_col,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// PillRadio: padding 12x6, radius 9; unselected ButtonBg + primary text,
/// selected accent + white, hover ButtonHover (selected keeps accent).
/// Returns clicked. `rect` must be pre-measured (see `pill_size`).
pub fn pill_radio(ui: &Ui, id: Id, rect: Rect, selected: bool, label: &str, theme: &Theme) -> bool {
    let painter = ui.painter();
    let resp = ui.interact(rect, id, Sense::click());
    let (bg, fg) = if selected {
        (theme.accent, Color32::WHITE)
    } else if resp.hovered() {
        (theme.button_hover, theme.text_primary)
    } else {
        (theme.button_bg, theme.text_primary)
    };
    painter.rect_filled(rect, CornerRadius::same(9), bg);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::font(12.0),
        fg,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// PillRadio size for a label: padding 12x6 around the 12px text.
pub fn pill_size(painter: &egui::Painter, label: &str) -> Vec2 {
    let gal = painter.layout_no_wrap(label.to_string(), theme::font(12.0), Color32::WHITE);
    Vec2::new(gal.size().x + 24.0, gal.size().y + 12.0)
}

/// ThemeCheckBox: 18x18 square radius 5, 1.5px secondary border; checked =
/// accent fill + white check; text 13px, 8px gap. Returns clicked.
pub fn theme_checkbox(
    ui: &Ui,
    id: Id,
    row: Rect,
    checked: bool,
    label: &str,
    theme: &Theme,
) -> bool {
    let painter = ui.painter();
    let resp = ui.interact(row, id, Sense::click());
    let box_rect = Rect::from_min_size(row.left_top(), Vec2::splat(18.0));
    if checked {
        painter.rect_filled(box_rect, CornerRadius::same(5), theme.accent);
        draw_check(painter, box_rect, Color32::WHITE);
    } else {
        painter.rect_stroke(
            box_rect,
            CornerRadius::same(5),
            Stroke::new(1.5, theme.text_secondary),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        pos2(box_rect.right() + 8.0, row.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font(13.0),
        theme.text_primary,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// ActionButton (Save): full width, padding 0,10, radius 10, ButtonBg ->
/// ButtonHover on hover + SPEC 15.3 glow, 13px primary text. Returns clicked.
pub fn action_button(
    ui: &Ui,
    id: Id,
    rect: Rect,
    label: &str,
    theme: &Theme,
    glow: &mut GlowTracker,
    now: Instant,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let p = glow.progress(id, resp.hovered(), now);
    let bg = theme::mix(theme.button_bg, theme.button_hover, p);
    ui.painter().rect_filled(rect, CornerRadius::same(10), bg);
    hover_glow(ui.painter(), rect, 10, theme, p);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::font(13.0),
        theme.text_primary,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

// ---- Tray menu item (SPEC 13.3 MenuItemButton + 10.3) ----------------------

/// MenuItemButton: padding 14,9, radius 8, transparent -> ButtonHover + glow on
/// hover, 13px primary text, left-aligned. Returns clicked.
pub fn menu_item(
    ui: &Ui,
    id: Id,
    rect: Rect,
    label: &str,
    theme: &Theme,
    glow: &mut GlowTracker,
    now: Instant,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let p = glow.progress(id, resp.hovered(), now);
    if p > 0.0 {
        let bg = theme.button_hover.gamma_multiply(p);
        ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
        hover_glow(ui.painter(), rect, 8, theme, p);
    }
    ui.painter().text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font(13.0),
        theme.text_primary,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// Height of one menu item: 9px padding above/below the 13px text line.
pub fn menu_item_height(painter: &egui::Painter) -> f32 {
    let gal = painter.layout_no_wrap("Refresh".to_string(), theme::font(13.0), Color32::WHITE);
    9.0 + gal.size().y + 9.0
}

// ---- Skills mini button (SPEC 21.3 btn-mini) --------------------------------

/// btn-mini: padding 4,10, radius 8, ButtonBg -> ButtonHover + glow, 12px
/// primary text, small refresh glyph. Returns clicked.
pub fn btn_mini(
    ui: &Ui,
    id: Id,
    rect: Rect,
    label: &str,
    theme: &Theme,
    glow: &mut GlowTracker,
    now: Instant,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let p = glow.progress(id, resp.hovered(), now);
    let bg = theme::mix(theme.button_bg, theme.button_hover, p);
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    hover_glow(ui.painter(), rect, 8, theme, p);
    // Small refresh glyph (12x12): a 300-degree stroke arc + arrowhead, drawn
    // as primitives so no font has to carry U+27F3.
    let c = pos2(rect.left() + 10.0 + 5.0, rect.center().y);
    let r = 4.0;
    let mut pts = Vec::with_capacity(13);
    for i in 0..=12 {
        let a = (-60.0f32 + 300.0 * i as f32 / 12.0).to_radians();
        pts.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
    }
    ui.painter()
        .add(egui::Shape::line(pts, Stroke::new(1.4, theme.text_primary)));
    let a0 = (-60.0f32).to_radians();
    let tip = pos2(c.x + (r + 2.5) * a0.cos(), c.y + (r + 2.5) * a0.sin());
    let b1 = pos2(
        c.x + (r - 1.5) * (a0 - 0.9).cos(),
        c.y + (r - 1.5) * (a0 - 0.9).sin(),
    );
    let b2 = pos2(
        c.x + (r - 1.5) * (a0 + 0.9).cos(),
        c.y + (r - 1.5) * (a0 + 0.9).sin(),
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![tip, b1, b2],
        theme.text_primary,
        Stroke::NONE,
    ));
    ui.painter().text(
        pos2(rect.left() + 10.0 + 12.0 + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font(12.0),
        theme.text_primary,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// btn-mini size for a label: padding 4,10 + glyph 12 + 4 gap + text.
pub fn btn_mini_size(painter: &egui::Painter, label: &str) -> Vec2 {
    let gal = painter.layout_no_wrap(label.to_string(), theme::font(12.0), Color32::WHITE);
    Vec2::new(10.0 + 12.0 + 4.0 + gal.size().x + 10.0, gal.size().y + 8.0)
}

// ---- Text truncation (qt clamplabel equivalents) -----------------------------

/// Single-line ellipsis: shorten with U+2026 until the line fits `max_w`.
pub fn ellipsize_single_line(ctx: &Context, text: &str, font_id: &FontId, max_w: f32) -> String {
    let width = |s: &str| {
        ctx.fonts_mut(|f| f.layout_no_wrap(s.to_owned(), font_id.clone(), Color32::WHITE))
            .size()
            .x
    };
    if width(text) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let mut cand: String = chars[..mid].iter().collect();
        cand.push('\u{2026}');
        if width(&cand) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = chars[..lo].iter().collect();
    out.push('\u{2026}');
    out
}

/// Multi-line clamp: wrap `text` at `wrap_width` and shorten with U+2026 until
/// it fits in `max_rows` rows (the 2-line description clamp, SPEC 21.3).
pub fn clamp_rows(
    ctx: &Context,
    text: &str,
    font_id: &FontId,
    wrap_width: f32,
    max_rows: usize,
) -> String {
    let rows = |s: &str| {
        ctx.fonts_mut(|f| f.layout(s.to_owned(), font_id.clone(), Color32::WHITE, wrap_width))
            .rows
            .len()
    };
    if rows(text) <= max_rows {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let mut cand: String = chars[..mid].iter().collect();
        cand.push('\u{2026}');
        if rows(&cand) <= max_rows {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = chars[..lo].iter().collect();
    out.push('\u{2026}');
    out
}
