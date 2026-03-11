/// Structured logging module for the Lapis Sapientiae agent.
///
/// All agent decisions and actions are logged as structured JSON for auditability.
use common::LapisResult;
use serde::Serialize;
use std::time::SystemTime;

/// A structured log entry.
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub module: String,
    pub event: String,
    pub data: serde_json::Value,
}

/// Initializes the logging subsystem.
pub fn init() -> LapisResult<()> {
    Ok(())
}

/// Logs a structured event as JSON to stderr.
pub fn log_event(module: &str, event: &str, data: &str) -> LapisResult<()> {
    let entry = LogEntry {
        timestamp: iso_now(),
        module: module.to_string(),
        event: event.to_string(),
        data: serde_json::Value::String(data.to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap_or_else(|_| format!("{{\"error\":\"serialize failed\",\"module\":\"{module}\",\"event\":\"{event}\"}}"));
    eprintln!("{json}");
    Ok(())
}

/// Logs a structured event with arbitrary JSON data.
pub fn log_event_json(module: &str, event: &str, data: serde_json::Value) -> LapisResult<()> {
    let entry = LogEntry {
        timestamp: iso_now(),
        module: module.to_string(),
        event: event.to_string(),
        data,
    };
    let json = serde_json::to_string(&entry).unwrap_or_else(|_| format!("{{\"error\":\"serialize failed\",\"module\":\"{module}\",\"event\":\"{event}\"}}"));
    eprintln!("{json}");
    Ok(())
}

/// Returns the current time as an ISO 8601 string.
fn iso_now() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{secs}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_succeeds() {
        assert!(init().is_ok());
    }

    #[test]
    fn log_event_succeeds() {
        assert!(log_event("test", "test_event", "hello").is_ok());
    }

    #[test]
    fn log_event_json_succeeds() {
        assert!(log_event_json("test", "json_event", serde_json::json!({"key": "value"})).is_ok());
    }

    #[test]
    fn iso_now_returns_nonempty() {
        let ts = iso_now();
        assert!(!ts.is_empty());
    }
}
