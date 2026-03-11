/// Orchestrator module — main loop coordinating planner and executor.
///
/// Phase 9: abort flag, rate limiting, and confirmation gate for real execution.
/// Phase 10: iterative VLM-guided action loop for perception-driven tasks.
use actions::{ActionParams, ActionType};
use common::LapisResult;
use config::AppConfig;
use perception::VlmConfig;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    Aborted,
    AwaitingConfirmation,
}

/// Plan summary sent to GUI for confirmation before real execution.
#[derive(Debug, Clone, Serialize)]
pub struct PlanConfirmation {
    pub instruction: String,
    pub steps: Vec<PlanStepSummary>,
    pub source: String,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanStepSummary {
    pub id: u32,
    pub description: String,
    pub action_type: String,
}

/// Processes a user instruction end-to-end (async for LLM planning + perception).
/// Calls `on_step` for each step start/completion so the IPC layer can push notifications.
/// `abort_flag` is checked before each step; if true, execution stops.
/// `confirm_fn` is called when simulation_mode is false to get user confirmation.
/// It receives the plan and returns a oneshot Receiver<bool>.
pub async fn handle_instruction<F, C>(
    instruction: &str,
    config: &AppConfig,
    screen_context: Option<&str>,
    abort_flag: Arc<AtomicBool>,
    mut on_step: F,
    confirm_fn: Option<C>,
) -> LapisResult<String>
where
    F: FnMut(StepNotification),
    C: FnOnce(PlanConfirmation) -> tokio::sync::oneshot::Receiver<bool>,
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

    // Safety gate: if not simulation mode, request confirmation before executing
    if !config.simulation_mode {
        if let Some(confirm) = confirm_fn {
            let confirmation = PlanConfirmation {
                instruction: instruction.to_string(),
                steps: plan.steps.iter().map(|s| PlanStepSummary {
                    id: s.id,
                    description: s.description.clone(),
                    action_type: format!("{:?}", s.action_type),
                }).collect(),
                source: source_label.to_string(),
                reasoning: plan.reasoning.clone(),
            };

            on_step(StepNotification {
                step_id: 0,
                total_steps: total,
                description: "Awaiting user confirmation for real execution".into(),
                status: StepStatus::AwaitingConfirmation,
                result: None,
            });

            let receiver = confirm(confirmation);
            match receiver.await {
                Ok(true) => {
                    logging::log_event("orchestrator", "confirmed", "User confirmed real execution")?;
                }
                Ok(false) => {
                    logging::log_event("orchestrator", "cancelled", "User cancelled execution")?;
                    return Ok(format!("Cancelled '{}' — user declined execution", instruction));
                }
                Err(_) => {
                    return Ok(format!("Cancelled '{}' — confirmation channel closed", instruction));
                }
            }
        }
    }

    // Build VLM config for visual verification
    let vlm_config = VlmConfig {
        endpoint: config.vlm_endpoint.clone(),
        model: config.vlm_model.clone(),
    };

    let mut verified_count = 0u32;
    let mut failed_count = 0u32;
    let mut completed_steps = 0u32;

    for step in &plan.steps {
        // Check abort flag before each step
        if abort_flag.load(Ordering::Relaxed) {
            on_step(StepNotification {
                step_id: step.id,
                total_steps: total,
                description: "Execution aborted by user".into(),
                status: StepStatus::Aborted,
                result: None,
            });
            logging::log_event("orchestrator", "aborted", &format!(
                "Aborted '{}' at step {}/{}", instruction, step.id, total
            ))?;
            return Ok(format!(
                "ABORTED '{}' at step {}/{} (completed={}, verified={}, failed={})",
                instruction, step.id, total, completed_steps, verified_count, failed_count
            ));
        }

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
            completed_steps += 1;
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

        // Rate limiting between real actions
        if !config.simulation_mode && config.rate_limit_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(config.rate_limit_ms)).await;
        }
    }

    let summary = format!(
        "Completed '{}' — {} steps via {} (simulation={}, verified={}, failed={})",
        instruction, total, source_label, config.simulation_mode, verified_count, failed_count
    );
    logging::log_event("orchestrator", "completed", &summary)?;
    Ok(summary)
}

/// Maximum iterations for the VLM-guided action loop.
const MAX_VLM_ITERATIONS: u32 = 15;

/// Structured prompt for moondream to decide the next action.
fn build_vlm_action_prompt(instruction: &str, step_num: u32, history: &str) -> String {
    format!(
        "You are a computer automation agent. The user wants: \"{instruction}\"\n\
         This is step {step_num}. {history}\n\
         Look at the screenshot and decide the NEXT action to take.\n\
         Respond with EXACTLY ONE line in one of these formats:\n\
         CLICK x y - click at pixel coordinates (e.g. CLICK 500 300)\n\
         RIGHTCLICK x y - right-click at coordinates\n\
         TYPE text here - type this text\n\
         KEY Enter - press a key (Enter, Tab, Escape, etc)\n\
         COMBO Ctrl+s - press a key combination\n\
         DONE - the task is complete\n\
         FAIL reason - cannot complete the task\n\n\
         Respond with only the action line, nothing else."
    )
}

/// Parse a VLM response into an action type and parameters.
fn parse_vlm_action(response: &str) -> Option<(ActionType, ActionParams, String)> {
    let line = response.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())?;

    let upper = line.to_uppercase();
    let parts: Vec<&str> = line.splitn(3, ' ').collect();

    if upper.starts_with("DONE") {
        return None; // signals completion
    }

    if upper.starts_with("FAIL") {
        return None;
    }

    if upper.starts_with("CLICK") && parts.len() >= 3 {
        let x = parts[1].trim().parse::<i32>().ok()?;
        let y = parts[2].trim().parse::<i32>().ok()?;
        let mut params = ActionParams::new();
        params.insert("x".into(), x.to_string());
        params.insert("y".into(), y.to_string());
        params.insert("button".into(), "left".into());
        return Some((ActionType::MouseClick, params, format!("Click at ({x}, {y})")));
    }

    if upper.starts_with("RIGHTCLICK") && parts.len() >= 3 {
        let x = parts[1].trim().parse::<i32>().ok()?;
        let y = parts[2].trim().parse::<i32>().ok()?;
        let mut params = ActionParams::new();
        params.insert("x".into(), x.to_string());
        params.insert("y".into(), y.to_string());
        params.insert("button".into(), "right".into());
        return Some((ActionType::MouseClick, params, format!("Right-click at ({x}, {y})")));
    }

    if upper.starts_with("TYPE") {
        let text = if parts.len() >= 2 {
            line[parts[0].len()..].trim().to_string()
        } else {
            return None;
        };
        let mut params = ActionParams::new();
        params.insert("text".into(), text.clone());
        return Some((ActionType::KeyboardType, params, format!("Type: {text}")));
    }

    if upper.starts_with("KEY") && parts.len() >= 2 {
        let key = parts[1].trim().to_string();
        let mut params = ActionParams::new();
        params.insert("key".into(), key.clone());
        return Some((ActionType::KeyboardPress, params, format!("Press key: {key}")));
    }

    if upper.starts_with("COMBO") && parts.len() >= 2 {
        let combo = parts[1].trim().to_string();
        let mut params = ActionParams::new();
        params.insert("keys".into(), combo.clone());
        return Some((ActionType::KeyboardCombo, params, format!("Key combo: {combo}")));
    }

    None
}

/// Iterative VLM-guided action loop.
/// Takes screenshots, asks the VLM what to do, executes the action, repeats.
/// Used when the keyword planner can't handle the instruction (generic/complex tasks).
pub async fn handle_instruction_iterative<F>(
    instruction: &str,
    config: &AppConfig,
    abort_flag: Arc<AtomicBool>,
    mut on_step: F,
) -> LapisResult<String>
where
    F: FnMut(StepNotification),
{
    logging::log_event("orchestrator", "iterative_start", instruction)?;

    let vlm_config = VlmConfig {
        endpoint: config.vlm_endpoint.clone(),
        model: config.vlm_model.clone(),
    };

    let mut history = String::new();
    let mut completed = 0u32;
    let mut failed = 0u32;

    for step_num in 1..=MAX_VLM_ITERATIONS {
        // Check abort
        if abort_flag.load(Ordering::Relaxed) {
            on_step(StepNotification {
                step_id: step_num,
                total_steps: MAX_VLM_ITERATIONS,
                description: "Aborted by user".into(),
                status: StepStatus::Aborted,
                result: None,
            });
            return Ok(format!("ABORTED '{}' at step {} (completed={})", instruction, step_num, completed));
        }

        // Notify: capturing screen
        on_step(StepNotification {
            step_id: step_num,
            total_steps: MAX_VLM_ITERATIONS,
            description: format!("Analyzing screen (step {step_num})..."),
            status: StepStatus::Started,
            result: None,
        });

        // Capture screenshot and ask VLM
        let prompt = build_vlm_action_prompt(instruction, step_num, &history);
        let (_screenshot, vlm_response) = perception::capture_and_analyze(&vlm_config, &prompt).await?;

        let raw_response = vlm_response.description.trim().to_string();
        logging::log_event("orchestrator", "vlm_response", &format!("step {step_num}: {raw_response}"))?;

        // Parse the VLM response
        let parsed = parse_vlm_action(&raw_response);

        // Check for DONE or FAIL
        let upper = raw_response.to_uppercase();
        if upper.contains("DONE") || parsed.is_none() && upper.contains("COMPLETE") {
            on_step(StepNotification {
                step_id: step_num,
                total_steps: step_num,
                description: "Task completed".into(),
                status: StepStatus::Completed,
                result: Some(raw_response.clone()),
            });
            let summary = format!(
                "Completed '{}' — {} VLM-guided steps (completed={}, failed={})",
                instruction, step_num, completed, failed
            );
            logging::log_event("orchestrator", "iterative_done", &summary)?;
            return Ok(summary);
        }

        if upper.starts_with("FAIL") {
            on_step(StepNotification {
                step_id: step_num,
                total_steps: step_num,
                description: format!("VLM cannot proceed: {raw_response}"),
                status: StepStatus::Failed,
                result: Some(raw_response.clone()),
            });
            return Ok(format!("Failed '{}' — VLM: {}", instruction, raw_response));
        }

        match parsed {
            Some((action_type, params, description)) => {
                on_step(StepNotification {
                    step_id: step_num,
                    total_steps: MAX_VLM_ITERATIONS,
                    description: description.clone(),
                    status: StepStatus::Started,
                    result: None,
                });

                // Execute the action
                let result = actions::dispatch(&action_type, &params, config.simulation_mode)?;

                let status = if result.success {
                    completed += 1;
                    StepStatus::Completed
                } else {
                    failed += 1;
                    StepStatus::Failed
                };

                history.push_str(&format!("Step {step_num}: {description} -> {}\n", result.description));

                on_step(StepNotification {
                    step_id: step_num,
                    total_steps: MAX_VLM_ITERATIONS,
                    description: description.clone(),
                    status,
                    result: Some(result.description.clone()),
                });

                logging::log_event_json("orchestrator", "iterative_step", serde_json::json!({
                    "step": step_num,
                    "action": format!("{:?}", action_type),
                    "description": description,
                    "success": result.success,
                    "simulated": result.simulated,
                }))?;

                // Rate limiting
                if !config.simulation_mode && config.rate_limit_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(config.rate_limit_ms)).await;
                }
            }
            None => {
                // VLM gave unparseable response — try to continue
                history.push_str(&format!("Step {step_num}: (unparseable VLM response: {raw_response})\n"));
                on_step(StepNotification {
                    step_id: step_num,
                    total_steps: MAX_VLM_ITERATIONS,
                    description: format!("VLM response unclear: {raw_response}"),
                    status: StepStatus::Failed,
                    result: Some(raw_response.clone()),
                });
                failed += 1;
            }
        }
    }

    let summary = format!(
        "Reached max iterations for '{}' — {} steps (completed={}, failed={})",
        instruction, MAX_VLM_ITERATIONS, completed, failed
    );
    logging::log_event("orchestrator", "iterative_max", &summary)?;
    Ok(summary)
}

/// Check if a plan should use the iterative VLM loop instead of direct execution.
/// Returns true when:
/// 1. The plan is a generic fallback (SystemLaunch with only "instruction" param), OR
/// 2. The keyword planner oversimplified a complex instruction (matched one keyword
///    but the instruction contains additional verbs/actions it didn't plan for).
pub fn is_generic_plan(plan: &planner::Plan) -> bool {
    // Case 1: explicit generic fallback
    let is_fallback = plan.steps.len() == 1
        && plan.steps[0].action_type == ActionType::SystemLaunch
        && plan.steps[0].parameters.get("instruction").is_some()
        && plan.steps[0].parameters.get("application").is_none();
    if is_fallback {
        return true;
    }

    // Case 2: keyword plan is too simple for a multi-action instruction
    if matches!(plan.source, planner::PlanSource::Keyword) {
        let lower = plan.instruction.to_lowercase();
        let action_verbs = [
            "crea", "crear", "create", "haz", "hacer", "make",
            "escribe", "escribir", "write", "type",
            "abre", "abrir", "open",
            "ve ", "ir ", "go ", "navega", "navigate",
            "busca", "search", "find",
            "cierra", "cerrar", "close",
            "borra", "borrar", "delete", "elimina",
            "copia", "copiar", "copy",
            "mueve", "mover", "move",
            "renombra", "rename",
            "click", "presiona", "press",
        ];
        let verb_count = action_verbs.iter().filter(|v| lower.contains(**v)).count();
        // If instruction has 2+ distinct action verbs but keyword only produced 1-step plan,
        // the instruction is too complex for keyword matching alone
        if verb_count >= 2 && plan.steps.len() <= 1 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn handle_instruction_succeeds_with_defaults() {
        let config = AppConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let mut notifications = Vec::new();
        let result = handle_instruction(
            "test instruction",
            &config,
            None,
            abort_flag,
            |n| { notifications.push(n); },
            None::<fn(PlanConfirmation) -> tokio::sync::oneshot::Receiver<bool>>,
        )
        .await
        .unwrap();
        assert!(result.contains("test instruction"));
    }

    #[tokio::test]
    async fn notepad_instruction_produces_notifications() {
        let config = AppConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let mut notifications = Vec::new();
        let result = handle_instruction(
            "open notepad",
            &config,
            None,
            abort_flag,
            |n| { notifications.push(n); },
            None::<fn(PlanConfirmation) -> tokio::sync::oneshot::Receiver<bool>>,
        )
        .await
        .unwrap();
        assert!(notifications.len() >= 4);
        assert!(result.contains("2 steps"));
    }

    #[tokio::test]
    async fn abort_flag_stops_execution() {
        let config = AppConfig::default();
        let abort_flag = Arc::new(AtomicBool::new(true)); // pre-set to abort
        let result = handle_instruction(
            "open notepad",
            &config,
            None,
            abort_flag,
            |_| {},
            None::<fn(PlanConfirmation) -> tokio::sync::oneshot::Receiver<bool>>,
        )
        .await
        .unwrap();
        assert!(result.contains("ABORTED"));
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

    #[test]
    fn plan_confirmation_is_serializable() {
        let c = PlanConfirmation {
            instruction: "test".into(),
            steps: vec![PlanStepSummary {
                id: 1,
                description: "step 1".into(),
                action_type: "MouseClick".into(),
            }],
            source: "LLM".into(),
            reasoning: Some("because".into()),
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("MouseClick"));
    }

    // --- parse_vlm_action tests ---

    #[test]
    fn parse_vlm_action_click() {
        let result = parse_vlm_action("CLICK 500 300");
        assert!(result.is_some());
        let (action, params, _desc) = result.unwrap();
        assert_eq!(action, ActionType::MouseClick);
        assert_eq!(params.get("x").unwrap(), "500");
        assert_eq!(params.get("y").unwrap(), "300");
        assert_eq!(params.get("button").unwrap(), "left");
    }

    #[test]
    fn parse_vlm_action_rightclick() {
        let result = parse_vlm_action("RIGHTCLICK 100 200");
        assert!(result.is_some());
        let (action, params, _) = result.unwrap();
        assert_eq!(action, ActionType::MouseClick);
        assert_eq!(params.get("button").unwrap(), "right");
    }

    #[test]
    fn parse_vlm_action_type_text() {
        let result = parse_vlm_action("TYPE hello world");
        assert!(result.is_some());
        let (action, params, _) = result.unwrap();
        assert_eq!(action, ActionType::KeyboardType);
        assert_eq!(params.get("text").unwrap(), "hello world");
    }

    #[test]
    fn parse_vlm_action_key() {
        let result = parse_vlm_action("KEY Enter");
        assert!(result.is_some());
        let (action, params, _) = result.unwrap();
        assert_eq!(action, ActionType::KeyboardPress);
        assert_eq!(params.get("key").unwrap(), "Enter");
    }

    #[test]
    fn parse_vlm_action_combo() {
        let result = parse_vlm_action("COMBO Ctrl+s");
        assert!(result.is_some());
        let (action, params, _) = result.unwrap();
        assert_eq!(action, ActionType::KeyboardCombo);
        assert_eq!(params.get("keys").unwrap(), "Ctrl+s");
    }

    #[test]
    fn parse_vlm_action_done_returns_none() {
        assert!(parse_vlm_action("DONE").is_none());
    }

    #[test]
    fn parse_vlm_action_fail_returns_none() {
        assert!(parse_vlm_action("FAIL cannot find button").is_none());
    }

    #[test]
    fn parse_vlm_action_unparseable_returns_none() {
        assert!(parse_vlm_action("I see a desktop with icons").is_none());
    }

    #[test]
    fn parse_vlm_action_empty_returns_none() {
        assert!(parse_vlm_action("").is_none());
    }

    // --- is_generic_plan tests ---

    #[test]
    fn is_generic_plan_detects_fallback() {
        let plan = planner::Plan {
            instruction: "do something complex".into(),
            steps: vec![planner::PlanStep {
                id: 1,
                action_type: ActionType::SystemLaunch,
                parameters: [("instruction".to_string(), "do something complex".to_string())].into_iter().collect(),
                description: "Process instruction".into(),
                expected_outcome: "Done".into(),
            }],
            reasoning: None,
            source: planner::PlanSource::Keyword,
        };
        assert!(is_generic_plan(&plan));
    }

    #[test]
    fn is_generic_plan_rejects_known_app_launch() {
        let plan = planner::Plan {
            instruction: "open notepad".into(),
            steps: vec![planner::PlanStep {
                id: 1,
                action_type: ActionType::SystemLaunch,
                parameters: [("application".to_string(), "notepad.exe".to_string())].into_iter().collect(),
                description: "Launch Notepad".into(),
                expected_outcome: "Notepad opens".into(),
            }],
            reasoning: None,
            source: planner::PlanSource::Keyword,
        };
        assert!(!is_generic_plan(&plan));
    }

    #[test]
    fn is_generic_plan_detects_multi_verb_instruction() {
        let plan = planner::Plan {
            instruction: "abre notepad y escribe hola".into(),
            steps: vec![planner::PlanStep {
                id: 1,
                action_type: ActionType::SystemLaunch,
                parameters: [("application".to_string(), "notepad.exe".to_string())].into_iter().collect(),
                description: "Launch Notepad".into(),
                expected_outcome: "Notepad opens".into(),
            }],
            reasoning: None,
            source: planner::PlanSource::Keyword,
        };
        assert!(is_generic_plan(&plan));
    }

    #[test]
    fn is_generic_plan_rejects_multi_step_plan() {
        let plan = planner::Plan {
            instruction: "abre notepad y escribe hola".into(),
            steps: vec![
                planner::PlanStep {
                    id: 1,
                    action_type: ActionType::SystemLaunch,
                    parameters: [("application".to_string(), "notepad.exe".to_string())].into_iter().collect(),
                    description: "Launch Notepad".into(),
                    expected_outcome: "Notepad opens".into(),
                },
                planner::PlanStep {
                    id: 2,
                    action_type: ActionType::KeyboardType,
                    parameters: [("text".to_string(), "hola".to_string())].into_iter().collect(),
                    description: "Type hola".into(),
                    expected_outcome: "Text typed".into(),
                },
            ],
            reasoning: None,
            source: planner::PlanSource::Keyword,
        };
        assert!(!is_generic_plan(&plan));
    }
}
