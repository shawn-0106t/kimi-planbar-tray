// Main usage panel window, 1:1 port of rust/index.html + rust/src/main.css
// (SPEC sections 10.1, 12, 15). The window is a translucent frameless
// top-level; rounding is painted on the child container via QSS
// border-radius (unreliable on top-level windows, QT-MIGRATION 4.1).

#ifndef PANELWINDOW_H
#define PANELWINDOW_H

#include "../quota.h"
#include "../theme.h"
#include "../update.h"

#include <QLabel>
#include <QPushButton>
#include <QStringList>
#include <QWidget>

class QFrame;
class QPropertyAnimation;

// 6px capsule progress bar, 3px radius (SPEC 11.2).
class BarWidget : public QWidget
{
public:
    explicit BarWidget(QWidget *parent = nullptr);
    void setPercent(double percent); // raw value; painting clamps to [0,100]
    void setColors(const QColor &track, const QColor &fill);
    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent *event) override;

private:
    double m_percent = 0.0;
    QColor m_track;
    QColor m_fill;
};

// Action button: 16px icon on top, 12px label below, 2px gap (SPEC 12.7).
class IconButton : public QPushButton
{
    Q_OBJECT
public:
    IconButton(const QStringList &svgPathData, const QString &label, QWidget *parent = nullptr);
    void setIconColor(const QColor &color);

private:
    QStringList m_svgPathData;
    QLabel *m_iconLabel;
};

// QFrame that emits clicked() (the version row, SPEC 12.6).
class ClickableFrame : public QFrame
{
    Q_OBJECT
public:
    using QFrame::QFrame;
signals:
    void clicked();
protected:
    void mouseReleaseEvent(QMouseEvent *event) override;
};

class PanelWindow : public QWidget
{
    Q_OBJECT
public:
    explicit PanelWindow(QWidget *parent = nullptr);

    void applyTheme(const QString &effectiveTheme);
    void renderQuota(const std::optional<QuotaResult> &r); // nullopt = no data yet
    void renderUpdate(const UpdateStatus &u);

    // Show/hide animations (SPEC 15.1 / 15.2). playShow assumes the window is
    // already visible at its final position; playHide emits hideFinished()
    // ~170ms later (fade-out done), when the caller must actually hide().
    void playShow();
    void playHide();

signals:
    void hideFinished();
    void deactivated(); // WindowDeactivate, for focus-loss auto-hide
    void refreshRequested();
    void consoleRequested();
    void releasesRequested();
    void settingsRequested(); // Phase 3: settings window
    void exitRequested();

protected:
    bool event(QEvent *event) override;

private:
    void buildUi();
    void reloadLogo();
    void reloadIcons(); // re-render at the current screen's DPR
    void setCard(QLabel *pct, BarWidget *fill, QLabel *reset,
                 const std::optional<QuotaSegment> &seg);
    void renderExtra(const std::optional<ExtraInfo> &extra);

    QWidget *m_root = nullptr; // rounded, shadowed container
    QLabel *m_logo = nullptr;
    QLabel *m_lastUpdated = nullptr;
    QLabel *m_weekPct = nullptr;
    QLabel *m_fivePct = nullptr;
    BarWidget *m_weekBar = nullptr;
    BarWidget *m_fiveBar = nullptr;
    QLabel *m_weekReset = nullptr;
    QLabel *m_fiveReset = nullptr;
    QLabel *m_extraBalance = nullptr;
    QWidget *m_extraMonthly = nullptr;
    BarWidget *m_extraBar = nullptr;
    QLabel *m_extraMonthlyText = nullptr;
    QLabel *m_cliVersion = nullptr;
    QLabel *m_badge = nullptr;
    QWidget *m_versionRow = nullptr;
    QList<IconButton *> m_buttons;
    QList<QObject *> m_glows; // HoverGlow instances, recreated per theme

    ThemeColors m_colors;
    // Generation counter: a pending hide timer must not fire after a newer
    // show/hide, or a rapid hide->show->hide sequence would cut the last hide
    // animation short (mirrors the frontend's hideGen).
    int m_hideGen = 0;
};

#endif // PANELWINDOW_H
