using System;
using System.Diagnostics;

namespace LapisGui.Services;

/// <summary>
/// Manages external processes (Core Agent, Ollama) in visible terminal windows.
/// Kills all spawned processes on disposal.
/// </summary>
public class ProcessManager : IDisposable
{
    private Process? _coreAgentProcess;
    private Process? _ollamaProcess;

    /// <summary>
    /// Spawns an executable inside a visible cmd.exe window with a title.
    /// </summary>
    private static Process? SpawnInVisibleTerminal(string title, string executable, string arguments = "")
    {
        try
        {
            var args = string.IsNullOrEmpty(arguments)
                ? $"/k title {title} && \"{executable}\""
                : $"/k title {title} && \"{executable}\" {arguments}";

            var psi = new ProcessStartInfo
            {
                FileName = "cmd.exe",
                Arguments = args,
                UseShellExecute = true,
                CreateNoWindow = false,
            };

            return Process.Start(psi);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[ProcessManager] Failed to start {title}: {ex.Message}");
            return null;
        }
    }

    /// <summary>Start the Rust core agent in a visible terminal.</summary>
    public void StartCoreAgent(string exePath)
    {
        _coreAgentProcess = SpawnInVisibleTerminal("Lapis Core Agent", exePath);
    }

    /// <summary>Start Ollama server in a visible terminal.</summary>
    public void StartOllama()
    {
        _ollamaProcess = SpawnInVisibleTerminal("Ollama Server", "ollama", "serve");
    }

    /// <summary>Kill all managed processes and their child process trees.</summary>
    public void KillAll()
    {
        KillProcess(ref _coreAgentProcess);
        KillProcess(ref _ollamaProcess);
    }

    private static void KillProcess(ref Process? proc)
    {
        if (proc is null) return;
        try
        {
            if (!proc.HasExited)
                proc.Kill(entireProcessTree: true);
        }
        catch { /* already exited */ }
        finally
        {
            proc.Dispose();
            proc = null;
        }
    }

    public void Dispose()
    {
        KillAll();
        GC.SuppressFinalize(this);
    }
}
