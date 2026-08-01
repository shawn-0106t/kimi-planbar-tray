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
    public static TrayManager Tray { get; private set; } = null!;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        ShutdownMode = ShutdownMode.OnExplicitShutdown;

        Settings = SettingsService.Load();
        Theme = new ThemeService();
        Quota = new QuotaService();
        Updates = new UpdateService();

        // Headless 自检模式：拉取一次额度并打印 JSON 后退出
        // （Task.Run 脱离 UI 线程的 SynchronizationContext，避免死锁；
        //   先于单实例检查，保证有 GUI 实例运行时自检仍可用）
        if (e.Args.Contains("--test-fetch"))
        {
            var r = Task.Run(() => Quota.FetchAsync()).GetAwaiter().GetResult();
            Console.WriteLine(JsonSerializer.Serialize(r,
                new JsonSerializerOptions { WriteIndented = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }));
            Shutdown();
            return;
        }

        // Headless UI 自检：构造两个窗口验证资源解析与 XAML 加载
        if (e.Args.Contains("--test-ui"))
        {
            Theme.Apply(Settings.Data.Theme);
            try
            {
                var w = new MainWindow();
                Console.WriteLine("MainWindow OK");
                var s = new SettingsWindow();
                Console.WriteLine("SettingsWindow OK");
                var m = new TrayMenuWindow();
                Console.WriteLine("TrayMenuWindow OK");
            }
            catch (Exception ex)
            {
                Console.WriteLine("UI-FAIL: " + ex.GetType().Name + ": " + ex.Message);
                Console.WriteLine(ex.InnerException?.Message);
            }
            Shutdown();
            return;
        }

        // 截图模式：真实额度数据 + 指定主题渲染悬浮窗为 PNG（供 README 使用）
        // 用法：--screenshot <输出路径> [--dark]
        if (e.Args.Contains("--screenshot"))
        {
            int i = e.Args.ToList().IndexOf("--screenshot");
            var path = i + 1 < e.Args.Length ? e.Args[i + 1] : "screenshot.png";
            Theme.Apply(e.Args.Contains("--dark") ? "dark" : "light");
            var r = Task.Run(() => Quota.FetchAsync()).GetAwaiter().GetResult();
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

    protected override void OnExit(ExitEventArgs e)
    {
        Tray?.Dispose();
        Quota?.Dispose();
        try { _mutex?.ReleaseMutex(); } catch { }
        base.OnExit(e);
    }
}
