using System;
using System.Collections.ObjectModel;
using Avalonia.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace LapisGui.ViewModels;

public partial class ChatMessage : ObservableObject
{
    public string Role { get; init; } = "";
    public string Content { get; init; } = "";
    public string Timestamp { get; init; } = "";
    public IBrush RoleColor { get; init; } = Brushes.Gray;

    public static ChatMessage System(string content) => new()
    {
        Role = "SYSTEM",
        Content = content,
        Timestamp = DateTime.Now.ToString("HH:mm"),
        RoleColor = new SolidColorBrush(Color.Parse("#4a5068"))
    };

    public static ChatMessage User(string content) => new()
    {
        Role = "YOU",
        Content = content,
        Timestamp = DateTime.Now.ToString("HH:mm"),
        RoleColor = new SolidColorBrush(Color.Parse("#6c8aff"))
    };

    public static ChatMessage Agent(string content) => new()
    {
        Role = "AGENT",
        Content = content,
        Timestamp = DateTime.Now.ToString("HH:mm"),
        RoleColor = new SolidColorBrush(Color.Parse("#50c878"))
    };
}

public partial class LogEntry : ObservableObject
{
    public string Level { get; init; } = "";
    public string Message { get; init; } = "";
    public IBrush LevelColor { get; init; } = Brushes.Gray;

    public static LogEntry Info(string msg) => new()
    {
        Level = "INFO",
        Message = msg,
        LevelColor = new SolidColorBrush(Color.Parse("#4a5068"))
    };

    public static LogEntry Warn(string msg) => new()
    {
        Level = "WARN",
        Message = msg,
        LevelColor = new SolidColorBrush(Color.Parse("#f0a050"))
    };

    public static LogEntry Error(string msg) => new()
    {
        Level = "ERR",
        Message = msg,
        LevelColor = new SolidColorBrush(Color.Parse("#e05050"))
    };
}

public partial class MainWindowViewModel : ViewModelBase
{
    [ObservableProperty]
    private string _connectionStatus = "Disconnected";

    [ObservableProperty]
    private bool _isSimulationMode = true;

    [ObservableProperty]
    private string _userInput = string.Empty;

    public IBrush StatusColor => ConnectionStatus == "Connected"
        ? new SolidColorBrush(Color.Parse("#50c878"))
        : new SolidColorBrush(Color.Parse("#e05050"));

    partial void OnConnectionStatusChanged(string value)
    {
        OnPropertyChanged(nameof(StatusColor));
    }

    public ObservableCollection<ChatMessage> ChatMessages { get; } = new()
    {
        ChatMessage.System("Lapis Sapientiae started."),
        ChatMessage.System("Waiting for Core Agent connection...")
    };

    public ObservableCollection<LogEntry> LogEntries { get; } = new()
    {
        LogEntry.Info("GUI initialized"),
        LogEntry.Info("IPC client ready"),
        LogEntry.Warn("Core agent not connected")
    };

    [RelayCommand]
    private void SendMessage()
    {
        if (string.IsNullOrWhiteSpace(UserInput))
            return;

        ChatMessages.Add(ChatMessage.User(UserInput));
        LogEntries.Add(LogEntry.Info($"User sent: {UserInput}"));

        ChatMessages.Add(ChatMessage.Agent($"(stub) Received: {UserInput}"));
        UserInput = string.Empty;
    }
}
