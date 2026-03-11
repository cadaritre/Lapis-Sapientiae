using System;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Styling;
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

public partial class ConversationItem : ObservableObject
{
    [ObservableProperty]
    private string _title = "";

    [ObservableProperty]
    private string _subtitle = "";

    [ObservableProperty]
    private IBrush _statusColor = new SolidColorBrush(Color.Parse("#3d4052"));

    public ObservableCollection<ChatMessage> Messages { get; } = new();

    public static ConversationItem Create(string title, string subtitle)
    {
        var item = new ConversationItem
        {
            Title = title,
            Subtitle = subtitle,
            StatusColor = new SolidColorBrush(Color.Parse("#6c8aff"))
        };
        item.Messages.Add(ChatMessage.System($"Project '{title}' created."));
        return item;
    }
}

/// <summary>Represents a line of output from a managed process.</summary>
public partial class TerminalLine : ObservableObject
{
    public string Text { get; init; } = "";
    public IBrush Color { get; init; } = Brushes.Gray;

    public static TerminalLine Normal(string text) => new()
    {
        Text = text,
        Color = new SolidColorBrush(Avalonia.Media.Color.Parse("#8b8fa3"))
    };

    public static TerminalLine Status(string text) => new()
    {
        Text = text,
        Color = new SolidColorBrush(Avalonia.Media.Color.Parse("#50c878"))
    };

    public static TerminalLine Error(string text) => new()
    {
        Text = text,
        Color = new SolidColorBrush(Avalonia.Media.Color.Parse("#e05050"))
    };
}

public partial class MainWindowViewModel : ViewModelBase
{
    private readonly AgentService _agent = new();
    private readonly ProcessManager _processManager = new();

    [ObservableProperty]
    private string _connectionStatus = "Disconnected";

    [ObservableProperty]
    private bool _isSimulationMode = true;

    partial void OnIsSimulationModeChanged(bool value)
    {
        _ = SendSimulationModeAsync(value);
    }

    private async Task SendSimulationModeAsync(bool simMode)
    {
        if (!_agent.IsConnected) return;
        try
        {
            await _agent.ConfigureSimulationAsync(simMode);
            LogEntries.Add(simMode
                ? LogEntry.Info("Simulation mode ON")
                : LogEntry.Warn("Simulation mode OFF — REAL ACTIONS ENABLED"));
        }
        catch { }
    }

    [ObservableProperty]
    private string _userInput = string.Empty;

    [ObservableProperty]
    private bool _isSettingsOpen;

    [ObservableProperty]
    private bool _isDarkMode;

    partial void OnIsDarkModeChanged(bool value)
    {
        if (Application.Current is not null)
            Application.Current.RequestedThemeVariant = value ? ThemeVariant.Dark : ThemeVariant.Light;
    }

    [ObservableProperty]
    private bool _isSending;

    [ObservableProperty]
    private bool _isExecuting;

    [ObservableProperty]
    private bool _isConfirmationPending;

    [ObservableProperty]
    private string _confirmationPlanText = string.Empty;

    [ObservableProperty]
    private ConversationItem? _selectedConversation;

    [ObservableProperty]
    private Bitmap? _screenshotImage;

    [ObservableProperty]
    private string _screenshotInfo = "No capture";

    // ── Service status ──

    [ObservableProperty]
    private bool _isCoreAgentRunning;

    [ObservableProperty]
    private bool _isOllamaRunning;

    // ── Right panel tab selection: 0=Screenshot, 1=Services, 2=Logs ──

    [ObservableProperty]
    private int _rightPanelTab;

    public SettingsViewModel Settings { get; } = new();

    public ObservableCollection<ConversationItem> Conversations { get; } = new();

    /// <summary>Terminal output from Core Agent process.</summary>
    public ObservableCollection<TerminalLine> CoreAgentOutput { get; } = new();

    /// <summary>Terminal output from Ollama process.</summary>
    public ObservableCollection<TerminalLine> OllamaOutput { get; } = new();

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

        _agent.NotificationReceived += (method, paramsNode) =>
        {
            if (method == "agent.step_progress" && paramsNode is not null)
            {
                Dispatcher.UIThread.Post(() => HandleStepProgress(paramsNode));
            }
            else if (method == "agent.confirm_plan" && paramsNode is not null)
            {
                Dispatcher.UIThread.Post(() => HandleConfirmPlan(paramsNode));
            }
        };

        // Wire up process manager events
        _processManager.OutputReceived += (source, line) =>
        {
            Dispatcher.UIThread.Post(() =>
            {
                var collection = source == "Core Agent" ? CoreAgentOutput : OllamaOutput;
                var entry = line.StartsWith("---")
                    ? TerminalLine.Status(line)
                    : line.Contains("error", StringComparison.OrdinalIgnoreCase) || line.Contains("ERR")
                        ? TerminalLine.Error(line)
                        : TerminalLine.Normal(line);
                collection.Add(entry);
                // Keep max 500 lines per terminal
                while (collection.Count > 500) collection.RemoveAt(0);
            });
        };

        _processManager.ProcessStateChanged += (source, running) =>
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (source == "Core Agent")
                    IsCoreAgentRunning = running;
                else
                    IsOllamaRunning = running;

                LogEntries.Add(running
                    ? LogEntry.Info($"{source} started")
                    : LogEntry.Warn($"{source} stopped"));
            });
        };

        // Create default conversation
        var defaultConv = ConversationItem.Create("General", "Default workspace");
        defaultConv.Messages.Clear();
        defaultConv.Messages.Add(ChatMessage.System("Lapis Sapientiae started."));
        defaultConv.Messages.Add(ChatMessage.System("Launching services..."));
        Conversations.Add(defaultConv);
        SelectedConversation = defaultConv;

        _ = StartServicesAndConnectAsync();
    }

    partial void OnSelectedConversationChanged(ConversationItem? value)
    {
        OnPropertyChanged(nameof(ChatMessages));
    }

    private void HandleStepProgress(System.Text.Json.Nodes.JsonNode paramsNode)
    {
        var stepId = paramsNode["step_id"]?.GetValue<int>() ?? 0;
        var total = paramsNode["total_steps"]?.GetValue<int>() ?? 0;
        var desc = paramsNode["description"]?.GetValue<string>() ?? "";
        var status = paramsNode["status"]?.GetValue<string>() ?? "";
        var result = paramsNode["result"]?.GetValue<string>();

        if (status == "started")
        {
            ChatMessages.Add(ChatMessage.System($"[{stepId}/{total}] {desc}..."));
            LogEntries.Add(LogEntry.Info($"Step {stepId}/{total} started: {desc}"));
        }
        else if (status == "completed")
        {
            LogEntries.Add(LogEntry.Info($"Step {stepId}/{total} completed: {result ?? desc}"));
        }
        else if (status == "failed")
        {
            LogEntries.Add(LogEntry.Error($"Step {stepId}/{total} failed: {result ?? desc}"));
        }
        else if (status == "aborted")
        {
            ChatMessages.Add(ChatMessage.System($"Execution aborted at step {stepId}/{total}"));
            LogEntries.Add(LogEntry.Warn($"Aborted at step {stepId}/{total}"));
        }
        else if (status == "awaiting_confirmation")
        {
            LogEntries.Add(LogEntry.Info("Awaiting user confirmation for real execution"));
        }
    }

    private void HandleConfirmPlan(System.Text.Json.Nodes.JsonNode paramsNode)
    {
        var instruction = paramsNode["instruction"]?.GetValue<string>() ?? "";
        var stepsArray = paramsNode["steps"]?.AsArray();
        var reasoning = paramsNode["reasoning"]?.GetValue<string>();

        var planText = $"Instruction: {instruction}\n";
        if (!string.IsNullOrEmpty(reasoning))
            planText += $"Reasoning: {reasoning}\n";
        planText += "\nSteps:\n";

        if (stepsArray is not null)
        {
            foreach (var step in stepsArray)
            {
                var id = step?["id"]?.GetValue<int>() ?? 0;
                var desc = step?["description"]?.GetValue<string>() ?? "";
                var actionType = step?["action_type"]?.GetValue<string>() ?? "";
                planText += $"  {id}. [{actionType}] {desc}\n";
            }
        }

        ConfirmationPlanText = planText;
        IsConfirmationPending = true;
        ChatMessages.Add(ChatMessage.System("Real execution requires confirmation. Review the plan above."));
    }

    private async Task StartServicesAndConnectAsync()
    {
        // Resolve path to core agent executable
        var guiDir = AppDomain.CurrentDomain.BaseDirectory;
        var coreAgentPath = Path.GetFullPath(Path.Combine(guiDir, "..", "..", "..", "..", "core-agent", "target", "release", "lapis-core.exe"));

        // Start Ollama server
        _processManager.StartOllama();
        LogEntries.Add(LogEntry.Info("Starting Ollama server..."));

        // Wait a moment for Ollama to initialize
        await Task.Delay(2000);

        // Start Core Agent
        if (File.Exists(coreAgentPath))
        {
            _processManager.StartCoreAgent(coreAgentPath);
            LogEntries.Add(LogEntry.Info("Starting Core Agent..."));
        }
        else
        {
            LogEntries.Add(LogEntry.Warn($"Core Agent not found at: {coreAgentPath}"));
            ChatMessages.Add(ChatMessage.System($"Core Agent not found. Build with:\ncd core-agent && cargo build --release"));
        }

        // Wait for services to initialize
        await Task.Delay(3000);
        ChatMessages.Add(ChatMessage.System("Services launched. Connecting..."));

        // Retry connection a few times
        for (int i = 0; i < 3; i++)
        {
            var connected = await TryConnectAsync();
            if (connected) break;
            await Task.Delay(2000);
        }
    }

    /// <summary>Kill all managed processes on shutdown.</summary>
    public void Shutdown()
    {
        _processManager.KillAll();
        _agent.Dispose();
    }

    private async Task<bool> TryConnectAsync()
    {
        LogEntries.Add(LogEntry.Info("Connecting to Core Agent on port 9100..."));
        var connected = await _agent.ConnectAsync();

        if (connected)
        {
            ConnectionStatus = "Connected";
            ChatMessages.Add(ChatMessage.System("Connected to Core Agent."));
            LogEntries.Add(LogEntry.Info("Connected to Core Agent"));

            // Send current VLM config
            await _agent.ConfigureAsync(Settings.VisionEndpoint, Settings.VisionModel);

            // Send simulation mode state
            await _agent.ConfigureSimulationAsync(IsSimulationMode);

            // Send reasoning config if API key is set
            if (!string.IsNullOrEmpty(Settings.ApiKey))
            {
                await _agent.ConfigureReasoningAsync(
                    Settings.SelectedProvider, Settings.ApiKey, Settings.ReasoningModel);
            }

            return true;
        }
        else
        {
            ConnectionStatus = "Disconnected";
            LogEntries.Add(LogEntry.Warn("Core Agent not available — retrying..."));
            return false;
        }
    }

    // ── Commands ──

    [RelayCommand]
    private async Task ToggleSettings()
    {
        IsSettingsOpen = !IsSettingsOpen;
        if (IsSettingsOpen)
        {
            await Settings.CheckOllamaInstalledAsync();
        }
    }

    [RelayCommand]
    private async Task SaveSettings()
    {
        IsSettingsOpen = false;
        LogEntries.Add(LogEntry.Info("Settings saved"));

        if (!_agent.IsConnected) return;

        // Vision config: local uses Ollama endpoint, cloud uses cloud provider endpoint
        if (Settings.IsVisionLocal)
        {
            var ok = await _agent.ConfigureAsync(Settings.VisionEndpoint, Settings.VisionModel);
            if (ok)
                LogEntries.Add(LogEntry.Info($"VLM configured (local): {Settings.VisionModel} @ {Settings.VisionEndpoint}"));
            else
                LogEntries.Add(LogEntry.Warn("Failed to send VLM config to Core Agent"));
        }
        else if (!string.IsNullOrEmpty(Settings.VisionCloudApiKey))
        {
            // For cloud vision, we send via configure with cloud endpoint
            var endpoint = Settings.VisionCloudProvider switch
            {
                "OpenAI" => "https://api.openai.com",
                "Gemini" => "https://generativelanguage.googleapis.com",
                _ => Settings.VisionEndpoint
            };
            var ok = await _agent.ConfigureAsync(endpoint, Settings.VisionCloudModel);
            if (ok)
                LogEntries.Add(LogEntry.Info($"VLM configured (cloud): {Settings.VisionCloudProvider} / {Settings.VisionCloudModel}"));
            else
                LogEntries.Add(LogEntry.Warn("Failed to send cloud VLM config to Core Agent"));
        }

        // Reasoning config: local uses Ollama, cloud uses cloud provider
        if (Settings.IsReasoningLocal)
        {
            var rok = await _agent.ConfigureReasoningAsync(
                "ollama", string.Empty, Settings.ReasoningLocalModel);
            if (rok)
                LogEntries.Add(LogEntry.Info($"Reasoning configured (local): {Settings.ReasoningLocalModel}"));
            else
                LogEntries.Add(LogEntry.Warn("Failed to send local reasoning config to Core Agent"));
        }
        else if (!string.IsNullOrEmpty(Settings.ApiKey))
        {
            var rok = await _agent.ConfigureReasoningAsync(
                Settings.SelectedProvider, Settings.ApiKey, Settings.ReasoningModel);
            if (rok)
                LogEntries.Add(LogEntry.Info($"Reasoning configured (cloud): {Settings.SelectedProvider} / {Settings.ReasoningModel}"));
            else
                LogEntries.Add(LogEntry.Warn("Failed to send reasoning config to Core Agent"));
        }
    }

    [RelayCommand]
    private async Task Reconnect()
    {
        LogEntries.Add(LogEntry.Info("Reconnecting..."));
        await _agent.DisconnectAsync();
        await TryConnectAsync();
    }

    [RelayCommand]
    private async Task CaptureScreenshot()
    {
        if (!_agent.IsConnected) return;

        try
        {
            var result = await _agent.RequestScreenshotAsync();
            if (result is not null)
            {
                var (width, height, png_base64) = result.Value;
                var bytes = Convert.FromBase64String(png_base64);
                using var ms = new MemoryStream(bytes);
                ScreenshotImage = new Bitmap(ms);
                ScreenshotInfo = $"{width}x{height}";
                LogEntries.Add(LogEntry.Info($"Screenshot captured: {width}x{height}"));
            }
        }
        catch (Exception ex)
        {
            LogEntries.Add(LogEntry.Error($"Screenshot failed: {ex.Message}"));
        }
    }

    [RelayCommand]
    private void NewConversation()
    {
        var count = Conversations.Count + 1;
        var conv = ConversationItem.Create($"Project {count}", DateTime.Now.ToString("MMM dd, HH:mm"));
        Conversations.Add(conv);
        SelectedConversation = conv;
        LogEntries.Add(LogEntry.Info($"Created project: {conv.Title}"));
    }

    [RelayCommand]
    private void DeleteConversation()
    {
        if (SelectedConversation is null || Conversations.Count <= 1)
            return;

        var toRemove = SelectedConversation;
        var idx = Conversations.IndexOf(toRemove);
        Conversations.Remove(toRemove);
        SelectedConversation = Conversations[Math.Max(0, idx - 1)];
        LogEntries.Add(LogEntry.Info($"Deleted project: {toRemove.Title}"));
    }

    [RelayCommand]
    private async Task AnalyzeScreen()
    {
        if (!_agent.IsConnected) return;

        ChatMessages.Add(ChatMessage.System("Analyzing screen with VLM..."));
        LogEntries.Add(LogEntry.Info("Requesting VLM screen analysis..."));

        try
        {
            // Send config first
            await _agent.ConfigureAsync(Settings.VisionEndpoint, Settings.VisionModel);

            var result = await _agent.AnalyzeScreenAsync();
            if (result is not null)
            {
                var (width, height, description, model) = result.Value;
                ScreenshotInfo = $"{width}x{height} | {model}";
                ChatMessages.Add(ChatMessage.Agent($"[VLM: {model}]\n{description}"));
                LogEntries.Add(LogEntry.Info($"VLM analysis complete ({model})"));

                await CaptureScreenshot();
            }
            else
            {
                ChatMessages.Add(ChatMessage.Agent("VLM analysis returned no result."));
                LogEntries.Add(LogEntry.Warn("VLM analysis returned null"));
            }
        }
        catch (Exception ex)
        {
            ChatMessages.Add(ChatMessage.Agent($"VLM error: {ex.Message}"));
            LogEntries.Add(LogEntry.Error($"VLM error: {ex.Message}"));
        }
    }

    [RelayCommand]
    private void RestartCoreAgent()
    {
        _processManager.StopCoreAgent();
        var guiDir = AppDomain.CurrentDomain.BaseDirectory;
        var coreAgentPath = Path.GetFullPath(Path.Combine(guiDir, "..", "..", "..", "..", "core-agent", "target", "release", "lapis-core.exe"));
        if (File.Exists(coreAgentPath))
        {
            CoreAgentOutput.Clear();
            _processManager.StartCoreAgent(coreAgentPath);
        }
    }

    [RelayCommand]
    private void RestartOllama()
    {
        _processManager.StopOllama();
        OllamaOutput.Clear();
        _processManager.StartOllama();
    }

    [RelayCommand]
    private void StopCoreAgent()
    {
        _processManager.StopCoreAgent();
    }

    [RelayCommand]
    private void StopOllama()
    {
        _processManager.StopOllama();
    }

    public IBrush StatusColor => ConnectionStatus == "Connected"
        ? new SolidColorBrush(Color.Parse("#50c878"))
        : new SolidColorBrush(Color.Parse("#e05050"));

    partial void OnConnectionStatusChanged(string value)
    {
        OnPropertyChanged(nameof(StatusColor));
    }

    public ObservableCollection<ChatMessage> ChatMessages =>
        SelectedConversation?.Messages ?? new ObservableCollection<ChatMessage>();

    public ObservableCollection<LogEntry> LogEntries { get; } = new()
    {
        LogEntry.Info("GUI initialized"),
        LogEntry.Info("IPC client ready")
    };

    [RelayCommand]
    private async Task AbortExecution()
    {
        if (!_agent.IsConnected) return;
        var ok = await _agent.AbortAsync();
        if (ok)
        {
            ChatMessages.Add(ChatMessage.System("Abort signal sent."));
            LogEntries.Add(LogEntry.Warn("Abort signal sent to Core Agent"));
        }
    }

    [RelayCommand]
    private async Task ConfirmExecution()
    {
        IsConfirmationPending = false;
        if (!_agent.IsConnected) return;
        await _agent.ConfirmExecutionAsync(true);
        ChatMessages.Add(ChatMessage.System("Execution confirmed. Running..."));
        LogEntries.Add(LogEntry.Info("User confirmed real execution"));
    }

    [RelayCommand]
    private async Task CancelExecution()
    {
        IsConfirmationPending = false;
        if (!_agent.IsConnected) return;
        await _agent.ConfirmExecutionAsync(false);
        ChatMessages.Add(ChatMessage.System("Execution cancelled."));
        LogEntries.Add(LogEntry.Warn("User cancelled execution"));
    }

    [RelayCommand]
    private async Task SendMessage()
    {
        if (string.IsNullOrWhiteSpace(UserInput) || IsSending)
            return;

        var instruction = UserInput;
        UserInput = string.Empty;
        ChatMessages.Add(ChatMessage.User(instruction));
        LogEntries.Add(LogEntry.Info($"Sending: {instruction}"));

        if (SelectedConversation is not null)
        {
            SelectedConversation.Subtitle = instruction.Length > 30
                ? instruction[..30] + "..."
                : instruction;
        }

        if (!_agent.IsConnected)
        {
            ChatMessages.Add(ChatMessage.Agent("(offline) Core Agent not connected. Start it and reconnect."));
            LogEntries.Add(LogEntry.Warn("Cannot send — not connected"));
            return;
        }

        IsSending = true;
        IsExecuting = true;
        try
        {
            var response = await _agent.SendInstructionAsync(instruction);
            ChatMessages.Add(ChatMessage.Agent(response));
            LogEntries.Add(LogEntry.Info("Agent responded"));

            await CaptureScreenshot();
        }
        catch (Exception ex)
        {
            ChatMessages.Add(ChatMessage.Agent($"Error: {ex.Message}"));
            LogEntries.Add(LogEntry.Error($"RPC error: {ex.Message}"));
        }
        finally
        {
            IsSending = false;
            IsExecuting = false;
            IsConfirmationPending = false;
        }
    }
}
