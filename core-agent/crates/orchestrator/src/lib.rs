/// Orchestrator module — main loop coordinating planner and executor.
///
/// Phase 8: visual executor with before/after screenshots and VLM verification.
use common::LapisResult;
use config::AppConfig;
use perception::VlmConfig;
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

/// Processes a user instruction end-to-end (async for LLM planning + perception).
/// Calls `on_step` for each step start/completion so the IPC layer can push notifications.
pub async fn handle_instruction<F>(
    instruction: &str,
    config: &AppConfig,
    screen_context: Option<&str>,
    mut on_step: F,
) -> LapisResult<String>
where
    F: FnMut(StepNotification),
{
    logging::log_event("orchestrator", "received", instruction)?;

    let plan = planner::create_plan(instruction, config, screen_context).await?;
    let total = plan.steps.len() as u32;

    let source_label = match plan.source {
        planner::PlanSource::Llm => "LLM",
        planner::PlanSource::Keyword => "keyword",
    };

    logging::log_event(
        "orchestrator",
        "plan_created",
        &format!("{total} steps via {source_label} for '{instruction}'"),
    )?;

    // Send reasoning as a system notification if available
    if let Some(ref reasoning) = plan.reasoning {
        on_step(StepNotification {
            step_id: 0,
            total_steps: total,
            description: format!("[{source_label}] {reasoning}"),
            status: StepStatus::Started,
            result: None,
        });
    }

    if total == 0 {
        return Ok(format!("No steps generated for '{instruction}'"));
    }

    // Build VLM config for visual verification (only if VLM endpoint is reachable)
    let vlm_config = VlmConfig {
        endpoint: config.vlm_endpoint.clone(),
        model: config.vlm_model.clone(),
    };

    let mut verified_count = 0u32;
    let mut failed_count = 0u32;

    for step in &plan.steps {
        on_step(StepNotification {
            step_id: step.id,
            total_steps: total,
            description: step.description.clone(),
            status: StepStatus::Started,
            result: None,
        });

        // Use perception-aware executor
        let exec_result = executor::execute_step_with_perception(
            step,
            config.simulation_mode,
            Some(&vlm_config),
        )
        .await?;

        let action_success = exec_result.action_result.success;
        let mut result_desc = exec_result.action_result.description.clone();

        // Append verification info if available
        if let Some(ref verification) = exec_result.verification {
            let verdict = if verification.matches_expected { "VERIFIED" } else { "MISMATCH" };
            result_desc = format!("{result_desc} [{verdict}: {}]", verification.description);
            if verification.matches_expected {
                verified_count += 1;
            }
        }

        // Append screenshot info
        if let (Some(before), Some(after)) = (&exec_result.before_screenshot, &exec_result.after_screenshot) {
            result_desc = format!(
                "{result_desc} (screenshots: {}x{} → {}x{})",
                before.width, before.height, after.width, after.height
            );
        }

        let status = if action_success {
            StepStatus::Completed
        } else {
            failed_count += 1;
            StepStatus::Failed
        };

        on_step(StepNotification {
            step_id: step.id,
            total_steps: total,
            description: step.description.clone(),
            status,
            result: Some(result_desc),
        });

        logging::log_event_json(
            "orchestrator",
            "step_result",
            serde_json::json!({
                "step_id": step.id,
                "success": action_success,
                "simulated": exec_result.action_result.simulated,
                "description": exec_result.action_result.description,
                "verified": exec_result.verification.as_ref().map(|v| v.matches_expected),
            }),
        )?;
    }

    let summary = format!(
        "Completed '{}' — {} steps via {} (simulation={}, verified={}, failed={})",
        instruction, total, source_label, config.simulation_mode, verified_count, failed_count
    );
    logging::log_event("orchestrator", "completed", &summary)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn handle_instruction_succeeds_with_defaults() {
        let config = AppConfig::default();
        let mut notifications = Vec::new();
        let result = handle_instruction("test instruction", &config, None, |n| {
            notifications.push(n);
        })
        .await
        .unwrap();
        assert!(result.contains("test instruction"));
    }

    #[tokio::test]
    async fn notepad_instruction_produces_notifications() {
        let config = AppConfig::default();
        let mut notifications = Vec::new();
        let result = handle_instruction("open notepad", &config, None, |n| {
            notifications.push(n);
        })
        .await
        .unwrap();
        // At least 2 steps × 2 notifications each (started + completed) = 4
        assert!(notifications.len() >= 4);
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
