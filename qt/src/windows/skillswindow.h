// Skills window, 1:1 port of rust/skills.html + skills.css (SPEC section 21):
// read-only grouped skill list, summary row with a manual rescan button.
// Zero background cost: the scan runs only when the window opens (first time)
// or Refresh is pressed. Singleton — hidden, never destroyed.

#ifndef SKILLSWINDOW_H
#define SKILLSWINDOW_H

#include "../skills.h"
#include "../theme.h"

#include <QWidget>

class QLabel;
class QPushButton;
class QVBoxLayout;
class QScrollArea;

class SkillsWindow : public QWidget
{
    Q_OBJECT
public:
    explicit SkillsWindow(QWidget *parent = nullptr);

    void applyTheme(const QString &effectiveTheme);
    void showLoading();
    void showFailed();
    // Render the grouped list. All external text is plain QLabel text
    // (Qt::PlainText — the Qt equivalent of the "no innerHTML" rule).
    void renderSkills(const QList<SkillInfo> &skills);

signals:
    void refreshRequested(); // rescan (refresh=true)
    void closeRequested();

protected:
    void closeEvent(QCloseEvent *event) override;

private:
    QLabel *m_summaryText = nullptr;
    QVBoxLayout *m_listLayout = nullptr;
    QPushButton *m_refreshBtn = nullptr;
    QString m_descColor; // secondary text color, applied to ClampLabel descs
    QList<QObject *> m_glows; // HoverGlow instances, recreated per theme
};

#endif // SKILLSWINDOW_H
