namespace LapisGui.Services;

/// <summary>
/// High-level service for communicating with the Core Agent over IPC.
/// Phase 2: stub — real implementation in Phase 3.
/// </summary>
public class AgentService
{
    public bool IsConnected { get; private set; }

    public Task ConnectAsync()
    {
        // Phase 2: stub
        IsConnected = false;
        return Task.CompletedTask;
    }

    public Task<string> SendInstructionAsync(string instruction)
    {
        // Phase 2: stub
        return Task.FromResult($"(stub) Received: {instruction}");
    }
}
