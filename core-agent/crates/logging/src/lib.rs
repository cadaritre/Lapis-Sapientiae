/// Structured logging module for the Lapis Sapientiae agent.
///
/// All agent decisions and actions are logged here for auditability.
use common::LapisResult;

/// Initializes the logging subsystem.
pub fn init() -> LapisResult<()> {
    // Phase 1: stub — real implementation in Phase 4+
    Ok(())
}

/// Logs a structured event.
pub fn log_event(module: &str, event: &str, data: &str) -> LapisResult<()> {
    // Phase 1: simple stderr output
    eprintln!("[{module}] {event}: {data}");
    Ok(())
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
}
