# Development Roadmap

Each phase is small, focused, and must be completed before starting the next. Every phase leaves the system in a compilable and runnable state.

---

## Phase 0 — Documentation and repository structure

**Objective:** Establish project documentation and directory layout.

**Deliverables:**
- `docs/README.md` — project vision, architecture overview, build instructions
- `docs/RULES.md` — mandatory development rules
- `docs/ROADMAP.md` — this file
- `docs/ARCHITECTURE.md` — detailed system architecture
- Complete directory tree with placeholder files

**Definition of done:**
- All four documents exist and are internally consistent.
- Directory structure matches ARCHITECTURE.md.

**Risks:** None.

**Dependencies:** None.

---

## Phase 1 — Core Agent skeleton (Rust)

**Objective:** Create the Rust workspace with all crate stubs that compile and run.

**Deliverables:**
- `core-agent/Cargo.toml` — workspace manifest listing all crates
- One `Cargo.toml` + `src/lib.rs` per crate (orchestrator, planner, executor, perception, actions, system-info, logging, config, ipc)
- `core-agent/src/main.rs` — binary entry point that prints a startup message
- Basic error type shared across crates

**Definition of done:**
- `cargo build` succeeds with zero warnings.
- `cargo run` prints a startup banner.
- `cargo test` passes (trivial tests).

**Risks:**
- Workspace configuration issues across crates.

**Dependencies:** Phase 0.

---

## Phase 2 — GUI skeleton (C# Avalonia)

**Objective:** Create the Avalonia desktop application with a basic window.

**Deliverables:**
- `desktop-gui/` — Avalonia project with MVVM structure
- Main window with placeholder chat panel and log panel
- Project compiles and launches

**Definition of done:**
- `dotnet build` succeeds.
- `dotnet run` opens a window with placeholder UI elements.

**Risks:**
- Avalonia SDK version compatibility.

**Dependencies:** Phase 0.

---

## Phase 3 — IPC between GUI and Core

**Objective:** Establish bidirectional JSON-RPC communication between the two processes.

**Deliverables:**
- Core: IPC server (named pipe or local TCP) that accepts connections and echoes messages
- GUI: IPC client that connects and sends/receives JSON-RPC messages
- Shared message schema (request/response types documented)
- Integration test that starts both processes and exchanges a ping/pong

**Definition of done:**
- GUI sends a `ping` request; Core responds with `pong`.
- Connection loss is handled gracefully on both sides.

**Risks:**
- Platform-specific pipe behavior (Windows vs. Unix).
- Serialization mismatches between Rust and C#.

**Dependencies:** Phase 1, Phase 2.

---

## Phase 4 — Basic agent loop (simulation mode)

**Objective:** Implement the core orchestration loop that receives a user message, creates a plan, and simulates execution.

**Deliverables:**
- Orchestrator: main loop (receive instruction → plan → execute → report)
- Planner: stub that returns a hardcoded plan for any input
- Executor: stub that logs each plan step as "simulated"
- Logging: structured JSON log output for each step

**Definition of done:**
- User sends a message via GUI.
- Core creates a plan, simulates each step, logs results.
- GUI displays the plan and simulated results.

**Risks:**
- Over-engineering the loop before real actions exist.

**Dependencies:** Phase 3.

---

## Phase 5 — Simulated actions system

**Objective:** Define the action vocabulary and implement all actions in simulation mode.

**Deliverables:**
- Action trait/interface with `execute_simulated()` and `execute_real()` methods
- Mouse actions: move, click, double-click, drag (simulated)
- Keyboard actions: type text, press key, key combo (simulated)
- Window actions: focus, minimize, maximize, close (simulated)
- File actions: open, read, write (simulated)
- Each action logs its parameters and simulated outcome

**Definition of done:**
- All action types are defined and simulated.
- Executor can dispatch plan steps to the correct action handler.
- Full audit trail in logs.

**Risks:**
- Action vocabulary too broad or too narrow; start minimal and expand.

**Dependencies:** Phase 4.

---

## Phase 6 — Basic perception (screenshots)

**Objective:** Capture screenshots and send them to the GUI for display.

**Deliverables:**
- Perception module captures the screen (or a window) as PNG
- Screenshots are sent to the GUI via IPC
- GUI displays the latest screenshot in the viewer panel
- Screenshots are associated with plan steps in the log

**Definition of done:**
- Core captures a screenshot on demand.
- GUI displays it within 1 second.
- Screenshots are logged with timestamps.

**Risks:**
- Platform-specific screen capture APIs.
- Large image transfer over IPC.

**Dependencies:** Phase 5.

---

## Phase 7 — Planner integration

**Objective:** Connect the planner to an LLM for real task decomposition.

**Deliverables:**
- Config module supports API key and model selection
- Planner sends user instructions + current screenshot to the LLM
- Planner receives structured plan (list of steps with action types)
- Plan is validated before execution
- Fallback to hardcoded plan if LLM is unavailable

**Definition of done:**
- User gives a natural language instruction.
- Planner produces a structured, multi-step plan via LLM.
- Plan is displayed in GUI and logged.

**Risks:**
- LLM response format variability.
- API rate limits and latency.

**Dependencies:** Phase 6.

---

## Phase 8 — Visual executor integration

**Objective:** Executor uses perception to verify each step before and after execution.

**Deliverables:**
- Executor takes a screenshot before each action
- Executor takes a screenshot after each action
- Executor compares expected vs. actual state (basic heuristic or LLM-based)
- If a step fails, executor reports to planner for re-planning

**Definition of done:**
- Each simulated action is bracketed by before/after screenshots.
- Executor detects at least one type of failure (e.g., expected window not found).

**Risks:**
- UI state comparison is inherently fragile.

**Dependencies:** Phase 7.

---

## Phase 9 — Real system actions

**Objective:** Enable real mouse, keyboard, and system actions (opt-in, behind safety gates).

**Deliverables:**
- Actions module: `execute_real()` implementations for all action types
- Safety gate: user must explicitly confirm before real execution begins
- Kill switch: user can abort execution at any time from GUI
- Rate limiting on actions to prevent runaway execution

**Definition of done:**
- Agent can perform a simple real task (e.g., open Notepad and type text).
- User can stop execution mid-task.
- All actions are logged with full audit trail.

**Risks:**
- Unintended system interaction.
- Safety and control must be rock-solid before enabling this phase.

**Dependencies:** Phase 8.

---

## Phase 10 — Advanced logging, error handling, and robustness

**Objective:** Harden the system for reliable daily use.

**Deliverables:**
- Log viewer in GUI with filtering, search, and export
- Retry logic for transient failures (network, IPC)
- Graceful degradation when LLM is unavailable
- Crash recovery: Core and GUI can reconnect after restart
- Configuration validation on startup
- Performance metrics (action latency, plan duration)

**Definition of done:**
- System recovers gracefully from Core crash, GUI crash, and network loss.
- Logs can be searched and exported.
- No unhandled panics or unhandled exceptions in normal operation.

**Risks:**
- Edge cases in crash recovery.

**Dependencies:** Phase 9.
