#include "panel.h"

#include "app.h"
#include "windows/panelwindow.h"

#include <QGuiApplication>
#include <QScreen>

namespace panel {

void showPanel(App *app)
{
    PanelWindow *w = app->panelWindow();
    // Primary-monitor work area in logical px (Qt's availableGeometry is
    // already DIP — the Rust edition's physical-px dance is unnecessary).
    const QRect wa = QGuiApplication::primaryScreen()->availableGeometry();
    // +22 compensates the margin growth (6 -> 28, shadow fade room): the card's
    // visible right/bottom edges still sit 18px off the work-area corner.
    w->move(wa.x() + wa.width() - (424 + 12 - 22), wa.y() + wa.height() - (520 + 12 - 22));
    app->state().panelHiding = false;
    w->show();
    w->raise();
    w->activateWindow();
    w->playShow();
}

void startHide(App *app)
{
    if (app->state().panelHiding)
        return; // re-entry guard (Rust panel_hiding.swap(true))
    app->state().panelHiding = true;
    app->panelWindow()->playHide();
}

void finishHide(App *app)
{
    if (!app->state().panelHiding)
        return; // stale finish after a newer show must not hide the window
    app->state().panelHiding = false;
    app->panelWindow()->hide();
    AppState &st = app->state();
    st.hasLastHide = true;
    st.lastHide.start();
}

void togglePanel(App *app, bool fromTray)
{
    if (fromTray) {
        AppState &st = app->state();
        if (st.hasLastHide && st.lastHide.elapsed() < 300)
            return;
    }
    if (app->panelWindow()->isVisible())
        startHide(app);
    else
        showPanel(app);
}

void onMainBlur(App *app)
{
    const AppState &st = app->state();
    if (st.settingsOpen || st.skillsOpen)
        return;
    if (app->panelWindow()->isVisible())
        startHide(app);
}

} // namespace panel
