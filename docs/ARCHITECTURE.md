# System Architecture

## Overview

Lapis Sapientiae is composed of two independent processes that communicate over IPC:

```
┌─────────────────────────────────────────────────┐
│                  CORE AGENT (Rust)               │
│                                                  │
│  ┌─────────────┐   ┌──────────┐   ┌──────────┐  │
│  │ Orchestrator │──►│ Planner  │──►│ Executor │  │
│  └──────┬──────┘   └────┬─────┘   └────┬─────┘  │
│         │               │              │         │
│         │          ┌─────▼─────┐  ┌─────▼─────┐  │
│         │          │ Perception│  │  Actions   │  │
│         │          └───────────┘  └───────────┘  │
│         │                                        │
│  ┌──────▼──────┐  ┌─────────┐  ┌──────────────┐ │
│  │   Logging   │  │ Config  │  │ System-info  │ │
│  └─────────────┘  └─────────┘  └──────────────┘ │
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │            IPC Server (JSON-RPC)          │    │
│  └──────────────────┬───────────────────────┘    │
└─────────────────────┼───────────────────────────-┘
                      │  named pipe / local TCP
┌─────────────────────┼────────────────────────────┐
│  ┌──────────────────▼───────────────────────┐    │
│  │           IPC Client (JSON-RPC)           │    │
│  └──────────────────────────────────────────┘    │
│                                                  │
│  ┌──────────┐  ┌────────────┐  ┌─────────────┐  │
│  │  Views   │  │ ViewModels │  │  Services   │  │
│  └──────────┘  └────────────┘  └─────────────┘  │
│                                                  │
│  ┌──────────┐                                    │
│  │  State   │                                    │
│  └──────────┘                                    │
│                                                  │
│               DESKTOP GUI (C# / Avalonia)        │
└──────────────────────────────────────────────────┘
```

---

## Core Agent modules

### Orchestrator

The main loop of the agent. Responsibilities:

- Receive user instructions from the IPC server.
- Pass instructions to the Planner.
- Feed the plan to the Executor step by step.
- Collect results and send them back to the GUI.
- Manage agent lifecycle (start, pause, stop, abort).

The orchestrator owns the control flow. It is the only module that coordinates between Planner, Executor, and Perception.

### Planner

Interprets user intent and produces a structured plan.

- Input: user instruction (text) + optional screenshot (current state).
- Output: ordered list of `PlanStep` structs.
- In early phases, returns hardcoded plans.
- In Phase 7+, calls an LLM to generate plans.
- Supports re-planning when the executor reports a step failure.

```
PlanStep {
    id: u32,
    action_type: ActionType,
    parameters: HashMap<String, Value>,
    description: String,
    expected_outcome: String,
}
```

### Executor

Executes a single plan step.

- Receives a `PlanStep` from the orchestrator.
- Dispatches to the appropriate action handler.
- In simulation mode: logs what *would* happen.
- In real mode: performs the action, then verifies the result via perception.
- Reports success or failure back to the orchestrator.

### Perception

Captures and analyzes the visual state of the desktop.

- Captures screenshots (full screen or specific window).
- Encodes screenshots as PNG for transfer.
- In later phases: sends screenshots to an LLM for UI element detection.
- Provides the planner and executor with structured state information.

### Actions

Concrete system interactions. Each action implements a common trait:

```rust
pub trait Action {
    fn execute_simulated(&self, params: &ActionParams) -> Result<ActionResult>;
    fn execute_real(&self, params: &ActionParams) -> Result<ActionResult>;
}
```

Action categories:

| Category | Examples |
|---|---|
| Mouse | move, click, double-click, right-click, drag, scroll |
| Keyboard | type text, press key, key combination |
| Window | focus, minimize, maximize, close, resize |
| File | open, read contents, write, list directory |
| System | launch application, list processes |

### System-info

Read-only queries about the operating system:

- List running processes.
- Get active window title.
- Query display resolution.
- Read environment variables.

This module **never** modifies system state.

### Logging

Structured logging for auditability.

- Every action, plan step, and decision is logged.
- Log format: JSON lines (one JSON object per line).
- Fields: `timestamp`, `level`, `module`, `event`, `data`.
- Logs are written to file and streamed to the GUI via IPC.

### Config

Runtime configuration loaded at startup.

- Simulation mode toggle (default: on).
- Vision model: endpoint and model name (default: Ollama `moondream`).
- Reasoning provider, API key, model, and endpoint (supports Claude, OpenAI, Gemini, custom).
- IPC transport: local TCP on port 9100.
- Logging level and output path.
- Action rate limits.

Config can be updated at runtime via `agent.configure` and `agent.configure_reasoning` IPC methods.

### IPC

JSON-RPC 2.0 server.

- Transport: named pipes (primary) or local TCP (fallback).
- Handles connection lifecycle (accept, reconnect, timeout).
- Serializes/deserializes all messages as JSON.
- Supports both request/response and notification (server → client) patterns.

---

## Desktop GUI modules

### Views

Avalonia XAML views:

- **MainWindow** — shell with sidebar, chat panel, settings overlay, and tabbed right panel.
- All colors use `{DynamicResource}` keys from `App.axaml` ThemeDictionaries for theme switching.
- Light/dark theme controlled via `Application.Current.RequestedThemeVariant`.

### ViewModels

MVVM view models (CommunityToolkit.Mvvm):

- **MainWindowViewModel** — coordinates panels, manages process lifecycle, theme toggle (`IsDarkMode`), settings open/save.
- **SettingsViewModel** — vision local/cloud toggle, reasoning local/cloud toggle, Ollama model pull commands, Ollama detection, API key management.

### Services

- **AgentService** — JSON-RPC client over TCP port 9100. Methods: `agent.ping`, `agent.instruct`, `agent.configure`, `agent.configure_reasoning`, `agent.screenshot`, `agent.analyze_screen`.
- **ProcessManager** — manages Ollama and Core Agent child processes with stdout/stderr capture and lifecycle events.

---

## Communication flows

### User sends an instruction

```
User types message in ChatView
  → ChatViewModel.SendMessage()
    → AgentService.SendInstruction(text)
      → IpcClient sends JSON-RPC request: { method: "agent.instruct", params: { text: "..." } }
        → Core IPC server receives request
          → Orchestrator.handle_instruction(text)
            → Planner.create_plan(text, screenshot?)
              ← returns Vec<PlanStep>
            → for each step:
                Executor.execute(step, simulation_mode)
                  → Action.execute_simulated(params)
                  ← ActionResult
                Logging.log(step, result)
                IPC notification → GUI: { method: "agent.step_completed", params: { step, result } }
            ← final result
          → IPC response → GUI
        → AgentService receives response
      → ChatViewModel displays result
```

### Screenshot capture flow

```
Orchestrator (or Executor) requests screenshot
  → Perception.capture_screen()
    ← PNG bytes
  → IPC notification → GUI: { method: "agent.screenshot", params: { image: base64 } }
    → ScreenshotViewModel.UpdateImage(bytes)
```

---

## Error handling strategy

- All modules return `Result<T, LapisError>` in Rust.
- `LapisError` is an enum with variants per module (e.g., `Ipc(IpcError)`, `Action(ActionError)`).
- Errors propagate upward to the orchestrator, which decides whether to retry, re-plan, or abort.
- All errors are logged before being returned.
- The GUI displays errors to the user with enough context to understand what happened.

---

## Simulation mode

Simulation mode is a **first-class concept**, not an afterthought.

- Every action has a simulated implementation that logs its intent without side effects.
- The executor checks the global simulation flag before dispatching.
- Simulation results follow the same data structures as real results.
- The GUI indicates clearly whether the system is in simulation or real mode.
- Transitioning from simulation to real mode requires explicit user action in the settings.

---

## Security considerations

- The agent never executes actions without a plan that can be reviewed.
- Real mode requires explicit opt-in.
- Kill switch: the GUI can send an `agent.abort` request at any time.
- Rate limiting prevents runaway action loops.
- All actions are logged for post-hoc review.
- The system does not store or transmit credentials (API keys are loaded from config, never logged).
