// Embedded images: the Kimi logo (rust/src-tauri/icons/icon.png copy) for the
// panel header + tray icon, and the four bottom-button icons pre-rasterized
// from assets/icons/*.svg (96x96, white fill — tinted with the theme text
// color at draw time, which is exactly the SVGs' fill="currentColor" intent).

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

pub const LOGO_PNG: &[u8] = include_bytes!("assets/icon.png");
pub const ICON_CONSOLE_PNG: &[u8] = include_bytes!("assets/icons/console.png");
pub const ICON_REFRESH_PNG: &[u8] = include_bytes!("assets/icons/refresh.png");
pub const ICON_SETTINGS_PNG: &[u8] = include_bytes!("assets/icons/settings.png");
pub const ICON_EXIT_PNG: &[u8] = include_bytes!("assets/icons/exit.png");

pub struct Icons {
    pub logo: TextureHandle,
    pub console: TextureHandle,
    pub refresh: TextureHandle,
    pub settings: TextureHandle,
    pub exit: TextureHandle,
}

fn texture(ctx: &Context, name: &str, png: &[u8]) -> TextureHandle {
    let decoded = image::load_from_memory(png)
        .unwrap_or_else(|e| panic!("embedded {name} is not decodable PNG: {e}"))
        .to_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    let image = ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}

impl Icons {
    pub fn load(ctx: &Context) -> Icons {
        Icons {
            logo: texture(ctx, "kimi-logo", LOGO_PNG),
            console: texture(ctx, "icon-console", ICON_CONSOLE_PNG),
            refresh: texture(ctx, "icon-refresh", ICON_REFRESH_PNG),
            settings: texture(ctx, "icon-settings", ICON_SETTINGS_PNG),
            exit: texture(ctx, "icon-exit", ICON_EXIT_PNG),
        }
    }
}

// ---- Tray icon (SPEC 14) --------------------------------------------------

/// tray_icon::Icon from the embedded logo PNG; None when decoding fails
/// (then the caller falls back to the programmatic blue ball).
pub fn tray_icon_from_logo() -> Option<tray_icon::Icon> {
    let decoded = image::load_from_memory(LOGO_PNG).ok()?.to_rgba8();
    let (w, h) = (decoded.width(), decoded.height());
    tray_icon::Icon::from_rgba(decoded.into_raw(), w, h).ok()
}

/// Programmatic 32x32 blue ball, drawn exactly like the WPF fallback
/// (circle (1,1,30,30) #1A88FF + highlight ellipse (7,5,10,7) white@90);
/// ported from rust/src-tauri/src/tray.rs.
pub fn fallback_tray_icon() -> Option<tray_icon::Icon> {
    const S: usize = 32;
    let mut px = vec![0u8; S * S * 4];
    for y in 0..S {
        for x in 0..S {
            let dx = x as f64 + 0.5 - 16.0;
            let dy = y as f64 + 0.5 - 16.0;
            if dx * dx + dy * dy <= 15.0 * 15.0 {
                let i = (y * S + x) * 4;
                px[i] = 0x1A;
                px[i + 1] = 0x88;
                px[i + 2] = 0xFF;
                px[i + 3] = 0xFF;
            }
        }
    }
    let a = 90.0 / 255.0;
    for y in 0..S {
        for x in 0..S {
            let dx = (x as f64 + 0.5 - 12.0) / 5.0;
            let dy = (y as f64 + 0.5 - 8.5) / 3.5;
            if dx * dx + dy * dy <= 1.0 {
                let i = (y * S + x) * 4;
                if px[i + 3] > 0 {
                    px[i] = (255.0 * a + px[i] as f64 * (1.0 - a)) as u8;
                    px[i + 1] = (255.0 * a + px[i + 1] as f64 * (1.0 - a)) as u8;
                    px[i + 2] = (255.0 * a + px[i + 2] as f64 * (1.0 - a)) as u8;
                }
            }
        }
    }
    tray_icon::Icon::from_rgba(px, S as u32, S as u32).ok()
}
