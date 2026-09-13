#include "skills.h"

#include "credentials.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>

namespace {

// Parse `name:` / `description:` from the YAML frontmatter of a SKILL.md.
// Only the first 4 KiB are read — frontmatter always sits at the top.
// Lossy decode tolerates a truncated multi-byte char at the 4 KiB cut and
// stray non-UTF-8 (e.g. GBK) bytes; a UTF-8 BOM before the `---` fence is
// stripped. Deliberately a line parser, not a YAML library: zero new
// dependencies. Known limits (same as the Rust edition): an indented nested
// `name:` (e.g. under `metadata:`) can be picked up when the top-level one is
// absent, and folded scalars (`description: >-`) show the raw indicator.
std::pair<std::optional<QString>, std::optional<QString>> parseFrontmatter(const QString &path)
{
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly))
        return {std::nullopt, std::nullopt};
    const QByteArray buf = f.read(4096);
    // Qt's fromUtf8 replaces invalid/truncated sequences with U+FFFD,
    // matching String::from_utf8_lossy
    const QString head = QString::fromUtf8(buf);
    const QStringList lines = head.split(QLatin1Char('\n'));
    if (lines.isEmpty())
        return {std::nullopt, std::nullopt};
    QString first = lines.first().trimmed();
    while (first.startsWith(QChar(0xFEFF)))
        first.remove(0, 1);
    if (first != QLatin1String("---"))
        return {std::nullopt, std::nullopt};

    std::optional<QString> name;
    std::optional<QString> description;
    for (int i = 1; i < lines.size(); ++i) {
        // Rust trim_end: strip trailing whitespace only
        QString line = lines[i];
        while (!line.isEmpty() && line.back().isSpace())
            line.chop(1);
        if (line.trimmed() == QLatin1String("---"))
            break;
        const int colon = line.indexOf(QLatin1Char(':'));
        if (colon < 0)
            continue;
        const QString key = line.left(colon).trimmed();
        // Frontmatter values may be single/double quoted (Rust trim_matches:
        // strips ALL leading/trailing quote chars)
        QString value = line.mid(colon + 1).trimmed();
        while (value.startsWith(QLatin1Char('"')))
            value.remove(0, 1);
        while (value.endsWith(QLatin1Char('"')))
            value.chop(1);
        while (value.startsWith(QLatin1Char('\'')))
            value.remove(0, 1);
        while (value.endsWith(QLatin1Char('\'')))
            value.chop(1);
        if (key == QLatin1String("name") && !name)
            name = value;
        else if (key == QLatin1String("description") && !description)
            description = value;
    }
    return {name, description};
}

// Collect skills from one `<dir>/<id>/SKILL.md` layout.
void collect(const QString &dir, const QString &source, QList<SkillInfo> &out)
{
    const QDir d(dir);
    const QFileInfoList entries = d.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QFileInfo &entry : entries) {
        const QString id = entry.fileName();
        const QString skillMd = QDir(entry.absoluteFilePath()).filePath(QStringLiteral("SKILL.md"));
        if (!QFileInfo(skillMd).isFile())
            continue;
        const auto [name, description] = parseFrontmatter(skillMd);
        out.append({id, name.value_or(id), description.value_or(QString()), source});
    }
}

} // namespace

namespace skills {

QList<SkillInfo> collectDir(const QString &dir, const QString &source)
{
    QList<SkillInfo> out;
    collect(dir, source, out);
    return out;
}

QList<SkillInfo> scan()
{
    QList<SkillInfo> out;
    const std::optional<QString> home = credentials::homeDir();
    if (!home)
        return out;
    const QString kimi = credentials::kimiHome(*home);

    collect(QDir(kimi).filePath(QStringLiteral("skills")), QStringLiteral("Kimi Code"), out);
    collect(QDir(*home).filePath(QStringLiteral(".agents/skills")), QStringLiteral("Agents"), out);

    // Managed plugins: ~/.kimi-code/plugins/managed/<plugin>/skills/<id>/
    const QDir plugins(QDir(kimi).filePath(QStringLiteral("plugins/managed")));
    const QFileInfoList pluginDirs = plugins.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QFileInfo &pdir : pluginDirs) {
        const QString source = QStringLiteral("Plugin: %1").arg(pdir.fileName());
        collect(QDir(pdir.absoluteFilePath()).filePath(QStringLiteral("skills")), source, out);
    }

    std::sort(out.begin(), out.end(), [](const SkillInfo &a, const SkillInfo &b) {
        if (a.source != b.source)
            return a.source < b.source;
        return a.name.toLower() < b.name.toLower();
    });
    return out;
}

} // namespace skills
