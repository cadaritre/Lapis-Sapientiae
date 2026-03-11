/// IPC server module for JSON-RPC communication with the GUI.
use common::LapisResult;

/// Placeholder for the IPC server. Real implementation in Phase 3.
pub struct IpcServer;

impl IpcServer {
    pub fn new() -> LapisResult<Self> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_creates_successfully() {
        assert!(IpcServer::new().is_ok());
    }
}
