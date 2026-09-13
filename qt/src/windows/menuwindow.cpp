#include "menuwindow.h"

#include "hoverglow.h"

#include <QApplication>
#include <QCursor>
#include <QGraphicsDropShadowEffect>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QPushButton>
#include <QScreen>

#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

namespace {

// Window size includes the 24px transparent shadow fade room per side; the
// visible card is 140px wide (SPEC 10.3 legacy 150px width minus margins).
constexpr int kWindowW = 188;
constexpr int kMargin = 24;

} // namespace

MenuWindow::MenuWindow(QWidget *parent) : QWidget(parent)
{
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    setFixedWidth(kWindowW);

    m_root = new QWidget(this);
    m_root->setObjectName(QStringLiteral("menuRoot"));

    // DropShadowEffect BlurRadius=20 Depth=2 Opacity=0.3 (SPEC 10.3)
    auto *shadow = new QGraphicsDropShadowEffect(m_root);
    shadow->setBlurRadius(20);
    shadow->setOffset(0, 2);
    shadow->setColor(QColor(0, 0, 0, 77)); // 0.3 * 255
    m_root->setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(m_root);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(0);

    const struct {
        const char *label;
        const char *action;
    } items[] = {
        {"Open", "open"},
        {"Refresh", "refresh"},
        {"Settings", "settings"},
        {"Skills", "skills"},
        {"Exit", "quit"},
    };
    for (const auto &item : items) {
        auto *btn = new QPushButton(QString::fromLatin1(item.label), m_root);
        btn->setObjectName(QStringLiteral("menuItem"));
        btn->setCursor(Qt::PointingHandCursor);
        btn->setFlat(true);
        connect(btn, &QPushButton::clicked, this,
                [this, action = item.action] { emit actionTriggered(QString::fromLatin1(action)); });
        layout->addWidget(btn);
        m_items.append(btn);
    }
}

void MenuWindow::applyTheme(const QString &effectiveTheme)
{
    const ThemeColors c = theme::colors(effectiveTheme);
    const QString glow = effectiveTheme == QLatin1String("dark")
        ? QStringLiteral("rgba(26, 136, 255, 115)")
        : QStringLiteral("rgba(26, 136, 255, 89)");
    QString qss = QStringLiteral(
        "QWidget#menuRoot { background: %1; border-radius: 12px; }\n"
        "QPushButton#menuItem { background: transparent; border: 1px solid transparent; border-radius: 8px;"
        " color: %2; font-size: 13px; text-align: left; padding: 9px 14px; }\n"
        "QPushButton#menuItem:hover { background: %3; border-color: @GLOW@; }\n")
                      .arg(c.windowBg, c.textPrimary, c.buttonHover);
    qss.replace(QStringLiteral("@GLOW@"), glow);
    setStyleSheet(qss);

    // SPEC 15.3 hover glow on the menu items (halo + 140ms fade; ring in QSS)
    qDeleteAll(m_glows);
    m_glows.clear();
    for (QPushButton *item : m_items)
        m_glows.append(new HoverGlow(item, effectiveTheme == QLatin1String("dark"), 8));
}

void MenuWindow::showAtCursor()
{
    // Content-sized height (WPF SizeToContent): card + 2*24px shadow room
    const int cardH = m_root->layout()->totalSizeHint().height();
    const int windowH = cardH + 2 * kMargin;
    setFixedHeight(windowH);
    m_root->setGeometry(kMargin, kMargin, kWindowW - 2 * kMargin, cardH);

    // Qt logical coordinates throughout: QCursor::pos() and availableGeometry
    // are already DIP on the cursor's monitor (QT-MIGRATION 4.5).
    const QPoint cursor = QCursor::pos();
    QScreen *screen = QGuiApplication::screenAt(cursor);
    if (!screen)
        screen = QGuiApplication::primaryScreen();
    const QRect wa = screen->availableGeometry();
    const int waRight = wa.x() + wa.width();
    const int waBottom = wa.y() + wa.height();
    // Legacy geometry: the visible card is 150px wide / windowH-48 tall; the
    // window origin shifts -19px so the card lands where the old 5px-margin
    // layout put it (rust panel.rs show_menu).
    const int legacyW = kWindowW - 38; // 150
    const int legacyH = windowH - 38;
    const int left = std::min(cursor.x(), waRight - legacyW - 8) - 19;
    const int top =
        (cursor.y() + legacyH + 24 > waBottom ? cursor.y() - legacyH - 8 : cursor.y() + 8) - 19;
    move(left, top);
    show();
    raise();
    activateWindow();
    // SPEC 10.3: must steal the foreground, otherwise Deactivated closes the
    // menu at once
    SetForegroundWindow(reinterpret_cast<HWND>(winId()));
}

bool MenuWindow::event(QEvent *event)
{
    if (event->type() == QEvent::WindowDeactivate)
        emit deactivated();
    return QWidget::event(event);
}
