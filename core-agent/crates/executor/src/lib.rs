/// Executor module — carries out individual plan steps.
///
/// Phase 5: dispatches each step to the correct action handler via `actions::dispatch`.
use actions::ActionResult;
use common::LapisResult;
use planner::PlanStep;

/// Executes a single plan step by dispatching to the appropriate action handler.
pub fn execute_step(step: &PlanStep, simulation: bool) -> LapisResult<ActionResult> {
    let mode = if simulation { "simulated" } else { "real" };
    logging::log_event(
        "executor",
        "dispatch",
        &format!("[{mode}] step {}: {:?} — {}", step.id, step.action_type, step.description),
    )?;

    let result = actions::dispatch(&step.action_type, &step.parameters, simulation)?;

    logging::log_event(
        "executor",
        "result",
        &format!("step {}: {}", step.id, result.description),
    )?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actions::ActionType;

    fn step_with(action: ActionType, params: Vec<(&str, &str)>) -> PlanStep {
        PlanStep {
            id: 1,
            action_type: action,
            parameters: params.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            description: "test step".into(),
            expected_outcome: "expected".into(),
        }
    }

    #[test]
    fn execute_simulated_mouse_click() {
        let step = step_with(ActionType::MouseClick, vec![("x", "100"), ("y", "200"), ("button", "left")]);
        let result = execute_step(&step, true).unwrap();
        assert!(result.success);
        assert!(result.simulated);
        assert!(result.description.contains("left-clicked"));
    }

    #[test]
    fn execute_simulated_keyboard_type() {
        let step = step_with(ActionType::KeyboardType, vec![("text", "hello")]);
        let result = execute_step(&step, true).unwrap();
        assert!(result.description.contains("hello"));
    }

    #[test]
    fn execute_simulated_system_launch() {
        let step = step_with(ActionType::SystemLaunch, vec![("application", "calc.exe")]);
        let result = execute_step(&step, true).unwrap();
        assert!(result.description.contains("calc.exe"));
    }

    #[test]
    fn execute_simulated_window_close() {
        let step = step_with(ActionType::WindowClose, vec![("window_title", "Notepad")]);
        let result = execute_step(&step, true).unwrap();
        assert!(result.description.contains("closed"));
    }

    #[test]
    fn real_execution_fails_gracefully() {
        let step = step_with(ActionType::MouseClick, vec![]);
        let result = execute_step(&step, false);
        assert!(result.is_err());
    }
}
