// Tray icon, 1:1 port of rust/src-tauri/src/tray.rs (SPEC section 14):
// static icon, left-click toggles the panel, right-click opens the menu,
// hover prefetches (10s throttle), tooltip text only.
// Qt specifics (QT-MIGRATION 7.3/7.4): the icon is show()n exactly once per
// process lifetime (repeated hide->show is a known Qt/Windows bug), and hover
// is detected by polling geometry() since QSystemTrayIcon has no hover event.

#ifndef TRAYICON_H
#define TRAYICON_H

#include <QObject>
#include <QPoint>
#include <climits>

class QSystemTrayIcon;
class QTimer;
class App;

class TrayIcon : public QObject
{
    Q_OBJECT
public:
    explicit TrayIcon(App *app);

    // "Kimi Planbar Tray  5h X% · week Y%" (+ " (update failed)" on error).
    void updateTooltip();

private:
    App *m_app;
    QSystemTrayIcon *m_tray;
    QTimer *m_hoverTimer = nullptr;          // hover-to-refresh probe (SPEC 14)
    QPoint m_lastHoverPos{INT_MIN, INT_MIN}; // cursor pos at the previous tick
};

#endif // TRAYICON_H
