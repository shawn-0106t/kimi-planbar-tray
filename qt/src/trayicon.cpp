#include "trayicon.h"

#include "app.h"
#include "format_util.h"
#include "panel.h"
#include "polling.h"
#include "state.h"

#include <QCursor>
#include <QImage>
#include <QSystemTrayIcon>
#include <QTimer>

namespace {

// Programmatic 32x32 blue ball, drawn exactly like the WPF/Rust fallback
// (circle (1,1,30,30) #1A88FF + highlight ellipse (7,5,10,7) white@90).
QIcon fallbackIcon()
{
    constexpr int S = 32;
    QImage img(S, S, QImage::Format_RGBA8888);
    img.fill(Qt::transparent);
    for (int y = 0; y < S; ++y) {
        for (int x = 0; x < S; ++x) {
            const double dx = x + 0.5 - 16.0;
            const double dy = y + 0.5 - 16.0;
            if (dx * dx + dy * dy <= 15.0 * 15.0)
                img.setPixelColor(x, y, QColor(0x1A, 0x88, 0xFF));
        }
    }
    constexpr double a = 90.0 / 255.0;
    for (int y = 0; y < S; ++y) {
        for (int x = 0; x < S; ++x) {
            const double dx = (x + 0.5 - 12.0) / 5.0;
            const double dy = (y + 0.5 - 8.5) / 3.5;
            if (dx * dx + dy * dy <= 1.0) {
                QColor c = img.pixelColor(x, y);
                if (c.alpha() > 0) {
                    c.setRed(static_cast<int>(255.0 * a + c.red() * (1.0 - a)));
                    c.setGreen(static_cast<int>(255.0 * a + c.green() * (1.0 - a)));
                    c.setBlue(static_cast<int>(255.0 * a + c.blue() * (1.0 - a)));
                    img.setPixelColor(x, y, c);
                }
            }
        }
    }
    return QIcon(QPixmap::fromImage(img));
}

QString pctText(const std::optional<QuotaSegment> &seg)
{
    return seg ? fmtPercent(seg->percent) : QStringLiteral("?");
}

} // namespace

TrayIcon::TrayIcon(App *app) : QObject(app), m_app(app)
{
    m_tray = new QSystemTrayIcon(this);
    const QIcon icon(QStringLiteral(":/assets/icon.png"));
    m_tray->setIcon(icon.isNull() ? fallbackIcon() : icon);
    m_tray->setToolTip(QStringLiteral("Kimi Planbar Tray"));

    connect(m_tray, &QSystemTrayIcon::activated, this,
            [this](QSystemTrayIcon::ActivationReason reason) {
                switch (reason) {
                // Rust uses MouseUp (not click) semantics; Qt's Trigger/Context
                // fire on mouse-up for tray icons, matching that choice.
                case QSystemTrayIcon::Trigger:
                    panel::togglePanel(m_app, true);
                    break;
                case QSystemTrayIcon::Context:
                    m_app->showMenu(); // right-click menu window (SPEC 10.3 / 14)
                    break;
                default:
                    break;
                }
            });

    // Hover-to-refresh, throttled at 10s (SPEC 14). QSystemTrayIcon has no
    // hover event and a nativeEventFilter probe against the shell's tray
    // callback message did not observe it (QT-MIGRATION 7.3 approach A
    // abandoned), so hover is detected by polling the icon rect
    // (Shell_NotifyIconGetRect via QSystemTrayIcon::geometry()) at 500ms.
    // A hover "event" = cursor inside the rect AND moved since the last tick,
    // matching the Rust edition's Enter/Move-driven semantics (a parked
    // cursor does not retrigger).
    m_hoverTimer = new QTimer(this);
    m_hoverTimer->setInterval(500);
    connect(m_hoverTimer, &QTimer::timeout, this, [this] {
        const QPoint pos = QCursor::pos();
        const bool inside = m_tray->geometry().contains(pos);
        const bool moved = pos != m_lastHoverPos;
        m_lastHoverPos = pos;
        if (!inside || !moved)
            return;
        AppState &st = m_app->state();
        if (st.hasLastHover && st.lastHover.elapsed() < 10000)
            return;
        st.hasLastHover = true;
        st.lastHover.start();
        m_app->poller()->safeRefresh();
    });
    m_hoverTimer->start();

    // Exactly one show() per lifetime (QT-MIGRATION 7.4)
    m_tray->show();
}

void TrayIcon::updateTooltip()
{
    const std::optional<QuotaResult> &last = m_app->state().lastQuota;
    QString text = QStringLiteral("Kimi Planbar Tray");
    if (last) {
        text = QStringLiteral("Kimi Planbar Tray  5h %1 · week %2")
                   .arg(pctText(last->fiveHour), pctText(last->week));
        if (last->error)
            text += QStringLiteral(" (update failed)");
    }
    m_tray->setToolTip(text);
}
