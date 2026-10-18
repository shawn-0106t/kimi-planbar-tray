using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;

namespace KimiPlanbarTray;

// Read-only skill list window (SPEC 21). Lists render only from
// App.Skills.Updated — all user-controlled text goes through TextBlock.Text
// (no string markup), mirroring the rust frontend's textContent-only rule.
public partial class SkillsWindow : Window
{
    public SkillsWindow()
    {
        InitializeComponent();
        App.Skills.Updated += Render;
        Closed += (_, _) => App.Skills.Updated -= Render;
    }

    // 两行裁切：12px 字号 × 1.4 行距 × 2 行（BlockLineHeight 固定行高，
    // 超出部分随 MaxHeight 裁掉；完整描述在卡片 ToolTip）
    private const double DescLineHeight = 17;
    private const double DescMaxLines = 2;

    private void Render(List<SkillInfo>? skills)
    {
        if (skills == null)
        {
            SummaryText.Text = "Failed to load skills";
            return;
        }
        SummaryText.Text = skills.Count > 0 ? $"{skills.Count} skills" : "No skills found";
        ListPanel.Children.Clear();
        string lastSource = "";
        foreach (var s in skills)
        {
            if (s.Source != lastSource)
            {
                lastSource = s.Source;
                var groupTitle = new TextBlock
                {
                    Text = s.Source,
                    FontSize = 12,
                    FontWeight = FontWeights.SemiBold,
                    Margin = new Thickness(2, ListPanel.Children.Count == 0 ? 2 : 12, 2, 6),
                };
                groupTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");
                ListPanel.Children.Add(groupTitle);
            }

            var card = new Border { CornerRadius = new CornerRadius(10), Padding = new Thickness(10, 12, 10, 12), Margin = new Thickness(0, 0, 0, 6) };
            card.SetResourceReference(Border.BackgroundProperty, "CardBgBrush");

            var stack = new StackPanel();
            var name = new TextBlock
            {
                Text = s.Name,
                FontSize = 13,
                FontWeight = FontWeights.SemiBold,
                TextTrimming = TextTrimming.CharacterEllipsis,
            };
            name.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
            var desc = new TextBlock
            {
                Text = s.Description.Length > 0 ? s.Description : "(no description)",
                FontSize = 12,
                Margin = new Thickness(0, 4, 0, 0),
                TextTrimming = TextTrimming.CharacterEllipsis,
                LineHeight = DescLineHeight,
                LineStackingStrategy = LineStackingStrategy.BlockLineHeight,
                MaxHeight = DescLineHeight * DescMaxLines,
                ClipToBounds = true,
            };
            desc.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");
            stack.Children.Add(name);
            stack.Children.Add(desc);
            card.Child = stack;
            // Full description on the card tooltip (rust: item.title)
            if (s.Description.Length > 0) card.ToolTip = s.Description;
            ListPanel.Children.Add(card);
        }
    }

    private void RefreshClick(object sender, RoutedEventArgs e) => App.Skills.Load(refresh: true);

    // Singleton reuse: close only hides; the next open re-renders from the
    // cache (rust close_skills hides the window, never destroys it)
    private void CloseClick(object sender, RoutedEventArgs e) => Hide();

    private void TitleBarDrag(object sender, MouseButtonEventArgs e)
    {
        if (e.LeftButton == MouseButtonState.Pressed) DragMove();
    }
}
