# Lapis Sapientiae

An AI-powered desktop agent that interprets natural language instructions and executes tasks on the user's computer.

## Vision

Lapis Sapientiae enables users to control their desktop through conversation. The user describes what they want done — open an application, fill out a form, navigate a website, automate a workflow — and the agent reasons about the task, perceives the screen, and executes the necessary actions.

The system prioritizes safety through a mandatory simulation mode, auditability of every action, and a strict separation between reasoning and execution.

## Architecture overview

The system consists of two independent applications that communicate over IPC:

```
┌─────────────────┐       JSON-RPC / IPC        ┌──────────────────┐
│   CORE AGENT    │◄──────────────────────────►  │   DESKTOP GUI    │
│     (Rust)      │   named pipes / local TCP    │  (C# / Avalonia) │
└─────────────────┘                              └──────────────────┘
```

### Core Agent (Rust)

Handles all intelligence and system interaction:

- **Orchestrator** — main loop, lifecycle management
- **Planner** — interprets user intent, decomposes goals into subtasks
- **Executor** — carries out individual actions based on plan steps
- **Perception** — captures and analyzes screenshots, detects UI state
- **Actions** — mouse, keyboard, window management, file operations
- **System-info** — OS queries, process listing, environment data
- **Logging** — structured, auditable logging of every decision and action
- **Config** — runtime configuration, model settings, safety parameters
- **IPC** — JSON-RPC server for communication with the GUI

### Desktop GUI (C# / Avalonia)

Provides the user interface:

- Chat panel for natural language interaction
- Live screenshot viewer
- Task history and execution log
- Start / stop / pause controls
- System configuration panel

The GUI **never** executes system actions directly. Every action flows through the Core Agent.

## Project structure

```
lapis-sapientiae/
├── core-agent/            # Rust workspace
│   ├── Cargo.toml         # workspace manifest
│   ├── src/               # binary entry point
│   └── crates/
│       ├── orchestrator/
│       ├── planner/
│       ├── executor/
│       ├── perception/
│       ├── actions/
│       ├── system-info/
│       ├── logging/
│       ├── config/
│       └── ipc/
├── desktop-gui/           # C# Avalonia application
│   └── src/
│       ├── Views/
│       ├── ViewModels/
│       ├── Services/
│       ├── IpcClient/
│       └── State/
└── docs/
    ├── README.md
    ├── RULES.md
    ├── ROADMAP.md
    └── ARCHITECTURE.md
```

## Building

### Core Agent

Prerequisites: Rust toolchain (stable).

```bash
cd core-agent
cargo build
```

### Desktop GUI

Prerequisites: .NET 8 SDK.

```bash
cd desktop-gui
dotnet build
```

## Running

**Phase 0–5 operate in simulation mode only.** No real system actions are executed until Phase 9.

```bash
# Start the core agent
cd core-agent && cargo run

# In a separate terminal, start the GUI
cd desktop-gui && dotnet run
```

The GUI connects to the Core Agent over local IPC automatically.

## Documentation

| Document | Purpose |
|---|---|
| [RULES.md](RULES.md) | Mandatory development rules |
| [ROADMAP.md](ROADMAP.md) | Phased development plan |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Detailed system architecture |
