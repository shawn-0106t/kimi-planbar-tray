// System theme follow (UI-SPEC section 11): read HKCU AppsUseLightTheme and emit
// "theme-changed" when the effective theme flips. The WPF reference hooks
// SystemEvents.UserPreferenceChanged; here we poll the registry every 3s, which
// is cheap and needs no hidden message window.

use crate::state::AppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

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
pub fn apply_now(app: &AppHandle) {
    let st = app.state::<AppState>();
    let configured = st.settings.read().unwrap().theme.clone();
    let eff = effective(&configured);
    let mut cur = st.effective_theme.write().unwrap();
    if *cur != eff {
        *cur = eff.clone();
        let _ = app.emit("theme-changed", eff);
    }
}

pub async fn watch(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        apply_now(&app);
    }
}
