using System.Collections.Concurrent;
using System.IO;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace LapisGui.IpcClient;

/// <summary>
/// JSON-RPC 2.0 client over TCP with newline-delimited framing.
/// </summary>
public class JsonRpcClient : IDisposable
{
    private TcpClient? _tcp;
    private StreamWriter? _writer;
    private StreamReader? _reader;
    private CancellationTokenSource? _cts;
    private Task? _readLoop;
    private int _nextId;

    private readonly ConcurrentDictionary<int, TaskCompletionSource<JsonNode?>> _pending = new();

    public bool Connected => _tcp?.Connected == true;

    /// <summary>Fired when the server sends a notification (no id).</summary>
    public event Action<string, JsonNode?>? NotificationReceived;

    /// <summary>Fired when the connection drops.</summary>
    public event Action? Disconnected;

    public async Task ConnectAsync(string host, int port)
    {
        _tcp = new TcpClient();
        await _tcp.ConnectAsync(host, port);

        var stream = _tcp.GetStream();
        _writer = new StreamWriter(stream, new UTF8Encoding(false)) { AutoFlush = true };
        _reader = new StreamReader(stream, Encoding.UTF8);

        _cts = new CancellationTokenSource();
        _readLoop = Task.Run(() => ReadLoopAsync(_cts.Token));
    }

    /// <summary>
    /// Send a JSON-RPC request and await the response.
    /// </summary>
    public async Task<JsonNode?> SendRequestAsync(string method, JsonNode? parameters = null)
    {
        if (_writer is null)
            throw new InvalidOperationException("Not connected");

        var id = Interlocked.Increment(ref _nextId);
        var tcs = new TaskCompletionSource<JsonNode?>();
        _pending[id] = tcs;

        var request = new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["method"] = method,
            ["id"] = id
        };
        if (parameters is not null)
            request["params"] = parameters.DeepClone();

        var line = request.ToJsonString();
        await _writer.WriteLineAsync(line);

        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(300));
        timeout.Token.Register(() => tcs.TrySetCanceled());

        try
        {
            return await tcs.Task;
        }
        finally
        {
            _pending.TryRemove(id, out _);
        }
    }

    private async Task ReadLoopAsync(CancellationToken ct)
    {
        try
        {
            while (!ct.IsCancellationRequested && _reader is not null)
            {
                var line = await _reader.ReadLineAsync(ct);
                if (line is null)
                    break;

                line = line.Trim();
                if (string.IsNullOrEmpty(line))
                    continue;

                try
                {
                    var msg = JsonNode.Parse(line);
                    if (msg is null) continue;

                    var id = msg["id"];
                    if (id is not null && id.GetValueKind() == JsonValueKind.Number)
                    {
                        var reqId = id.GetValue<int>();
                        if (_pending.TryRemove(reqId, out var tcs))
                        {
                            var error = msg["error"];
                            if (error is not null)
                                tcs.TrySetException(new Exception(
                                    error["message"]?.GetValue<string>() ?? "Unknown RPC error"));
                            else
                                tcs.TrySetResult(msg["result"]);
                        }
                    }
                    else
                    {
                        var method = msg["method"]?.GetValue<string>() ?? "unknown";
                        NotificationReceived?.Invoke(method, msg["params"]);
                    }
                }
                catch (JsonException)
                {
                    // Malformed message, skip
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (IOException) { }
        finally
        {
            Disconnected?.Invoke();
        }
    }

    public async Task DisconnectAsync()
    {
        _cts?.Cancel();
        if (_readLoop is not null)
        {
            try { await _readLoop; } catch { /* ignore */ }
        }
        _writer?.Dispose();
        _reader?.Dispose();
        _tcp?.Dispose();
        _tcp = null;
        _writer = null;
        _reader = null;
    }

    public void Dispose()
    {
        _cts?.Cancel();
        _writer?.Dispose();
        _reader?.Dispose();
        _tcp?.Dispose();
        GC.SuppressFinalize(this);
    }
}
