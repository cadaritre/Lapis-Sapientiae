/// Orchestrator module — main loop coordinating planner and executor.
///
/// In Phase 1 this provides a single `handle_instruction` function
/// that creates a plan and executes each step.
use common::LapisResult;
use config::AppConfig;

/// Processes a user instruction end-to-end.
pub fn handle_instruction(instruction: &str, config: &AppConfig) -> LapisResult<String> {
    logging::log_event("orchestrator", "received", instruction)?;

    let plan = planner::create_plan(instruction)?;
    logging::log_event(
        "orchestrator",
        "plan_created",
        &format!("{} steps", plan.steps.len()),
    )?;

    for step in &plan.steps {
        let result = executor::execute_step(step, config.simulation_mode)?;
        logging::log_event(
            "orchestrator",
            "step_result",
            &format!("step {}: success={}", step.id, result.success),
        )?;
    }

    let summary = format!(
        "Completed '{}' — {} steps executed (simulation={})",
        instruction,
        plan.steps.len(),
        config.simulation_mode
    );
    logging::log_event("orchestrator", "completed", &summary)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_instruction_succeeds_with_defaults() {
        let config = AppConfig::default();
        let result = handle_instruction("test instruction", &config).unwrap();
        assert!(result.contains("test instruction"));
    }
}
