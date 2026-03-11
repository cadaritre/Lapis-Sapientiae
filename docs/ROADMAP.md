# Development Roadmap

Each phase is small, focused, and must be completed before starting the next. Every phase leaves the system in a compilable and runnable state.

---

## Phase 0 — Documentation and repository structure ✓

**Status: COMPLETE**

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

## Phase 1 — Core Agent skeleton (Rust) ✓

**Status: COMPLETE**

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

## Phase 2 — GUI skeleton (C# Avalonia) ✓

**Status: COMPLETE**

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

## Phase 3 — IPC between GUI and Core ✓

**Status: COMPLETE**

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

## Phase 4 — Basic agent loop (simulation mode) ✓

**Status: COMPLETE**

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

## Phase 5 — Simulated actions system ✓

**Status: COMPLETE**

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

## Phase 6 — Perception + VLM integration ✓

**Status: COMPLETE**

**Objective:** Capture screenshots, send to GUI, and analyze via local VLM.

**Deliverables:**
- Perception module captures the screen via xcap, auto-downscales to 1080p max
- Screenshots sent to GUI via IPC as base64 PNG
- GUI displays the latest screenshot in a tabbed right panel
- VLM analysis via Ollama API (moondream model)
- Embedded terminal panels in GUI for Core Agent and Ollama output
- Process manager with stdout/stderr capture, external Ollama detection
- Service status indicators (green/red dots) in sidebar
- Restart/stop controls for Core Agent and Ollama

**Definition of done:**
- Core captures screenshots on demand (downscaled to 1080p). ✓
- GUI displays screenshots within 1 second. ✓
- VLM describes screen content via moondream model. ✓
- All background processes visible in GUI terminals. ✓

**Risks:**
- Platform-specific screen capture APIs.
- VLM processing time (addressed with 300s timeout).

**Dependencies:** Phase 5.

---

## Phase 7 — LLM reasoning integration ✓

**Status: COMPLETE**

**Objective:** Connect the planner to a cloud LLM for intelligent task decomposition.

**Deliverables:**
- Config module supports reasoning provider, API key, model, and endpoint
- Multi-provider support: Claude, OpenAI, Gemini, and custom (OpenAI-compatible)
- Planner sends user instructions + optional screen context to the LLM
- LLM returns structured JSON plan with action types, parameters, descriptions
- Automatic keyword-based fallback if LLM is unavailable or API key not set
- `agent.configure_reasoning` IPC method for runtime config from GUI
- GUI Settings panel with provider selection, API key input, and model auto-mapping
- Reasoning config sent to Core Agent on connection and settings save

**Definition of done:**
- User enters API key in GUI settings. ✓
- Planner sends instruction to Claude API and receives structured plan. ✓
- Plan steps are executed and displayed in GUI chat. ✓
- Falls back to keyword planner if no API key configured. ✓

**Risks:**
- LLM response format variability (mitigated with JSON parsing + markdown stripping).
- API rate limits and latency (60s timeout per request).

**Dependencies:** Phase 6.

---

## Phase 8 — Visual executor integration ✓

**Status: COMPLETE**

**Objective:** Executor uses perception to verify each step before and after execution.

**Deliverables:**
- `execute_step_with_perception()` captures before/after screenshots for each step
- VLM-based verification: after each action, analyzes screen to confirm expected outcome
- `VerificationResult` struct with `matches_expected`, `description`, `model`
- Orchestrator reports verification status in step notifications (VERIFIED/MISMATCH)
- Tracks verified and failed step counts in execution summary
- Backward-compatible `execute_step()` preserved for simple execution

**Definition of done:**
- Each action is bracketed by before/after screenshots. ✓
- VLM analyzes screen after action to verify expected outcome. ✓
- Verification results included in step notifications to GUI. ✓

**Risks:**
- VLM verification adds latency per step (mitigated: optional, only when VLM available).
- UI state comparison via VLM is heuristic-based (YES/NO parsing).

**Dependencies:** Phase 7.

---

## Phase 8.5 — Settings overhaul and theming ✓

**Status: COMPLETE**

**Objective:** Complete redesign of the settings panel with light/dark theme support, local/cloud model selection, and Ollama management.

**Deliverables:**
- Light/dark theme system via Avalonia `ThemeDictionaries` with named brush resources
- Default light mode with clean white design, blue (`#6C8AFF`) and orange (`#F0A050`) brand accents
- Dark mode with deep navy/charcoal palette, toggled via sun/moon switch in settings
- All hardcoded colors replaced with `{DynamicResource}` throughout the entire GUI
- Vision model settings: Local (Ollama) or Cloud (OpenAI/Gemini/Custom) with provider selector
- Reasoning model settings: Local (Ollama) or Cloud (Claude/OpenAI/Gemini/Custom)
- Ollama model pull terminal: type model name, click Pull, see real-time download progress
- Ollama installation detection: orange warning banner when Ollama is not installed
- API key fields with visibility toggle for both vision and reasoning cloud providers
- Settings save sends appropriate config to Core Agent based on local/cloud selection

**Definition of done:**
- App opens in light mode by default. ✓
- Toggle switches entire UI to dark mode without restart. ✓
- Local/Cloud radio toggles show/hide relevant fields. ✓
- Pull terminal runs `ollama pull <model>` with live output. ✓
- Ollama detection shows warning when not installed. ✓
- Settings save routes config correctly for local vs cloud. ✓

**Risks:**
- None significant; purely UI-side changes.

**Dependencies:** Phase 8.

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
