using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;

namespace LapisGui.Views;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
    }

    private void OnInputKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter && DataContext is ViewModels.MainWindowViewModel vm)
        {
            vm.SendMessageCommand.Execute(null);
            e.Handled = true;
        }
    }

    private void OnMinimizeClick(object? sender, RoutedEventArgs e)
    {
        WindowState = WindowState.Minimized;
    }

    private void OnCloseClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    protected override void OnClosing(WindowClosingEventArgs e)
    {
        if (DataContext is ViewModels.MainWindowViewModel vm)
        {
            vm.Shutdown();
        }
        base.OnClosing(e);
    }

    private void OnTitleBarPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            BeginMoveDrag(e);
        }
    }

    private void OnTabClick(object? sender, RoutedEventArgs e)
    {
        if (sender is not Button btn) return;
        int idx;
        if (btn.Tag is int i) idx = i;
        else if (btn.Tag is string s && int.TryParse(s, out var parsed)) idx = parsed;
        else return;

        // Update visibility
        if (ScreenshotPanel is not null) ScreenshotPanel.IsVisible = idx == 0;
        if (CoreAgentPanel is not null) CoreAgentPanel.IsVisible = idx == 1;
        if (OllamaPanel is not null) OllamaPanel.IsVisible = idx == 2;
        if (LogsPanel is not null) LogsPanel.IsVisible = idx == 3;

        // Update tab button backgrounds using theme resources
        if (btn.Parent is StackPanel tabBar)
        {
            var selectedBg = this.FindResource("SelectedBg") as IBrush ?? Brushes.Transparent;
            var activeText = this.FindResource("TextSecondary") as IBrush ?? Brushes.Gray;
            var inactiveText = this.FindResource("TextMuted") as IBrush ?? Brushes.DarkGray;

            foreach (var child in tabBar.Children)
            {
                if (child is Button tabBtn)
                {
                    var isActive = tabBtn == btn;
                    tabBtn.Background = isActive ? selectedBg : Brushes.Transparent;
                    if (tabBtn.Content is TextBlock tb)
                    {
                        tb.Foreground = isActive ? activeText : inactiveText;
                    }
                }
            }
        }
    }
}
