namespace LapisGui.State;

/// <summary>
/// Global application state for the GUI.
/// Phase 2: stub — expanded in later phases.
/// </summary>
public class AppState
{
    public bool IsConnected { get; set; }
    public bool IsSimulationMode { get; set; } = true;
    public string? ActiveTaskId { get; set; }
}
