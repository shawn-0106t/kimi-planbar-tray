// Main panel: the full SPEC 12 layout drawn with painter primitives, plus the
// SPEC 15 enter/exit animation math and the hide state machine.
//
// Layout mirrors the rust/ frontend's flex box, computed bottom-up: the
// action row, version row and Extra card anchor to the content bottom, the
// two usage cards absorb the remaining height (CSS flex:1).

use std::time::{Duration, Instant};

use egui::{epaint::Shadow, Align2, Color32, CornerRadius, CursorIcon, Id, Rect, Sense, Ui, Vec2};

use crate::format_util;
use crate::icons::Icons;
use crate::quota::{ExtraState, QuotaResult, QuotaSegment};
use crate::update::UpdateStatus;

use super::theme::{self, Theme};
use super::widgets::{self, GlowTracker};

// SPEC 15.1: fade 160ms linear + translateY 16->0 over 220ms CubicEase-out,
// both starting together.
pub const ENTER_FADE_SECS: f32 = 0.160;
pub const ENTER_MOVE_SECS: f32 = 0.220;
pub const ENTER_OFFSET: f32 = 16.0;
// SPEC 15.2: fade 130ms linear + translateY ->12 over 160ms CubicEase-in.
pub const EXIT_FADE_SECS: f32 = 0.130;
pub const EXIT_MOVE_SECS: f32 = 0.160;
pub const EXIT_OFFSET: f32 = 12.0;
/// The window actually hides when the exit animation is done; the rust/
/// frontend fires its completion timer at 170ms (both exit durations + slack).
pub const EXIT_DONE_SECS: f32 = 0.170;

/// Panel visibility state machine (port of rust/ panel.rs + frontend anims).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PanelState {
    /// Invisible; no repaints scheduled.
    Hidden,
    /// Slide-in playing since `t0`.
    Showing { t0: Instant },
    /// Slide-in finished; idle-visible.
    Shown,
    /// Slide-out playing since `t0`; the window hides when it completes.
    Hiding { t0: Instant },
}

impl PanelState {
    pub fn is_visible(&self) -> bool {
        !matches!(self, PanelState::Hidden)
    }

    /// (alpha, y_offset) to draw the panel with right now.
    pub fn appearance(&self, now: Instant) -> (f32, f32) {
        match *self {
            PanelState::Hidden => (0.0, ENTER_OFFSET),
            PanelState::Showing { t0 } => {
                let t = now.duration_since(t0).as_secs_f32();
                let alpha = (t / ENTER_FADE_SECS).clamp(0.0, 1.0);
                let u = (t / ENTER_MOVE_SECS).clamp(0.0, 1.0);
                // CubicEase EaseOut: 1-(1-t)^3 -> remaining offset = 16*(1-u)^3
                (alpha, ENTER_OFFSET * (1.0 - u).powi(3))
            }
            PanelState::Shown => (1.0, 0.0),
            PanelState::Hiding { t0 } => {
                let t = now.duration_since(t0).as_secs_f32();
                let alpha = 1.0 - (t / EXIT_FADE_SECS).clamp(0.0, 1.0);
                let u = (t / EXIT_MOVE_SECS).clamp(0.0, 1.0);
                // CubicEase EaseIn: t^3
                (alpha, EXIT_OFFSET * u.powi(3))
            }
        }
    }

    /// Duration until the current animation ends, if one is playing.
    pub fn animating_for(&self, now: Instant) -> Option<Duration> {
        match *self {
            PanelState::Showing { t0 } => Some(
                Duration::from_secs_f32(ENTER_MOVE_SECS).saturating_sub(now.duration_since(t0)),
            ),
            PanelState::Hiding { t0 } => {
                Some(Duration::from_secs_f32(EXIT_DONE_SECS).saturating_sub(now.duration_since(t0)))
            }
            _ => None,
        }
    }
}

/// Actions the paint pass collected from clicks; app.rs executes them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PanelAction {
    OpenReleases,
    OpenConsole,
    Refresh,
    OpenSettings,
    OpenSkills,
    Quit,
}

/// Extra-card monthly sub-panel values (used, limit) when the SPEC 12.4
/// condition holds: MonthlyEnabled && MonthlyLimitCents > 0 && used present.
fn monthly_values(extra: Option<&crate::quota::ExtraInfo>) -> Option<(i64, i64)> {
    let extra = extra?;
    if !extra.monthly_enabled {
        return None;
    }
    let limit = extra.monthly_limit_cents?;
    if limit <= 0 {
        return None;
    }
    let used = extra.monthly_used_cents?;
    Some((used, limit))
}

pub struct PanelPaint<'a> {
    pub theme: &'a Theme,
    pub alpha: f32,
    pub y_off: f32,
    pub icons: &'a Icons,
}

/// Paint the whole panel (window shadow + card + SPEC 12 content) and collect
/// click actions. Skipped entirely while the panel is Hidden.
pub fn paint(
    ui: &mut Ui,
    p: &PanelPaint,
    quota: Option<&QuotaResult>,
    update: &UpdateStatus,
    glow: &mut GlowTracker,
    now: Instant,
) -> Vec<PanelAction> {
    let mut actions = Vec::new();
    let painter = ui.painter().clone();
    let theme = p.theme;
    let alpha = p.alpha;

    let full = ui.max_rect();
    let card = full
        .shrink(crate::panel_pos::CARD_INSET)
        .translate(Vec2::new(0.0, p.y_off));
    let content = card.shrink(16.0);

    // SPEC 10.1 window: DropShadowEffect Blur 24 / Depth 2 / Opacity 0.25 on a
    // 14px-rounded card; the window background is transparent around it.
    let shadow = Shadow {
        offset: [0, 2],
        blur: 24,
        spread: 2,
        color: Color32::from_black_alpha(64).gamma_multiply(alpha),
    };
    painter.add(shadow.as_shape(card, CornerRadius::same(14)));
    painter.rect_filled(
        card,
        CornerRadius::same(14),
        theme.window_bg.gamma_multiply(alpha),
    );

    // ---- Header (SPEC 12.1): logo 20x20 + title 17 semibold + right status
    let title_gal = painter.layout_no_wrap(
        "Kimi Planbar Tray".to_string(),
        theme::semibold(17.0),
        theme.text_primary.gamma_multiply(alpha),
    );
    let updated_text = match quota {
        None => String::new(),
        Some(q) if q.error.is_some() => "Update failed".to_string(),
        Some(q) => format_util::fmt_updated(q.fetched_at),
    };
    let updated_gal = painter.layout_no_wrap(
        updated_text.clone(),
        theme::font(11.0),
        theme.text_secondary.gamma_multiply(alpha),
    );
    let hdr_h = 20.0f32
        .max(title_gal.size().y)
        .max(if updated_text.is_empty() {
            0.0
        } else {
            updated_gal.size().y
        });
    let header = Rect::from_min_size(
        content.left_top() + Vec2::new(2.0, 0.0),
        Vec2::new(content.width() - 4.0, hdr_h),
    );
    let logo_rect = Rect::from_center_size(
        egui::pos2(header.left() + 10.0, header.center().y),
        Vec2::splat(20.0),
    );
    painter.image(
        p.icons.logo.id(),
        logo_rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE.gamma_multiply(alpha),
    );
    painter.galley(
        egui::pos2(
            header.left() + 20.0 + 10.0,
            header.center().y - title_gal.size().y * 0.5,
        ),
        title_gal,
        theme.text_primary.gamma_multiply(alpha),
    );
    if !updated_text.is_empty() {
        painter.galley(
            egui::pos2(
                header.right() - updated_gal.size().x,
                header.center().y - updated_gal.size().y * 0.5,
            ),
            updated_gal,
            theme.text_secondary.gamma_multiply(alpha),
        );
    }
    let cards_top = content.top() + hdr_h + 16.0;

    // ---- Bottom-up rows: actions, version, extra, then usage cards
    let text13_h = painter
        .layout_no_wrap(
            "Kimi Code CLI".to_string(),
            theme::font(13.0),
            Color32::WHITE,
        )
        .size()
        .y;
    let balance_h = painter
        .layout_no_wrap("--".to_string(), theme::bold(18.0), Color32::WHITE)
        .size()
        .y;
    let label12_h = widgets::action_row_height(&painter);

    let actions_h = label12_h;
    let actions_row = Rect::from_min_max(
        egui::pos2(content.left(), content.bottom() - actions_h),
        egui::pos2(content.right(), content.bottom()),
    );

    let version_h = 10.0 * 2.0 + text13_h;
    let version_row = Rect::from_min_max(
        egui::pos2(content.left(), actions_row.top() - 12.0 - version_h),
        egui::pos2(content.right(), actions_row.top() - 12.0),
    );

    let mut extra_h = 12.0 * 2.0 + balance_h.max(text13_h);
    let extra_ref = quota.and_then(|q| q.extra.as_ref());
    let monthly = monthly_values(extra_ref);
    if let Some((used, limit)) = monthly {
        let text = format!(
            "Used {} this month / {} limit",
            format_util::fmt_yuan(used),
            format_util::fmt_yuan(limit)
        );
        let text11_h = painter
            .layout_no_wrap(text, theme::font(11.0), Color32::WHITE)
            .size()
            .y;
        extra_h += 8.0 + 6.0 + 8.0 + text11_h;
    }
    let extra_row = Rect::from_min_max(
        egui::pos2(content.left(), version_row.top() - 12.0 - extra_h),
        egui::pos2(content.right(), version_row.top() - 12.0),
    );

    let cards_row = Rect::from_min_max(
        egui::pos2(content.left(), cards_top),
        egui::pos2(content.right(), extra_row.top() - 12.0),
    );
    let card_w = (cards_row.width() - 12.0) / 2.0;
    let week_rect = Rect::from_min_max(
        cards_row.left_top(),
        egui::pos2(cards_row.left() + card_w, cards_row.bottom()),
    );
    let five_rect = Rect::from_min_max(
        egui::pos2(cards_row.right() - card_w, cards_row.top()),
        cards_row.right_bottom(),
    );

    usage_card(
        &painter,
        week_rect,
        "Weekly usage",
        quota.and_then(|q| q.week.as_ref()),
        theme,
        alpha,
    );
    usage_card(
        &painter,
        five_rect,
        "5-hour usage",
        quota.and_then(|q| q.five_hour.as_ref()),
        theme,
        alpha,
    );
    extra_card(&painter, extra_row, extra_ref, theme, alpha);

    // ---- Version row (SPEC 12.6): whole-row clickable card
    let version_id = Id::new("panel-version-row");
    let v_resp = ui.interact(version_row, version_id, Sense::click());
    let v_glow = glow.progress(version_id, v_resp.hovered(), now);
    painter.rect_filled(
        version_row,
        CornerRadius::same(12),
        theme.card_bg.gamma_multiply(alpha),
    );
    widgets::hover_glow(&painter, version_row, 12, theme, v_glow * alpha);
    painter.text(
        egui::pos2(content.left() + 14.0, version_row.center().y),
        Align2::LEFT_CENTER,
        "Kimi Code CLI",
        theme::font(13.0),
        theme.text_primary.gamma_multiply(alpha),
    );
    let cli_text = update
        .local_version
        .clone()
        .unwrap_or_else(|| "Not detected".to_string());
    let mut right_x = version_row.right() - 14.0;
    if update.update_available {
        let badge_gal = painter.layout_no_wrap(
            "Update available".to_string(),
            theme::font(11.0),
            theme.badge_fg.gamma_multiply(alpha),
        );
        let badge_w = badge_gal.size().x + 16.0; // Padding=8,2
        let badge_h = badge_gal.size().y + 4.0;
        let badge_rect = Rect::from_min_size(
            egui::pos2(right_x - badge_w, version_row.center().y - badge_h * 0.5),
            Vec2::new(badge_w, badge_h),
        );
        painter.rect_filled(
            badge_rect,
            CornerRadius::same(8),
            theme.badge_bg.gamma_multiply(alpha),
        );
        painter.galley(
            egui::pos2(
                badge_rect.left() + 8.0,
                badge_rect.center().y - badge_gal.size().y * 0.5,
            ),
            badge_gal,
            theme.badge_fg.gamma_multiply(alpha),
        );
        right_x -= badge_w + 8.0; // Margin=8,0,0,0
    }
    let cli_gal = painter.layout_no_wrap(
        cli_text.clone(),
        theme::font(13.0),
        theme.text_secondary.gamma_multiply(alpha),
    );
    #[cfg(debug_assertions)]
    if std::env::var("KPT_GLOW_DEBUG").is_ok() {
        eprintln!(
            "[vrow] cli_text={cli_text:?} size={:?} right_x={right_x} row={version_row:?}",
            cli_gal.size()
        );
    }
    painter.galley(
        egui::pos2(
            right_x - cli_gal.size().x,
            version_row.center().y - cli_gal.size().y * 0.5,
        ),
        cli_gal,
        theme.text_secondary.gamma_multiply(alpha),
    );
    if v_resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if v_resp.clicked() {
        actions.push(PanelAction::OpenReleases);
    }

    // ---- Bottom buttons (SPEC 12.7): four equal buttons, gaps 9/6/9 px
    // (WPF margins don't collapse: 6+3 / 3+3 / 3+6).
    let bw = (actions_row.width() - 24.0) / 4.0;
    let defs: [(&str, &str, &egui::TextureHandle, f32); 4] = [
        ("panel-btn-console", "Console", &p.icons.console, 0.0),
        ("panel-btn-refresh", "Refresh", &p.icons.refresh, bw + 9.0),
        (
            "panel-btn-settings",
            "Settings",
            &p.icons.settings,
            bw * 2.0 + 15.0,
        ),
        ("panel-btn-exit", "Exit", &p.icons.exit, bw * 3.0 + 24.0),
    ];
    for (name, label, icon, dx) in defs {
        let rect = Rect::from_min_size(
            egui::pos2(content.left() + dx, actions_row.top()),
            Vec2::new(bw, actions_h),
        );
        let resp = widgets::action_button(
            ui,
            Id::new(name),
            rect,
            icon,
            label,
            theme,
            alpha,
            glow,
            now,
        );
        if resp.clicked() {
            actions.push(match name {
                "panel-btn-console" => PanelAction::OpenConsole,
                "panel-btn-refresh" => PanelAction::Refresh,
                "panel-btn-settings" => PanelAction::OpenSettings,
                _ => PanelAction::Quit,
            });
        }
    }

    actions
}

/// One usage card (SPEC 12.2): title 13 secondary, big percent 32 bold (raw
/// value), capsule bar on the clamped value, reset countdown 11 secondary.
fn usage_card(
    painter: &egui::Painter,
    rect: Rect,
    title: &str,
    seg: Option<&QuotaSegment>,
    theme: &Theme,
    alpha: f32,
) {
    painter.rect_filled(
        rect,
        CornerRadius::same(12),
        theme.card_bg.gamma_multiply(alpha),
    );
    let inner = rect.shrink(16.0);
    let title_gal = painter.layout_no_wrap(
        title.to_string(),
        theme::font(13.0),
        theme.text_secondary.gamma_multiply(alpha),
    );
    let title_h = title_gal.size().y;
    painter.galley(inner.left_top(), title_gal, theme.text_secondary);

    let pct_text = seg
        .map(|s| format_util::fmt_percent(s.percent))
        .unwrap_or_else(|| "--".to_string());
    let pct_gal = painter.layout_no_wrap(
        pct_text,
        theme::bold(32.0),
        theme.text_primary.gamma_multiply(alpha),
    );
    let pct_h = pct_gal.size().y;
    let pct_y = inner.top() + title_h + 10.0; // Margin=0,10,0,10
    painter.galley(egui::pos2(inner.left(), pct_y), pct_gal, theme.text_primary);

    let bar_y = pct_y + pct_h + 10.0;
    let bar_rect = Rect::from_min_size(
        egui::pos2(inner.left(), bar_y),
        Vec2::new(inner.width(), 6.0),
    );
    widgets::progress_bar(
        painter,
        bar_rect,
        seg.map(|s| s.percent).unwrap_or(0.0),
        theme,
        alpha,
    );

    if let Some(reset) = seg.and_then(|s| s.reset_at).map(format_util::format_reset) {
        painter.text(
            egui::pos2(inner.left(), bar_rect.bottom() + 12.0), // Margin=0,12,0,0
            Align2::LEFT_TOP,
            reset,
            theme::font(11.0),
            theme.text_secondary.gamma_multiply(alpha),
        );
    }
}

/// Extra Usage card (SPEC 12.4): title + balance row, three balance states,
/// optional monthly sub-panel (bar + text).
fn extra_card(
    painter: &egui::Painter,
    rect: Rect,
    extra: Option<&crate::quota::ExtraInfo>,
    theme: &Theme,
    alpha: f32,
) {
    painter.rect_filled(
        rect,
        CornerRadius::same(12),
        theme.card_bg.gamma_multiply(alpha),
    );
    let balance_text = match extra {
        None => "--".to_string(),
        Some(e) => match e.state {
            ExtraState::Ready => e
                .balance_cents
                .map(format_util::fmt_yuan)
                .unwrap_or_else(|| "--".to_string()),
            ExtraState::NoData => "No data".to_string(),
            ExtraState::NotActivated => "Not activated".to_string(),
        },
    };
    let title_gal = painter.layout_no_wrap(
        "Extra Usage".to_string(),
        theme::font(13.0),
        theme.text_secondary.gamma_multiply(alpha),
    );
    let balance_gal = painter.layout_no_wrap(
        balance_text,
        theme::bold(18.0),
        theme.text_primary.gamma_multiply(alpha),
    );
    // Row: align-items center; height = max(title, balance).
    let row_h = title_gal.size().y.max(balance_gal.size().y);
    let row_center = egui::pos2(0.0, rect.top() + 12.0 + row_h * 0.5);
    painter.galley(
        egui::pos2(rect.left() + 16.0, row_center.y - title_gal.size().y * 0.5),
        title_gal,
        theme.text_secondary,
    );
    painter.galley(
        egui::pos2(
            rect.right() - 16.0 - balance_gal.size().x,
            row_center.y - balance_gal.size().y * 0.5,
        ),
        balance_gal,
        theme.text_primary,
    );

    if let Some((used, limit)) = monthly_values(extra) {
        let bar_rect = Rect::from_min_size(
            egui::pos2(rect.left() + 16.0, rect.top() + 12.0 + row_h + 8.0),
            Vec2::new(rect.width() - 32.0, 6.0),
        );
        widgets::progress_bar(
            painter,
            bar_rect,
            used as f64 / limit as f64 * 100.0,
            theme,
            alpha,
        );
        let text = format!(
            "Used {} this month / {} limit",
            format_util::fmt_yuan(used),
            format_util::fmt_yuan(limit)
        );
        painter.text(
            egui::pos2(rect.left() + 16.0, bar_rect.bottom() + 8.0),
            Align2::LEFT_TOP,
            text,
            theme::font(11.0),
            theme.text_secondary.gamma_multiply(alpha),
        );
    }
}
