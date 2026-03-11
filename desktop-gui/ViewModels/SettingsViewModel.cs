using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace LapisGui.ViewModels;

public partial class SettingsViewModel : ViewModelBase
{
    // ── Vision Model (Local) ──

    [ObservableProperty]
    private string _visionEndpoint = "http://localhost:11434";

    [ObservableProperty]
    private string _visionModel = "llava:latest";

    // ── Reasoning Model (Cloud) ──

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

    public string MaskedApiKey => string.IsNullOrEmpty(ApiKey)
        ? string.Empty
        : ApiKey.Length <= 4
            ? new string('*', ApiKey.Length)
            : new string('*', ApiKey.Length - 4) + ApiKey[^4..];

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

    [RelayCommand]
    private void ToggleApiKeyVisibility()
    {
        IsApiKeyVisible = !IsApiKeyVisible;
    }
}
