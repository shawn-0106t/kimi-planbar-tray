#include "app.h"

#include "async_util.h"
#include "panel.h"
#include "polling.h"
#include "skills.h"
#include "theme.h"
#include "trayicon.h"
#include "update.h"
#include "windows/menuwindow.h"
#include "windows/panelwindow.h"
#include "windows/settingswindow.h"
#include "windows/skillswindow.h"

#include <QCoreApplication>
#include <QDesktopServices>
#include <QGuiApplication>
#include <QStyleHints>
#include <QUrl>

#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <dwmapi.h>

namespace {
const char kReleasesUrl[] = "https://github.com/MoonshotAI/kimi-code/releases";
const char kConsoleUrl[] = "https://www.kimi.com/code/console?from=kfc_overview_topbar";
} // namespace

void disableDwmCornerRounding(QWidget *window)
{
    const HWND hwnd = reinterpret_cast<HWND>(window->winId());
    const DWORD pref = 1; // DWMWCP_DONOTROUND
    DwmSetWindowAttribute(hwnd, 33 /* DWMWA_WINDOW_CORNER_PREFERENCE */, &pref, sizeof(pref));
}

App::App(QObject *parent) : QObject(parent)
{
    m_state.settings = settings::load();
    m_state.effectiveTheme = theme::effective(m_state.settings.theme);

    m_panel = new PanelWindow();
    m_settings = new SettingsWindow();
    m_skills = new SkillsWindow();
    m_menu = new MenuWindow();
    // The four windows stay parentless (a widget parent would change their
    // top-level window behavior); ~App deletes them. App is a stack member of
    // main() and dies before QApplication, so widget teardown is safe.
    QWidget *wins[] = {m_panel, m_settings, m_skills, m_menu};
    for (QWidget *w : wins)
        disableDwmCornerRounding(w);
    // Initial theme paint on all four windows (applyThemeNow only re-applies
    // on flips, so the first paint is done explicitly)
    m_panel->applyTheme(m_state.effectiveTheme);
    m_settings->applyTheme(m_state.effectiveTheme);
    m_skills->applyTheme(m_state.effectiveTheme);
    m_menu->applyTheme(m_state.effectiveTheme);
    m_panel->renderQuota(std::nullopt);
    m_panel->renderUpdate(UpdateStatus{});

    // Panel wiring
    connect(m_panel, &PanelWindow::deactivated, this, [this] { panel::onMainBlur(this); });
    connect(m_panel, &PanelWindow::hideFinished, this, [this] { panel::finishHide(this); });
    connect(m_panel, &PanelWindow::refreshRequested, this, &App::refreshNow);
    connect(m_panel, &PanelWindow::consoleRequested, this, [] {
        QDesktopServices::openUrl(QUrl(QString::fromLatin1(kConsoleUrl)));
    });
    connect(m_panel, &PanelWindow::releasesRequested, this, [] {
        QDesktopServices::openUrl(QUrl(QString::fromLatin1(kReleasesUrl)));
    });
    connect(m_panel, &PanelWindow::settingsRequested, this, &App::openSettings);
    connect(m_panel, &PanelWindow::exitRequested, this, [] { QCoreApplication::quit(); });

    // Settings window wiring (SPEC 13.2)
    connect(m_settings, &SettingsWindow::saveRequested, this, &App::saveSettings);
    connect(m_settings, &SettingsWindow::closeRequested, this, &App::closeSettings);

    // Skills window wiring (SPEC 21)
    connect(m_skills, &SkillsWindow::refreshRequested, this, [this] { loadSkills(true); });
    connect(m_skills, &SkillsWindow::closeRequested, this, &App::closeSkills);

    // Menu window wiring (SPEC 10.3 / 14): close on focus loss, then dispatch
    connect(m_menu, &MenuWindow::deactivated, this, [this] { m_menu->hide(); });
    connect(m_menu, &MenuWindow::actionTriggered, this, &App::menuAction);

    m_poller = new Poller(this);

    // System theme watch (only effective while the setting is "system")
    connect(QGuiApplication::styleHints(), &QStyleHints::colorSchemeChanged, this,
            [this](Qt::ColorScheme) { applyThemeNow(); });
}

App::~App()
{
    delete m_panel;
    delete m_settings;
    delete m_skills;
    delete m_menu;
}

void App::start()
{
    // Tray icon created (and shown exactly once) only on the GUI path, so
    // --test-ui never flashes an icon
    m_tray = new TrayIcon(this);
    m_poller->start();
    checkUpdate(); // mirrors the startup check_and_emit spawn in lib.rs
}

void App::refreshNow()
{
    {
        AppState &st = m_state;
        if (st.hasLastManualRefresh && st.lastManualRefresh.elapsed() < 2000)
            return; // 2s debounce
        st.hasLastManualRefresh = true;
        st.lastManualRefresh.start();
    }
    m_poller->safeRefresh();
    checkUpdate();
}

void App::checkUpdate()
{
    runAsync<UpdateStatus>(this, [] { return update::check(); }, [this](UpdateStatus &&st) {
        m_state.update = st;
        m_panel->renderUpdate(m_state.update);
    });
}

void App::applyThemeNow()
{
    const QString eff = theme::effective(m_state.settings.theme);
    if (m_state.effectiveTheme != eff) {
        m_state.effectiveTheme = eff;
        m_panel->applyTheme(eff);
        m_settings->applyTheme(eff);
        m_skills->applyTheme(eff);
        m_menu->applyTheme(eff);
    }
}

void App::openSettings()
{
    m_state.settingsOpen = true;
    // Singleton reuse: re-pin the logical size on every open (DPI guard) and
    // backfill the current settings (SPEC 13.2)
    m_settings->setFixedSize(404, 464);
    m_settings->backfill(m_state.settings);
    m_settings->show();
    m_settings->raise();
    m_settings->activateWindow();
}

void App::closeSettings()
{
    m_state.settingsOpen = false;
    m_settings->hide();
}

void App::saveSettings(const QString &themeName, qint64 refreshMinutes, bool autoStart)
{
    {
        SettingsData &cur = m_state.settings;
        cur.theme = themeName;
        // Clamp at the boundary: polling multiplies this by 60 seconds
        cur.refreshMinutes = std::clamp<qint64>(refreshMinutes, 1, 30);
        cur.autoStart = autoStart;
        settings::save(cur);
        settings::applyAutoStart(cur);
    }
    applyThemeNow();
    m_poller->reschedule();
    closeSettings();
}

void App::openSkills()
{
    m_state.skillsOpen = true;
    m_skills->setFixedSize(404, 520);
    m_skills->show();
    m_skills->raise();
    m_skills->activateWindow();
    // Lazy one-shot scan (SPEC 21.2: zero cost until the window is opened)
    if (!m_state.skillsCache)
        loadSkills(false);
}

void App::closeSkills()
{
    m_state.skillsOpen = false;
    m_skills->hide();
}

void App::loadSkills(bool refresh)
{
    if (m_skillsLoading)
        return; // a scan worker is already running
    if (!refresh && m_state.skillsCache) {
        m_skills->renderSkills(*m_state.skillsCache);
        return;
    }
    m_skillsLoading = true;
    m_skills->showLoading();
    runAsync<QList<SkillInfo>>(this, [] { return skills::scan(); },
                               [this](QList<SkillInfo> &&list) {
                                   m_skillsLoading = false;
                                   m_state.skillsCache = list;
                                   m_skills->renderSkills(list);
                               });
}

void App::showMenu()
{
    m_menu->showAtCursor();
}

void App::menuAction(const QString &action)
{
    // Menu window closes first (SPEC 14)
    m_menu->hide();
    if (action == QLatin1String("open"))
        panel::togglePanel(this, false);
    else if (action == QLatin1String("refresh"))
        refreshNow();
    else if (action == QLatin1String("settings"))
        openSettings();
    else if (action == QLatin1String("skills"))
        openSkills();
    else if (action == QLatin1String("quit"))
        QCoreApplication::quit();
}
