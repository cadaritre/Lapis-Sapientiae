using System.Text.Json.Nodes;
using LapisGui.IpcClient;

namespace LapisGui.Services;

/// <summary>
/// High-level service for communicating with the Core Agent over IPC.
/// </summary>
public class AgentService : IDisposable
{
    private readonly JsonRpcClient _client = new();

    public bool IsConnected => _client.Connected;

    /// <summary>Fired when a notification arrives from the Core Agent.</summary>
    public event Action<string, JsonNode?>? NotificationReceived
    {
        add => _client.NotificationReceived += value;
        remove => _client.NotificationReceived -= value;
    }

    /// <summary>Fired when the connection drops.</summary>
    public event Action? Disconnected
    {
        add => _client.Disconnected += value;
        remove => _client.Disconnected -= value;
    }

    /// <summary>Connect to the Core Agent on localhost.</summary>
    public async Task<bool> ConnectAsync(int port = 9100)
    {
        try
        {
            await _client.ConnectAsync("127.0.0.1", port);
            // Verify with a ping
            await _client.SendRequestAsync("agent.ping");
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>Send an instruction to the agent and return the summary.</summary>
    public async Task<string> SendInstructionAsync(string instruction)
    {
        var parameters = new JsonObject { ["text"] = instruction };
        var result = await _client.SendRequestAsync("agent.instruct", parameters);
        return result?["summary"]?.GetValue<string>() ?? "(no response)";
    }

    /// <summary>Get agent status.</summary>
    public async Task<JsonNode?> GetStatusAsync()
    {
        return await _client.SendRequestAsync("agent.status");
    }

    public async Task DisconnectAsync()
    {
        await _client.DisconnectAsync();
    }

    public void Dispose()
    {
        _client.Dispose();
        GC.SuppressFinalize(this);
    }
}
