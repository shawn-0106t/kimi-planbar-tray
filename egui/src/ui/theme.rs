// SPEC 11 color table (Moonlit light / Moondark dark) plus the CJK-capable
// font chain. All UI code consumes these values instead of egui's built-in
// light/dark visuals; the effective theme is resolved in AppState
// ("system" follows the registry watcher, theme_watch.rs).

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, FontId};

/// SPEC 11.1 brush table, one field per brush key.
#[derive(Clone, Copy)]
pub struct Theme {
    pub accent: Color32,
    pub window_bg: Color32,
    pub card_bg: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub progress_track: Color32,
    pub button_bg: Color32,
    pub button_hover: Color32,
    pub badge_bg: Color32,
    pub badge_fg: Color32,
}

impl Theme {
    pub const fn light() -> Theme {
        Theme {
            accent: Color32::from_rgb(0x1A, 0x88, 0xFF),
            window_bg: Color32::from_rgb(0xF3, 0xF4, 0xF6),
            card_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            text_primary: Color32::from_rgb(0x1F, 0x23, 0x29),
            text_secondary: Color32::from_rgb(0x6B, 0x72, 0x80),
            progress_track: Color32::from_rgb(0xE5, 0xE7, 0xEB),
            button_bg: Color32::from_rgb(0xE9, 0xEC, 0xF0),
            button_hover: Color32::from_rgb(0xDC, 0xE2, 0xE9),
            badge_bg: Color32::from_rgb(0xFF, 0xF0, 0xE0),
            badge_fg: Color32::from_rgb(0xE0, 0x6D, 0x00),
        }
    }

    pub const fn dark() -> Theme {
        Theme {
            accent: Color32::from_rgb(0x1A, 0x88, 0xFF),
            window_bg: Color32::from_rgb(0x17, 0x19, 0x1E),
            card_bg: Color32::from_rgb(0x23, 0x26, 0x2D),
            text_primary: Color32::from_rgb(0xF2, 0xF3, 0xF5),
            text_secondary: Color32::from_rgb(0x9A, 0xA0, 0xA8),
            progress_track: Color32::from_rgb(0x3A, 0x3E, 0x47),
            button_bg: Color32::from_rgb(0x2C, 0x30, 0x39),
            button_hover: Color32::from_rgb(0x3A, 0x40, 0x4B),
            badge_bg: Color32::from_rgb(0x3D, 0x2E, 0x1A),
            badge_fg: Color32::from_rgb(0xF0, 0xA0, 0x40),
        }
    }

    /// Resolve an effective theme name ("light" | "dark") to the color table.
    pub fn from_name(name: &str) -> Theme {
        if name == "dark" {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    /// SPEC 15.3 hover-glow intensities: (ring alpha, halo alpha, halo extent).
    /// Dark theme draws a stronger, wider halo.
    pub fn glow_params(&self) -> (f32, f32, f32) {
        if self.accent == Self::dark().accent && self.window_bg == Self::dark().window_bg {
            (0.45, 0.22, 10.0)
        } else {
            (0.35, 0.18, 8.0)
        }
    }
}

// ---- Font families -------------------------------------------------------
//
// egui has no font-weight selection, so the weights become named families:
//   regular   -> "kimi" family is NOT used; Proportional carries regular text
//   semibold  -> FontFamily::Name(SEMIBOLD_FAMILY): titles (SPEC 12.1)
//   bold      -> FontFamily::Name(BOLD_FAMILY): big percent, balances
// Each named family falls back to the regular fonts, so a missing file only
// costs weight, never text.

pub const SEMIBOLD_FAMILY: &str = "kimi-semibold";
pub const BOLD_FAMILY: &str = "kimi-bold";

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD_FAMILY.into()))
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD_FAMILY.into()))
}

// Latin faces (WPF/rust frontend default to Segoe UI; weights as declared).
const LATIN_REGULAR: &str = "C:\\Windows\\Fonts\\segoeui.ttf";
const LATIN_BOLD: &str = "C:\\Windows\\Fonts\\segoeuib.ttf";
const LATIN_SEMIBOLD: &str = "C:\\Windows\\Fonts\\seguisb.ttf";
const CJK_BOLD: &str = "C:\\Windows\\Fonts\\msyhbd.ttc";

// CJK regular probe chain, in order (all system paths).
const CJK_CANDIDATES: [(&str, &str); 3] = [
    ("cjk", "C:\\Windows\\Fonts\\msyh.ttc"),
    ("cjk", "C:\\Windows\\Fonts\\simhei.ttf"),
    ("cjk", "C:\\Windows\\Fonts\\simsun.ttc"),
];

fn try_read_font(fonts: &mut FontDefinitions, name: &str, path: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            fonts
                .font_data
                .insert(name.to_string(), FontData::from_owned(bytes).into());
            true
        }
        Err(_) => false,
    }
}

fn push_front(fonts: &mut FontDefinitions, family: FontFamily, name: &str) {
    fonts
        .families
        .entry(family)
        .or_default()
        .insert(0, name.to_string());
}

fn push_back(fonts: &mut FontDefinitions, family: FontFamily, name: &str) {
    fonts
        .families
        .entry(family)
        .or_default()
        .push(name.to_string());
}

/// Install the font set, starting the CJK probe at candidate `start`. The
/// named weight families are ALWAYS bound (falling back to the regular fonts
/// when a face is missing), so text layout can never panic on an unbound
/// family. Returns the index of the CJK candidate actually installed, or None
/// when none was readable. Takes effect on the NEXT frame (begin_pass
/// rebuilds the font set), so coverage is verified a frame later by
/// [`font_covers_cjk`] — same next-frame rule the Phase 0 spike hit.
pub fn install_fonts(ctx: &Context, start: usize) -> Option<usize> {
    // First readable candidate at/after `start` wins.
    let mut installed = None;
    let mut cjk: Option<(String, Vec<u8>)> = None;
    for (i, (name, path)) in CJK_CANDIDATES.iter().enumerate().skip(start) {
        if let Ok(bytes) = std::fs::read(path) {
            cjk = Some(((*name).to_string(), bytes));
            installed = Some(i);
            break;
        }
    }

    let mut fonts = FontDefinitions::default();

    // Regular: Segoe UI first (Latin look matches the rust/ frontend), then
    // the built-in fonts (emoji), then the CJK fallback last.
    if try_read_font(&mut fonts, "segoe-ui", LATIN_REGULAR) {
        push_front(&mut fonts, FontFamily::Proportional, "segoe-ui");
    }
    if let Some((name, bytes)) = &cjk {
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes.clone()).into());
        push_back(&mut fonts, FontFamily::Proportional, name);
        push_back(&mut fonts, FontFamily::Monospace, name);
    }

    // Weighted families: Latin face + CJK bold (falls back to regular CJK).
    let mut bold_chain: Vec<String> = Vec::new();
    let mut semi_chain: Vec<String> = Vec::new();
    if try_read_font(&mut fonts, "segoe-ui-bold", LATIN_BOLD) {
        bold_chain.push("segoe-ui-bold".to_string());
        semi_chain.push("segoe-ui-bold".to_string());
    }
    if try_read_font(&mut fonts, "cjk-bold", CJK_BOLD) {
        bold_chain.push("cjk-bold".to_string());
        semi_chain.push("cjk-bold".to_string());
    }
    if try_read_font(&mut fonts, "segoe-ui-semibold", LATIN_SEMIBOLD) {
        semi_chain.push("segoe-ui-semibold".to_string());
    }
    if let Some((name, _)) = &cjk {
        bold_chain.push(name.clone());
        semi_chain.push(name.clone());
    }
    bold_chain.push("segoe-ui".to_string());
    semi_chain.push("segoe-ui".to_string());
    if let Some((name, _)) = &cjk {
        bold_chain.push(name.clone());
        semi_chain.push(name.clone());
    }
    fonts
        .families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), bold_chain);
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD_FAMILY.into()), semi_chain);

    ctx.set_fonts(fonts);
    installed
}

/// Verify CJK + yen coverage of the regular family (call one frame AFTER
/// install_fonts; the font set rebuilds at begin_pass).
pub fn font_covers_cjk(ctx: &Context) -> bool {
    let jia = ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), '\u{52a0}'));
    let yen = ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), '\u{a5}'));
    jia && yen
}

// ---- Font probe state machine (port of the spike's) ----------------------

pub enum FontProbe {
    /// Font set installed; verifying coverage of the chosen candidate.
    Await { idx: usize },
    /// Chain settled (covered, or nothing else to try).
    Done,
}

impl FontProbe {
    /// Install at construction (so the first frame paints with real fonts),
    /// then advance one verification step per frame.
    pub fn new(ctx: &Context) -> FontProbe {
        match install_fonts(ctx, 0) {
            Some(idx) => FontProbe::Await { idx },
            None => FontProbe::Done, // no CJK font readable: tofu expected
        }
    }

    /// Advance one step per frame: verify the candidate installed on the
    /// previous frame (set_fonts takes effect at the NEXT begin_pass — a
    /// same-frame coverage check would misreport), else advance the chain.
    pub fn step(&mut self, ctx: &Context) {
        if let FontProbe::Await { idx } = self {
            let next = *idx + 1;
            if font_covers_cjk(ctx) || install_fonts(ctx, next).is_none() {
                *self = FontProbe::Done;
            } else {
                *self = FontProbe::Await { idx: next };
            }
        }
    }
}

/// Linear blend in gamma space (used for the 140ms hover background transition).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t + 0.5) as u8;
    Color32::from_rgba_unmultiplied(
        ch(a.r(), b.r()),
        ch(a.g(), b.g()),
        ch(a.b(), b.b()),
        ch(a.a(), b.a()),
    )
}
