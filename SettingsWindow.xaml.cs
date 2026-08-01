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
        ThemeSystem.IsChecked = d.Theme == "system";
        ThemeLight.IsChecked = d.Theme == "light";
        ThemeDark.IsChecked = d.Theme == "dark";
        AutoStartBox.IsChecked = d.AutoStart;
        foreach (var child in IntervalPanel.Children)
        {
            if (child is RadioButton { Tag: string tag } rb && tag == d.RefreshMinutes.ToString())
            {
                rb.IsChecked = true;
                break;
            }
        }
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
                d.RefreshMinutes = mins;
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
