#include "settingswindow.h"

#include "hoverglow.h"
#include "titlebar.h"

#include <QAbstractButton>
#include <QButtonGroup>
#include <QCloseEvent>
#include <QGraphicsDropShadowEffect>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QPainter>
#include <QPushButton>
#include <QScreen>
#include <QStyleOption>

namespace {

constexpr int kWindowW = 404;
constexpr int kWindowH = 464;
constexpr int kMargin = 28;

} // namespace

// Painted checkable row: 18px circle radio (8px inner dot) or 18px rounded
// square checkbox (white tick), text 8px to the right (SPEC 13.3).
class SettingsWindow::ChoiceItem : public QAbstractButton
{
public:
    enum Shape { Circle, Box };

    ChoiceItem(Shape shape, const QString &text, QWidget *parent)
        : QAbstractButton(parent), m_shape(shape)
    {
        setCheckable(true);
        setCursor(Qt::PointingHandCursor);
        setText(text);
        setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }

    void setColors(const ThemeColors &c)
    {
        m_colors = c;
        update();
    }

    QSize sizeHint() const override
    {
        return QSize(200, std::max(18, fontMetrics().height()));
    }

protected:
    void paintEvent(QPaintEvent *) override
    {
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        const int box = 18;
        const int top = (height() - box) / 2;
        const QRectF r(0, top, box, box);
        if (m_shape == Circle) {
            // ThemeRadio: 1.5px outline, transparent fill, 8px accent dot
            p.setPen(QPen(QColor(m_colors.textSecondary), 1.5));
            p.setBrush(Qt::NoBrush);
            p.drawEllipse(r.adjusted(0.75, 0.75, -0.75, -0.75));
            if (isChecked()) {
                p.setPen(Qt::NoPen);
                p.setBrush(QColor(m_colors.accent));
                p.drawEllipse(QRectF(5, top + 5, 8, 8));
            }
        } else {
            // ThemeCheckBox: 1.5px border, radius 5; checked -> accent + tick
            if (isChecked()) {
                p.setPen(Qt::NoPen);
                p.setBrush(QColor(m_colors.accent));
                p.drawRoundedRect(r, 5, 5);
                p.setPen(Qt::white);
                QFont f = font();
                f.setPixelSize(12);
                f.setBold(true);
                p.setFont(f);
                p.drawText(r, Qt::AlignCenter, QChar(0x2713)); // ✓
            } else {
                p.setPen(QPen(QColor(m_colors.textSecondary), 1.5));
                p.setBrush(Qt::NoBrush);
                p.drawRoundedRect(r.adjusted(0.75, 0.75, -0.75, -0.75), 5, 5);
            }
        }
        // Text: radios turn accent on hover (SPEC 13.3); checkbox text does not
        const bool accentText = m_shape == Circle && underMouse();
        p.setPen(QColor(accentText ? m_colors.accent : m_colors.textPrimary));
        QFont f = font();
        f.setPixelSize(13);
        p.setFont(f);
        p.drawText(QRect(26, 0, width() - 26, height()), Qt::AlignVCenter | Qt::AlignLeft, text());
    }

private:
    Shape m_shape;
    ThemeColors m_colors;
};

SettingsWindow::SettingsWindow(QWidget *parent) : QWidget(parent)
{
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    setFixedSize(kWindowW, kWindowH);
    // Windows are created centered on the primary work area (tauri.conf
    // `center: true`) and keep that position afterwards
    const QRect wa = QGuiApplication::primaryScreen()->availableGeometry();
    move(wa.x() + (wa.width() - kWindowW) / 2, wa.y() + (wa.height() - kWindowH) / 2);

    auto *root = new QWidget(this);
    root->setObjectName(QStringLiteral("root"));
    root->setGeometry(kMargin, kMargin, kWindowW - 2 * kMargin, kWindowH - 2 * kMargin);
    auto *shadow = new QGraphicsDropShadowEffect(root);
    shadow->setBlurRadius(24);
    shadow->setOffset(0, 2);
    shadow->setColor(QColor(0, 0, 0, 64));
    root->setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(root);
    layout->setContentsMargins(16, 16, 16, 16);
    layout->setSpacing(0);

    QPushButton *closeBtn = nullptr;
    auto *titlebar = DraggableTitleBar::create(this, QStringLiteral("Kimi Planbar Tray Settings"),
                                               &closeBtn);
    connect(closeBtn, &QPushButton::clicked, this, &SettingsWindow::closeRequested);
    layout->addWidget(titlebar);
    layout->addSpacing(16);

    auto *body = new QVBoxLayout;
    body->setContentsMargins(6, 0, 4, 0);
    body->setSpacing(0);

    auto *themeTitle = new QLabel(QStringLiteral("Theme"), root);
    themeTitle->setObjectName(QStringLiteral("sectionTitle"));
    body->addWidget(themeTitle);

    auto *themeGroup = new QButtonGroup(this);
    const struct {
        ChoiceItem **member;
        const char *label;
        int topMargin;
    } radios[] = {
        {&m_themeSystem, "System default", 10},
        {&m_themeLight, "Moonlit (light)", 8},
        {&m_themeDark, "Moondark (dark)", 8},
    };
    for (const auto &d : radios) {
        auto *item = new ChoiceItem(ChoiceItem::Circle, QString::fromLatin1(d.label), root);
        item->setObjectName(QStringLiteral("choiceItem"));
        themeGroup->addButton(item);
        body->addSpacing(d.topMargin);
        body->addWidget(item);
        *d.member = item;
    }

    auto *intervalTitle = new QLabel(QStringLiteral("Refresh interval"), root);
    intervalTitle->setObjectName(QStringLiteral("sectionTitle"));
    body->addSpacing(20);
    body->addWidget(intervalTitle);

    auto *pills = new QHBoxLayout;
    pills->setContentsMargins(0, 0, 0, 0);
    pills->setSpacing(6);
    auto *pillGroup = new QButtonGroup(this);
    for (const char *label : {"1 min", "5 min", "10 min", "30 min"}) {
        auto *pill = new QPushButton(QString::fromLatin1(label), root);
        pill->setObjectName(QStringLiteral("pill"));
        pill->setCheckable(true);
        pill->setCursor(Qt::PointingHandCursor);
        pillGroup->addButton(pill);
        pills->addWidget(pill);
        m_pills.append(pill);
    }
    pills->addStretch();
    body->addSpacing(10);
    body->addLayout(pills);

    m_autostart = new ChoiceItem(ChoiceItem::Box,
                                 QStringLiteral("Launch at Windows startup"), root);
    m_autostart->setObjectName(QStringLiteral("choiceItem"));
    body->addSpacing(22);
    body->addWidget(m_autostart);

    auto *save = new QPushButton(QStringLiteral("Save"), root);
    save->setObjectName(QStringLiteral("saveBtn"));
    save->setCursor(Qt::PointingHandCursor);
    m_saveBtn = save;
    body->addSpacing(26);
    body->addWidget(save);
    body->addStretch();
    connect(save, &QPushButton::clicked, this, [this] {
        QString theme = QStringLiteral("system");
        if (m_themeLight->isChecked())
            theme = QStringLiteral("light");
        else if (m_themeDark->isChecked())
            theme = QStringLiteral("dark");
        static const qint64 minutes[] = {1, 5, 10, 30};
        qint64 refreshMinutes = 5;
        for (int i = 0; i < m_pills.size(); ++i)
            if (m_pills[i]->isChecked())
                refreshMinutes = minutes[i];
        emit saveRequested(theme, refreshMinutes, m_autostart->isChecked());
    });

    layout->addLayout(body, 1);
}

void SettingsWindow::applyTheme(const QString &effectiveTheme)
{
    const ThemeColors c = theme::colors(effectiveTheme);
    QString qss = QStringLiteral(
        "QWidget#root { background: %1; border-radius: 14px; }\n"
        "QLabel { background: transparent; }\n"
        "QLabel#tbTitle { font-size: 15px; font-weight: 600; color: %2; }\n"
        "QLabel#sectionTitle { font-size: 13px; font-weight: 600; color: %3; }\n"
        "QPushButton#chromeClose { background: transparent; border: none; border-radius: 6px;"
        " color: %3; font-size: 14px; padding: 2px 8px; }\n"
        "QPushButton#chromeClose:hover { background: %4; color: %2; }\n"
        "QPushButton#pill { background: %5; color: %2; border: none; border-radius: 9px;"
        " font-size: 12px; padding: 6px 12px; }\n"
        "QPushButton#pill:hover { background: %4; }\n"
        "QPushButton#pill:checked, QPushButton#pill:checked:hover { background: %6; color: #ffffff; }\n"
        "QPushButton#saveBtn { background: %5; color: %2; border: 1px solid transparent;"
        " border-radius: 10px; font-size: 13px; padding: 10px 0; }\n"
        "QPushButton#saveBtn:hover { background: %4; border-color: @GLOW@; }\n"
        "QToolTip { background: %1; color: %2; border: 1px solid %7; padding: 4px 6px; }\n")
                      .arg(c.windowBg, c.textPrimary, c.textSecondary, c.buttonHover, c.buttonBg,
                           c.accent, c.progressTrack);
    const QString glow = effectiveTheme == QLatin1String("dark")
        ? QStringLiteral("rgba(26, 136, 255, 115)")
        : QStringLiteral("rgba(26, 136, 255, 89)");
    qss.replace(QStringLiteral("@GLOW@"), glow);
    setStyleSheet(qss);
    for (ChoiceItem *item : {m_themeSystem, m_themeLight, m_themeDark, m_autostart})
        item->setColors(c);

    // SPEC 15.3 hover glow on the Save ActionButton
    qDeleteAll(m_glows);
    m_glows.clear();
    m_glows.append(new HoverGlow(m_saveBtn, effectiveTheme == QLatin1String("dark")));
}

void SettingsWindow::backfill(const SettingsData &data)
{
    // Iterate with an explicit fallback: a malformed persisted theme value
    // must leave "System default" checked (mirrors settings.ts backfill)
    ChoiceItem *matched = nullptr;
    if (data.theme == QLatin1String("light"))
        matched = m_themeLight;
    else if (data.theme == QLatin1String("dark"))
        matched = m_themeDark;
    else if (data.theme == QLatin1String("system"))
        matched = m_themeSystem;
    (matched ? matched : m_themeSystem)->setChecked(true);

    static const qint64 minutes[] = {1, 5, 10, 30};
    for (int i = 0; i < m_pills.size(); ++i)
        m_pills[i]->setChecked(data.refreshMinutes == minutes[i]);
    m_autostart->setChecked(data.autoStart);
}

void SettingsWindow::closeEvent(QCloseEvent *event)
{
    // Singleton reuse: never destroy, only hide (mirrors CloseRequested
    // prevention in the Rust edition's on_window_event)
    event->ignore();
    emit closeRequested();
}
