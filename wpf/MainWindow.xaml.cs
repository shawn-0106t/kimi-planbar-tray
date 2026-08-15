using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;

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

    private bool _hiding;

    public void ShowNearTray()
    {
        _hiding = false;
        var wa = SystemParameters.WorkArea;
        // +22 compensates the root Border margin growing 6 -> 28 (shadow fade
        // room): the visual card's right/bottom edges stay 18px off the work
        // area, exactly where they were before
        Left = wa.Right - Width - 12 + 22;
        Top = wa.Bottom - Height - 12 + 22;
        Show();
        Activate();
        // 原生风格的滑入 + 淡入（AllowsTransparency 分层窗口上 AnimateWindow 不可靠，
        // 用 WPF 动画保证可见效果；GPU 合成，开销可忽略）
        var tt = new TranslateTransform(0, 16);
        RootBorder.RenderTransform = tt;
        Opacity = 0;
        var fade = new DoubleAnimation(0, 1, TimeSpan.FromMilliseconds(160));
        var slide = new DoubleAnimation(16, 0, TimeSpan.FromMilliseconds(220))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut }
        };
        BeginAnimation(OpacityProperty, fade);
        tt.BeginAnimation(TranslateTransform.YProperty, slide);
        Render();
        RenderVersion();
    }

    public void HideAnimated()
    {
        if (_hiding) return;
        _hiding = true;
        var tt = RootBorder.RenderTransform as TranslateTransform ?? new TranslateTransform();
        RootBorder.RenderTransform = tt;
        var fade = new DoubleAnimation(0, TimeSpan.FromMilliseconds(130));
        var slide = new DoubleAnimation(12, TimeSpan.FromMilliseconds(160))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseIn }
        };
        fade.Completed += (_, _) =>
        {
            Hide();
            Opacity = 1;
            App.Tray?.NotifyPopupHidden();
        };
        BeginAnimation(OpacityProperty, fade);
        tt.BeginAnimation(TranslateTransform.YProperty, slide);
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
        RenderExtra(r?.Extra);
        LastUpdated.Text = r == null ? "" : r.Error != null ? "Update failed" : $"Updated {r.FetchedAt.LocalDateTime:HH:mm}";
    }

    private void RenderExtra(ExtraInfo? e)
    {
        if (e == null)
        {
            ExtraBalance.Text = "--";
            ExtraMonthlyPanel.Visibility = Visibility.Collapsed;
            return;
        }
        ExtraBalance.Text = e.State switch
        {
            ExtraState.Ready => e.BalanceCents.HasValue ? FmtYuan(e.BalanceCents.Value) : "--",
            ExtraState.NoData => "No data",
            _ => "Not activated",
        };
        if (e.MonthlyEnabled && e.MonthlyLimitCents is > 0 && e.MonthlyUsedCents.HasValue)
        {
            double p = Math.Clamp((double)e.MonthlyUsedCents.Value / e.MonthlyLimitCents.Value * 100, 0, 100);
            ExtraBarCol.Width = new GridLength(p, GridUnitType.Star);
            ExtraRestCol.Width = new GridLength(100 - p, GridUnitType.Star);
            ExtraMonthly.Text = $"Used {FmtYuan(e.MonthlyUsedCents.Value)} this month / {FmtYuan(e.MonthlyLimitCents.Value)} limit";
            ExtraMonthlyPanel.Visibility = Visibility.Visible;
        }
        else
        {
            ExtraMonthlyPanel.Visibility = Visibility.Collapsed;
        }
    }

    // 分 → ¥ 显示，整元省略小数（编码安全：WPF 全程 UTF-16，无 GBK 环节）
    private static string FmtYuan(long cents)
    {
        if (cents < 0) return "-" + FmtYuan(-cents);
        long yuan = cents / 100, frac = cents % 100;
        return "¥" + yuan + (frac > 0 ? "." + frac.ToString("00") : "");
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
        if (span < TimeSpan.Zero) return "Resets soon";
        if (span.TotalDays >= 1) return $"Resets in {(int)span.TotalDays}d {span.Hours}h";
        if (span.TotalHours >= 1) return $"Resets in {(int)span.TotalHours}h {span.Minutes}m";
        return $"Resets in {Math.Max(1, span.Minutes)}m";
    }

    private void RenderVersion()
    {
        CliVersion.Text = App.Updates.LocalVersion ?? "Not detected";
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
}
