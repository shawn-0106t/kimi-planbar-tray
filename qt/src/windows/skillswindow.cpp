#include "skillswindow.h"

#include "clamplabel.h"
#include "hoverglow.h"
#include "titlebar.h"

#include <QCloseEvent>
#include <QGraphicsDropShadowEffect>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QScreen>
#include <QScrollArea>

namespace {

constexpr int kWindowW = 404;
constexpr int kWindowH = 520;
constexpr int kMargin = 28;

void clearLayout(QLayout *layout)
{
    while (QLayoutItem *item = layout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
}

} // namespace

SkillsWindow::SkillsWindow(QWidget *parent) : QWidget(parent)
{
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    setFixedSize(kWindowW, kWindowH);
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
    auto *titlebar = DraggableTitleBar::create(this, QStringLiteral("Kimi Skills"), &closeBtn);
    connect(closeBtn, &QPushButton::clicked, this, &SkillsWindow::closeRequested);
    layout->addWidget(titlebar);
    layout->addSpacing(16);

    // Summary + manual rescan (SPEC 21.2)
    auto *summary = new QHBoxLayout;
    summary->setContentsMargins(6, 0, 4, 0);
    m_summaryText = new QLabel(QStringLiteral("Loading…"), root);
    m_summaryText->setObjectName(QStringLiteral("summaryText"));
    summary->addWidget(m_summaryText);
    summary->addStretch();
    auto *refresh = new QPushButton(QChar(0x27F3) + QStringLiteral(" Refresh"), root); // ⟳
    refresh->setObjectName(QStringLiteral("miniBtn"));
    refresh->setCursor(Qt::PointingHandCursor);
    connect(refresh, &QPushButton::clicked, this, &SkillsWindow::refreshRequested);
    m_refreshBtn = refresh;
    summary->addWidget(refresh);
    layout->addLayout(summary);
    layout->addSpacing(10);

    // Scrollable grouped list (SPEC 21.3)
    auto *scroll = new QScrollArea(root);
    scroll->setObjectName(QStringLiteral("list"));
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    auto *listContent = new QWidget;
    listContent->setObjectName(QStringLiteral("listContent"));
    m_listLayout = new QVBoxLayout(listContent);
    m_listLayout->setContentsMargins(6, 0, 4, 0);
    m_listLayout->setSpacing(0);
    m_listLayout->addStretch();
    scroll->setWidget(listContent);
    layout->addWidget(scroll, 1);
}

void SkillsWindow::applyTheme(const QString &effectiveTheme)
{
    const ThemeColors c = theme::colors(effectiveTheme);
    QString qss = QStringLiteral(
        "QWidget#root { background: %1; border-radius: 14px; }\n"
        "QScrollArea#list { background: transparent; border: none; }\n"
        "QWidget#listContent { background: transparent; }\n"
        "QLabel { background: transparent; }\n"
        "QLabel#tbTitle { font-size: 15px; font-weight: 600; color: %2; }\n"
        "QLabel#summaryText { font-size: 12px; color: %3; }\n"
        "QLabel#groupTitle { font-size: 12px; font-weight: 600; color: %3; }\n"
        "QLabel#skillName { font-size: 13px; font-weight: 600; color: %2; }\n"
        "QLabel#skillDesc { font-size: 12px; color: %3; }\n"
        "QFrame#skillCard { background: %4; border-radius: 10px; }\n"
        "QPushButton#chromeClose { background: transparent; border: none; border-radius: 6px;"
        " color: %3; font-size: 14px; padding: 2px 8px; }\n"
        "QPushButton#chromeClose:hover { background: %5; color: %2; }\n"
        "QPushButton#miniBtn { background: %6; color: %2; border: 1px solid transparent;"
        " border-radius: 8px; font-size: 12px; padding: 4px 10px; }\n"
        "QPushButton#miniBtn:hover { background: %5; border-color: @GLOW@; }\n"
        "QScrollBar:vertical { background: transparent; width: 8px; margin: 0; }\n"
        "QScrollBar::handle:vertical { background: %7; border-radius: 4px; min-height: 24px; }\n"
        "QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }\n"
        "QScrollBar::add-page:vertical, QScrollBar::sub-page:vertical { background: transparent; }\n"
        "QToolTip { background: %1; color: %2; border: 1px solid %7; padding: 4px 6px; }\n")
                      .arg(c.windowBg, c.textPrimary, c.textSecondary, c.cardBg, c.buttonHover,
                           c.buttonBg, c.progressTrack);
    const QString glow = effectiveTheme == QLatin1String("dark")
        ? QStringLiteral("rgba(26, 136, 255, 115)")
        : QStringLiteral("rgba(26, 136, 255, 89)");
    qss.replace(QStringLiteral("@GLOW@"), glow);
    setStyleSheet(qss);
    m_descColor = c.textSecondary;
    for (ClampLabel *label : findChildren<ClampLabel *>())
        label->setColor(QColor(m_descColor));

    // SPEC 15.3 hover glow on the Refresh mini-button
    qDeleteAll(m_glows);
    m_glows.clear();
    m_glows.append(new HoverGlow(m_refreshBtn, effectiveTheme == QLatin1String("dark"), 8));
}

void SkillsWindow::showLoading()
{
    m_summaryText->setText(QStringLiteral("Loading…"));
}

void SkillsWindow::showFailed()
{
    m_summaryText->setText(QStringLiteral("Failed to load skills"));
}

void SkillsWindow::renderSkills(const QList<SkillInfo> &skills)
{
    clearLayout(m_listLayout);
    m_summaryText->setText(skills.isEmpty() ? QStringLiteral("No skills found")
                                            : QStringLiteral("%1 skills").arg(skills.size()));

    QString lastSource;
    for (const SkillInfo &s : skills) {
        if (s.source != lastSource) {
            lastSource = s.source;
            auto *group = new QLabel(s.source);
            group->setTextFormat(Qt::PlainText); // plugin dir names are external input
            group->setObjectName(QStringLiteral("groupTitle"));
            group->setContentsMargins(2, 12, 2, 6);
            m_listLayout->addWidget(group);
        }
        auto *card = new QFrame;
        card->setObjectName(QStringLiteral("skillCard"));
        auto *cl = new QVBoxLayout(card);
        cl->setContentsMargins(12, 10, 12, 10);
        cl->setSpacing(4);
        // External data is rendered as plain text only — the Qt equivalent of
        // the frontend's textContent rule (QLabel auto-detects rich text
        // unless forced).
        auto *name = new QLabel(s.name, card);
        name->setTextFormat(Qt::PlainText);
        name->setObjectName(QStringLiteral("skillName"));
        cl->addWidget(name);
        auto *desc = new ClampLabel(s.description.isEmpty() ? QStringLiteral("(no description)")
                                                            : s.description,
                                    card);
        desc->setColor(QColor(m_descColor));
        QFont descFont = desc->font();
        descFont.setPixelSize(12);
        desc->setFont(descFont);
        if (!s.description.isEmpty())
            // Tooltips also auto-detect rich text; escape so markup shows literally
            card->setToolTip(s.description.toHtmlEscaped());
        cl->addWidget(desc);
        auto *wrapper = new QWidget;
        auto *wl = new QVBoxLayout(wrapper);
        wl->setContentsMargins(0, 0, 0, 6);
        wl->addWidget(card);
        m_listLayout->addWidget(wrapper);
    }
    m_listLayout->addStretch();
}

void SkillsWindow::closeEvent(QCloseEvent *event)
{
    // Singleton reuse: never destroy, only hide
    event->ignore();
    emit closeRequested();
}
