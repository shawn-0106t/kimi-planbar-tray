using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;

namespace KimiPlanbarTray;

// 托盘右键菜单：WPF 自绘，与悬浮窗/设置窗同一视觉语言
public partial class TrayMenuWindow : Window
{
    public TrayMenuWindow()
    {
        InitializeComponent();
        // Close 期间 Deactivated 会同步重入；延迟到当前关闭流程结束后再判断，
        // 此时若已在关闭（IsVisible=false）则为 no-op，彻底避免重入崩溃
        Deactivated += (_, _) => Dispatcher.BeginInvoke(new Action(() =>
        {
            if (IsVisible) Close();
        }));
    }

    public void ShowAtCursor()
    {
        // Cursor.Position 返回物理像素，WPF 坐标是 DIP（系统 DPI 感知），
        // 高 DPI 缩放下必须换算，否则菜单会偏出屏幕（表现为"没弹出来"）
        var pos = System.Windows.Forms.Cursor.Position;
        Show();
        var hwnd = new WindowInteropHelper(this).Handle;
        double scale = GetDpiForWindow(hwnd) / 96.0;
        if (scale <= 0) scale = 1;
        double cx = pos.X / scale, cy = pos.Y / scale;
        var wa = SystemParameters.WorkArea;
        double h = ActualHeight, w = ActualWidth;
        Left = Math.Min(cx, wa.Right - w - 8);
        // 光标靠近屏幕底部时菜单向上弹出，否则向下
        Top = cy + h + 24 > wa.Bottom ? cy - h - 8 : cy + 8;
        Activate();
        // 托盘点击的输入归 explorer，后台进程必须显式抢前台，
        // 否则菜单拿不到焦点会立刻被 Deactivated 关闭（表现为"没弹出来"）
        SetForegroundWindow(hwnd);
    }

    private void OpenClick(object sender, RoutedEventArgs e)
    {
        Close();
        App.Tray.TogglePopup();
    }

    private async void RefreshClick(object sender, RoutedEventArgs e)
    {
        Close();
        await App.Quota.SafeRefresh();
        await App.Updates.CheckAsync();
    }

    private void SettingsClick(object sender, RoutedEventArgs e)
    {
        Close();
        App.Tray.ShowSettings();
    }

    private void ExitClick(object sender, RoutedEventArgs e) => Application.Current.Shutdown();

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr hWnd);
}
