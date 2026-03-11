# Development Rules

These rules are **mandatory** for every contributor and every phase of development. No exceptions.

## 1. Modular architecture

- Every logical concern lives in its own module (Rust crate or C# project/namespace).
- Modules communicate through well-defined interfaces, never by reaching into internals.
- No circular dependencies between modules.

## 2. Separation of concerns

- **Planner** and **Executor** are always separate. The planner decides *what* to do; the executor decides *how* to do it.
- **GUI** and **Core** are always separate processes. The GUI never executes system actions directly.
- **Perception** is its own module. Analysis of screen state does not live inside the executor or actions.

## 3. Phase-based development

- The project is built in strict, sequential phases (see ROADMAP.md).
- Each phase has a clear objective, a small scope, and a definition of done.
- **Never** implement features from a future phase. If an idea belongs to a later phase, document it under "Postergado para fases futuras" and move on.
- Every phase must leave the system in a compilable, runnable state.

## 4. Simulation before reality

- All action modules must support a **simulation mode** that logs what *would* happen without touching the real system.
- Simulation mode must be the default. Real execution is opt-in and gated behind explicit configuration.
- Phases 0 through 8 operate exclusively in simulation mode.

## 5. Auditability

- Every action the agent takes (or simulates) must be logged with:
  - timestamp
  - action type
  - parameters
  - result or simulated result
  - source plan step
- Logs must be structured (JSON) and human-readable.
- The user must be able to review the full trace of any task after the fact.

## 6. Naming conventions

- Rust: `snake_case` for functions, variables, modules; `PascalCase` for types and traits.
- C#: `PascalCase` for public members; `camelCase` for private fields prefixed with `_`.
- File and folder names: `kebab-case` for Rust crates, `PascalCase` for C# folders.
- All names must be descriptive. No abbreviations unless universally understood (`IPC`, `JSON`, `UI`).

## 7. Dependency discipline

- Add a dependency only when it provides clear, significant value.
- Prefer standard library solutions over external crates/packages.
- Every new dependency must be justified in the commit message or PR description.
- Pin dependency versions explicitly.

## 8. Error handling

- Never use `unwrap()` or `expect()` in production Rust code (test code is acceptable).
- All errors must propagate through `Result<T, E>` with meaningful error types.
- C# code must use structured exception handling; no bare `catch (Exception)` blocks that swallow errors.
- Errors must be logged before being returned or re-thrown.

## 9. No premature optimization

- Write clear, correct code first.
- Optimize only when measurement shows a real bottleneck.
- Simplicity wins over cleverness in every case.

## 10. Communication protocol

- Core and GUI communicate exclusively via JSON-RPC over named pipes or local TCP.
- All messages are serialized as JSON.
- The protocol must be versioned from the start (field: `"jsonrpc": "2.0"`).
- Every request type and response type must be documented.

## 11. Testing

- Every module must have unit tests for its core logic.
- Integration tests cover the boundaries between modules.
- IPC communication must have dedicated integration tests.
- Tests must pass before any phase is considered complete.

## 12. Documentation

- ARCHITECTURE.md, ROADMAP.md, and RULES.md must stay up to date as the system evolves.
- Public APIs (functions, traits, interfaces) must have doc comments.
- Complex logic must have inline comments explaining *why*, not *what*.

## The golden rule

> When in doubt between complexity and simplicity, **choose simplicity**.
