// Tray right-click menu window, 1:1 port of rust/menu.html + menu.css
// (SPEC 10.3): 5 items (Open / Refresh / Settings / Skills / Exit), shown at
// the cursor with horizontal clamping and bottom-edge flip-up, closed on
// focus loss. Same frameless/translucent/rounded-child-container pattern as
// the panel. Qt computes the content height directly from the layout, so the
// Rust edition's menu_height IPC round-trip is unnecessary here.

#ifndef MENUWINDOW_H
#define MENUWINDOW_H

#include "../theme.h"

#include <QWidget>

class QPushButton;

class MenuWindow : public QWidget
{
    Q_OBJECT
public:
    explicit MenuWindow(QWidget *parent = nullptr);

    void applyTheme(const QString &effectiveTheme);
    // ShowAtCursor (SPEC 10.3): position against the cursor's monitor work
    // area, grab the foreground so the menu actually receives focus.
    void showAtCursor();

signals:
    void actionTriggered(const QString &action); // open|refresh|settings|skills|quit
    void deactivated();                          // focus loss -> hide immediately

protected:
    bool event(QEvent *event) override;

private:
    QWidget *m_root = nullptr;
    QList<QPushButton *> m_items;
    QList<QObject *> m_glows; // HoverGlow instances, recreated per theme
};

#endif // MENUWINDOW_H
