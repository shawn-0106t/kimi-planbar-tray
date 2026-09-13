#include "update.h"

#include "http_util.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QRegularExpression>

#include <windows.h>

namespace {

const char kChangelogUrl[] = "https://moonshotai.github.io/kimi-code/en/release-notes/changelog.md";
const char kGithubLatestUrl[] = "https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest";

// x.y.z -> comparable tuple (semantic version compare, SPEC 17.3).
// Mirrors Rust: splits on '.', first three parts must parse as u64.
std::optional<std::tuple<quint64, quint64, quint64>> parseSemver(const QString &v)
{
    const QStringList parts = v.split(QLatin1Char('.'));
    if (parts.size() < 3)
        return std::nullopt;
    quint64 nums[3];
    for (int i = 0; i < 3; ++i) {
        bool ok = false;
        nums[i] = parts[i].toULongLong(&ok);
        if (!ok)
            return std::nullopt;
    }
    return std::make_tuple(nums[0], nums[1], nums[2]);
}

// `kimi --version`, 5000ms timeout then kill; first \d+\.\d+\.\d+ in stdout+stderr.
std::optional<QString> detectLocalVersion()
{
    QProcess proc;
    proc.setProgram(QStringLiteral("kimi"));
    proc.setArguments({QStringLiteral("--version")});
    // Same as the Rust edition's creation_flags(CREATE_NO_WINDOW)
    proc.setCreateProcessArgumentsModifier([](QProcess::CreateProcessArguments *args) {
        args->flags |= CREATE_NO_WINDOW;
    });
    proc.start();
    if (!proc.waitForStarted())
        return std::nullopt;
    if (!proc.waitForFinished(5000)) {
        proc.kill();
        proc.waitForFinished(1000);
        return std::nullopt;
    }
    const QString text =
        QString::fromUtf8(proc.readAllStandardOutput()) + QString::fromUtf8(proc.readAllStandardError());
    static const QRegularExpression re(QStringLiteral("\\d+\\.\\d+\\.\\d+"));
    const QRegularExpressionMatch m = re.match(text);
    if (!m.hasMatch())
        return std::nullopt;
    return m.captured(0);
}

// Official docs changelog: Range bytes=0-4095, first `## x.y.z` heading wins.
// (GitHub Pages may ignore Range and return 200 with the full body; both are fine.)
std::optional<QString> fetchLatestFromChangelog()
{
    const SyncHttpResult resp =
        syncGet(QUrl(QString::fromLatin1(kChangelogUrl)), {{QByteArray("Range"), QByteArray("bytes=0-4095")}});
    if (!resp.ok || resp.status < 200 || resp.status >= 300)
        return std::nullopt;
    static const QRegularExpression re(QStringLiteral("^## (\\d+\\.\\d+\\.\\d+)"),
                                       QRegularExpression::MultilineOption);
    const QRegularExpressionMatch m = re.match(QString::fromUtf8(resp.body));
    if (!m.hasMatch())
        return std::nullopt;
    return m.captured(1);
}

// GitHub Releases API fallback (User-Agent header is mandatory).
std::optional<QString> fetchLatestFromGithub()
{
    const SyncHttpResult resp = syncGet(QUrl(QString::fromLatin1(kGithubLatestUrl)),
                                        {{QByteArray("User-Agent"), QByteArray("KimiPlanbarTray")}});
    if (!resp.ok || resp.status < 200 || resp.status >= 300)
        return std::nullopt;
    const QJsonDocument doc = QJsonDocument::fromJson(resp.body);
    if (!doc.isObject())
        return std::nullopt;
    const QString tag = doc.object().value(QStringLiteral("tag_name")).toString();
    if (tag.isEmpty())
        return std::nullopt;
    static const QRegularExpression re(QStringLiteral("\\d+\\.\\d+\\.\\d+"));
    const QRegularExpressionMatch m = re.match(tag);
    if (!m.hasMatch())
        return std::nullopt;
    return m.captured(0);
}

} // namespace

namespace update {

UpdateStatus check()
{
    const std::optional<QString> local = detectLocalVersion();
    std::optional<QString> latest = fetchLatestFromChangelog();
    if (!latest)
        latest = fetchLatestFromGithub();

    bool updateAvailable = false;
    if (latest && local) {
        const auto lv = parseSemver(*latest);
        const auto cv = parseSemver(*local);
        if (lv && cv)
            updateAvailable = *lv > *cv;
    }
    UpdateStatus st;
    st.localVersion = local;
    st.latestVersion = latest;
    st.updateAvailable = updateAvailable;
    st.checkFailed = !latest.has_value();
    return st;
}

} // namespace update
