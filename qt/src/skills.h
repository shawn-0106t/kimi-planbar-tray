// Read-only Kimi Code skill scanner, 1:1 port of rust/src-tauri/src/skills.rs
// (SPEC section 21): scan */SKILL.md and parse the YAML frontmatter for
// name/description. No writes, no watchers, no polling — the caller caches
// the result. No per-skill enabled/disabled state exists; .skill-lock.json
// is never read.

#ifndef SKILLS_H
#define SKILLS_H

#include <QList>
#include <QString>

struct SkillInfo {
    QString id;
    QString name;
    QString description;
    // Group label: "Kimi Code" | "Agents" | "Plugin: <name>"
    QString source;
};

namespace skills {

// Scan all three skill roots, grouped by source then sorted by name.
QList<SkillInfo> scan();

// Internal, exposed for the --test-skills parser self-check: collect skills
// from one `<dir>/<id>/SKILL.md` layout.
QList<SkillInfo> collectDir(const QString &dir, const QString &source);

} // namespace skills

#endif // SKILLS_H
