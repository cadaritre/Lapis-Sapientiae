using Avalonia.Controls;
using Avalonia.Input;

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
}
