/// Actions module — defines the trait, types, and handlers for all system actions.
///
/// Each action supports simulated and real execution modes.
/// Phase 5: all handlers implemented in simulation mode with detailed logging.
use common::{LapisError, LapisResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of action to perform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Dispatches an action type to its handler and executes it.
pub fn dispatch(
    action_type: &ActionType,
    params: &ActionParams,
    simulation: bool,
) -> LapisResult<ActionResult> {
    let handler: Box<dyn Action> = match action_type {
        ActionType::MouseClick => Box::new(MouseClickAction),
        ActionType::MouseMove => Box::new(MouseMoveAction),
        ActionType::MouseDrag => Box::new(MouseDragAction),
        ActionType::KeyboardType => Box::new(KeyboardTypeAction),
        ActionType::KeyboardPress => Box::new(KeyboardPressAction),
        ActionType::KeyboardCombo => Box::new(KeyboardComboAction),
        ActionType::WindowFocus => Box::new(WindowFocusAction),
        ActionType::WindowMinimize => Box::new(WindowMinimizeAction),
        ActionType::WindowMaximize => Box::new(WindowMaximizeAction),
        ActionType::WindowClose => Box::new(WindowCloseAction),
        ActionType::FileOpen => Box::new(FileOpenAction),
        ActionType::SystemLaunch => Box::new(SystemLaunchAction),
    };

    if simulation {
        handler.execute_simulated(params)
    } else {
        handler.execute_real(params)
    }
}

// ── Helper ──

fn param(params: &ActionParams, key: &str) -> String {
    params.get(key).cloned().unwrap_or_default()
}

fn sim_result(desc: String) -> ActionResult {
    ActionResult {
        success: true,
        simulated: true,
        description: desc,
    }
}

fn not_implemented(action: &str) -> LapisResult<ActionResult> {
    Err(LapisError::Action(format!(
        "Real execution not yet implemented for {action}"
    )))
}

// ── Mouse Actions ──

struct MouseClickAction;
impl Action for MouseClickAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let x = param(params, "x");
        let y = param(params, "y");
        let button = param(params, "button");
        let btn = if button.is_empty() { "left".to_string() } else { button };
        Ok(sim_result(format!(
            "SIM: {btn}-clicked at ({x}, {y})"
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("MouseClick")
    }
}

struct MouseMoveAction;
impl Action for MouseMoveAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let x = param(params, "x");
        let y = param(params, "y");
        Ok(sim_result(format!("SIM: moved cursor to ({x}, {y})")))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("MouseMove")
    }
}

struct MouseDragAction;
impl Action for MouseDragAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let from_x = param(params, "from_x");
        let from_y = param(params, "from_y");
        let to_x = param(params, "to_x");
        let to_y = param(params, "to_y");
        Ok(sim_result(format!(
            "SIM: dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})"
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("MouseDrag")
    }
}

// ── Keyboard Actions ──

struct KeyboardTypeAction;
impl Action for KeyboardTypeAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let text = param(params, "text");
        let len = text.len();
        Ok(sim_result(format!(
            "SIM: typed {len} characters: \"{text}\""
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("KeyboardType")
    }
}

struct KeyboardPressAction;
impl Action for KeyboardPressAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let key = param(params, "key");
        Ok(sim_result(format!("SIM: pressed key [{key}]")))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("KeyboardPress")
    }
}

struct KeyboardComboAction;
impl Action for KeyboardComboAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let keys = param(params, "keys");
        Ok(sim_result(format!("SIM: pressed combo [{keys}]")))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("KeyboardCombo")
    }
}

// ── Window Actions ──

struct WindowFocusAction;
impl Action for WindowFocusAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let title = param(params, "window_title");
        Ok(sim_result(format!(
            "SIM: focused window \"{title}\""
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("WindowFocus")
    }
}

struct WindowMinimizeAction;
impl Action for WindowMinimizeAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let title = param(params, "window_title");
        Ok(sim_result(format!(
            "SIM: minimized window \"{title}\""
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("WindowMinimize")
    }
}

struct WindowMaximizeAction;
impl Action for WindowMaximizeAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let title = param(params, "window_title");
        Ok(sim_result(format!(
            "SIM: maximized window \"{title}\""
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("WindowMaximize")
    }
}

struct WindowCloseAction;
impl Action for WindowCloseAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let title = param(params, "window_title");
        Ok(sim_result(format!(
            "SIM: closed window \"{title}\""
        )))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("WindowClose")
    }
}

// ── File Actions ──

struct FileOpenAction;
impl Action for FileOpenAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let path = param(params, "path");
        Ok(sim_result(format!("SIM: opened file \"{path}\"")))
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("FileOpen")
    }
}

// ── System Actions ──

struct SystemLaunchAction;
impl Action for SystemLaunchAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let app = param(params, "application");
        if app.is_empty() {
            let instruction = param(params, "instruction");
            Ok(sim_result(format!(
                "SIM: processed system instruction \"{instruction}\""
            )))
        } else {
            Ok(sim_result(format!(
                "SIM: launched application \"{app}\""
            )))
        }
    }
    fn execute_real(&self, _params: &ActionParams) -> LapisResult<ActionResult> {
        not_implemented("SystemLaunch")
    }
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
    fn dispatch_mouse_click_simulated() {
        let mut params = ActionParams::new();
        params.insert("x".into(), "100".into());
        params.insert("y".into(), "200".into());
        params.insert("button".into(), "left".into());
        let result = dispatch(&ActionType::MouseClick, &params, true).unwrap();
        assert!(result.simulated);
        assert!(result.description.contains("left-clicked"));
        assert!(result.description.contains("100"));
    }

    #[test]
    fn dispatch_mouse_move_simulated() {
        let mut params = ActionParams::new();
        params.insert("x".into(), "50".into());
        params.insert("y".into(), "75".into());
        let result = dispatch(&ActionType::MouseMove, &params, true).unwrap();
        assert!(result.description.contains("moved cursor"));
    }

    #[test]
    fn dispatch_mouse_drag_simulated() {
        let mut params = ActionParams::new();
        params.insert("from_x".into(), "0".into());
        params.insert("from_y".into(), "0".into());
        params.insert("to_x".into(), "100".into());
        params.insert("to_y".into(), "100".into());
        let result = dispatch(&ActionType::MouseDrag, &params, true).unwrap();
        assert!(result.description.contains("dragged"));
    }

    #[test]
    fn dispatch_keyboard_type_simulated() {
        let mut params = ActionParams::new();
        params.insert("text".into(), "Hello world".into());
        let result = dispatch(&ActionType::KeyboardType, &params, true).unwrap();
        assert!(result.description.contains("11 characters"));
        assert!(result.description.contains("Hello world"));
    }

    #[test]
    fn dispatch_keyboard_press_simulated() {
        let mut params = ActionParams::new();
        params.insert("key".into(), "Enter".into());
        let result = dispatch(&ActionType::KeyboardPress, &params, true).unwrap();
        assert!(result.description.contains("Enter"));
    }

    #[test]
    fn dispatch_keyboard_combo_simulated() {
        let mut params = ActionParams::new();
        params.insert("keys".into(), "Ctrl+S".into());
        let result = dispatch(&ActionType::KeyboardCombo, &params, true).unwrap();
        assert!(result.description.contains("Ctrl+S"));
    }

    #[test]
    fn dispatch_window_focus_simulated() {
        let mut params = ActionParams::new();
        params.insert("window_title".into(), "Notepad".into());
        let result = dispatch(&ActionType::WindowFocus, &params, true).unwrap();
        assert!(result.description.contains("focused"));
        assert!(result.description.contains("Notepad"));
    }

    #[test]
    fn dispatch_window_close_simulated() {
        let mut params = ActionParams::new();
        params.insert("window_title".into(), "active".into());
        let result = dispatch(&ActionType::WindowClose, &params, true).unwrap();
        assert!(result.description.contains("closed"));
    }

    #[test]
    fn dispatch_system_launch_simulated() {
        let mut params = ActionParams::new();
        params.insert("application".into(), "notepad.exe".into());
        let result = dispatch(&ActionType::SystemLaunch, &params, true).unwrap();
        assert!(result.description.contains("launched"));
        assert!(result.description.contains("notepad.exe"));
    }

    #[test]
    fn dispatch_file_open_simulated() {
        let mut params = ActionParams::new();
        params.insert("path".into(), "C:\\test.txt".into());
        let result = dispatch(&ActionType::FileOpen, &params, true).unwrap();
        assert!(result.description.contains("opened file"));
    }

    #[test]
    fn real_execution_returns_error() {
        let params = ActionParams::new();
        let result = dispatch(&ActionType::MouseClick, &params, false);
        assert!(result.is_err());
    }

    #[test]
    fn all_action_types_dispatch() {
        let all = vec![
            ActionType::MouseClick, ActionType::MouseMove, ActionType::MouseDrag,
            ActionType::KeyboardType, ActionType::KeyboardPress, ActionType::KeyboardCombo,
            ActionType::WindowFocus, ActionType::WindowMinimize, ActionType::WindowMaximize,
            ActionType::WindowClose, ActionType::FileOpen, ActionType::SystemLaunch,
        ];
        for at in all {
            let result = dispatch(&at, &ActionParams::new(), true).unwrap();
            assert!(result.simulated);
            assert!(result.success);
        }
    }
}
