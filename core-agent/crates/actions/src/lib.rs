/// Actions module — defines the trait, types, and handlers for all system actions.
///
/// Each action supports simulated and real execution modes.
/// Phase 9: real execution via enigo (mouse/keyboard) and Win32 API (windows).
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

// ── Helpers ──

fn param(params: &ActionParams, key: &str) -> String {
    params.get(key).cloned().unwrap_or_default()
}

fn param_i32(params: &ActionParams, key: &str) -> LapisResult<i32> {
    let val = param(params, key);
    val.parse::<i32>().map_err(|_| {
        LapisError::Action(format!("Invalid integer for param '{key}': '{val}'"))
    })
}

fn sim_result(desc: String) -> ActionResult {
    ActionResult {
        success: true,
        simulated: true,
        description: desc,
    }
}

fn real_result(desc: String) -> ActionResult {
    ActionResult {
        success: true,
        simulated: false,
        description: desc,
    }
}

fn action_err(msg: String) -> LapisResult<ActionResult> {
    Err(LapisError::Action(msg))
}

/// Map a key name string to an enigo Key variant.
fn parse_key_name(name: &str) -> LapisResult<enigo::Key> {
    use enigo::Key;
    let lower = name.to_lowercase();
    match lower.as_str() {
        "enter" | "return" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "escape" | "esc" => Ok(Key::Escape),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "space" => Ok(Key::Space),
        "up" | "uparrow" => Ok(Key::UpArrow),
        "down" | "downarrow" => Ok(Key::DownArrow),
        "left" | "leftarrow" => Ok(Key::LeftArrow),
        "right" | "rightarrow" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "insert" => Ok(Key::Insert),
        "capslock" => Ok(Key::CapsLock),
        "numlock" => Ok(Key::Numlock),
        "printscreen" | "printscr" => Ok(Key::PrintScr),
        "pause" => Ok(Key::Pause),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        "ctrl" | "control" => Ok(Key::Control),
        "alt" => Ok(Key::Alt),
        "shift" => Ok(Key::Shift),
        "win" | "windows" | "super" | "meta" | "cmd" | "command" => Ok(Key::Meta),
        s if s.len() == 1 => Ok(Key::Unicode(s.chars().next().unwrap())),
        _ => Err(LapisError::Action(format!("Unknown key name: '{name}'"))),
    }
}

/// Map a button name to an enigo Button variant.
fn parse_button(name: &str) -> enigo::Button {
    match name.to_lowercase().as_str() {
        "right" => enigo::Button::Right,
        "middle" => enigo::Button::Middle,
        _ => enigo::Button::Left,
    }
}

/// Create an Enigo instance.
fn create_enigo() -> LapisResult<enigo::Enigo> {
    enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|e| LapisError::Action(format!("Failed to create input controller: {e}")))
}

// ── Win32 window helpers (Windows only) ──

#[cfg(windows)]
mod win32 {
    use common::{LapisError, LapisResult};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow,
        SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
    };

    /// Find a window by partial title match (case-insensitive).
    pub fn find_window_by_title(title: &str) -> LapisResult<HWND> {
        let search = title.to_lowercase();

        struct SearchData {
            search: String,
            found: HWND,
        }

        let mut data = SearchData {
            search,
            found: HWND::default(),
        };

        unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam.0 as *mut SearchData);
            if IsWindowVisible(hwnd).as_bool() {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut buf = vec![0u16; (len + 1) as usize];
                    GetWindowTextW(hwnd, &mut buf);
                    let window_title = String::from_utf16_lossy(&buf[..len as usize]);
                    if window_title.to_lowercase().contains(&data.search) {
                        data.found = hwnd;
                        return BOOL(0); // stop enumeration
                    }
                }
            }
            BOOL(1) // continue
        }

        unsafe {
            let _ = EnumWindows(
                Some(enum_callback),
                LPARAM(&mut data as *mut SearchData as isize),
            );
        }

        if data.found == HWND::default() {
            Err(LapisError::Action(format!(
                "Window not found: '{title}'"
            )))
        } else {
            Ok(data.found)
        }
    }

    pub fn focus_window(hwnd: HWND) -> LapisResult<()> {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            if !SetForegroundWindow(hwnd).as_bool() {
                return Err(LapisError::Action("SetForegroundWindow failed".into()));
            }
        }
        Ok(())
    }

    pub fn minimize_window(hwnd: HWND) -> LapisResult<()> {
        unsafe {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
        Ok(())
    }

    pub fn maximize_window(hwnd: HWND) -> LapisResult<()> {
        unsafe {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        }
        Ok(())
    }

    pub fn close_window(hwnd: HWND) -> LapisResult<()> {
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| LapisError::Action(format!("PostMessage WM_CLOSE failed: {e}")))?;
        }
        Ok(())
    }

    pub fn get_foreground_window() -> HWND {
        unsafe { GetForegroundWindow() }
    }

    pub fn shell_open(path: &str) -> LapisResult<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide_path: Vec<u16> = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_open: Vec<u16> = std::ffi::OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            use windows::Win32::UI::Shell::ShellExecuteW;
            let result = ShellExecuteW(
                HWND::default(),
                PCWSTR(wide_open.as_ptr()),
                PCWSTR(wide_path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            // ShellExecuteW returns > 32 on success
            if result.0 as usize <= 32 {
                return Err(LapisError::Action(format!(
                    "ShellExecuteW failed for '{path}' (code: {:?})",
                    result.0
                )));
            }
        }
        Ok(())
    }
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::{Coordinate, Direction, Mouse};
        let x = param_i32(params, "x")?;
        let y = param_i32(params, "y")?;
        let button = parse_button(&param(params, "button"));
        let btn_name = param(params, "button");
        let btn_label = if btn_name.is_empty() { "left" } else { &btn_name };

        let mut enigo = create_enigo()?;
        enigo.move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| LapisError::Action(format!("move_mouse failed: {e}")))?;
        enigo.button(button, Direction::Click)
            .map_err(|e| LapisError::Action(format!("button click failed: {e}")))?;

        Ok(real_result(format!("{btn_label}-clicked at ({x}, {y})")))
    }
}

struct MouseMoveAction;
impl Action for MouseMoveAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let x = param(params, "x");
        let y = param(params, "y");
        Ok(sim_result(format!("SIM: moved cursor to ({x}, {y})")))
    }
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::{Coordinate, Mouse};
        let x = param_i32(params, "x")?;
        let y = param_i32(params, "y")?;
        let mut enigo = create_enigo()?;
        enigo.move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| LapisError::Action(format!("move_mouse failed: {e}")))?;
        Ok(real_result(format!("moved cursor to ({x}, {y})")))
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::{Button, Coordinate, Direction, Mouse};
        let from_x = param_i32(params, "from_x")?;
        let from_y = param_i32(params, "from_y")?;
        let to_x = param_i32(params, "to_x")?;
        let to_y = param_i32(params, "to_y")?;

        let mut enigo = create_enigo()?;
        enigo.move_mouse(from_x, from_y, Coordinate::Abs)
            .map_err(|e| LapisError::Action(format!("move_mouse failed: {e}")))?;
        enigo.button(Button::Left, Direction::Press)
            .map_err(|e| LapisError::Action(format!("button press failed: {e}")))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        enigo.move_mouse(to_x, to_y, Coordinate::Abs)
            .map_err(|e| LapisError::Action(format!("move_mouse failed: {e}")))?;
        enigo.button(Button::Left, Direction::Release)
            .map_err(|e| LapisError::Action(format!("button release failed: {e}")))?;

        Ok(real_result(format!(
            "dragged from ({from_x}, {from_y}) to ({to_x}, {to_y})"
        )))
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::Keyboard;
        let text = param(params, "text");
        let len = text.len();
        let mut enigo = create_enigo()?;
        enigo.text(&text)
            .map_err(|e| LapisError::Action(format!("text input failed: {e}")))?;
        Ok(real_result(format!("typed {len} characters: \"{text}\"")))
    }
}

struct KeyboardPressAction;
impl Action for KeyboardPressAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let key = param(params, "key");
        Ok(sim_result(format!("SIM: pressed key [{key}]")))
    }
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::{Direction, Keyboard};
        let key_name = param(params, "key");
        let key = parse_key_name(&key_name)?;
        let mut enigo = create_enigo()?;
        enigo.key(key, Direction::Click)
            .map_err(|e| LapisError::Action(format!("key press failed: {e}")))?;
        Ok(real_result(format!("pressed key [{key_name}]")))
    }
}

struct KeyboardComboAction;
impl Action for KeyboardComboAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let keys = param(params, "keys");
        Ok(sim_result(format!("SIM: pressed combo [{keys}]")))
    }
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        use enigo::{Direction, Keyboard};
        let keys_str = param(params, "keys");
        let parts: Vec<&str> = keys_str.split('+').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return action_err("No keys specified for combo".into());
        }

        let mut enigo = create_enigo()?;
        let mut pressed_keys = Vec::new();

        // Press all modifiers, then click the last key, then release modifiers
        let (modifiers, main_key_str) = parts.split_at(parts.len() - 1);

        for modifier in modifiers {
            let key = parse_key_name(modifier)?;
            enigo.key(key, Direction::Press)
                .map_err(|e| LapisError::Action(format!("key press failed for '{modifier}': {e}")))?;
            pressed_keys.push(key);
        }

        let main_key = parse_key_name(main_key_str[0])?;
        enigo.key(main_key, Direction::Click)
            .map_err(|e| LapisError::Action(format!("key click failed: {e}")))?;

        // Release modifiers in reverse order
        for key in pressed_keys.iter().rev() {
            let _ = enigo.key(*key, Direction::Release);
        }

        Ok(real_result(format!("pressed combo [{keys_str}]")))
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        #[cfg(windows)]
        {
            let title = param(params, "window_title");
            let hwnd = win32::find_window_by_title(&title)?;
            win32::focus_window(hwnd)?;
            Ok(real_result(format!("focused window \"{title}\"")))
        }
        #[cfg(not(windows))]
        {
            let _ = params;
            action_err("Window actions are only supported on Windows".into())
        }
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        #[cfg(windows)]
        {
            let title = param(params, "window_title");
            let hwnd = win32::find_window_by_title(&title)?;
            win32::minimize_window(hwnd)?;
            Ok(real_result(format!("minimized window \"{title}\"")))
        }
        #[cfg(not(windows))]
        {
            let _ = params;
            action_err("Window actions are only supported on Windows".into())
        }
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        #[cfg(windows)]
        {
            let title = param(params, "window_title");
            let hwnd = win32::find_window_by_title(&title)?;
            win32::maximize_window(hwnd)?;
            Ok(real_result(format!("maximized window \"{title}\"")))
        }
        #[cfg(not(windows))]
        {
            let _ = params;
            action_err("Window actions are only supported on Windows".into())
        }
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        #[cfg(windows)]
        {
            let title = param(params, "window_title");
            let hwnd = if title == "active" || title.is_empty() {
                let fg = win32::get_foreground_window();
                if fg.0 == std::ptr::null_mut() {
                    return action_err("No active window found".into());
                }
                fg
            } else {
                win32::find_window_by_title(&title)?
            };
            win32::close_window(hwnd)?;
            Ok(real_result(format!("closed window \"{title}\"")))
        }
        #[cfg(not(windows))]
        {
            let _ = params;
            action_err("Window actions are only supported on Windows".into())
        }
    }
}

// ── File Actions ──

struct FileOpenAction;
impl Action for FileOpenAction {
    fn execute_simulated(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let path = param(params, "path");
        Ok(sim_result(format!("SIM: opened file \"{path}\"")))
    }
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let path = param(params, "path");
        if path.is_empty() {
            return action_err("Missing 'path' parameter for FileOpen".into());
        }
        #[cfg(windows)]
        {
            win32::shell_open(&path)?;
            Ok(real_result(format!("opened file \"{path}\"")))
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new("xdg-open")
                .arg(&path)
                .spawn()
                .map_err(|e| LapisError::Action(format!("Failed to open file: {e}")))?;
            Ok(real_result(format!("opened file \"{path}\"")))
        }
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
    fn execute_real(&self, params: &ActionParams) -> LapisResult<ActionResult> {
        let mut app = param(params, "application");

        // If no explicit application, try to resolve from instruction text
        if app.is_empty() {
            let instruction = param(params, "instruction").to_lowercase();
            app = resolve_app_from_instruction(&instruction);
            if app.is_empty() {
                return action_err(format!(
                    "Cannot execute generic instruction in real mode: \"{instruction}\""
                ));
            }
        }

        std::process::Command::new(&app)
            .spawn()
            .map_err(|e| LapisError::Action(format!("Failed to launch '{app}': {e}")))?;

        // Give the application time to start
        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(real_result(format!("launched application \"{app}\"")))
    }
}

/// Try to resolve an application executable from a natural language instruction.
fn resolve_app_from_instruction(instruction: &str) -> String {
    let mappings: &[(&[&str], &str)] = &[
        (&["notepad", "bloc de notas"], "notepad.exe"),
        (&["calculator", "calculadora", "calc"], "calc.exe"),
        (&["explorer", "explorador", "file manager", "archivos"], "explorer.exe"),
        (&["browser", "navegador", "chrome"], "chrome.exe"),
        (&["edge"], "msedge.exe"),
        (&["firefox"], "firefox.exe"),
        (&["paint"], "mspaint.exe"),
        (&["cmd", "terminal", "command prompt", "consola"], "cmd.exe"),
        (&["powershell"], "powershell.exe"),
        (&["task manager", "administrador de tareas"], "taskmgr.exe"),
        (&["settings", "configuracion", "configuración"], "ms-settings:"),
        (&["word"], "winword.exe"),
        (&["excel"], "excel.exe"),
        (&["powerpoint"], "powerpnt.exe"),
    ];

    for (keywords, app) in mappings {
        if keywords.iter().any(|kw| instruction.contains(kw)) {
            return app.to_string();
        }
    }
    String::new()
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

    #[test]
    fn parse_key_name_common_keys() {
        assert!(parse_key_name("Enter").is_ok());
        assert!(parse_key_name("Tab").is_ok());
        assert!(parse_key_name("Escape").is_ok());
        assert!(parse_key_name("Ctrl").is_ok());
        assert!(parse_key_name("Alt").is_ok());
        assert!(parse_key_name("Shift").is_ok());
        assert!(parse_key_name("Win").is_ok());
        assert!(parse_key_name("F1").is_ok());
        assert!(parse_key_name("F12").is_ok());
        assert!(parse_key_name("a").is_ok());
        assert!(parse_key_name("Z").is_ok());
    }

    #[test]
    fn parse_key_name_unknown() {
        assert!(parse_key_name("UnknownKeyXYZ").is_err());
    }

    #[test]
    fn real_result_is_not_simulated() {
        let r = real_result("test".into());
        assert!(!r.simulated);
        assert!(r.success);
    }
}
