/// Executor module — carries out individual plan steps with visual verification.
///
/// Phase 8: captures before/after screenshots for each step and optionally
/// verifies the outcome via VLM analysis.
use actions::ActionResult;
use common::LapisResult;
use perception::VlmConfig;
use planner::PlanStep;
use serde::{Deserialize, Serialize};

/// Extended result from executing a step with visual context.
#[derive(Debug, Clone, Serialize)]
pub struct StepExecutionResult {
    pub action_result: ActionResult,
    pub before_screenshot: Option<ScreenshotInfo>,
    pub after_screenshot: Option<ScreenshotInfo>,
    pub verification: Option<VerificationResult>,
}

/// Minimal screenshot metadata (no base64 to keep it lightweight in notifications).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotInfo {
    pub width: u32,
    pub height: u32,
}

/// Result of VLM-based step verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub matches_expected: bool,
    pub description: String,
    pub model: String,
}

/// Executes a single plan step by dispatching to the appropriate action handler.
/// Captures before/after screenshots for visual verification.
pub async fn execute_step_with_perception(
    step: &PlanStep,
    simulation: bool,
    vlm_config: Option<&VlmConfig>,
) -> LapisResult<StepExecutionResult> {
    let mode = if simulation { "simulated" } else { "real" };
    logging::log_event(
        "executor",
        "dispatch",
        &format!("[{mode}] step {}: {:?} — {}", step.id, step.action_type, step.description),
    )?;

    // Capture before screenshot
    let before_screenshot = match perception::capture_screen() {
        Ok(s) => Some(ScreenshotInfo { width: s.width, height: s.height }),
        Err(e) => {
            logging::log_event("executor", "warn", &format!("Before screenshot failed: {e}"))?;
            None
        }
    };

    // Execute the action
    let action_result = actions::dispatch(&step.action_type, &step.parameters, simulation)?;

    logging::log_event(
        "executor",
        "result",
        &format!("step {}: {}", step.id, action_result.description),
    )?;

    // Capture after screenshot
    let after_screenshot = match perception::capture_screen() {
        Ok(s) => Some(ScreenshotInfo { width: s.width, height: s.height }),
        Err(e) => {
            logging::log_event("executor", "warn", &format!("After screenshot failed: {e}"))?;
            None
        }
    };

    // Optionally verify with VLM
    let verification = if let Some(vlm_cfg) = vlm_config {
        if !step.expected_outcome.is_empty() {
            match verify_step_outcome(step, vlm_cfg).await {
                Ok(v) => Some(v),
                Err(e) => {
                    logging::log_event("executor", "warn", &format!("Verification failed: {e}"))?;
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(StepExecutionResult {
        action_result,
        before_screenshot,
        after_screenshot,
        verification,
    })
}

/// Verify the step outcome by analyzing the current screen with VLM.
async fn verify_step_outcome(
    step: &PlanStep,
    vlm_config: &VlmConfig,
) -> LapisResult<VerificationResult> {
    let prompt = format!(
        "I just performed this action: '{}'. \
         The expected outcome was: '{}'. \
         Look at the current screen and tell me: \
         does the screen state match the expected outcome? \
         Answer with YES or NO first, then briefly explain what you see.",
        step.description,
        step.expected_outcome
    );

    let (_, analysis) = perception::capture_and_analyze(vlm_config, &prompt).await?;
    let desc_lower = analysis.description.to_lowercase();
    let matches = desc_lower.starts_with("yes") || desc_lower.contains("matches") || desc_lower.contains("correct");

    Ok(VerificationResult {
        matches_expected: matches,
        description: analysis.description,
        model: analysis.model,
    })
}

/// Simple step execution without perception (backward compatible).
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
