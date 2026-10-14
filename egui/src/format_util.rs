// Formatting helpers, 1:1 port of rust/src/common.ts (SPEC 12.3 / 12.5);
// the same algorithms as qt/src/format_util.h.

use chrono::{DateTime, Local};

/// FormatReset: span = at - now, English countdown text (SPEC 12.3).
pub fn format_reset(at: DateTime<Local>) -> String {
    let span_ms = (at - Local::now()).num_milliseconds();
    if span_ms < 0 {
        return "Resets soon".to_string();
    }
    let total_sec = span_ms / 1000;
    let days = total_sec / 86400;
    if days >= 1 {
        return format!("Resets in {}d {}h", days, (total_sec / 3600) % 24);
    }
    let total_hours = total_sec / 3600;
    if total_hours >= 1 {
        return format!("Resets in {}h {}m", total_hours, (total_sec / 60) % 60);
    }
    let minutes = total_sec / 60;
    format!("Resets in {}m", minutes.max(1))
}

/// FmtYuan: cents -> yuan string, fraction omitted for whole yuan (SPEC 12.5).
pub fn fmt_yuan(cents: i64) -> String {
    if cents < 0 {
        return format!("-{}", fmt_yuan(-cents));
    }
    let yuan = cents / 100;
    let frac = cents % 100;
    if frac > 0 {
        format!("\u{a5}{}.{:02}", yuan, frac)
    } else {
        format!("\u{a5}{}", yuan)
    }
}

/// `{Percent:0}%` — display uses the raw (unclamped) percent (SPEC 12.2).
pub fn fmt_percent(percent: f64) -> String {
    format!("{}%", percent.round() as i64)
}

/// Progress bar widths use the clamped percent (SPEC 11.2).
pub fn clamp_percent(percent: f64) -> f64 {
    percent.clamp(0.0, 100.0)
}

/// Header "Updated HH:mm" (24h local time, SPEC 12.1).
pub fn fmt_updated(at: DateTime<Local>) -> String {
    at.format("Updated %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_yuan_matches_spec_examples() {
        assert_eq!(fmt_yuan(1234), "\u{a5}12.34");
        assert_eq!(fmt_yuan(10000), "\u{a5}100");
        assert_eq!(fmt_yuan(-1234), "-\u{a5}12.34");
        assert_eq!(fmt_yuan(105), "\u{a5}1.05");
    }
}
