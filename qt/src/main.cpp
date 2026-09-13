// Entry point of the Qt edition. Order per QT-MIGRATION.md section 3:
// parse CLI -> --test-* self-check branches -> CreateMutexW single instance
// -> QApplication. The self-check branches run BEFORE the single-instance
// check so they work while a GUI instance is live (SPEC 19), mirroring
// rust/src-tauri/src/main.rs.

#include "app.h"
#include "panel.h"
#include "quota.h"
#include "skills.h"
#include "update.h"
#include "windows/menuwindow.h"
#include "windows/settingswindow.h"
#include "windows/skillswindow.h"

#include <QApplication>
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFont>
#include <QFontDatabase>
#include <QScreen>
#include <QThread>
#include <QTimer>
#include <QWidget>

#include <charconv>
#include <cmath>
#include <cstring>

#include <windows.h>

namespace {

// GUI-subsystem exes have no console: attach to the parent console so the
// --test-* output is visible; failure (no parent console) is silently ignored
// (QT-MIGRATION.md 4.6).
// The inherited STD_OUTPUT_HANDLE is used directly: it is a valid pipe when
// launched from Git Bash and the parent's console buffer when launched from
// cmd/PowerShell, so plain WriteFile covers both. CRT stdio (printf) is NOT
// usable in a GUI-subsystem exe without a console. Only when there is no
// stdout handle at all do we fall back to AttachConsole + CONOUT$.
void printStdout(const QString &text)
{
    const QByteArray utf8 = text.toUtf8();
    HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);
    bool owned = false; // only a handle we opened gets closed, never the inherited one
    if (!h || h == INVALID_HANDLE_VALUE) {
        if (AttachConsole(ATTACH_PARENT_PROCESS)) {
            h = CreateFileW(L"CONOUT$", GENERIC_WRITE, FILE_SHARE_WRITE, nullptr, OPEN_EXISTING,
                            0, nullptr);
            owned = (h && h != INVALID_HANDLE_VALUE);
        }
    }
    if (!h || h == INVALID_HANDLE_VALUE)
        return; // silently ignored, per the error-handling baseline
    DWORD written = 0;
    WriteFile(h, utf8.constData(), static_cast<DWORD>(utf8.size()), &written, nullptr);
    if (owned)
        CloseHandle(h);
}

// Named mutex identical to the Rust/WPF editions (SPEC 20): guarantees mutual
// exclusion across all three editions. Do not rename.
bool acquireSingleInstanceMutex()
{
    HANDLE h = CreateMutexW(nullptr, TRUE, L"KimiPlanbarTray.SingleInstance");
    if (!h)
        return true; // mutex creation failure must not block startup
    if (GetLastError() == ERROR_ALREADY_EXISTS)
        return false;
    // Leak the handle on purpose: it must stay open until process exit
    static HANDLE keepAlive = h;
    (void)keepAlive;
    return true;
}

// C# string interpolation prints True/False
const char *dotnetBool(bool b)
{
    return b ? "True" : "False";
}

// --- serde_json/chrono-compatible serialization helpers -------------------

// f64 the way Rust's serde_json (ryu) prints it: shortest round-trip, with a
// trailing ".0" for integral values (e.g. "2.0", "7.000000000000001").
QString fmtF64(double v)
{
    char buf[32];
    const auto res = std::to_chars(buf, buf + sizeof(buf), v);
    QString s = QString::fromLatin1(buf, static_cast<int>(res.ptr - buf));
    if (!s.contains(QLatin1Char('.')) && !s.contains(QLatin1Char('e'))
        && !s.contains(QLatin1Char('E')) && !s.contains(QLatin1String("nan"))
        && !s.contains(QLatin1String("inf")))
        s += QStringLiteral(".0");
    return s;
}

QString fmtOffset(const QDateTime &dt)
{
    int off = dt.offsetFromUtc();
    const QChar sign = off < 0 ? QLatin1Char('-') : QLatin1Char('+');
    off = std::abs(off);
    return QStringLiteral("%1%2:%3")
        .arg(sign)
        .arg(off / 3600, 2, 10, QLatin1Char('0'))
        .arg((off % 3600) / 60, 2, 10, QLatin1Char('0'));
}

// chrono's RFC3339 AutoSi for a QDateTime: no fraction at whole seconds,
// else millisecond digits (Qt is ms-based; 6/9-digit shapes cannot occur).
QString fmtDateTime(const QDateTime &dt)
{
    QString s = dt.toString(QStringLiteral("yyyy-MM-ddTHH:mm:ss"));
    if (dt.time().msec() != 0)
        s += QStringLiteral(".%1").arg(dt.time().msec(), 3, 10, QLatin1Char('0'));
    return s + fmtOffset(dt);
}

// fetchedAt carries nanosecond digits from the precise system clock (100ns
// units on Windows -> the last two digits are always 0), like chrono's
// AutoSi printing 9 digits for a nanosecond-precision value.
QString fmtFetchedAt(const QDateTime &dt, int nanos)
{
    return dt.toString(QStringLiteral("yyyy-MM-ddTHH:mm:ss"))
        + QStringLiteral(".%1").arg(nanos, 9, 10, QLatin1Char('0')) + fmtOffset(dt);
}

QString jsonEscape(const QString &s)
{
    QString out;
    out.reserve(s.size() + 2);
    for (const QChar c : s) {
        switch (c.unicode()) {
        case '"': out += QStringLiteral("\\\""); break;
        case '\\': out += QStringLiteral("\\\\"); break;
        case '\n': out += QStringLiteral("\\n"); break;
        case '\r': out += QStringLiteral("\\r"); break;
        case '\t': out += QStringLiteral("\\t"); break;
        default:
            if (c.unicode() < 0x20)
                out += QStringLiteral("\\u%1").arg(static_cast<int>(c.unicode()), 4, 16, QLatin1Char('0'));
            else
                out += c;
        }
    }
    return out;
}

const char *extraStateName(ExtraState s)
{
    switch (s) {
    case ExtraState::NotActivated: return "NotActivated";
    case ExtraState::NoData: return "NoData";
    case ExtraState::Ready: return "Ready";
    }
    return "NotActivated";
}

QString optI64(const std::optional<qint64> &v)
{
    return v ? QString::number(*v) : QStringLiteral("null");
}

// serde_json::to_string_pretty shape: 2-space indent, fixed key order.
QString quotaResultJson(const QuotaResult &r)
{
    QStringList lines;
    lines << QStringLiteral("{");
    const auto segment = [&lines](const char *name, const std::optional<QuotaSegment> &seg,
                                  bool comma) {
        const QString tail = comma ? QStringLiteral(",") : QString();
        if (!seg) {
            lines << QStringLiteral("  \"%1\": null%2").arg(QLatin1String(name), tail);
            return;
        }
        lines << QStringLiteral("  \"%1\": {").arg(QLatin1String(name));
        lines << QStringLiteral("    \"percent\": %1,").arg(fmtF64(seg->percent));
        lines << QStringLiteral("    \"resetAt\": %1")
                         .arg(seg->resetAt ? QStringLiteral("\"%1\"").arg(fmtDateTime(*seg->resetAt))
                                           : QStringLiteral("null"));
        lines << QStringLiteral("  }%1").arg(tail);
    };
    segment("fiveHour", r.fiveHour, true);
    segment("week", r.week, true);

    if (!r.extra) {
        lines << QStringLiteral("  \"extra\": null,");
    } else {
        const ExtraInfo &e = *r.extra;
        lines << QStringLiteral("  \"extra\": {");
        lines << QStringLiteral("    \"state\": \"%1\",").arg(QLatin1String(extraStateName(e.state)));
        lines << QStringLiteral("    \"balanceCents\": %1,").arg(optI64(e.balanceCents));
        lines << QStringLiteral("    \"monthlyEnabled\": %1,")
                     .arg(e.monthlyEnabled ? QStringLiteral("true") : QStringLiteral("false"));
        lines << QStringLiteral("    \"monthlyUsedCents\": %1,").arg(optI64(e.monthlyUsedCents));
        lines << QStringLiteral("    \"monthlyLimitCents\": %1").arg(optI64(e.monthlyLimitCents));
        lines << QStringLiteral("  },");
    }

    lines << QStringLiteral("  \"fetchedAt\": \"%1\",")
                     .arg(fmtFetchedAt(r.fetchedAt, r.fetchedAtNanos));
    lines << QStringLiteral("  \"error\": %1")
                     .arg(r.error ? QStringLiteral("\"%1\"").arg(jsonEscape(*r.error))
                                  : QStringLiteral("null"));
    lines << QStringLiteral("}");
    return lines.join(QLatin1Char('\n'));
}

int testFetch(int argc, char *argv[])
{
    QCoreApplication app(argc, argv); // QNetworkAccessManager needs one
    const QuotaResult r = quota::fetch();
    printStdout(quotaResultJson(r) + QLatin1Char('\n'));
    return 0;
}

int testUpdate(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    const UpdateStatus st = update::check();
    printStdout(QStringLiteral("local=%1 latest=%2 updateAvailable=%3 checkFailed=%4\n")
                    .arg(st.localVersion ? *st.localVersion : QString())
                    .arg(st.latestVersion ? *st.latestVersion : QString())
                    .arg(QLatin1String(dotnetBool(st.updateAvailable)))
                    .arg(QLatin1String(dotnetBool(st.checkFailed))));
    return 0;
}

bool hasArg(int argc, char *argv[], const char *arg)
{
    for (int i = 1; i < argc; ++i)
        if (std::strcmp(argv[i], arg) == 0)
            return true;
    return false;
}

// The web frontend's font stack is `'Segoe UI', system-ui, sans-serif`
// (rust/src/theme.css): Latin in Segoe UI, CJK in the platform default. The
// Qt equivalent of system-ui is the system default GUI font (Microsoft YaHei
// on zh-CN), which covers Latin and CJK uniformly — forcing Segoe UI alone
// would leave Chinese to an arbitrary fallback.
void applySystemFont()
{
    QApplication::setFont(QFontDatabase::systemFont(QFontDatabase::GeneralFont));
}

// Headless UI self-check, port of rust lib.rs run_ui_test (SPEC 19):
// construct all four windows, print one OK line per window, exit after ~6s.
// Runs without the single-instance mutex so it works alongside a live GUI.
int testUi(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName("kimi-planbar-tray");
    applySystemFont();
    QApplication::setQuitOnLastWindowClosed(false);

    App theApp;
    // Suppress the panel focus-loss auto-hide so the panel stays visible for
    // inspection (same as the Rust test setting settings_open=true)
    theApp.state().settingsOpen = true;
    panel::showPanel(&theApp);
    theApp.settingsWindow()->show();
    theApp.skillsWindow()->show();
    theApp.loadSkills(false); // lets the skills list render, like skills-show
    // Stagger the two centered windows so screenshots never overlap
    // (test-mode layout only; production keeps both centered)
    const QRect wa = QGuiApplication::primaryScreen()->availableGeometry();
    theApp.skillsWindow()->move(wa.x() + (wa.width() - 404) / 2 - 212,
                                wa.y() + (wa.height() - 520) / 2);
    theApp.menuWindow()->showAtCursor();
    printStdout(QStringLiteral("MainWindow OK\nSettingsWindow OK\nSkillsWindow OK\n"
                               "TrayMenuWindow OK\n"));
    // Exit after ~6s headless (rust run_ui_test: thread sleep + process::exit)
    QThread *killer = QThread::create([] {
        QThread::sleep(6);
        QCoreApplication::exit(0);
    });
    QObject::connect(killer, &QThread::finished, killer, &QObject::deleteLater);
    killer->start();
    return QApplication::exec();
}

// Skills frontmatter parser self-check (Qt-only extra; ports the two
// cargo-test cases from rust/src-tauri/src/skills.rs against temp fixtures).
int testSkills()
{
    QDir temp(QDir::temp().filePath(
        QStringLiteral("kpt-skills-test-%1").arg(QCoreApplication::applicationPid())));
    temp.removeRecursively();
    const auto writeSkill = [&temp](const QString &id, const QByteArray &content) {
        QDir d(temp.filePath(id));
        d.mkpath(QStringLiteral("."));
        QFile f(d.filePath(QStringLiteral("SKILL.md")));
        if (f.open(QIODevice::WriteOnly))
            f.write(content);
    };
    // Case 1: name/description, quote stripping, id fallback
    writeSkill(QStringLiteral("a"), "---\nname: Alpha\ndescription: \"Does things\"\n---\nbody");
    writeSkill(QStringLiteral("b"), "---\ndescription: 'Only desc'\n---\n");
    writeSkill(QStringLiteral("c"), "no frontmatter at all");
    temp.mkpath(QStringLiteral("empty-no-skillmd"));
    // Case 2: UTF-8 BOM fence; GBK bytes in the description -> U+FFFD
    writeSkill(QStringLiteral("bom"), "\xEF\xBB\xBF---\nname: Bom\ndescription: x\n---\n");
    writeSkill(QStringLiteral("gbk"),
               QByteArray("---\nname: Gbk\ndescription: ") + QByteArray("\xD6\xD0\xCE\xC4", 4)
                   + "\n---\n");

    const QList<SkillInfo> out = skills::collectDir(temp.absolutePath(), QStringLiteral("Test"));
    QStringList failures;
    const auto find = [&out](const QString &id) -> const SkillInfo * {
        for (const SkillInfo &s : out)
            if (s.id == id)
                return &s;
        return nullptr;
    };
    const auto expect = [&failures](bool ok, const QString &what) {
        if (!ok)
            failures << what;
    };
    expect(out.size() == 5, QStringLiteral("expected 5 skills, got %1").arg(out.size()));
    if (const SkillInfo *a = find(QStringLiteral("a"))) {
        expect(a->name == QLatin1String("Alpha"), QStringLiteral("a.name"));
        expect(a->description == QLatin1String("Does things"), QStringLiteral("a.description"));
    } else failures << QStringLiteral("a missing");
    if (const SkillInfo *b = find(QStringLiteral("b"))) {
        expect(b->name == QLatin1String("b"), QStringLiteral("b.name fallback"));
        expect(b->description == QLatin1String("Only desc"), QStringLiteral("b.description"));
    } else failures << QStringLiteral("b missing");
    if (const SkillInfo *c = find(QStringLiteral("c"))) {
        expect(c->name == QLatin1String("c"), QStringLiteral("c.name fallback"));
        expect(c->description.isEmpty(), QStringLiteral("c.description empty"));
    } else failures << QStringLiteral("c missing");
    if (const SkillInfo *bom = find(QStringLiteral("bom")))
        expect(bom->name == QLatin1String("Bom"), QStringLiteral("bom.name"));
    else failures << QStringLiteral("bom missing");
    if (const SkillInfo *gbk = find(QStringLiteral("gbk"))) {
        expect(gbk->name == QLatin1String("Gbk"), QStringLiteral("gbk.name"));
        expect(gbk->description.contains(QChar(0xFFFD)),
               QStringLiteral("gbk.description U+FFFD"));
    } else failures << QStringLiteral("gbk missing");
    temp.removeRecursively();

    if (failures.isEmpty()) {
        printStdout(QStringLiteral("skills parser: PASS (5 fixture skills)\n"));
        return 0;
    }
    printStdout(QStringLiteral("skills parser: FAIL: %1\n").arg(failures.join(QStringLiteral(", "))));
    return 1;
}

} // namespace

int main(int argc, char *argv[])
{
    // Headless quota self-check: fetch once, print indented JSON, exit
    if (hasArg(argc, argv, "--test-fetch"))
        return testFetch(argc, argv);

    // Headless update-check self-check: single-line summary, exit
    if (hasArg(argc, argv, "--test-update"))
        return testUpdate(argc, argv);

    // Headless UI self-check: construct the four windows, print OK lines, exit
    if (hasArg(argc, argv, "--test-ui"))
        return testUi(argc, argv);

    // Headless skills-parser self-check (Qt-only extra): fixture scan, exit
    if (hasArg(argc, argv, "--test-skills")) {
        QCoreApplication app(argc, argv);
        return testSkills();
    }

    if (!acquireSingleInstanceMutex())
        return 0; // another instance (Rust, Qt or WPF edition) is already running

    // GUI path: tray icon + hidden panel + polling (SPEC section 20 startup)
    QApplication app(argc, argv);
    QApplication::setApplicationName("kimi-planbar-tray");
    QApplication::setApplicationVersion("1.7.2"); // unified with rust/ since Phase-4 parity
    applySystemFont();
    // Tray-only app: do not quit when the (singleton, hidden) windows close
    QApplication::setQuitOnLastWindowClosed(false);

    App theApp;
    theApp.start();

    return QApplication::exec();
}
