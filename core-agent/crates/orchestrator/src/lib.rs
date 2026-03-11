/// Orchestrator module — main loop coordinating planner and executor.
///
/// Phase 4: produces structured step notifications for real-time GUI updates.
use common::LapisResult;
use config::AppConfig;
use serde::Serialize;

/// A notification emitted when a step starts or completes.
#[derive(Debug, Clone, Serialize)]
pub struct StepNotification {
    pub step_id: u32,
    pub total_steps: u32,
    pub description: String,
    pub status: StepStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Started,
    Completed,
    Failed,
}

/// Processes a user instruction end-to-end.
/// Calls `on_step` for each step start/completion so the IPC layer can push notifications.
pub fn handle_instruction<F>(
    instruction: &str,
    config: &AppConfig,
    mut on_step: F,
) -> LapisResult<String>
where
    F: FnMut(StepNotification),
{
    logging::log_event("orchestrator", "received", instruction)?;

    let plan = planner::create_plan(instruction)?;
    let total = plan.steps.len() as u32;
    logging::log_event(
        "orchestrator",
        "plan_created",
        &format!("{total} steps for '{instruction}'"),
    )?;

    if total == 0 {
        return Ok(format!("No steps generated for '{instruction}'"));
    }

    for step in &plan.steps {
        on_step(StepNotification {
            step_id: step.id,
            total_steps: total,
            description: step.description.clone(),
            status: StepStatus::Started,
            result: None,
        });

        let result = executor::execute_step(step, config.simulation_mode)?;

        let status = if result.success {
            StepStatus::Completed
        } else {
            StepStatus::Failed
        };

        on_step(StepNotification {
            step_id: step.id,
            total_steps: total,
            description: step.description.clone(),
            status,
            result: Some(result.description.clone()),
        });

        logging::log_event_json(
            "orchestrator",
            "step_result",
            serde_json::json!({
                "step_id": step.id,
                "success": result.success,
                "simulated": result.simulated,
                "description": result.description,
            }),
        )?;
    }

    let summary = format!(
        "Completed '{}' — {} steps executed (simulation={})",
        instruction, total, config.simulation_mode
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
        let mut notifications = Vec::new();
        let result = handle_instruction("test instruction", &config, |n| {
            notifications.push(n);
        })
        .unwrap();
        assert!(result.contains("test instruction"));
    }

    #[test]
    fn notepad_instruction_produces_notifications() {
        let config = AppConfig::default();
        let mut notifications = Vec::new();
        let result = handle_instruction("open notepad", &config, |n| {
            notifications.push(n);
        })
        .unwrap();
        // 2 steps × 2 notifications each (started + completed) = 4
        assert_eq!(notifications.len(), 4);
        assert!(result.contains("2 steps"));
    }

    #[test]
    fn notifications_are_serializable() {
        let n = StepNotification {
            step_id: 1,
            total_steps: 3,
            description: "test".into(),
            status: StepStatus::Completed,
            result: Some("done".into()),
        };
        let json = serde_json::to_string(&n).unwrap();
        assert!(json.contains("completed"));
    }
}
