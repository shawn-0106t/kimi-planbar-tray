#include "settings.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcessEnvironment>
#include <QSettings>

namespace settings {

QString configDir()
{
    const QString exeDir = QCoreApplication::applicationDirPath();
    if (!exeDir.isEmpty() && QFileInfo::exists(exeDir + QStringLiteral("/portable.dat")))
        return exeDir;
    const QString appdata =
        QProcessEnvironment::systemEnvironment().value(QStringLiteral("APPDATA"));
    if (!appdata.isEmpty())
        return QDir(appdata).filePath(QStringLiteral("KimiPlanbarTray"));
    return exeDir.isEmpty() ? QStringLiteral(".") : exeDir;
}

static QString filePath()
{
    return QDir(configDir()).filePath(QStringLiteral("settings.json"));
}

SettingsData load()
{
    SettingsData d;
    QFile f(filePath());
    if (!f.open(QIODevice::ReadOnly | QIODevice::Text))
        return d;
    const QJsonDocument doc = QJsonDocument::fromJson(f.readAll());
    if (!doc.isObject())
        return d; // invalid JSON -> defaults, like serde's unwrap_or_default
    const QJsonObject o = doc.object();
    // Tolerate a partial settings.json: missing fields take defaults
    const QJsonValue theme = o.value(QStringLiteral("Theme"));
    if (theme.isString())
        d.theme = theme.toString();
    const QJsonValue refresh = o.value(QStringLiteral("RefreshMinutes"));
    if (refresh.isDouble())
        d.refreshMinutes = static_cast<qint64>(refresh.toDouble());
    const QJsonValue autoStart = o.value(QStringLiteral("AutoStart"));
    if (autoStart.isBool())
        d.autoStart = autoStart.toBool();
    return d;
}

void save(const SettingsData &data)
{
    const QString dir = configDir();
    QDir().mkpath(dir);
    // Hand-serialized to keep serde_json::to_string_pretty byte shape
    // (2-space indent, key order Theme/RefreshMinutes/AutoStart). The theme
    // string is escaped anyway (defense in depth — it is an enum by design).
    QString theme = data.theme;
    theme.replace(QLatin1Char('\\'), QStringLiteral("\\\\"));
    theme.replace(QLatin1Char('"'), QStringLiteral("\\\""));
    const QString json = QStringLiteral("{\n  \"Theme\": \"%1\",\n  \"RefreshMinutes\": %2,\n"
                                        "  \"AutoStart\": %3\n}")
                             .arg(theme)
                             .arg(data.refreshMinutes)
                             .arg(data.autoStart ? QStringLiteral("true") : QStringLiteral("false"));
    QFile f(filePath());
    if (f.open(QIODevice::WriteOnly | QIODevice::Text))
        f.write(json.toUtf8());
}

void applyAutoStart(const SettingsData &data)
{
    QSettings run(QStringLiteral(
                      "HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"),
                  QSettings::NativeFormat);
    if (data.autoStart) {
        const QString exe = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
        run.setValue(QStringLiteral("KimiPlanbarTray"), QStringLiteral("\"%1\"").arg(exe));
    } else {
        run.remove(QStringLiteral("KimiPlanbarTray"));
    }
}

} // namespace settings
