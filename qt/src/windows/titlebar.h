// Shared custom title bar for the settings/skills windows (SPEC 13.1):
// 18px logo + 15px SemiBold title + ChromeCloseButton, the whole row
// draggable (WPF DragMove equivalent). Header-only.

#ifndef TITLEBAR_H
#define TITLEBAR_H

#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPushButton>
#include <QWidget>
#include <optional>

class DraggableTitleBar : public QWidget
{
public:
    // Returns the title bar; *closeButton receives the "✕" ChromeCloseButton.
    static DraggableTitleBar *create(QWidget *window, const QString &title,
                                     QPushButton **closeButton)
    {
        auto *bar = new DraggableTitleBar(window);
        auto *layout = new QHBoxLayout(bar);
        layout->setContentsMargins(2, 0, 2, 0);
        layout->setSpacing(0);
        auto *logo = new QLabel(bar);
        logo->setObjectName(QStringLiteral("tbLogo"));
        logo->setFixedSize(18, 18);
        logo->setPixmap(QPixmap(QStringLiteral(":/assets/kimi-logo.png"))
                            .scaled(18, 18, Qt::KeepAspectRatio, Qt::SmoothTransformation));
        layout->addWidget(logo);
        layout->addSpacing(10);
        auto *text = new QLabel(title, bar);
        text->setObjectName(QStringLiteral("tbTitle"));
        layout->addWidget(text);
        layout->addStretch();
        auto *close = new QPushButton(QChar(0x2715), bar); // ✕
        close->setObjectName(QStringLiteral("chromeClose"));
        close->setFixedWidth(28);
        close->setCursor(Qt::PointingHandCursor);
        layout->addWidget(close);
        if (closeButton)
            *closeButton = close;
        return bar;
    }

protected:
    // SPEC 13.1: DragMove on left button down (outside the close button)
    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton)
            m_dragOffset = event->globalPosition().toPoint() - window()->frameGeometry().topLeft();
        QWidget::mousePressEvent(event);
    }

    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (m_dragOffset && (event->buttons() & Qt::LeftButton))
            window()->move(event->globalPosition().toPoint() - *m_dragOffset);
        QWidget::mouseMoveEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        m_dragOffset.reset();
        QWidget::mouseReleaseEvent(event);
    }

private:
    using QWidget::QWidget;
    std::optional<QPoint> m_dragOffset;
};

#endif // TITLEBAR_H
