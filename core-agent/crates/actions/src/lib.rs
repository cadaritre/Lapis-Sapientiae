/// Actions module — defines the trait and types for all system actions.
///
/// Each action supports simulated and real execution modes.
use common::LapisResult;
use std::collections::HashMap;

/// The type of action to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    MouseClick,
    MouseMove,
    MouseDrag,
    KeyboardType,
    KeyboardPress,
    KeyboardCombo,
    WindowFocus,
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    FileOpen,
    SystemLaunch,
}

/// Parameters for an action, stored as string key-value pairs.
pub type ActionParams = HashMap<String, String>;

/// Result of executing an action.
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub simulated: bool,
    pub description: String,
}

/// Trait that all action handlers must implement.
pub trait Action {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult>;
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_type_is_cloneable() {
        let t = ActionType::MouseClick;
        let t2 = t.clone();
        assert_eq!(t, t2);
    }

    #[test]
    fn action_result_describes_simulation() {
        let r = ActionResult {
            success: true,
            simulated: true,
            description: "clicked at (100, 200)".into(),
        };
        assert!(r.simulated);
    }
}
