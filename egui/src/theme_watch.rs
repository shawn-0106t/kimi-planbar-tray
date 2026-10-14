// System theme follow (SPEC section 20): read HKCU AppsUseLightTheme and emit
// ThemeChanged when the effective theme flips. The WPF reference hooks
// SystemEvents.UserPreferenceChanged; the Tauri edition polls the registry
// every 3s (cheap, no hidden message window) — ported as-is, with the emit
// replaced by the AppEvent bus. A real WM_SETTINGCHANGE message-loop watch is
// a Phase 2 option once the win32 event loop exists (docs/EGUI-MIGRATION.md).

use crate::ctx::{AppCtx, AppEvent};
use std::time::Duration;

/// 0 = dark, 1 (or missing) = light.
pub fn system_theme() -> &'static str {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let value: u32 = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value("AppsUseLightTheme"))
        .unwrap_or(1);
    if value == 0 {
        "dark"
    } else {
        "light"
    }
}

/// Resolve the configured theme ("system" follows the OS) to light|dark.
pub fn effective(configured: &str) -> String {
    match configured {
        "light" => "light".to_string(),
        "dark" => "dark".to_string(),
        _ => system_theme().to_string(),
    }
}

/// Recompute and emit immediately (called after settings are saved).
pub fn apply_now(ctx: &AppCtx) {
    let st = &ctx.state;
    let configured = st.settings.read().unwrap().theme.clone();
    let eff = effective(&configured);
    let mut cur = st.effective_theme.write().unwrap();
    if *cur != eff {
        *cur = eff.clone();
        let _ = ctx.events.send(AppEvent::ThemeChanged(eff));
    }
}

pub async fn watch(ctx: AppCtx) {
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        apply_now(&ctx);
    }
}
