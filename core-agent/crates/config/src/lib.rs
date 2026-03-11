/// Configuration module for the Lapis Sapientiae agent.
///
/// Loads runtime settings from file and environment.
use common::LapisResult;

/// Runtime configuration for the agent.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Whether actions are simulated (true) or real (false).
    pub simulation_mode: bool,
    /// IPC transport: "pipe" or "tcp".
    pub ipc_transport: String,
    /// TCP port used when ipc_transport is "tcp".
    pub ipc_port: u16,
    /// Logging level: "debug", "info", "warn", "error".
    pub log_level: String,
    /// VLM endpoint URL (e.g. Ollama).
    pub vlm_endpoint: String,
    /// VLM model name (e.g. "llava:latest").
    pub vlm_model: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            simulation_mode: true,
            ipc_transport: "tcp".into(),
            ipc_port: 9100,
            log_level: "info".into(),
            vlm_endpoint: "http://localhost:11434".into(),
            vlm_model: "moondream".into(),
        }
    }
}

/// Loads configuration. Phase 1: returns defaults only.
pub fn load() -> LapisResult<AppConfig> {
    Ok(AppConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_simulation_mode() {
        let cfg = AppConfig::default();
        assert!(cfg.simulation_mode);
    }

    #[test]
    fn load_returns_defaults() {
        let cfg = load().unwrap();
        assert_eq!(cfg.ipc_port, 9100);
        assert_eq!(cfg.vlm_model, "moondream");
    }
}
