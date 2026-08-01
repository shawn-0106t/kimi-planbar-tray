using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;

namespace KimiPlanbarTray;

public partial class MainWindow : Window
{
    private bool _suppressDeactivate;

    public MainWindow()
    {
        InitializeComponent();
        App.Quota.Updated += Render;
        App.Updates.Updated += RenderVersion;
        Deactivated += (_, _) => { if (!_suppressDeactivate) HideAnimated(); };
        Closed += (_, _) =>
        {
            App.Quota.Updated -= Render;
            App.Updates.Updated -= RenderVersion;
        };
    }

    public void ShowNearTray()
    {
        var wa = SystemParameters.WorkArea;
        Left = wa.Right - Width - 12;
        Top = wa.Bottom - Height - 12;
        // 先建句柄但不 Show，由 AnimateWindow 完成原生滑出动画；
        // 动画结束后 WPF 再 Show() 同步状态（此时窗口已可见，无视觉跳变）
        var hwnd = new WindowInteropHelper(this).EnsureHandle();
        AnimateWindow(hwnd, 220, AW_SLIDE | AW_VER_NEGATIVE);
        Dispatcher.BeginInvoke(new Action(() =>
        {
            Show();
            Activate();
        }), System.Windows.Threading.DispatcherPriority.ApplicationIdle);
        Render();
        RenderVersion();
    }

    public void HideAnimated()
    {
        AnimateWindow(new WindowInteropHelper(this).Handle, 180,
            AW_HIDE | AW_SLIDE | AW_VER_POSITIVE);
        Hide();
        App.Tray?.NotifyPopupHidden();
    }

    public void RefreshView()
    {
        Render();
        RenderVersion();
    }

    // 把当前窗口内容渲染为 PNG（截图模式用）
    public void CapturePng(string path)
    {
        var bmp = new System.Windows.Media.Imaging.RenderTargetBitmap(
            (int)ActualWidth, (int)ActualHeight, 96, 96,
            System.Windows.Media.PixelFormats.Pbgra32);
        bmp.Render(this);
        var enc = new System.Windows.Media.Imaging.PngBitmapEncoder();
        enc.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bmp));
        var dir = Path.GetDirectoryName(Path.GetFullPath(path));
        if (dir != null) Directory.CreateDirectory(dir);
        using var fs = File.Create(path);
        enc.Save(fs);
    }

    private void Render()
    {
        var r = App.Quota.Last;
        SetCard(WeekPct, WeekBarCol, WeekRestCol, WeekReset, r?.Week);
        SetCard(FivePct, FiveBarCol, FiveRestCol, FiveReset, r?.FiveHour);
        LastUpdated.Text = r == null ? "" : r.Error != null ? "更新失败" : $"更新于 {r.FetchedAt.LocalDateTime:HH:mm}";
    }

    private static void SetCard(System.Windows.Controls.TextBlock pct,
        ColumnDefinition barCol, ColumnDefinition restCol,
        System.Windows.Controls.TextBlock reset, QuotaSegment? s)
    {
        if (s == null)
        {
            pct.Text = "--";
            barCol.Width = new GridLength(0, GridUnitType.Star);
            restCol.Width = new GridLength(100, GridUnitType.Star);
            reset.Text = "";
            return;
        }
        double p = Math.Clamp(s.Percent, 0, 100);
        pct.Text = $"{s.Percent:0}%";
        barCol.Width = new GridLength(p, GridUnitType.Star);
        restCol.Width = new GridLength(100 - p, GridUnitType.Star);
        reset.Text = s.ResetAt.HasValue ? FormatReset(s.ResetAt.Value) : "";
    }

    private static string FormatReset(DateTimeOffset at)
    {
        var span = at - DateTimeOffset.Now;
        if (span < TimeSpan.Zero) return "即将重置";
        if (span.TotalDays >= 1) return $"{(int)span.TotalDays}天{span.Hours}小时后重置";
        if (span.TotalHours >= 1) return $"{(int)span.TotalHours}小时{span.Minutes}分钟后重置";
        return $"{Math.Max(1, span.Minutes)}分钟后重置";
    }

    private void RenderVersion()
    {
        CliVersion.Text = App.Updates.LocalVersion ?? "未检测到";
        NewVersionBadge.Visibility =
            App.Updates.UpdateAvailable ? Visibility.Visible : Visibility.Collapsed;
    }

    private async void RefreshClick(object sender, RoutedEventArgs e)
    {
        await App.Quota.SafeRefresh();
        await App.Updates.CheckAsync();
    }

    private void SettingsClick(object sender, RoutedEventArgs e)
    {
        _suppressDeactivate = true;
        App.Tray.ShowSettings();
        _suppressDeactivate = false;
    }

    private void ExitClick(object sender, RoutedEventArgs e) => Application.Current.Shutdown();

    private void OpenReleases(object sender, System.Windows.Input.MouseButtonEventArgs e)
    {
        try
        {
            Process.Start(new ProcessStartInfo(
                "https://github.com/MoonshotAI/kimi-code/releases") { UseShellExecute = true });
        }
        catch { }
    }

    private const int AW_SLIDE = 0x40000;
    private const int AW_HIDE = 0x10000;
    private const int AW_VER_POSITIVE = 0x4;
    private const int AW_VER_NEGATIVE = 0x8;

    [DllImport("user32.dll")]
    private static extern bool AnimateWindow(IntPtr hwnd, int time, int flags);
}
