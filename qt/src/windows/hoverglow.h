// Hover glow for clickable controls (SPEC 15.3): an accent-colored outer halo
// that fades in/out over 140ms on hover.
//
// Implementation note (Phase-4 flicker fix): the first version animated a
// QGraphicsDropShadowEffect on the target widget, which flickered badly inside
// WA_TranslucentBackground frameless windows (effect enable/disable switches
// render paths, and every frame re-renders the subtree to a pixmap). This
// version is flicker-free: the halo is a separate child widget of the
// TOP-LEVEL window, painted after the QSS-styled subtree, with a transparent
// center and WA_TransparentForMouseEvents — the target's own painting is never
// touched. The halo is concentric 1px rounded-rect strokes with outward-
// decreasing alpha (box-shadow falloff), driven by a qreal progress member
// whose animation only calls update(). The 1px accent ring stays in QSS.

#ifndef HOVERGLOW_H
#define HOVERGLOW_H

#include <QEvent>
#include <QPainter>
#include <QVariantAnimation>
#include <QWidget>

class HoverGlow : public QWidget
{
public:
    // Peak alpha and radius per SPEC 15.3: light = 8px @ 18%, dark = 10px @ 22%.
    // cornerRadius should match the target's QSS border-radius.
    HoverGlow(QWidget *target, bool dark, int cornerRadius = 10)
        : QWidget(target->window()), m_target(target), m_radius(dark ? 10 : 8),
          m_corner(cornerRadius), m_peak(dark ? 0.22 : 0.18)
    {
        setAttribute(Qt::WA_TransparentForMouseEvents);
        setAttribute(Qt::WA_TranslucentBackground);
        // Watch the whole ancestor chain: the panel's slide animation moves the
        // root container, and children get no Move events of their own — the
        // halo would desync without this (moves with the target otherwise).
        for (QWidget *w = target; w; w = w->parentWidget())
            w->installEventFilter(this);
        syncGeometry();
        QWidget::hide();
    }

protected:
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        const bool isTarget = watched == m_target;
        switch (event->type()) {
        case QEvent::Enter:
            if (!isTarget)
                break;
            m_hovered = true;
            syncGeometry();
            show();
            animateTo(1.0);
            break;
        case QEvent::Leave:
            if (!isTarget)
                break;
            m_hovered = false;
            animateTo(0.0);
            break;
        case QEvent::Move:
        case QEvent::Resize:
            syncGeometry(); // any ancestor (e.g. the sliding panel root) counts
            break;
        default:
            break;
        }
        return QWidget::eventFilter(watched, event);
    }

    void paintEvent(QPaintEvent *) override
    {
        if (m_progress <= 0.0)
            return;
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        // Concentric strokes, outermost faintest (box-shadow falloff)
        for (int i = 0; i < m_radius; ++i) {
            QColor c(26, 136, 255);
            c.setAlphaF(m_peak * m_progress * (1.0 - static_cast<qreal>(i) / m_radius));
            p.setPen(QPen(c, 1));
            p.setBrush(Qt::NoBrush);
            p.drawRoundedRect(QRectF(rect()).adjusted(i + 0.5, i + 0.5, -i - 0.5, -i - 0.5),
                              m_corner + i, m_corner + i);
        }
    }

private:
    void syncGeometry()
    {
        // Target rect in window coordinates, inflated by the halo radius
        const QRect r(m_target->mapTo(window(), QPoint(0, 0)), m_target->size());
        setGeometry(r.adjusted(-m_radius, -m_radius, m_radius, m_radius));
    }

    void animateTo(qreal to)
    {
        auto *anim = new QVariantAnimation(this);
        anim->setDuration(140);
        anim->setStartValue(m_progress);
        anim->setEndValue(to);
        anim->setEasingCurve(QEasingCurve::OutCubic);
        connect(anim, &QVariantAnimation::valueChanged, this, [this](const QVariant &v) {
            m_progress = v.toReal();
            update();
        });
        connect(anim, &QVariantAnimation::finished, this, [this, to] {
            if (to == 0.0 && !m_hovered) // a newer Enter may have superseded this Leave
                hide();
        });
        anim->start(QAbstractAnimation::DeleteWhenStopped);
    }

    QWidget *m_target;
    int m_radius;
    int m_corner;
    qreal m_peak;
    qreal m_progress = 0.0;
    bool m_hovered = false;
};

#endif // HOVERGLOW_H
