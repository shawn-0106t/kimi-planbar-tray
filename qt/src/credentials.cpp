#include "credentials.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QProcessEnvironment>
#include <QRegularExpression>

namespace credentials {

std::optional<QString> homeDir()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString p = env.value(QStringLiteral("USERPROFILE"));
    if (!p.isEmpty())
        return p;
    // Fallback: HOMEDRIVE + HOMEPATH
    const QString d = env.value(QStringLiteral("HOMEDRIVE"));
    const QString h = env.value(QStringLiteral("HOMEPATH"));
    if (!d.isEmpty() && !h.isEmpty())
        return d + h;
    return std::nullopt;
}

QString kimiHome(const QString &home)
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString p = env.value(QStringLiteral("KIMI_CODE_HOME"));
    if (!p.isEmpty())
        return p;
    return QDir(home).filePath(QStringLiteral(".kimi-code"));
}

// JSON number-or-string -> double (server models numbers as strings).
static std::optional<double> asF64(const QJsonValue &v)
{
    if (v.isDouble())
        return v.toDouble();
    if (v.isString()) {
        bool ok = false;
        const double d = v.toString().trimmed().toDouble(&ok);
        if (ok)
            return d;
    }
    return std::nullopt;
}

// config.toml provider match: section must be [providers.*], base_url must
// contain api.kimi.com/coding, api_key must be non-empty.
static std::optional<QString> matchProvider(const std::optional<QString> &section,
                                            const std::optional<QString> &baseUrl,
                                            const std::optional<QString> &apiKey)
{
    if (section && baseUrl && apiKey && section->startsWith(QStringLiteral("providers."))
        && baseUrl->contains(QStringLiteral("api.kimi.com/coding")) && !apiKey->isEmpty()) {
        return apiKey;
    }
    return std::nullopt;
}

std::optional<QString> loadToken()
{
    const std::optional<QString> home = homeDir();
    if (!home)
        return std::nullopt;
    const QString kimi = kimiHome(*home);

    // 1) OAuth access token from the credentials store
    QFile cred(QDir(kimi).filePath(QStringLiteral("credentials/kimi-code.json")));
    if (cred.open(QIODevice::ReadOnly | QIODevice::Text)) {
        const QJsonDocument doc = QJsonDocument::fromJson(cred.readAll());
        if (doc.isObject()) {
            const QJsonObject v = doc.object();
            const QJsonValue at = v.value(QStringLiteral("access_token"));
            if (at.isString()) {
                const double exp = asF64(v.value(QStringLiteral("expires_at"))).value_or(0.0);
                const double now = static_cast<double>(QDateTime::currentSecsSinceEpoch());
                if (exp > now + 30.0)
                    return at.toString();
            }
        }
    }

    // 2) config.toml fallback: line-by-line parse (not a full TOML parser)
    QFile cfg(QDir(kimi).filePath(QStringLiteral("config.toml")));
    if (!cfg.open(QIODevice::ReadOnly | QIODevice::Text))
        return std::nullopt;
    const QString text = QString::fromUtf8(cfg.readAll());
    static const QRegularExpression kv(
        QStringLiteral("^(base_url|api_key)\\s*=\\s*\"([^\"]*)\""));
    std::optional<QString> section;
    std::optional<QString> baseUrl;
    std::optional<QString> apiKey;
    const QStringList lines = text.split(QLatin1Char('\n'));
    for (const QString &raw : lines) {
        const QString line = raw.trimmed();
        if (line.startsWith(QLatin1Char('['))) {
            // Settle the previous section before starting a new one
            if (std::optional<QString> found = matchProvider(section, baseUrl, apiKey))
                return found;
            // Rust trim_matches: strip all leading/trailing '[' and ']' chars
            int start = 0;
            int end = line.size();
            while (start < end && (line[start] == QLatin1Char('[') || line[start] == QLatin1Char(']')))
                ++start;
            while (end > start && (line[end - 1] == QLatin1Char('[') || line[end - 1] == QLatin1Char(']')))
                --end;
            section = line.mid(start, end - start);
            baseUrl.reset();
            apiKey.reset();
            continue;
        }
        const QRegularExpressionMatch m = kv.match(line);
        if (m.hasMatch()) {
            if (m.captured(1) == QLatin1String("base_url"))
                baseUrl = m.captured(2);
            else
                apiKey = m.captured(2);
        }
    }
    return matchProvider(section, baseUrl, apiKey);
}

} // namespace credentials
