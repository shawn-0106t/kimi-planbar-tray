using System.Linq;
using System.Text.Json;
using System.Threading;
using System.Windows;

namespace KimiPlanbarTray;

public partial class App : Application
{
    private Mutex? _mutex;
    public static SettingsService Settings { get; private set; } = null!;
    public static ThemeService Theme { get; private set; } = null!;
    public static QuotaService Quota { get; private set; } = null!;
    public static UpdateService Updates { get; private set; } = null!;
    public static SkillsService Skills { get; private set; } = null!;
    public static TrayManager Tray { get; private set; } = null!;

    // --test-ui keeps the panel visible for inspection (rust run_ui_test sets
    // settings_open=true for the same reason)
    public static bool HeadlessUiTest { get; private set; }

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        ShutdownMode = ShutdownMode.OnExplicitShutdown;

        Settings = SettingsService.Load();
        Theme = new ThemeService();
        Quota = new QuotaService();
        Updates = new UpdateService();
        Skills = new SkillsService();

        // Headless 自检模式：拉取一次额度并打印 JSON 后退出
        // （Task.Run 脱离 UI 线程的 SynchronizationContext，避免死锁；
        //   先于单实例检查，保证有 GUI 实例运行时自检仍可用）
        if (e.Args.Contains("--test-fetch"))
        {
            var r = Task.Run(() => Quota.FetchAsync()).GetAwaiter().GetResult();
            // camelCase + string enums so the output diffs field-by-field
            // against the rust edition (serde rename_all = "camelCase");
            // settings.json keeps its cross-edition PascalCase schema
            Console.WriteLine(JsonSerializer.Serialize(r, new JsonSerializerOptions
            {
                WriteIndented = true,
                Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                Converters = { new System.Text.Json.Serialization.JsonStringEnumConverter() }
            }));
            Shutdown();
            return;
        }

        // Headless 版本检测自检：打印本地/最新版本后退出
        if (e.Args.Contains("--test-update"))
        {
            Task.Run(() => Updates.CheckAsync()).GetAwaiter().GetResult();
            Console.WriteLine($"local={Updates.LocalVersion} latest={Updates.LatestVersion} " +
                              $"updateAvailable={Updates.UpdateAvailable} checkFailed={Updates.CheckFailed}");
            Shutdown();
            return;
        }

        // Headless skills 解析器自检：对 rust skills.rs 的两个测试夹具
        // （BOM / 缺字段）跑 frontmatter 解析，打印 PASS 或差异详情
        if (e.Args.Contains("--test-skills"))
        {
            Console.WriteLine(SkillsService.RunParserTest());
            Shutdown();
            return;
        }

        // Headless UI 自检：构造四个窗口验证资源解析与 XAML 加载，实际 Show
        // 约 6 秒供目检后退出（rust run_ui_test / qt testUi 同款）
        if (e.Args.Contains("--test-ui"))
        {
            Theme.Apply(Settings.Data.Theme);
            HeadlessUiTest = true;
            try
            {
                var w = new MainWindow();
                Console.WriteLine("MainWindow OK");
                var s = new SettingsWindow();
                Console.WriteLine("SettingsWindow OK");
                var k = new SkillsWindow();
                Console.WriteLine("SkillsWindow OK");
                var m = new TrayMenuWindow();
                Console.WriteLine("TrayMenuWindow OK");
                w.ShowNearTray();
                k.Show();
                // Stagger the two centered windows so screenshots never overlap
                // (test-mode layout only; production keeps both centered)
                k.Left -= 212;
                s.Show();
                App.Skills.Load(refresh: false); // let the skills list render
                m.ShowAtCursor();
            }
            catch (Exception ex)
            {
                Console.WriteLine("UI-FAIL: " + ex.GetType().Name + ": " + ex.Message);
                Console.WriteLine(ex.InnerException?.Message);
            }
            _ = Task.Delay(6000).ContinueWith(_ => Dispatcher.BeginInvoke(new Action(Shutdown)));
            return;
        }

        // 截图模式：真实额度数据 + 指定主题渲染悬浮窗为 PNG（供 README 使用）
        // 用法：--screenshot <输出路径> [--dark] [--mock 用模拟数据（含 Extra 余额）]
        if (e.Args.Contains("--screenshot"))
        {
            int i = e.Args.ToList().IndexOf("--screenshot");
            var path = i + 1 < e.Args.Length ? e.Args[i + 1] : "screenshot.png";
            Theme.Apply(e.Args.Contains("--dark") ? "dark" : "light");
            QuotaResult? r;
            if (e.Args.Contains("--mock"))
            {
                r = new QuotaResult
                {
                    FetchedAt = DateTimeOffset.Now,
                    FiveHour = new QuotaSegment { Percent = 42, ResetAt = DateTimeOffset.Now.AddHours(3.5) },
                    Week = new QuotaSegment { Percent = 68, ResetAt = DateTimeOffset.Now.AddDays(4) },
                    Extra = new ExtraInfo
                    {
                        State = ExtraState.Ready,
                        BalanceCents = 1234,
                        MonthlyEnabled = true,
                        MonthlyUsedCents = 4567,
                        MonthlyLimitCents = 10000
                    }
                };
            }
            else
            {
                r = Task.Run(() => Quota.FetchAsync()).GetAwaiter().GetResult();
            }
            Quota.Inject(r);
            Task.Run(() => Updates.CheckAsync()).GetAwaiter().GetResult();
            var w = new MainWindow();
            w.Show();
            w.RefreshView();
            w.UpdateLayout();
            w.CapturePng(path);
            w.Close();
            Console.WriteLine("saved: " + path);
            Shutdown();
            return;
        }

        _mutex = new Mutex(true, "KimiPlanbarTray.SingleInstance", out bool created);
        if (!created) { Shutdown(); return; }

        Theme.Apply(Settings.Data.Theme);
        Theme.HookSystemEvents();
        Tray = new TrayManager();
        Quota.StartAutoRefresh();
        _ = Updates.CheckAsync();
    }

    // Panel "Refresh" button / tray-menu "Refresh" (SPEC 12.7 / 14): SafeRefresh
    // + CheckAsync, debounced — calls within 2s of the previous one are silently
    // ignored (rust lib.rs refresh_now). UI-thread only (both callers are).
    private static DateTimeOffset _lastManualRefresh = DateTimeOffset.MinValue;

    public static async Task ManualRefreshAsync()
    {
        var now = DateTimeOffset.UtcNow;
        if (now - _lastManualRefresh < TimeSpan.FromSeconds(2)) return;
        _lastManualRefresh = now;
        // Both checks run concurrently, like rust refresh_now's two spawns
        var updateCheck = Updates.CheckAsync();
        await Quota.SafeRefresh();
        await updateCheck;
    }

    protected override void OnExit(ExitEventArgs e)
    {
        Tray?.Dispose();
        Quota?.Dispose();
        try { _mutex?.ReleaseMutex(); } catch { }
        base.OnExit(e);
    }
}
