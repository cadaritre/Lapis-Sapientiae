using System;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Threading.Tasks;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace LapisGui.ViewModels;

public partial class SettingsViewModel : ViewModelBase
{
    // ── Vision: Local / Cloud toggle ──

    [ObservableProperty]
    private bool _isVisionLocal = true;

    [ObservableProperty]
    private bool _isVisionCloud;

    partial void OnIsVisionLocalChanged(bool value)
    {
        if (value) IsVisionCloud = false;
    }

    partial void OnIsVisionCloudChanged(bool value)
    {
        if (value) IsVisionLocal = false;
    }

    // ── Vision Local fields ──

    [ObservableProperty]
    private string _visionEndpoint = "http://localhost:11434";

    [ObservableProperty]
    private string _visionModel = "moondream";

    // Vision pull terminal
    [ObservableProperty]
    private string _visionPullModelName = string.Empty;

    [ObservableProperty]
    private bool _isVisionPulling;

    public ObservableCollection<TerminalLine> VisionPullOutput { get; } = new();

    // ── Vision Cloud fields ──

    [ObservableProperty]
    private string _visionCloudProvider = "OpenAI";

    [ObservableProperty]
    private string _visionCloudApiKey = string.Empty;

    [ObservableProperty]
    private bool _isVisionApiKeyVisible;

    [ObservableProperty]
    private string _visionCloudModel = "gpt-4o";

    public ObservableCollection<string> VisionCloudProviders { get; } = new()
    {
        "OpenAI",
        "Gemini",
        "Custom"
    };

    // ── Reasoning: Local / Cloud toggle ──

    [ObservableProperty]
    private bool _isReasoningLocal;

    [ObservableProperty]
    private bool _isReasoningCloud = true;

    partial void OnIsReasoningLocalChanged(bool value)
    {
        if (value) IsReasoningCloud = false;
    }

    partial void OnIsReasoningCloudChanged(bool value)
    {
        if (value) IsReasoningLocal = false;
    }

    // ── Reasoning Local fields ──

    [ObservableProperty]
    private string _reasoningEndpoint = "http://localhost:11434";

    [ObservableProperty]
    private string _reasoningLocalModel = "llama3";

    // Reasoning pull terminal
    [ObservableProperty]
    private string _reasoningPullModelName = string.Empty;

    [ObservableProperty]
    private bool _isReasoningPulling;

    public ObservableCollection<TerminalLine> ReasoningPullOutput { get; } = new();

    // ── Reasoning Cloud fields (existing) ──

    [ObservableProperty]
    private string _selectedProvider = "Claude";

    [ObservableProperty]
    private string _apiKey = string.Empty;

    [ObservableProperty]
    private bool _isApiKeyVisible;

    [ObservableProperty]
    private string _reasoningModel = string.Empty;

    public ObservableCollection<string> Providers { get; } = new()
    {
        "Claude",
        "OpenAI",
        "Gemini",
        "Custom"
    };

    partial void OnApiKeyChanged(string value)
    {
        OnPropertyChanged(nameof(MaskedApiKey));
    }

    partial void OnSelectedProviderChanged(string value)
    {
        ReasoningModel = value switch
        {
            "Claude" => "claude-sonnet-4-20250514",
            "OpenAI" => "gpt-4o",
            "Gemini" => "gemini-2.0-flash",
            _ => string.Empty
        };
    }

    public string MaskedApiKey => string.IsNullOrEmpty(ApiKey)
        ? string.Empty
        : ApiKey.Length <= 4
            ? new string('*', ApiKey.Length)
            : new string('*', ApiKey.Length - 4) + ApiKey[^4..];

    // ── Ollama Detection ──

    [ObservableProperty]
    private bool _isOllamaInstalled = true;

    [ObservableProperty]
    private string _ollamaWarningMessage = string.Empty;

    // ── Commands ──

    [RelayCommand]
    private void ToggleApiKeyVisibility()
    {
        IsApiKeyVisible = !IsApiKeyVisible;
    }

    [RelayCommand]
    private void ToggleVisionApiKeyVisibility()
    {
        IsVisionApiKeyVisible = !IsVisionApiKeyVisible;
    }

    [RelayCommand]
    private async Task PullVisionModel()
    {
        if (string.IsNullOrWhiteSpace(VisionPullModelName) || IsVisionPulling) return;
        await PullOllamaModel(VisionPullModelName.Trim(), VisionPullOutput, v => IsVisionPulling = v);
    }

    [RelayCommand]
    private async Task PullReasoningModel()
    {
        if (string.IsNullOrWhiteSpace(ReasoningPullModelName) || IsReasoningPulling) return;
        await PullOllamaModel(ReasoningPullModelName.Trim(), ReasoningPullOutput, v => IsReasoningPulling = v);
    }

    public async Task CheckOllamaInstalledAsync()
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = "ollama",
                Arguments = "--version",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi);
            if (proc is not null)
            {
                await proc.WaitForExitAsync();
                IsOllamaInstalled = proc.ExitCode == 0;
            }
            else
            {
                IsOllamaInstalled = false;
            }
        }
        catch
        {
            IsOllamaInstalled = false;
        }

        OllamaWarningMessage = IsOllamaInstalled
            ? string.Empty
            : "Ollama is not installed. Install it from ollama.com to use local models.";
    }

    private async Task PullOllamaModel(string modelName, ObservableCollection<TerminalLine> output, Action<bool> setPulling)
    {
        setPulling(true);
        output.Clear();
        output.Add(TerminalLine.Status($"Pulling {modelName}..."));

        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = "ollama",
                Arguments = $"pull {modelName}",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var proc = Process.Start(psi);
            if (proc is null)
            {
                output.Add(TerminalLine.Error("Failed to start ollama process"));
                setPulling(false);
                return;
            }

            // Read stdout
            _ = Task.Run(async () =>
            {
                while (!proc.StandardOutput.EndOfStream)
                {
                    var line = await proc.StandardOutput.ReadLineAsync();
                    if (line is not null)
                    {
                        Avalonia.Threading.Dispatcher.UIThread.Post(() =>
                            output.Add(TerminalLine.Normal(line)));
                    }
                }
            });

            // Read stderr
            _ = Task.Run(async () =>
            {
                while (!proc.StandardError.EndOfStream)
                {
                    var line = await proc.StandardError.ReadLineAsync();
                    if (line is not null)
                    {
                        Avalonia.Threading.Dispatcher.UIThread.Post(() =>
                            output.Add(TerminalLine.Normal(line)));
                    }
                }
            });

            await proc.WaitForExitAsync();

            Avalonia.Threading.Dispatcher.UIThread.Post(() =>
            {
                if (proc.ExitCode == 0)
                    output.Add(TerminalLine.Status($"Successfully pulled {modelName}"));
                else
                    output.Add(TerminalLine.Error($"Pull failed (exit code {proc.ExitCode})"));
            });
        }
        catch (Exception ex)
        {
            Avalonia.Threading.Dispatcher.UIThread.Post(() =>
                output.Add(TerminalLine.Error($"Error: {ex.Message}")));
        }
        finally
        {
            Avalonia.Threading.Dispatcher.UIThread.Post(() => setPulling(false));
        }
    }
}
