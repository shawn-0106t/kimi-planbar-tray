#include "panelwindow.h"

#include "../format_util.h"
#include "../icons.h"
#include "hoverglow.h"

#include <QGraphicsDropShadowEffect>
#include <QHBoxLayout>
#include <QMouseEvent>
#include <QPainter>
#include <QPropertyAnimation>
#include <QSvgRenderer>
#include <QTimer>

namespace {

// Window size includes the 28px transparent shadow fade room around the card
// on every side (SPEC 10.1 + rust panel.rs margin-growth comment).
constexpr int kWindowW = 424;
constexpr int kWindowH = 520;
constexpr int kMargin = 28;
constexpr int kCardW = kWindowW - 2 * kMargin; // 368
constexpr int kCardH = kWindowH - 2 * kMargin; // 464

QPixmap svgIcon(const QStringList &paths, const QColor &color, qreal dpr)
{
    QString svg = QStringLiteral("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\">");
    for (const QString &d : paths)
        svg += QStringLiteral("<path d=\"%1\" fill=\"%2\"/>").arg(d, color.name());
    svg += QStringLiteral("</svg>");
    QSvgRenderer renderer(svg.toUtf8());
    QPixmap px(static_cast<int>(16 * dpr), static_cast<int>(16 * dpr));
    px.setDevicePixelRatio(dpr);
    px.fill(Qt::transparent);
    QPainter p(&px);
    renderer.render(&p, QRectF(0, 0, 16, 16));
    return px;
}

} // namespace

// ---- BarWidget ----

BarWidget::BarWidget(QWidget *parent) : QWidget(parent)
{
    setFixedHeight(6);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
}

void BarWidget::setPercent(double percent)
{
    m_percent = percent;
    update();
}

void BarWidget::setColors(const QColor &track, const QColor &fill)
{
    m_track = track;
    m_fill = fill;
    update();
}

QSize BarWidget::sizeHint() const
{
    return QSize(100, 6);
}

void BarWidget::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    p.setPen(Qt::NoPen);
    p.setBrush(m_track);
    p.drawRoundedRect(rect(), 3, 3);
    const double clamped = clampPercent(m_percent);
    if (clamped > 0.0) {
        const qreal w = width() * clamped / 100.0;
        p.setBrush(m_fill);
        p.drawRoundedRect(QRectF(0, 0, w, height()), 3, 3);
    }
}

// ---- IconButton ----

IconButton::IconButton(const QStringList &svgPathData, const QString &label, QWidget *parent)
    : QPushButton(parent), m_svgPathData(svgPathData)
{
    setObjectName(QStringLiteral("actionBtn"));
    setCursor(Qt::PointingHandCursor);
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 5, 0, 6);
    layout->setSpacing(2);
    m_iconLabel = new QLabel(this);
    m_iconLabel->setFixedSize(16, 16);
    m_iconLabel->setAlignment(Qt::AlignCenter);
    auto *text = new QLabel(label, this);
    text->setAlignment(Qt::AlignCenter);
    layout->addWidget(m_iconLabel, 0, Qt::AlignHCenter);
    layout->addWidget(text);
    // QPushButton's style-based sizeHint ignores the child layout (a single
    // text line), which let the actions row collapse; pin the intended height
    // (padding 5+6, icon 16, gap 2, one 12px text line ~= 16).
    setMinimumHeight(45);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
}

void IconButton::setIconColor(const QColor &color)
{
    m_iconLabel->setPixmap(svgIcon(m_svgPathData, color, devicePixelRatioF()));
}

// ---- ClickableFrame ----

void ClickableFrame::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && rect().contains(event->pos()))
        emit clicked();
    QFrame::mouseReleaseEvent(event);
}

// ---- PanelWindow ----

PanelWindow::PanelWindow(QWidget *parent) : QWidget(parent)
{
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    setFixedSize(kWindowW, kWindowH);
    buildUi();
}

void PanelWindow::buildUi()
{
    m_root = new QWidget(this);
    m_root->setObjectName(QStringLiteral("root"));
    m_root->setGeometry(kMargin, kMargin, kCardW, kCardH);

    // DropShadowEffect BlurRadius=24 Depth=2 Opacity=0.25 (SPEC 10.1)
    auto *shadow = new QGraphicsDropShadowEffect(m_root);
    shadow->setBlurRadius(24);
    shadow->setOffset(0, 2);
    shadow->setColor(QColor(0, 0, 0, 64)); // 0.25 * 255
    m_root->setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(m_root);
    layout->setContentsMargins(16, 16, 16, 16);
    layout->setSpacing(0);

    // Header (SPEC 12.1)
    auto *header = new QHBoxLayout;
    header->setContentsMargins(2, 0, 2, 0);
    header->setSpacing(0);
    auto *logo = new QLabel(m_root);
    logo->setObjectName(QStringLiteral("logo"));
    logo->setFixedSize(20, 20);
    m_logo = logo;
    reloadLogo();
    header->addWidget(logo);
    header->addSpacing(10);
    auto *title = new QLabel(QStringLiteral("Kimi Planbar Tray"), m_root);
    title->setObjectName(QStringLiteral("title"));
    header->addWidget(title);
    header->addStretch();
    m_lastUpdated = new QLabel(m_root);
    m_lastUpdated->setObjectName(QStringLiteral("lastUpdated"));
    header->addWidget(m_lastUpdated);
    layout->addLayout(header);
    layout->addSpacing(16);

    // Usage cards (SPEC 12.2)
    const auto makeCard = [this](const QString &cardTitle, QLabel **pct, BarWidget **bar,
                                 QLabel **reset) -> QFrame * {
        auto *card = new QFrame(m_root);
        card->setObjectName(QStringLiteral("card"));
        auto *cl = new QVBoxLayout(card);
        cl->setContentsMargins(16, 16, 16, 16);
        cl->setSpacing(0);
        auto *t = new QLabel(cardTitle, card);
        t->setObjectName(QStringLiteral("cardTitle"));
        cl->addWidget(t);
        cl->addSpacing(10);
        *pct = new QLabel(QStringLiteral("--"), card);
        (*pct)->setObjectName(QStringLiteral("pct"));
        cl->addWidget(*pct);
        cl->addSpacing(10);
        *bar = new BarWidget(card);
        cl->addWidget(*bar);
        cl->addSpacing(12);
        *reset = new QLabel(card);
        (*reset)->setObjectName(QStringLiteral("reset"));
        cl->addWidget(*reset);
        cl->addStretch();
        return card;
    };
    auto *cards = new QHBoxLayout;
    cards->setContentsMargins(0, 0, 0, 0);
    cards->setSpacing(12);
    // Order matches index.html: weekly card on the left, 5-hour on the right
    cards->addWidget(makeCard(QStringLiteral("Weekly usage"), &m_weekPct, &m_weekBar, &m_weekReset));
    cards->addWidget(makeCard(QStringLiteral("5-hour usage"), &m_fivePct, &m_fiveBar, &m_fiveReset));
    layout->addLayout(cards, 1);

    // Extra Usage card (SPEC 12.4)
    layout->addSpacing(12);
    auto *extra = new QFrame(m_root);
    extra->setObjectName(QStringLiteral("card"));
    auto *el = new QVBoxLayout(extra);
    el->setContentsMargins(16, 12, 16, 12);
    el->setSpacing(0);
    auto *extraRow = new QHBoxLayout;
    extraRow->setContentsMargins(0, 0, 0, 0);
    auto *extraTitle = new QLabel(QStringLiteral("Extra Usage"), extra);
    extraTitle->setObjectName(QStringLiteral("cardTitle"));
    extraRow->addWidget(extraTitle);
    extraRow->addStretch();
    m_extraBalance = new QLabel(QStringLiteral("--"), extra);
    m_extraBalance->setObjectName(QStringLiteral("extraBalance"));
    extraRow->addWidget(m_extraBalance);
    el->addLayout(extraRow);
    m_extraMonthly = new QWidget(extra);
    auto *ml = new QVBoxLayout(m_extraMonthly);
    ml->setContentsMargins(0, 8, 0, 0);
    ml->setSpacing(8);
    m_extraBar = new BarWidget(m_extraMonthly);
    ml->addWidget(m_extraBar);
    m_extraMonthlyText = new QLabel(m_extraMonthly);
    m_extraMonthlyText->setObjectName(QStringLiteral("extraMonthlyText"));
    ml->addWidget(m_extraMonthlyText);
    m_extraMonthly->setVisible(false);
    el->addWidget(m_extraMonthly);
    layout->addWidget(extra);

    // Version row (SPEC 12.6)
    layout->addSpacing(12);
    auto *version = new ClickableFrame(m_root);
    version->setObjectName(QStringLiteral("versionRow"));
    version->setCursor(Qt::PointingHandCursor);
    version->setToolTip(QStringLiteral("View Kimi Code releases"));
    auto *vl = new QHBoxLayout(version);
    vl->setContentsMargins(14, 10, 14, 10);
    vl->setSpacing(0);
    auto *versionLabel = new QLabel(QStringLiteral("Kimi Code CLI"), version);
    versionLabel->setObjectName(QStringLiteral("versionLabel"));
    vl->addWidget(versionLabel);
    vl->addStretch();
    m_cliVersion = new QLabel(QStringLiteral("--"), version);
    m_cliVersion->setObjectName(QStringLiteral("cliVersion"));
    vl->addWidget(m_cliVersion);
    vl->addSpacing(8);
    m_badge = new QLabel(QStringLiteral("Update available"), version);
    m_badge->setObjectName(QStringLiteral("badge"));
    m_badge->setVisible(false);
    vl->addWidget(m_badge);
    connect(version, &ClickableFrame::clicked, this, &PanelWindow::releasesRequested);
    layout->addWidget(version);
    m_versionRow = version;

    // Bottom buttons (SPEC 12.7)
    layout->addSpacing(12);
    auto *actions = new QHBoxLayout;
    actions->setContentsMargins(0, 0, 0, 0);
    actions->setSpacing(9); // WPF margins 6+3 = 9px gaps
    const struct {
        const QStringList *paths;
        const char *label;
        const char *tooltip;
        void (PanelWindow::*signal)();
    } defs[] = {
        {&icons::console, "Console", "Open Kimi Code Console", &PanelWindow::consoleRequested},
        {&icons::refresh, "Refresh", nullptr, &PanelWindow::refreshRequested},
        {&icons::settings, "Settings", nullptr, &PanelWindow::settingsRequested},
        {&icons::exit_, "Exit", nullptr, &PanelWindow::exitRequested},
    };
    for (const auto &d : defs) {
        auto *btn = new IconButton(*d.paths, QString::fromLatin1(d.label), m_root);
        if (d.tooltip)
            btn->setToolTip(QString::fromLatin1(d.tooltip));
        connect(btn, &QPushButton::clicked, this, d.signal);
        m_buttons.append(btn);
        actions->addWidget(btn);
    }
    layout->addLayout(actions);
}

void PanelWindow::applyTheme(const QString &effectiveTheme)
{
    m_colors = theme::colors(effectiveTheme);
    const ThemeColors &c = m_colors;
    const QString glow = effectiveTheme == QLatin1String("dark")
        ? QStringLiteral("rgba(26, 136, 255, 115)") // 45%
        : QStringLiteral("rgba(26, 136, 255, 89)"); // 35%
    // QSS approximation of the SPEC 15.3 hover glow: 1px accent ring only
    // (QSS has no ::after / box-shadow; the halo is dropped, see HANDOFF).
    QString qss = QStringLiteral(
        "QWidget#root { background: %1; border-radius: 14px; }\n"
        "QFrame#card { background: %2; border-radius: 12px; }\n"
        "ClickableFrame#versionRow { background: %2; border-radius: 12px; border: 1px solid transparent; }\n"
        "ClickableFrame#versionRow:hover { border-color: @GLOW@; }\n"
        "QLabel { background: transparent; }\n"
        "QLabel#title { font-size: 17px; font-weight: 600; color: %3; }\n"
        "QLabel#lastUpdated { font-size: 11px; color: %4; }\n"
        "QLabel#cardTitle { font-size: 13px; color: %4; }\n"
        "QLabel#pct { font-size: 32px; font-weight: bold; color: %3; }\n"
        "QLabel#reset { font-size: 11px; color: %4; }\n"
        "QLabel#extraBalance { font-size: 18px; font-weight: bold; color: %3; }\n"
        "QLabel#extraMonthlyText { font-size: 11px; color: %4; }\n"
        "QLabel#versionLabel { font-size: 13px; color: %3; }\n"
        "QLabel#cliVersion { font-size: 13px; color: %4; }\n"
        "QLabel#badge { background: %8; color: %9; border-radius: 8px; padding: 2px 8px; font-size: 11px; }\n"
        "QPushButton#actionBtn { background: %2; border: 1px solid transparent; border-radius: 10px; color: %3; font-size: 12px; }\n"
        "QPushButton#actionBtn:hover { background: %7; border-color: @GLOW@; }\n"
        "QPushButton#actionBtn QLabel { color: %3; font-size: 12px; }\n"
        "QToolTip { background: %2; color: %3; border: 1px solid %5; padding: 4px 6px; }\n")
                      .arg(c.windowBg, c.cardBg, c.textPrimary, c.textSecondary, c.progressTrack,
                           c.buttonBg, c.buttonHover, c.badgeBg, c.badgeFg);
    qss.replace(QStringLiteral("@GLOW@"), glow);
    setStyleSheet(qss);
    const QColor iconColor(c.textPrimary);
    for (IconButton *b : m_buttons)
        b->setIconColor(iconColor);
    const QColor track(c.progressTrack), fill(c.accent);
    for (BarWidget *b : {m_weekBar, m_fiveBar, m_extraBar})
        b->setColors(track, fill);

    // SPEC 15.3 hover glow: 8px/10px accent halo, 140ms fade (the 1px ring
    // stays in the QSS hover border above). Recreated per theme switch.
    qDeleteAll(m_glows);
    m_glows.clear();
    const bool dark = effectiveTheme == QLatin1String("dark");
    for (IconButton *b : m_buttons)
        m_glows.append(new HoverGlow(b, dark));
    m_glows.append(new HoverGlow(m_versionRow, dark, 12));
}

void PanelWindow::setCard(QLabel *pct, BarWidget *fill, QLabel *reset,
                          const std::optional<QuotaSegment> &seg)
{
    if (!seg) {
        pct->setText(QStringLiteral("--"));
        fill->setPercent(0.0);
        reset->clear();
        return;
    }
    pct->setText(fmtPercent(seg->percent));
    fill->setPercent(seg->percent);
    reset->setText(seg->resetAt ? formatReset(*seg->resetAt) : QString());
}

void PanelWindow::renderExtra(const std::optional<ExtraInfo> &extra)
{
    if (!extra) {
        m_extraBalance->setText(QStringLiteral("--"));
        m_extraMonthly->setVisible(false);
        return;
    }
    switch (extra->state) {
    case ExtraState::Ready:
        m_extraBalance->setText(extra->balanceCents ? fmtYuan(*extra->balanceCents)
                                                    : QStringLiteral("--"));
        break;
    case ExtraState::NoData:
        m_extraBalance->setText(QStringLiteral("No data"));
        break;
    case ExtraState::NotActivated:
        m_extraBalance->setText(QStringLiteral("Not activated"));
        break;
    }
    if (extra->monthlyEnabled && extra->monthlyLimitCents && *extra->monthlyLimitCents > 0
        && extra->monthlyUsedCents) {
        m_extraBar->setPercent(static_cast<double>(*extra->monthlyUsedCents) * 100.0
                               / static_cast<double>(*extra->monthlyLimitCents));
        m_extraMonthlyText->setText(QStringLiteral("Used %1 this month / %2 limit")
                                        .arg(fmtYuan(*extra->monthlyUsedCents),
                                             fmtYuan(*extra->monthlyLimitCents)));
        m_extraMonthly->setVisible(true);
    } else {
        m_extraMonthly->setVisible(false);
    }
}

void PanelWindow::renderQuota(const std::optional<QuotaResult> &r)
{
    setCard(m_weekPct, m_weekBar, m_weekReset, r ? r->week : std::nullopt);
    setCard(m_fivePct, m_fiveBar, m_fiveReset, r ? r->fiveHour : std::nullopt);
    renderExtra(r ? r->extra : std::nullopt);
    if (!r) {
        m_lastUpdated->clear();
    } else if (r->error) {
        m_lastUpdated->setText(QStringLiteral("Update failed"));
    } else {
        m_lastUpdated->setText(
            QStringLiteral("Updated %1").arg(r->fetchedAt.toString(QStringLiteral("HH:mm"))));
    }
}

void PanelWindow::renderUpdate(const UpdateStatus &u)
{
    m_cliVersion->setText(u.localVersion.value_or(QStringLiteral("Not detected")));
    m_badge->setVisible(u.updateAvailable);
}

void PanelWindow::playShow()
{
    ++m_hideGen; // supersede any pending hide timer
    // SPEC 15.1: fade 160ms linear + slide 16px up over 220ms EaseOut cubic
    setWindowOpacity(0.0);
    m_root->move(kMargin, kMargin + 16);
    auto *fade = new QPropertyAnimation(this, "windowOpacity", this);
    fade->setDuration(160);
    fade->setStartValue(0.0);
    fade->setEndValue(1.0);
    fade->setEasingCurve(QEasingCurve::Linear);
    fade->start(QAbstractAnimation::DeleteWhenStopped);
    auto *slide = new QPropertyAnimation(m_root, "pos", this);
    slide->setDuration(220);
    slide->setStartValue(QPoint(kMargin, kMargin + 16));
    slide->setEndValue(QPoint(kMargin, kMargin));
    slide->setEasingCurve(QEasingCurve::OutCubic);
    slide->start(QAbstractAnimation::DeleteWhenStopped);
}

void PanelWindow::playHide()
{
    const int gen = ++m_hideGen;
    // SPEC 15.2: fade out 130ms linear + slide 12px down over 160ms EaseIn
    // cubic; the window actually hides after the fade-out completes (170ms,
    // as the frontend's setTimeout in main.ts).
    auto *fade = new QPropertyAnimation(this, "windowOpacity", this);
    fade->setDuration(130);
    fade->setStartValue(1.0);
    fade->setEndValue(0.0);
    fade->setEasingCurve(QEasingCurve::Linear);
    fade->start(QAbstractAnimation::DeleteWhenStopped);
    auto *slide = new QPropertyAnimation(m_root, "pos", this);
    slide->setDuration(160);
    slide->setStartValue(QPoint(kMargin, kMargin));
    slide->setEndValue(QPoint(kMargin, kMargin + 12));
    slide->setEasingCurve(QEasingCurve::InCubic);
    slide->start(QAbstractAnimation::DeleteWhenStopped);
    QTimer::singleShot(170, this, [this, gen]() {
        if (gen != m_hideGen)
            return; // superseded by a newer show/hide
        emit hideFinished();
        // Reset visual state for the next show (SPEC 15.2: Opacity = 1)
        setWindowOpacity(1.0);
        m_root->move(kMargin, kMargin);
    });
}

bool PanelWindow::event(QEvent *event)
{
    if (event->type() == QEvent::WindowDeactivate)
        emit deactivated();
    // Icons/logo were rendered at the DPR of the screen the window was
    // created on; re-render when the window lands on a different-DPI monitor.
    if (event->type() == QEvent::ScreenChangeInternal)
        reloadIcons();
    return QWidget::event(event);
}

void PanelWindow::reloadLogo()
{
    const qreal dpr = devicePixelRatioF();
    QPixmap px(static_cast<int>(20 * dpr), static_cast<int>(20 * dpr));
    px.setDevicePixelRatio(dpr);
    px.fill(Qt::transparent);
    QPixmap src(QStringLiteral(":/assets/kimi-logo.png"));
    QPainter p(&px);
    p.setRenderHint(QPainter::SmoothPixmapTransform);
    p.drawPixmap(QRectF(0, 0, 20, 20), src, QRectF(src.rect()));
    m_logo->setPixmap(px);
}

void PanelWindow::reloadIcons()
{
    const QColor iconColor(m_colors.textPrimary);
    for (IconButton *b : m_buttons)
        b->setIconColor(iconColor);
    reloadLogo();
}
