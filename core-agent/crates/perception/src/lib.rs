/// Perception module — captures and analyzes visual state of the desktop.
///
/// Real screenshot capture is implemented in Phase 6.
use common::LapisResult;

/// Placeholder for a captured screenshot.
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Captures a screenshot. Phase 1: returns a stub.
pub fn capture_screen() -> LapisResult<Screenshot> {
    Ok(Screenshot {
        width: 0,
        height: 0,
        data: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_returns_stub() {
        let s = capture_screen().unwrap();
        assert_eq!(s.width, 0);
        assert!(s.data.is_empty());
    }
}
