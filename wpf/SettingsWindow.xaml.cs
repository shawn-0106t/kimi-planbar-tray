using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace KimiPlanbarTray;

public partial class SettingsWindow : Window
{
    public SettingsWindow()
    {
        InitializeComponent();
        var d = App.Settings.Data;
        // Backfill tolerates a malformed persisted value: fall back to the
        // defaults instead of leaving any control unchecked (rust settings.ts
        // backfill, SPEC 13.2)
        ThemeSystem.IsChecked = d.Theme == "system";
        ThemeLight.IsChecked = d.Theme == "light";
        ThemeDark.IsChecked = d.Theme == "dark";
        if (ThemeSystem.IsChecked != true && ThemeLight.IsChecked != true && ThemeDark.IsChecked != true)
            ThemeSystem.IsChecked = true;
        AutoStartBox.IsChecked = d.AutoStart;
        var wanted = d.RefreshMinutes.ToString();
        var intervalPills = IntervalPanel.Children.OfType<RadioButton>().ToList();
        var matched = intervalPills.FirstOrDefault(rb => rb.Tag as string == wanted);
        if (matched == null) matched = intervalPills.FirstOrDefault(rb => rb.Tag as string == "5");
        if (matched != null) matched.IsChecked = true;
    }

    private void SaveClick(object sender, RoutedEventArgs e)
    {
        var d = App.Settings.Data;
        d.Theme = ThemeLight.IsChecked == true ? "light"
                : ThemeDark.IsChecked == true ? "dark" : "system";
        foreach (var child in IntervalPanel.Children)
        {
            if (child is RadioButton { IsChecked: true, Tag: string tag }
                && int.TryParse(tag, out int mins))
            {
                // Clamp at the persistence boundary: polling multiplies this by
                // 60 seconds (rust save_settings: settings.refresh_minutes.clamp(1, 30))
                d.RefreshMinutes = Math.Clamp(mins, 1, 30);
                break;
            }
        }
        d.AutoStart = AutoStartBox.IsChecked == true;

        App.Settings.Save();
        App.Settings.ApplyAutoStart();
        App.Theme.Apply(d.Theme);
        App.Quota.Reschedule();
        Close();
    }

    private void CloseClick(object sender, RoutedEventArgs e) => Close();

    private void TitleBarDrag(object sender, MouseButtonEventArgs e)
    {
        if (e.LeftButton == MouseButtonState.Pressed) DragMove();
    }
}
