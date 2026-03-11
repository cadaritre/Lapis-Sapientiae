/// System information module — read-only queries about the OS.
///
/// This module never modifies system state.
use common::LapisResult;

/// Returns the name of the current operating system.
pub fn os_name() -> LapisResult<String> {
    Ok(std::env::consts::OS.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_name_is_not_empty() {
        let name = os_name().unwrap();
        assert!(!name.is_empty());
    }
}
