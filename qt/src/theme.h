// Theme (Moonlit light / Moondark dark), 1:1 port of rust/src/theme.css —
// the CSS variables are the single source of theme colors (SPEC section 11).
// System-theme resolution uses QStyleHints::colorScheme (QT-MIGRATION 3).

#ifndef THEME_H
#define THEME_H

#include <QString>

struct ThemeColors {
    QString accent;
    QString windowBg;
    QString cardBg;
    QString textPrimary;
    QString textSecondary;
    QString progressTrack;
    QString buttonBg;
    QString buttonHover;
    QString badgeBg;
    QString badgeFg;
};

namespace theme {

// 0 = dark, 1 (or missing/unknown) = light; reads AppsUseLightTheme via Qt.
QString systemTheme();

// Resolve the configured theme ("system" follows the OS) to light|dark.
QString effective(const QString &configured);

ThemeColors colors(const QString &effectiveTheme);

} // namespace theme

#endif // THEME_H
