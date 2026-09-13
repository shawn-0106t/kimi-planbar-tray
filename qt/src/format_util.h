// Formatting helpers, 1:1 port of rust/src/common.ts (SPEC 12.3 / 12.5).

#ifndef FORMAT_UTIL_H
#define FORMAT_UTIL_H

#include <QDateTime>
#include <QString>
#include <cmath>

// FormatReset: span = at - now, English countdown text.
inline QString formatReset(const QDateTime &at)
{
    if (!at.isValid())
        return QString();
    const qint64 spanMs = at.toMSecsSinceEpoch() - QDateTime::currentMSecsSinceEpoch();
    if (spanMs < 0)
        return QStringLiteral("Resets soon");
    const qint64 totalSec = spanMs / 1000;
    const qint64 days = totalSec / 86400;
    if (days >= 1) {
        const qint64 hours = (totalSec / 3600) % 24;
        return QStringLiteral("Resets in %1d %2h").arg(days).arg(hours);
    }
    const qint64 totalHours = totalSec / 3600;
    if (totalHours >= 1) {
        const qint64 minutes = (totalSec / 60) % 60;
        return QStringLiteral("Resets in %1h %2m").arg(totalHours).arg(minutes);
    }
    const qint64 minutes = totalSec / 60;
    return QStringLiteral("Resets in %1m").arg(std::max<qint64>(1, minutes));
}

// FmtYuan: cents -> yuan string, fraction omitted for whole yuan.
inline QString fmtYuan(qint64 cents)
{
    if (cents < 0)
        return QStringLiteral("-") + fmtYuan(-cents);
    const qint64 yuan = cents / 100;
    const qint64 frac = cents % 100;
    return QChar(0x00A5) + QString::number(yuan)
        + (frac > 0 ? QStringLiteral(".") + QString::number(frac).rightJustified(2, u'0')
                    : QString());
}

// {Percent:0}% — display uses the raw (unclamped) percent.
inline QString fmtPercent(double percent)
{
    return QStringLiteral("%1%").arg(static_cast<qint64>(std::round(percent)));
}

inline double clampPercent(double percent)
{
    return std::min(100.0, std::max(0.0, percent));
}

#endif // FORMAT_UTIL_H
