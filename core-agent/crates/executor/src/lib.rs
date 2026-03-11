/// Executor module — carries out individual plan steps.
///
/// Dispatches each step to the appropriate action handler.
/// In Phase 1 this is a stub that logs the step and returns success.
use actions::ActionResult;
use common::LapisResult;
use planner::PlanStep;

/// Executes a single plan step in simulation mode.
pub fn execute_step(step: &PlanStep, simulation: bool) -> LapisResult<ActionResult> {
    let mode = if simulation { "simulated" } else { "real" };
    logging::log_event(
        "executor",
        "execute_step",
        &format!("[{mode}] step {}: {}", step.id, step.description),
    )?;

    Ok(ActionResult {
        success: true,
        simulated: simulation,
        description: format!("Step {} {} ({})", step.id, step.description, mode),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actions::ActionType;
    use std::collections::HashMap;

    fn dummy_step() -> PlanStep {
        PlanStep {
            id: 1,
            action_type: ActionType::MouseClick,
            parameters: HashMap::new(),
            description: "click button".into(),
            expected_outcome: "button clicked".into(),
        }
    }

    #[test]
    fn execute_simulated_step() {
        let result = execute_step(&dummy_step(), true).unwrap();
        assert!(result.success);
        assert!(result.simulated);
    }

    #[test]
    fn execute_real_step_stub() {
        let result = execute_step(&dummy_step(), false).unwrap();
        assert!(result.success);
        assert!(!result.simulated);
    }
}
