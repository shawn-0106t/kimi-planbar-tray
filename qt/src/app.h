// Application singleton, port of the rust/src-tauri/src/lib.rs builder +
// window-event routing: owns AppState and all window/tray singletons, routes
// refresh/update/theme flows, and applies the DWM corner opt-out.
// Windows are singletons — hidden, never destroyed.

#ifndef APP_H
#define APP_H

#include "state.h"

#include <QObject>

class PanelWindow;
class SettingsWindow;
class SkillsWindow;
class MenuWindow;
class TrayIcon;
class Poller;
class QWidget;
struct QuotaResult;
struct UpdateStatus;

class App : public QObject
{
    Q_OBJECT
public:
    explicit App(QObject *parent = nullptr);
    // Deletes the four (parentless) singleton windows; App dies before
    // QApplication in main(), so widget teardown is safe.
    ~App() override;

    // Tray icon, polling loop, startup update check, theme watch.
    void start();

    AppState &state() { return m_state; }
    PanelWindow *panelWindow() { return m_panel; }
    SettingsWindow *settingsWindow() { return m_settings; }
    SkillsWindow *skillsWindow() { return m_skills; }
    MenuWindow *menuWindow() { return m_menu; }
    TrayIcon *trayIcon() { return m_tray; }
    Poller *poller() { return m_poller; }

    // Panel "Refresh" button / menu "Refresh" (SPEC 12.7 / 14), debounced:
    // invocations within 2s of the previous one are ignored.
    void refreshNow();

    // Async CLI update check; stores the result and re-renders the version row.
    void checkUpdate();

    // Recompute the effective theme and re-apply if it flipped (SPEC 20).
    void applyThemeNow();

    // Settings window (SPEC 13.2): open backfills, save persists + applies.
    void openSettings();
    void closeSettings();
    void saveSettings(const QString &theme, qint64 refreshMinutes, bool autoStart);

    // Skills window (SPEC 21): lazy scan on first open, Refresh rescans.
    void openSkills();
    void closeSkills();
    void loadSkills(bool refresh);

    // Tray menu (SPEC 10.3 / 14)
    void showMenu();
    void menuAction(const QString &action);

private:
    AppState m_state;
    PanelWindow *m_panel = nullptr;
    SettingsWindow *m_settings = nullptr;
    SkillsWindow *m_skills = nullptr;
    MenuWindow *m_menu = nullptr;
    TrayIcon *m_tray = nullptr;
    Poller *m_poller = nullptr;
    bool m_skillsLoading = false;
};

// Opt a window out of Windows 11 DWM auto corner rounding (DWMWA_WINDOW_CORNER_
// PREFERENCE = DWMWCP_DONOTROUND); the QSS paints its own radius. Call once per
// top-level window after its handle exists.
void disableDwmCornerRounding(QWidget *window);

#endif // APP_H
