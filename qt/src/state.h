// Shared application state, 1:1 port of rust/src-tauri/src/state.rs.
// Phase 2 adds the panel/tray fields (hide re-entry guard, hover throttle,
// manual-refresh debounce, hiding flag, window-open suppressors). The Rust
// edition guards these with RwLock/Mutex because tauri commands run on
// arbitrary threads; the Qt edition funnels every mutation onto the GUI
// thread (workers only compute and post results), so plain fields suffice.
// The reschedule/retime Notify pair is replaced by Poller's QTimer.

#ifndef STATE_H
#define STATE_H

#include "quota.h"
#include "settings.h"
#include "skills.h"
#include "update.h"

#include <QElapsedTimer>
#include <optional>

struct AppState {
    SettingsData settings;
    // Last successfully fetched quota snapshot (SPEC 16.5 keep-last-good)
    std::optional<QuotaResult> lastQuota;
    UpdateStatus update;
    // Effective theme after resolving "system": "light" | "dark"
    QString effectiveTheme = QStringLiteral("light");

    // Timestamp of the last auto-hide, for the 300ms tray-click re-entry guard
    bool hasLastHide = false;
    QElapsedTimer lastHide;
    // Timestamp of the last hover-triggered refresh (10s throttle)
    bool hasLastHover = false;
    QElapsedTimer lastHover;
    // Timestamp of the last manual "Refresh" command (2s debounce)
    bool hasLastManualRefresh = false;
    QElapsedTimer lastManualRefresh;

    bool panelHiding = false;
    // While the settings/skills window is open, panel focus-loss hide is
    // suppressed (SPEC 20 / 21)
    bool settingsOpen = false;
    bool skillsOpen = false;
    // Lazy one-shot cache for the skills window (no background scanning);
    // rescan only when the Refresh button passes refresh=true (SPEC 21.2)
    std::optional<QList<SkillInfo>> skillsCache;
};

#endif // STATE_H
