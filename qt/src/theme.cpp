#include "theme.h"

#include <QGuiApplication>
#include <QStyleHints>

namespace theme {

QString systemTheme()
{
    return QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark
        ? QStringLiteral("dark")
        : QStringLiteral("light");
}

QString effective(const QString &configured)
{
    if (configured == QLatin1String("light"))
        return QStringLiteral("light");
    if (configured == QLatin1String("dark"))
        return QStringLiteral("dark");
    return systemTheme();
}

ThemeColors colors(const QString &effectiveTheme)
{
    // Values copied verbatim from rust/src/theme.css
    if (effectiveTheme == QLatin1String("dark")) {
        return {QStringLiteral("#1a88ff"), QStringLiteral("#17191e"), QStringLiteral("#23262d"),
                QStringLiteral("#f2f3f5"), QStringLiteral("#9aa0a8"), QStringLiteral("#3a3e47"),
                QStringLiteral("#2c3039"), QStringLiteral("#3a404b"), QStringLiteral("#3d2e1a"),
                QStringLiteral("#f0a040")};
    }
    return {QStringLiteral("#1a88ff"), QStringLiteral("#f3f4f6"), QStringLiteral("#ffffff"),
            QStringLiteral("#1f2329"), QStringLiteral("#6b7280"), QStringLiteral("#e5e7eb"),
            QStringLiteral("#e9ecf0"), QStringLiteral("#dce2e9"), QStringLiteral("#fff0e0"),
            QStringLiteral("#e06d00")};
}

} // namespace theme
