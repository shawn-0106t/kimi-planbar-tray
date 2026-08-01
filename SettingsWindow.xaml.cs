using System.Windows;
using System.Windows.Controls;

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
        foreach (ComboBoxItem item in IntervalBox.Items)
        {
            if (item.Tag is string tag && tag == d.RefreshMinutes.ToString())
            {
                IntervalBox.SelectedItem = item;
                break;
            }
        }
    }

    private void SaveClick(object sender, RoutedEventArgs e)
    {
        var d = App.Settings.Data;
        d.Theme = ThemeLight.IsChecked == true ? "light"
                : ThemeDark.IsChecked == true ? "dark" : "system";
        if (IntervalBox.SelectedItem is ComboBoxItem { Tag: string tag }
            && int.TryParse(tag, out int mins))
            d.RefreshMinutes = mins;
        d.AutoStart = AutoStartBox.IsChecked == true;

        App.Settings.Save();
        App.Settings.ApplyAutoStart();
        App.Theme.Apply(d.Theme);
        App.Quota.Reschedule();
        Close();
    }
}
