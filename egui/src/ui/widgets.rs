// Reusable panel widgets (SPEC 12 / 15.3): capsule progress bar, hover glow
// (animated 1px accent ring + halo, 140ms ease), and the icon-over-label
// action button. All drawing is manual painting so the SPEC's exact geometry
// and per-color alpha (panel fade) stay under our control.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use egui::{
    Color32, Context, CornerRadius, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    Ui, Vec2,
};

use super::theme::{self, Theme};

/// Progress bar: 6px-high capsule, 3px radius; fill = clamped percent (SPEC 11.2).
pub fn progress_bar(painter: &egui::Painter, rect: Rect, percent: f64, theme: &Theme, alpha: f32) {
    painter.rect_filled(
        rect,
        CornerRadius::same(3),
        theme.progress_track.gamma_multiply(alpha),
    );
    let frac = crate::format_util::clamp_percent(percent) / 100.0;
    if frac > 0.0 {
        let fill = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width() * frac as f32, rect.height()),
        );
        painter.rect_filled(
            fill,
            CornerRadius::same(3),
            theme.accent.gamma_multiply(alpha),
        );
    }
}

// ---- Hover glow (SPEC 15.3) ----------------------------------------------

/// CSS `ease` = cubic-bezier(0.25, 0.1, 0.25, 1.0) — the transition curve the
/// rust/ frontend uses for the glow opacity and the background color.
fn css_ease(t: f32) -> f32 {
    cubic_bezier_y(t.clamp(0.0, 1.0), 0.25, 0.1, 0.25, 1.0)
}

fn cubic_bezier_y(x: f32, p1x: f32, p1y: f32, p2x: f32, p2y: f32) -> f32 {
    // Binary-search the curve parameter t with bezier_x(t) = x, then take y(t).
    // B(t) = 3t(1-t)²P1 + 3t²(1-t)P2 + t³P3 with P0=(0,0), P3=(1,1).
    let bx =
        |t: f32| 3.0 * t * (1.0 - t) * (1.0 - t) * p1x + 3.0 * t * t * (1.0 - t) * p2x + t * t * t;
    let by =
        |t: f32| 3.0 * t * (1.0 - t) * (1.0 - t) * p1y + 3.0 * t * t * (1.0 - t) * p2y + t * t * t;
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    for _ in 0..20 {
        let mid = (lo + hi) * 0.5;
        if bx(mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    by((lo + hi) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_ease_endpoints_and_midpoint() {
        assert!((css_ease(0.0) - 0.0).abs() < 1e-4);
        assert!((css_ease(1.0) - 1.0).abs() < 1e-4);
        // CSS ease(0.5) ~= 0.8024.
        assert!((css_ease(0.5) - 0.8024).abs() < 5e-3);
        // Monotonic.
        let mut prev = -1.0;
        for i in 0..=20 {
            let v = css_ease(i as f32 / 20.0);
            assert!(v >= prev, "css_ease not monotonic at {i}");
            prev = v;
        }
    }
}

/// Per-widget hover-glow animation state: progress 0..1 moved at 140ms per
/// full transition; eased with css_ease at draw time.
#[derive(Default)]
pub struct GlowTracker {
    states: HashMap<Id, Glow>,
}

#[derive(Clone, Copy)]
struct Glow {
    p: f32,
    target: f32,
    seen: bool,
    last: Instant,
}

pub const GLOW_DURATION: Duration = Duration::from_millis(140);

impl GlowTracker {
    /// Update the widget's progress and return the eased value to draw with.
    pub fn progress(&mut self, id: Id, hovered: bool, now: Instant) -> f32 {
        let glow = self.states.entry(id).or_insert(Glow {
            p: 0.0,
            target: 0.0,
            seen: true,
            last: now,
        });
        glow.seen = true;
        glow.target = if hovered { 1.0 } else { 0.0 };
        let dt = now.saturating_duration_since(glow.last);
        glow.last = now;
        let step = (dt.as_secs_f32() / GLOW_DURATION.as_secs_f32()).clamp(0.0, 1.0);
        if glow.p < glow.target {
            glow.p = (glow.p + step).min(glow.target);
        } else if glow.p > glow.target {
            glow.p = (glow.p - step).max(glow.target);
        }
        #[cfg(debug_assertions)]
        if std::env::var("KPT_GLOW_DEBUG").is_ok() && id == Id::unique("panel-btn-console") {
            eprintln!(
                "[glow] hovered={hovered} internal={:.3} target={} dt={dt:?} step={step:.3}",
                glow.p, glow.target,
            );
        }
        css_ease(glow.p)
    }

    /// Read-only peek at a widget's internal glow state (debug visualization).
    #[cfg(debug_assertions)]
    pub fn peek(&self, id: Id) -> Option<f32> {
        self.states.get(&id).map(|g| g.p)
    }

    /// Bookkeeping after a frame: drop widgets that vanished fully faded out,
    /// and keep repainting at frame rate while any transition is running.
    /// Returns true when at least one glow is still animating.
    pub fn end_frame(&mut self, ctx: &Context, now: Instant) -> bool {
        let mut animating = false;
        self.states.retain(|_, g| {
            if !g.seen && g.p <= 0.0 {
                return false; // faded out and gone from the panel: forget it
            }
            if g.p != g.target {
                animating = true;
            }
            true
        });
        for g in self.states.values_mut() {
            g.seen = false;
        }
        if animating {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let _ = now;
        animating
    }
}

/// Ring + concentric halo strokes around `rect` (SPEC 15.3): 1px accent ring
/// plus a decaying glow extending `halo_px`; `p` is the eased 0..1 progress.
pub fn hover_glow(painter: &egui::Painter, rect: Rect, radius: u8, theme: &Theme, p: f32) {
    if p <= 0.0 {
        return;
    }
    let (ring_alpha, halo_alpha, halo_px) = theme.glow_params();
    let ring = Stroke::new(1.0, theme.accent.gamma_multiply(ring_alpha * p));
    painter.rect_stroke(rect, CornerRadius::same(radius), ring, StrokeKind::Middle);
    const STEPS: f32 = 4.0;
    for i in 1..=4 {
        let t = i as f32 / STEPS;
        let expand = halo_px * t;
        let a = halo_alpha * (1.0 - t) * p;
        let stroke = Stroke::new(2.0, theme.accent.gamma_multiply(a));
        painter.rect_stroke(
            rect.expand(expand),
            CornerRadius::same((radius as f32 + expand) as u8),
            stroke,
            StrokeKind::Middle,
        );
    }
}

// ---- Buttons --------------------------------------------------------------

/// Paint one bottom action button (icon on top, label below, SPEC 12.7) and
/// return its click response. `alpha` is the panel enter/exit fade factor.
#[allow(clippy::too_many_arguments)]
pub fn action_button(
    ui: &Ui,
    id: Id,
    rect: Rect,
    icon: &TextureHandle,
    label: &str,
    theme: &Theme,
    alpha: f32,
    glow: &mut GlowTracker,
    now: Instant,
) -> egui::Response {
    let response = ui.interact(rect, id, Sense::click());
    let p = glow.progress(id, response.hovered(), now);
    #[cfg(debug_assertions)]
    if p > 0.01 {
        eprintln!("[btn] {id:?} hovered={} p={p:.2}", response.hovered());
    }

    // SPEC 12.7: the panel action buttons rest at CardBgBrush (same color as
    // the cards, to keep the info area dominant) and hover to ButtonHoverBrush
    // — NOT the ActionButton ButtonBg rest color (that is the settings Save
    // button, controls::action_button).
    let bg = theme::mix(theme.card_bg, theme.button_hover, p);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), bg.gamma_multiply(alpha));
    hover_glow(ui.painter(), rect, 10, theme, p * alpha);

    // Icon 16x16 at the top (5px padding), label 12px centered 2px below.
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 5.0 + 8.0),
        Vec2::splat(16.0),
    );
    ui.painter().image(
        icon.id(),
        icon_rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        theme.text_primary.gamma_multiply(alpha),
    );
    ui.painter().text(
        egui::pos2(rect.center().x, icon_rect.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        label,
        theme::font(12.0),
        theme.text_primary.gamma_multiply(alpha),
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

/// Measure the fixed height of the action-button row (icon + gap + label line
/// + vertical padding, SPEC 12.7) so the layout can be computed bottom-up.
pub fn action_row_height(painter: &egui::Painter) -> f32 {
    let label = painter.layout_no_wrap("Refresh".to_string(), theme::font(12.0), Color32::WHITE);
    5.0 + 16.0 + 2.0 + label.size().y + 6.0
}
