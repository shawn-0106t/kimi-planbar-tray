// Two-line-clamped text label with an ellipsis on the last visible line —
// the Qt equivalent of the CSS `-webkit-line-clamp: 2` skill description
// (SPEC 21.3). QLabel can only clip mid-glyph, so this paints via QTextLayout.

#ifndef CLAMPLABEL_H
#define CLAMPLABEL_H

#include <QColor>
#include <QPainter>
#include <QTextLayout>
#include <QWidget>

class ClampLabel : public QWidget
{
    Q_OBJECT
public:
    explicit ClampLabel(const QString &text, QWidget *parent = nullptr) : QWidget(parent)
    {
        setText(text);
        setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }

    void setText(const QString &text)
    {
        m_text = text;
        updateGeometry();
        update();
    }

    void setColor(const QColor &color)
    {
        m_color = color;
        update();
    }

    QSize sizeHint() const override
    {
        return QSize(100, fontMetrics().lineSpacing() * 2);
    }
    QSize minimumSizeHint() const override { return sizeHint(); }

protected:
    void paintEvent(QPaintEvent *) override
    {
        QPainter p(this);
        p.setPen(m_color);
        QTextLayout layout(m_text, font());
        QTextOption option;
        option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere); // CJK wraps anywhere
        layout.setTextOption(option);
        layout.beginLayout();
        QList<QTextLine> lines;
        bool overflow = false;
        while (lines.size() < 2) {
            QTextLine line = layout.createLine();
            if (!line.isValid())
                break;
            line.setLineWidth(width());
            lines.append(line);
        }
        if (lines.size() == 2 && layout.createLine().isValid())
            overflow = true;
        layout.endLayout();

        qreal y = 0;
        for (int i = 0; i < lines.size(); ++i) {
            const QTextLine &line = lines[i];
            if (overflow && i == lines.size() - 1) {
                // Elide the remaining text onto the last visible line
                const QString rest = m_text.mid(line.textStart());
                p.drawText(QPointF(0, y + fontMetrics().ascent()),
                           fontMetrics().elidedText(rest, Qt::ElideRight, width()));
            } else {
                line.draw(&p, QPointF(0, y));
            }
            y += fontMetrics().lineSpacing();
        }
    }

private:
    QString m_text;
    QColor m_color = Qt::black;
};

#endif // CLAMPLABEL_H
