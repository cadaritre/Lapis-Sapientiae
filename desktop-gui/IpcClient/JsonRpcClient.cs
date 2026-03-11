namespace LapisGui.IpcClient;

/// <summary>
/// JSON-RPC client for communication with the Core Agent.
/// Phase 2: stub — real implementation in Phase 3.
/// </summary>
public class JsonRpcClient
{
    public bool Connected { get; private set; }

    public Task ConnectAsync(string transport, int port)
    {
        // Phase 2: stub
        Connected = false;
        return Task.CompletedTask;
    }

    public Task DisconnectAsync()
    {
        Connected = false;
        return Task.CompletedTask;
    }
}
