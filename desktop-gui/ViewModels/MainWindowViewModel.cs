using System;
using System.Collections.ObjectModel;
using Avalonia.Media;
using Avalonia.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LapisGui.Services;

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
    private readonly AgentService _agent = new();

    [ObservableProperty]
    private string _connectionStatus = "Disconnected";

    [ObservableProperty]
    private bool _isSimulationMode = true;

    [ObservableProperty]
    private string _userInput = string.Empty;

    [ObservableProperty]
    private bool _isSettingsOpen;

    [ObservableProperty]
    private bool _isSending;

    public SettingsViewModel Settings { get; } = new();

    public MainWindowViewModel()
    {
        _agent.Disconnected += () =>
        {
            Dispatcher.UIThread.Post(() =>
            {
                ConnectionStatus = "Disconnected";
                LogEntries.Add(LogEntry.Warn("Connection to Core Agent lost"));
            });
        };

        // Try to connect on startup
        _ = TryConnectAsync();
    }

    private async Task TryConnectAsync()
    {
        LogEntries.Add(LogEntry.Info("Connecting to Core Agent on port 9100..."));
        var connected = await _agent.ConnectAsync();

        if (connected)
        {
            ConnectionStatus = "Connected";
            ChatMessages.Add(ChatMessage.System("Connected to Core Agent."));
            LogEntries.Add(LogEntry.Info("Connected to Core Agent"));
        }
        else
        {
            ConnectionStatus = "Disconnected";
            LogEntries.Add(LogEntry.Warn("Core Agent not available — running in offline mode"));
        }
    }

    [RelayCommand]
    private void ToggleSettings()
    {
        IsSettingsOpen = !IsSettingsOpen;
    }

    [RelayCommand]
    private void SaveSettings()
    {
        IsSettingsOpen = false;
        LogEntries.Add(LogEntry.Info("Settings saved"));
    }

    [RelayCommand]
    private async Task Reconnect()
    {
        LogEntries.Add(LogEntry.Info("Reconnecting..."));
        await _agent.DisconnectAsync();
        await TryConnectAsync();
    }

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
        LogEntry.Info("IPC client ready")
    };

    [RelayCommand]
    private async Task SendMessage()
    {
        if (string.IsNullOrWhiteSpace(UserInput) || IsSending)
            return;

        var instruction = UserInput;
        UserInput = string.Empty;
        ChatMessages.Add(ChatMessage.User(instruction));
        LogEntries.Add(LogEntry.Info($"Sending: {instruction}"));

        if (!_agent.IsConnected)
        {
            ChatMessages.Add(ChatMessage.Agent("(offline) Core Agent not connected. Start it and reconnect."));
            LogEntries.Add(LogEntry.Warn("Cannot send — not connected"));
            return;
        }

        IsSending = true;
        try
        {
            var response = await _agent.SendInstructionAsync(instruction);
            ChatMessages.Add(ChatMessage.Agent(response));
            LogEntries.Add(LogEntry.Info("Agent responded"));
        }
        catch (Exception ex)
        {
            ChatMessages.Add(ChatMessage.Agent($"Error: {ex.Message}"));
            LogEntries.Add(LogEntry.Error($"RPC error: {ex.Message}"));
        }
        finally
        {
            IsSending = false;
        }
    }
}
