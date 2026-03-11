using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;

namespace LapisGui.Services;

/// <summary>
/// Manages external processes (Core Agent, Ollama) with stdout/stderr capture.
/// Provides real-time output via events and tracks running state.
/// Kills all spawned processes on disposal.
/// </summary>
public class ProcessManager : IDisposable
{
    private Process? _coreAgentProcess;
    private Process? _ollamaProcess;

    /// <summary>Fired when a process emits a line of output (stdout or stderr).</summary>
    public event Action<string, string>? OutputReceived; // (source, line)

    /// <summary>Fired when a process starts or stops.</summary>
    public event Action<string, bool>? ProcessStateChanged; // (source, running)

    public bool IsCoreAgentRunning => _coreAgentProcess is not null && !_coreAgentProcess.HasExited;
    public bool IsOllamaRunning => _ollamaExternal || (_ollamaProcess is not null && !_ollamaProcess.HasExited);

    /// <summary>
    /// Spawns a process with stdout/stderr redirected for capture.
    /// </summary>
    private Process? SpawnWithCapture(string source, string executable, string arguments = "")
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = executable,
                Arguments = arguments,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                RedirectStandardInput = false,
            };

            var proc = new Process { StartInfo = psi, EnableRaisingEvents = true };

            proc.OutputDataReceived += (_, e) =>
            {
                if (e.Data is not null)
                    OutputReceived?.Invoke(source, e.Data);
            };

            proc.ErrorDataReceived += (_, e) =>
            {
                if (e.Data is not null)
                    OutputReceived?.Invoke(source, e.Data);
            };

            proc.Exited += (_, _) =>
            {
                OutputReceived?.Invoke(source, $"--- {source} process exited (code {proc.ExitCode}) ---");
                ProcessStateChanged?.Invoke(source, false);
            };

            proc.Start();
            proc.BeginOutputReadLine();
            proc.BeginErrorReadLine();

            OutputReceived?.Invoke(source, $"--- {source} started (PID {proc.Id}) ---");
            ProcessStateChanged?.Invoke(source, true);

            return proc;
        }
        catch (Exception ex)
        {
            OutputReceived?.Invoke(source, $"--- Failed to start {source}: {ex.Message} ---");
            ProcessStateChanged?.Invoke(source, false);
            return null;
        }
    }

    /// <summary>Start the Rust core agent with stdout/stderr capture.</summary>
    public void StartCoreAgent(string exePath)
    {
        if (IsCoreAgentRunning) return;
        _coreAgentProcess = SpawnWithCapture("Core Agent", exePath);
    }

    /// <summary>Start Ollama server with stdout/stderr capture.
    /// If Ollama is already listening on port 11434, reports it as external and skips spawning.</summary>
    public bool StartOllama()
    {
        if (IsOllamaRunning) return true;

        // Check if Ollama is already running externally (e.g. desktop app)
        try
        {
            using var tcp = new System.Net.Sockets.TcpClient();
            tcp.Connect("127.0.0.1", 11434);
            tcp.Close();
            OutputReceived?.Invoke("Ollama", "--- Ollama already running externally on port 11434 ---");
            ProcessStateChanged?.Invoke("Ollama", true);
            _ollamaExternal = true;
            return true;
        }
        catch { /* not running, we'll start it */ }

        _ollamaExternal = false;
        _ollamaProcess = SpawnWithCapture("Ollama", "ollama", "serve");
        return _ollamaProcess is not null;
    }

    private bool _ollamaExternal;

    /// <summary>Stop the Core Agent process.</summary>
    public void StopCoreAgent()
    {
        KillProcess(ref _coreAgentProcess, "Core Agent");
    }

    /// <summary>Stop the Ollama process.</summary>
    public void StopOllama()
    {
        KillProcess(ref _ollamaProcess, "Ollama");
    }

    /// <summary>Kill all managed processes and their child process trees.</summary>
    public void KillAll()
    {
        KillProcess(ref _coreAgentProcess, "Core Agent");
        KillProcess(ref _ollamaProcess, "Ollama");
    }

    private void KillProcess(ref Process? proc, string source)
    {
        if (proc is null) return;
        try
        {
            if (!proc.HasExited)
            {
                proc.Kill(entireProcessTree: true);
                OutputReceived?.Invoke(source, $"--- {source} killed ---");
            }
        }
        catch { /* already exited */ }
        finally
        {
            proc.Dispose();
            proc = null;
            ProcessStateChanged?.Invoke(source, false);
        }
    }

    public void Dispose()
    {
        KillAll();
        GC.SuppressFinalize(this);
    }
}
