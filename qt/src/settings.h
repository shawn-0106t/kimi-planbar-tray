// Settings persistence + autostart, 1:1 port of rust/src-tauri/src/settings.rs
// (SPEC section 18). Same settings.json schema and path resolution as the
// Rust edition, so both editions interoperate on the same file.

#ifndef SETTINGS_H
#define SETTINGS_H

#include <QString>

struct SettingsData {
    QString theme = QStringLiteral("system"); // system | light | dark
    qint64 refreshMinutes = 5;                // 1 | 5 | 10 | 30
    bool autoStart = false;
};

namespace settings {

// Portable mode: a `portable.dat` next to the exe pins the config dir to the
// exe directory; otherwise %APPDATA%\KimiPlanbarTray (SPEC 18.1).
QString configDir();

// Missing/unreadable file or partial/invalid JSON -> defaults per field.
SettingsData load();

void save(const SettingsData &data);

// HKCU Run key autostart (per-user, no UAC). All errors silently swallowed.
void applyAutoStart(const SettingsData &data);

} // namespace settings

#endif // SETTINGS_H
