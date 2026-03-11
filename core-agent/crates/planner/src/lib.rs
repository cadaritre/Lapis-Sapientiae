/// Planner module — interprets user instructions and produces structured plans.
///
/// In Phase 4 this uses keyword matching to generate realistic hardcoded plans.
/// LLM integration comes in Phase 7.
use actions::{ActionParams, ActionType};
use common::LapisResult;
use serde::{Deserialize, Serialize};

/// A single step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: u32,
    pub action_type: ActionType,
    pub parameters: ActionParams,
    pub description: String,
    pub expected_outcome: String,
}

/// A full plan composed of ordered steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub instruction: String,
    pub steps: Vec<PlanStep>,
}

/// Creates a plan from a user instruction.
/// Phase 4: keyword matching produces realistic multi-step plans.
pub fn create_plan(instruction: &str) -> LapisResult<Plan> {
    let lower = instruction.to_lowercase();
    let steps = match_instruction(&lower);

    Ok(Plan {
        instruction: instruction.to_string(),
        steps,
    })
}

/// Match keywords in the instruction to generate a realistic plan.
fn match_instruction(instruction: &str) -> Vec<PlanStep> {
    if instruction.contains("notepad") {
        plan_open_notepad()
    } else if instruction.contains("calculator") || instruction.contains("calc") {
        plan_open_calculator()
    } else if instruction.contains("type") || instruction.contains("write") || instruction.contains("escribe") {
        plan_type_text(instruction)
    } else if instruction.contains("click") || instruction.contains("press") {
        plan_click()
    } else if instruction.contains("close") || instruction.contains("cerrar") {
        plan_close_window()
    } else if instruction.contains("search") || instruction.contains("buscar") || instruction.contains("find") {
        plan_search(instruction)
    } else {
        plan_generic(instruction)
    }
}

fn plan_open_notepad() -> Vec<PlanStep> {
    vec![
        PlanStep {
            id: 1,
            action_type: ActionType::SystemLaunch,
            parameters: params(&[("application", "notepad.exe")]),
            description: "Launch Notepad application".into(),
            expected_outcome: "Notepad window appears on screen".into(),
        },
        PlanStep {
            id: 2,
            action_type: ActionType::WindowFocus,
            parameters: params(&[("window_title", "Untitled - Notepad")]),
            description: "Focus the Notepad window".into(),
            expected_outcome: "Notepad window is in foreground".into(),
        },
    ]
}

fn plan_open_calculator() -> Vec<PlanStep> {
    vec![
        PlanStep {
            id: 1,
            action_type: ActionType::SystemLaunch,
            parameters: params(&[("application", "calc.exe")]),
            description: "Launch Calculator application".into(),
            expected_outcome: "Calculator window appears on screen".into(),
        },
        PlanStep {
            id: 2,
            action_type: ActionType::WindowFocus,
            parameters: params(&[("window_title", "Calculator")]),
            description: "Focus the Calculator window".into(),
            expected_outcome: "Calculator window is in foreground".into(),
        },
    ]
}

fn plan_type_text(instruction: &str) -> Vec<PlanStep> {
    let text = instruction
        .replace("type", "")
        .replace("write", "")
        .replace("escribe", "")
        .trim()
        .to_string();
    let text = if text.is_empty() { "Hello, world!".to_string() } else { text };

    vec![
        PlanStep {
            id: 1,
            action_type: ActionType::MouseClick,
            parameters: params(&[("x", "400"), ("y", "300"), ("button", "left")]),
            description: "Click on the target text area".into(),
            expected_outcome: "Text cursor appears in the target area".into(),
        },
        PlanStep {
            id: 2,
            action_type: ActionType::KeyboardType,
            parameters: params(&[("text", &text)]),
            description: format!("Type: {text}"),
            expected_outcome: "Text appears in the target area".into(),
        },
    ]
}

fn plan_click() -> Vec<PlanStep> {
    vec![PlanStep {
        id: 1,
        action_type: ActionType::MouseClick,
        parameters: params(&[("x", "500"), ("y", "400"), ("button", "left")]),
        description: "Click at target position".into(),
        expected_outcome: "Element at position is activated".into(),
    }]
}

fn plan_close_window() -> Vec<PlanStep> {
    vec![PlanStep {
        id: 1,
        action_type: ActionType::WindowClose,
        parameters: params(&[("window_title", "active")]),
        description: "Close the active window".into(),
        expected_outcome: "Active window is closed".into(),
    }]
}

fn plan_search(instruction: &str) -> Vec<PlanStep> {
    let query = instruction
        .replace("search", "")
        .replace("buscar", "")
        .replace("find", "")
        .replace("for", "")
        .trim()
        .to_string();
    let query = if query.is_empty() { "example".to_string() } else { query };

    vec![
        PlanStep {
            id: 1,
            action_type: ActionType::KeyboardCombo,
            parameters: params(&[("keys", "Win+S")]),
            description: "Open Windows search".into(),
            expected_outcome: "Windows search bar appears".into(),
        },
        PlanStep {
            id: 2,
            action_type: ActionType::KeyboardType,
            parameters: params(&[("text", &query)]),
            description: format!("Type search query: {query}"),
            expected_outcome: "Search results appear".into(),
        },
        PlanStep {
            id: 3,
            action_type: ActionType::KeyboardPress,
            parameters: params(&[("key", "Enter")]),
            description: "Press Enter to execute search".into(),
            expected_outcome: "Top search result is opened".into(),
        },
    ]
}

fn plan_generic(instruction: &str) -> Vec<PlanStep> {
    vec![PlanStep {
        id: 1,
        action_type: ActionType::SystemLaunch,
        parameters: params(&[("instruction", instruction)]),
        description: format!("Process instruction: {instruction}"),
        expected_outcome: "Instruction processed successfully".into(),
    }]
}

/// Helper to build ActionParams from key-value pairs.
fn params(pairs: &[(&str, &str)]) -> ActionParams {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_plan_preserves_instruction() {
        let plan = create_plan("open notepad").unwrap();
        assert_eq!(plan.instruction, "open notepad");
    }

    #[test]
    fn notepad_plan_has_two_steps() {
        let plan = create_plan("open notepad").unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].action_type, ActionType::SystemLaunch);
        assert_eq!(plan.steps[1].action_type, ActionType::WindowFocus);
    }

    #[test]
    fn calculator_plan_has_two_steps() {
        let plan = create_plan("open calculator").unwrap();
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn type_plan_includes_text() {
        let plan = create_plan("type hello world").unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[1].action_type, ActionType::KeyboardType);
    }

    #[test]
    fn search_plan_has_three_steps() {
        let plan = create_plan("search for rust programming").unwrap();
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn generic_instruction_produces_single_step() {
        let plan = create_plan("do something unknown").unwrap();
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn plan_is_serializable() {
        let plan = create_plan("open notepad").unwrap();
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("SystemLaunch"));
    }
}
