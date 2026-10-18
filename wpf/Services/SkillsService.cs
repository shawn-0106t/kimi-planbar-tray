using System.Text;
using System.Windows;

namespace KimiPlanbarTray;

// One scanned skill (SPEC 21). There is no per-skill enabled/disabled state to
// display: Kimi Code does not persist one, and ~/.agents/.skill-lock.json is
// lark-cli's installer lock file with no `disabled` key — it is never read.
public class SkillInfo
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public string Description { get; set; } = "";
    public string Source { get; set; } = ""; // "Kimi Code" | "Agents" | "Plugin: <name>"
}

// Read-only Kimi Code skill scanner (SPEC 21), port of rust skills.rs (adapted
// from kimi-code-dashboard's routes/skills.py): scan */SKILL.md and parse the
// YAML frontmatter for name/description. No writes, no watchers, no polling —
// the first open scans and caches; the Refresh button forces a rescan.
public class SkillsService
{
    private List<SkillInfo>? _cache;
    private int _scanning; // single-flight guard for overlapping scans

    // List? null = scan failure ("Failed to load skills"), mirrors the rust
    // frontend's Result handling. Marshaled onto the UI thread (same pattern
    // as QuotaService.Updated).
    public event Action<List<SkillInfo>?>? Updated;

    public bool HasCache => _cache != null;

    // Cached skill list; refresh=true forces a rescan (SPEC 21.2). The scan
    // runs on a thread-pool thread so the file IO never blocks the UI thread.
    public void Load(bool refresh)
    {
        var cached = _cache;
        if (!refresh && cached != null) { Notify(cached); return; }
        if (Interlocked.Exchange(ref _scanning, 1) == 1) return; // scan already in flight
        Task.Run(() =>
        {
            List<SkillInfo>? list;
            try { list = Scan(); }
            catch { list = null; } // summary shows "Failed to load skills"
            if (list != null) _cache = list; // a failed rescan keeps the old cache
            Volatile.Write(ref _scanning, 0);
            Notify(list);
        });
    }

    private void Notify(List<SkillInfo>? list)
    {
        try { Application.Current?.Dispatcher.Invoke(() => Updated?.Invoke(list)); } catch { }
    }

    // Three scan roots with fixed group labels (SPEC 21.2); <kimi_home> honors
    // KIMI_CODE_HOME via QuotaService.KimiHome (same resolution as rust
    // credentials.rs::kimi_home).
    public static List<SkillInfo> Scan()
    {
        var outList = new List<SkillInfo>();
        string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        string kimi = QuotaService.KimiHome();

        Collect(Path.Combine(kimi, "skills"), "Kimi Code", outList);
        Collect(Path.Combine(home, ".agents", "skills"), "Agents", outList);

        // Managed plugins: <kimi_home>/plugins/managed/<plugin>/skills/<id>/
        try
        {
            var plugins = Path.Combine(kimi, "plugins", "managed");
            foreach (var pdir in Directory.EnumerateDirectories(plugins))
                Collect(Path.Combine(pdir, "skills"), $"Plugin: {Path.GetFileName(pdir)}", outList);
        }
        catch { }

        // Group by source, then name case-insensitively (SPEC 21.3)
        outList.Sort((a, b) =>
        {
            int c = string.CompareOrdinal(a.Source, b.Source);
            return c != 0 ? c : string.Compare(a.Name, b.Name, StringComparison.OrdinalIgnoreCase);
        });
        return outList;
    }

    // Collect skills from one <dir>/<id>/SKILL.md layout.
    internal static void Collect(string dir, string source, List<SkillInfo> outList)
    {
        string[] entries;
        try { entries = Directory.GetDirectories(dir); }
        catch { return; }
        foreach (var path in entries)
        {
            var id = Path.GetFileName(path);
            if (id.Length == 0) continue;
            var skillMd = Path.Combine(path, "SKILL.md");
            if (!File.Exists(skillMd)) continue;
            var (name, description) = ParseFrontmatter(skillMd);
            outList.Add(new SkillInfo
            {
                Id = id,
                Name = name ?? id, // name falls back to the directory name
                Description = description ?? "",
                Source = source,
            });
        }
    }

    // Parse `name:` / `description:` from the YAML frontmatter of a SKILL.md.
    // Only the first 4 KiB are read — frontmatter always sits at the top.
    // .NET's UTF-8 decoder replaces stray non-UTF-8 (e.g. GBK) bytes and a
    // truncated multi-byte char at the 4 KiB cut with U+FFFD (lossy); a UTF-8
    // BOM before the `---` fence is stripped. Deliberately a line parser, not
    // a YAML library — zero new dependencies. Known limits (kept in step with
    // rust): an indented nested `name:` can be picked up when the top-level
    // one is absent, and folded scalars (`description: >-`) show the raw
    // indicator.
    internal static (string? Name, string? Description) ParseFrontmatter(string path)
    {
        byte[] buf;
        try
        {
            using var fs = File.OpenRead(path);
            buf = new byte[4096];
            int n = 0, read;
            while (n < buf.Length && (read = fs.Read(buf, n, buf.Length - n)) > 0) n += read;
            if (n < buf.Length) Array.Resize(ref buf, n);
        }
        catch { return (null, null); }

        string head = Encoding.UTF8.GetString(buf);
        string[] lines = head.Split('\n');
        if (lines.Length == 0 || lines[0].Trim().TrimStart('\uFEFF') != "---")
            return (null, null);

        string? name = null, description = null;
        for (int i = 1; i < lines.Length; i++)
        {
            string line = lines[i].TrimEnd();
            if (line.Trim() == "---") break;
            int colon = line.IndexOf(':');
            if (colon < 0) continue;
            string key = line.Substring(0, colon).Trim();
            // Frontmatter values may be single/double quoted
            string value = line.Substring(colon + 1).Trim().Trim('"').Trim('\'');
            if (key == "name" && name == null) name = value;
            else if (key == "description" && description == null) description = value;
        }
        return (name, description);
    }

    // Headless parser self-check (--test-skills): ports the two rust
    // skills.rs cargo-test fixtures (missing fields + quotes; BOM + GBK bytes)
    // against a temp directory. ASCII-only output, printed before the
    // single-instance check (SPEC 19; qt edition runs the same fixtures).
    public static string RunParserTest()
    {
        var dir = Path.Combine(Path.GetTempPath(), $"kpt-skills-test-{Environment.ProcessId}");
        try
        {
            try { if (Directory.Exists(dir)) Directory.Delete(dir, true); } catch { }
            WriteSkill(dir, "a", "---\nname: Alpha\ndescription: \"Does things\"\n---\nbody");
            WriteSkill(dir, "b", "---\ndescription: 'Only desc'\n---\n");
            WriteSkill(dir, "c", "no frontmatter at all");
            Directory.CreateDirectory(Path.Combine(dir, "empty-no-skillmd"));
            WriteSkill(dir, "bom", "\uFEFF---\nname: Bom\ndescription: x\n---\n");
            // GBK-encoded bytes (common on Chinese Windows) in the description
            var gbk = new List<byte>();
            gbk.AddRange(Encoding.ASCII.GetBytes("---\nname: Gbk\ndescription: "));
            gbk.AddRange(new byte[] { 0xD6, 0xD0, 0xCE, 0xC4 }); // "中文" in GBK
            gbk.AddRange(Encoding.ASCII.GetBytes("\n---\n"));
            WriteSkillBytes(dir, "gbk", gbk.ToArray());

            var outList = new List<SkillInfo>();
            Collect(dir, "Test", outList);
            var failures = new List<string>();
            SkillInfo? Find(string id) => outList.FirstOrDefault(s => s.Id == id);
            void Expect(bool ok, string what) { if (!ok) failures.Add(what); }

            Expect(outList.Count == 5, $"expected 5 skills, got {outList.Count}");
            var a = Find("a");
            if (a != null)
            {
                Expect(a.Name == "Alpha", "a.name");
                Expect(a.Description == "Does things", "a.description");
            }
            else failures.Add("a missing");
            var b = Find("b");
            if (b != null)
            {
                Expect(b.Name == "b", "b.name fallback");
                Expect(b.Description == "Only desc", "b.description");
            }
            else failures.Add("b missing");
            var c = Find("c");
            if (c != null)
            {
                Expect(c.Name == "c", "c.name fallback");
                Expect(c.Description == "", "c.description empty");
            }
            else failures.Add("c missing");
            var bom = Find("bom");
            if (bom != null) Expect(bom.Name == "Bom", "bom.name");
            else failures.Add("bom missing");
            var g = Find("gbk");
            if (g != null)
            {
                Expect(g.Name == "Gbk", "gbk.name"); // lossy decode keeps the ASCII fields
                Expect(g.Description.Contains('\uFFFD'), "gbk.description U+FFFD");
            }
            else failures.Add("gbk missing");

            return failures.Count == 0
                ? "skills parser: PASS (5 fixture skills)"
                : "skills parser: FAIL: " + string.Join(", ", failures);
        }
        finally
        {
            try { if (Directory.Exists(dir)) Directory.Delete(dir, true); } catch { }
        }
    }

    private static void WriteSkill(string root, string id, string content)
        => WriteSkillBytes(root, id, new UTF8Encoding(false).GetBytes(content));

    private static void WriteSkillBytes(string root, string id, byte[] content)
    {
        var dir = Path.Combine(root, id);
        Directory.CreateDirectory(dir);
        File.WriteAllBytes(Path.Combine(dir, "SKILL.md"), content);
    }
}
