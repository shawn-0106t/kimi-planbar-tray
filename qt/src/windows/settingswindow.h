// Settings window, 1:1 port of rust/settings.html + settings.css
// (SPEC section 13): theme radios, interval pills, autostart checkbox,
// Save button. Singleton — hidden, never destroyed; backfills the current
// settings on every open (SPEC 13.2).

#ifndef SETTINGSWINDOW_H
#define SETTINGSWINDOW_H

#include "../settings.h"
#include "../theme.h"

#include <QList>
#include <QWidget>

class QPushButton;

class SettingsWindow : public QWidget
{
    Q_OBJECT
public:
    explicit SettingsWindow(QWidget *parent = nullptr);

    void applyTheme(const QString &effectiveTheme);
    // Backfill the controls from `data` (called on every open, SPEC 13.2).
    void backfill(const SettingsData &data);

signals:
    // theme: system|light|dark; refreshMinutes: 1|5|10|30
    void saveRequested(const QString &theme, qint64 refreshMinutes, bool autoStart);
    void closeRequested();

protected:
    void closeEvent(QCloseEvent *event) override; // never destroy: hide instead

private:
    class ChoiceItem; // painted circle radio / square checkbox row
    ChoiceItem *m_themeSystem = nullptr;
    ChoiceItem *m_themeLight = nullptr;
    ChoiceItem *m_themeDark = nullptr;
    QList<QPushButton *> m_pills; // 1/5/10/30 min, exclusive
    ChoiceItem *m_autostart = nullptr;
    QPushButton *m_saveBtn = nullptr;
    QList<QObject *> m_glows; // HoverGlow instances, recreated per theme
};

#endif // SETTINGSWINDOW_H
