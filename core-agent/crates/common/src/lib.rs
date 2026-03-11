/// Shared error type for the Lapis Sapientiae core agent.
///
/// All modules return `Result<T, LapisError>` so that errors propagate
/// uniformly through the orchestrator and can be logged consistently.
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub enum LapisError {
    Config(String),
    Ipc(String),
    Planner(String),
    Executor(String),
    Perception(String),
    Action(String),
    System(String),
    Logging(String),
    Orchestrator(String),
}

impl fmt::Display for LapisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LapisError::Config(msg) => write!(f, "[config] {msg}"),
            LapisError::Ipc(msg) => write!(f, "[ipc] {msg}"),
            LapisError::Planner(msg) => write!(f, "[planner] {msg}"),
            LapisError::Executor(msg) => write!(f, "[executor] {msg}"),
            LapisError::Perception(msg) => write!(f, "[perception] {msg}"),
            LapisError::Action(msg) => write!(f, "[action] {msg}"),
            LapisError::System(msg) => write!(f, "[system] {msg}"),
            LapisError::Logging(msg) => write!(f, "[logging] {msg}"),
            LapisError::Orchestrator(msg) => write!(f, "[orchestrator] {msg}"),
        }
    }
}

impl std::error::Error for LapisError {}

/// Convenience alias used across all modules.
pub type LapisResult<T> = Result<T, LapisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_includes_module_tag() {
        let err = LapisError::Config("missing file".into());
        assert_eq!(err.to_string(), "[config] missing file");
    }

    #[test]
    fn error_is_debug_printable() {
        let err = LapisError::Ipc("connection refused".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("connection refused"));
    }
}
