/// IPC server module for JSON-RPC 2.0 communication with the GUI.
///
/// Listens on a TCP port and handles newline-delimited JSON-RPC messages.
/// Phase 9: abort flag, confirmation gate, and rate limiting support.
use common::{LapisError, LapisResult};
use config::AppConfig;
use perception::VlmConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;

// ── JSON-RPC 2.0 types ──

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// A JSON-RPC notification (no id, server→client).
#[derive(Debug, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError { code, message }),
            id,
        }
    }
}

// ── Shared state for abort and confirmation ──

/// Shared state across connections for abort and confirmation mechanisms.
struct SharedState {
    config: Mutex<AppConfig>,
    abort_flag: AtomicBool,
    /// Pending confirmation sender. When the orchestrator needs confirmation,
    /// it stores a oneshot::Sender<bool> here. The GUI responds via agent.confirm_execution.
    confirm_sender: Mutex<Option<tokio::sync::oneshot::Sender<bool>>>,
}

// ── IPC Server ──

pub struct IpcServer {
    port: u16,
    state: Arc<SharedState>,
}

impl IpcServer {
    pub fn new(config: AppConfig) -> LapisResult<Self> {
        let port = config.ipc_port;
        Ok(Self {
            port,
            state: Arc::new(SharedState {
                config: Mutex::new(config),
                abort_flag: AtomicBool::new(false),
                confirm_sender: Mutex::new(None),
            }),
        })
    }

    /// Start listening and handle connections.
    pub async fn run(&self) -> LapisResult<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| LapisError::Ipc(format!("failed to bind {addr}: {e}")))?;

        println!("[ipc] Listening on {addr}");

        loop {
            let (stream, peer) = listener
                .accept()
                .await
                .map_err(|e| LapisError::Ipc(format!("accept error: {e}")))?;

            println!("[ipc] Client connected: {peer}");
            let state = Arc::clone(&self.state);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, &state).await {
                    eprintln!("[ipc] Connection error ({peer}): {e}");
                }
                println!("[ipc] Client disconnected: {peer}");
            });
        }
    }
}

/// Handle a single client connection: read lines, parse JSON-RPC, dispatch.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    state: &Arc<SharedState>,
) -> LapisResult<()> {
    let (reader, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| LapisError::Ipc(format!("read error: {e}")))?
    {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {e}"),
                );
                send_message(&writer, &err_resp).await;
                continue;
            }
        };

        let response = dispatch_request(&request, state, &writer).await;

        // Some methods (agent.instruct) send their own response asynchronously
        if let Some(resp) = response {
            send_message(&writer, &resp).await;
        }
    }

    Ok(())
}

/// Send a serializable message through the writer.
async fn send_message<T: Serialize>(
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    msg: &T,
) {
    let mut out = serde_json::to_string(msg).unwrap_or_default();
    out.push('\n');
    let mut w = writer.lock().unwrap_or_else(|e| e.into_inner());
    let bytes = out.as_bytes();
    let _ = std::io::Write::write_all(&mut WriterAdapter(&mut *w), bytes);
}

/// Adapter to use tokio OwnedWriteHalf with std::io::Write.
struct WriterAdapter<'a>(&'a mut tokio::net::tcp::OwnedWriteHalf);

impl<'a> std::io::Write for WriterAdapter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.0.try_write(buf) {
            Ok(n) => Ok(n),
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Route a JSON-RPC request to the appropriate handler.
/// Returns `None` for async methods that send their own response (e.g., agent.instruct).
async fn dispatch_request(
    req: &JsonRpcRequest,
    state: &Arc<SharedState>,
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Option<JsonRpcResponse> {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "agent.instruct" => {
            // Spawn instruct in a separate task so the read loop continues
            // processing messages (needed for agent.confirm_execution, agent.abort).
            let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let params_owned = req.params.clone();
            let state_clone = Arc::clone(state);
            let writer_clone = Arc::clone(writer);
            tokio::spawn(async move {
                let response = handle_instruct(id, &params_owned, &cfg, &state_clone, &writer_clone).await;
                send_message(&writer_clone, &response).await;
            });
            None // response sent by the spawned task
        }
        "agent.ping" => Some(JsonRpcResponse::success(id, serde_json::json!({"pong": true}))),
        "agent.status" => {
            let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
            Some(JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "simulation_mode": cfg.simulation_mode,
                    "version": "0.1.0",
                    "vlm_endpoint": cfg.vlm_endpoint,
                    "vlm_model": cfg.vlm_model,
                    "reasoning_provider": cfg.reasoning_provider,
                    "reasoning_model": cfg.reasoning_model,
                    "reasoning_configured": !cfg.reasoning_api_key.is_empty(),
                }),
            ))
        }
        "agent.screenshot" => Some(handle_screenshot(id)),
        "agent.configure" => Some(handle_configure(id, &req.params, state)),
        "agent.configure_reasoning" => Some(handle_configure_reasoning(id, &req.params, state)),
        "agent.analyze_screen" => Some(handle_analyze_screen(id, &req.params, state).await),
        "agent.abort" => {
            state.abort_flag.store(true, Ordering::Relaxed);
            println!("[ipc] Abort flag set by client");
            Some(JsonRpcResponse::success(id, serde_json::json!({"aborted": true})))
        }
        "agent.confirm_execution" => {
            let confirmed = req.params
                .as_ref()
                .and_then(|p| p.get("confirmed"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut sender_guard = state.confirm_sender.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(sender) = sender_guard.take() {
                let _ = sender.send(confirmed);
                println!("[ipc] Execution confirmation: {confirmed}");
                Some(JsonRpcResponse::success(id, serde_json::json!({"acknowledged": true})))
            } else {
                Some(JsonRpcResponse::error(id, -32000, "No pending confirmation".into()))
            }
        }
        _ => Some(JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method))),
    }
}

/// Handle the `agent.configure` method — update VLM/simulation config from GUI settings.
fn handle_configure(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    state: &Arc<SharedState>,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, -32602, "Missing parameters".into()),
    };

    let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(endpoint) = params.get("vlm_endpoint").and_then(|v| v.as_str()) {
        cfg.vlm_endpoint = endpoint.to_string();
        println!("[ipc] VLM endpoint updated: {endpoint}");
    }
    if let Some(model) = params.get("vlm_model").and_then(|v| v.as_str()) {
        cfg.vlm_model = model.to_string();
        println!("[ipc] VLM model updated: {model}");
    }
    if let Some(sim) = params.get("simulation_mode").and_then(|v| v.as_bool()) {
        cfg.simulation_mode = sim;
        println!("[ipc] Simulation mode: {sim}");
    }
    if let Some(rate) = params.get("rate_limit_ms").and_then(|v| v.as_u64()) {
        cfg.rate_limit_ms = rate;
        println!("[ipc] Rate limit: {rate}ms");
    }

    JsonRpcResponse::success(id, serde_json::json!({"configured": true}))
}

/// Handle the `agent.configure_reasoning` method — update LLM reasoning config.
fn handle_configure_reasoning(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    state: &Arc<SharedState>,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, -32602, "Missing parameters".into()),
    };

    let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(provider) = params.get("provider").and_then(|v| v.as_str()) {
        cfg.reasoning_provider = provider.to_lowercase();
        cfg.reasoning_endpoint = match cfg.reasoning_provider.as_str() {
            "claude" => "https://api.anthropic.com".to_string(),
            "openai" => "https://api.openai.com".to_string(),
            "gemini" => "https://generativelanguage.googleapis.com".to_string(),
            _ => cfg.reasoning_endpoint.clone(),
        };
        println!("[ipc] Reasoning provider: {} (endpoint: {})", cfg.reasoning_provider, cfg.reasoning_endpoint);
    }
    if let Some(key) = params.get("api_key").and_then(|v| v.as_str()) {
        cfg.reasoning_api_key = key.to_string();
        let masked = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len()-4..])
        } else {
            "***".to_string()
        };
        println!("[ipc] Reasoning API key set: {masked}");
    }
    if let Some(model) = params.get("model").and_then(|v| v.as_str()) {
        cfg.reasoning_model = model.to_string();
        println!("[ipc] Reasoning model: {model}");
    }
    if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
        cfg.reasoning_endpoint = endpoint.to_string();
        println!("[ipc] Reasoning endpoint override: {endpoint}");
    }

    JsonRpcResponse::success(id, serde_json::json!({
        "configured": true,
        "provider": cfg.reasoning_provider,
        "model": cfg.reasoning_model,
        "has_api_key": !cfg.reasoning_api_key.is_empty(),
    }))
}

/// Handle the `agent.analyze_screen` method — capture screenshot + send to VLM.
async fn handle_analyze_screen(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    state: &Arc<SharedState>,
) -> JsonRpcResponse {
    let prompt = params
        .as_ref()
        .and_then(|p| p.get("prompt"))
        .and_then(|v| v.as_str())
        .unwrap_or("Describe what you see on the screen in detail. Include any visible windows, text, buttons, and UI elements.");

    let vlm_cfg = {
        let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        VlmConfig {
            endpoint: cfg.vlm_endpoint.clone(),
            model: cfg.vlm_model.clone(),
        }
    };

    println!("[ipc] Analyzing screen with VLM: {} ({})", vlm_cfg.model, vlm_cfg.endpoint);

    match perception::capture_and_analyze(&vlm_cfg, prompt).await {
        Ok((screenshot, analysis)) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "width": screenshot.width,
                "height": screenshot.height,
                "description": analysis.description,
                "model": analysis.model,
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Analysis error: {e}")),
    }
}

/// Helper: build a step notification sender closure.
fn make_step_sender(
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> impl FnMut(orchestrator::StepNotification) {
    let w = Arc::clone(writer);
    move |step_notif| {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "agent.step_progress".into(),
            params: serde_json::to_value(&step_notif).unwrap_or_default(),
        };
        let mut out = serde_json::to_string(&notification).unwrap_or_default();
        out.push('\n');
        let mut guard = w.lock().unwrap_or_else(|e| e.into_inner());
        let _ = std::io::Write::write_all(
            &mut WriterAdapter(&mut *guard),
            out.as_bytes(),
        );
    }
}

/// Handle the `agent.instruct` method, sending step notifications.
/// Uses iterative VLM loop for generic/complex instructions, static planner for known patterns.
async fn handle_instruct(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    config: &AppConfig,
    state: &Arc<SharedState>,
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> JsonRpcResponse {
    let text = params
        .as_ref()
        .and_then(|p| p.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if text.is_empty() {
        return JsonRpcResponse::error(id, -32602, "Missing 'text' parameter".into());
    }

    let screen_context = params
        .as_ref()
        .and_then(|p| p.get("screen_context"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Reset abort flag at start of instruction
    state.abort_flag.store(false, Ordering::Relaxed);

    // Create abort proxy that watches the shared state abort flag
    let abort_proxy = Arc::new(AtomicBool::new(false));
    let abort_proxy_clone = Arc::clone(&abort_proxy);
    let state_for_abort = Arc::clone(state);
    let abort_watcher = tokio::spawn(async move {
        loop {
            if state_for_abort.abort_flag.load(Ordering::Relaxed) {
                abort_proxy_clone.store(true, Ordering::Relaxed);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });

    // First, try to create a plan to see if keyword/LLM can handle it
    let plan = planner::create_plan(text, config, screen_context.as_deref()).await;

    // Check if the plan is "generic" (keyword fallback couldn't handle it)
    let use_iterative = match &plan {
        Ok(p) => orchestrator::is_generic_plan(p),
        Err(_) => true, // if planning fails, try iterative
    };

    let result = if use_iterative {
        println!("[ipc] Using iterative VLM loop for: {text}");

        // For iterative mode, request confirmation first if in real mode
        if !config.simulation_mode {
            let confirmation = orchestrator::PlanConfirmation {
                instruction: text.to_string(),
                steps: vec![orchestrator::PlanStepSummary {
                    id: 1,
                    description: format!("VLM-guided execution: {text}"),
                    action_type: "Iterative".into(),
                }],
                source: "VLM".into(),
                reasoning: Some("Using vision model to guide actions step by step".into()),
            };

            let (tx, rx) = tokio::sync::oneshot::channel();
            {
                let mut sender_guard = state.confirm_sender.lock().unwrap_or_else(|e| e.into_inner());
                *sender_guard = Some(tx);
            }
            let notification = JsonRpcNotification {
                jsonrpc: "2.0".into(),
                method: "agent.confirm_plan".into(),
                params: serde_json::to_value(&confirmation).unwrap_or_default(),
            };
            let mut out = serde_json::to_string(&notification).unwrap_or_default();
            out.push('\n');
            {
                let mut w = writer.lock().unwrap_or_else(|e| e.into_inner());
                let _ = std::io::Write::write_all(&mut WriterAdapter(&mut *w), out.as_bytes());
            }

            // Send awaiting notification
            let mut on_step = make_step_sender(writer);
            on_step(orchestrator::StepNotification {
                step_id: 0,
                total_steps: 0,
                description: "Awaiting user confirmation for real execution".into(),
                status: orchestrator::StepStatus::AwaitingConfirmation,
                result: None,
            });

            match rx.await {
                Ok(true) => { /* confirmed */ }
                Ok(false) => {
                    abort_watcher.abort();
                    return JsonRpcResponse::success(id, serde_json::json!({
                        "summary": format!("Cancelled '{}' — user declined execution", text)
                    }));
                }
                Err(_) => {
                    abort_watcher.abort();
                    return JsonRpcResponse::success(id, serde_json::json!({
                        "summary": format!("Cancelled '{}' — confirmation channel closed", text)
                    }));
                }
            }
        }

        let on_step = make_step_sender(writer);
        orchestrator::handle_instruction_iterative(
            text,
            config,
            Arc::clone(&abort_proxy),
            on_step,
        ).await
    } else {
        // Standard path: keyword/LLM plan execution
        let writer_for_confirm = Arc::clone(writer);
        let confirm_fn = if !config.simulation_mode {
            let state_for_confirm = Arc::clone(state);
            Some(move |plan: orchestrator::PlanConfirmation| -> tokio::sync::oneshot::Receiver<bool> {
                let (tx, rx) = tokio::sync::oneshot::channel();
                {
                    let mut sender_guard = state_for_confirm.confirm_sender.lock().unwrap_or_else(|e| e.into_inner());
                    *sender_guard = Some(tx);
                }
                let notification = JsonRpcNotification {
                    jsonrpc: "2.0".into(),
                    method: "agent.confirm_plan".into(),
                    params: serde_json::to_value(&plan).unwrap_or_default(),
                };
                let mut out = serde_json::to_string(&notification).unwrap_or_default();
                out.push('\n');
                let mut w = writer_for_confirm.lock().unwrap_or_else(|e| e.into_inner());
                let _ = std::io::Write::write_all(
                    &mut WriterAdapter(&mut *w),
                    out.as_bytes(),
                );
                rx
            })
        } else {
            None
        };

        let on_step = make_step_sender(writer);
        orchestrator::handle_instruction(
            text,
            config,
            screen_context.as_deref(),
            Arc::clone(&abort_proxy),
            on_step,
            confirm_fn,
        )
        .await
    };

    abort_watcher.abort(); // clean up the watcher task

    match result {
        Ok(summary) => JsonRpcResponse::success(id, serde_json::json!({"summary": summary})),
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Agent error: {e}")),
    }
}

/// Handle the `agent.screenshot` method — capture and return base64 PNG.
fn handle_screenshot(id: serde_json::Value) -> JsonRpcResponse {
    match perception::capture_screen() {
        Ok(screenshot) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "width": screenshot.width,
                "height": screenshot.height,
                "png_base64": screenshot.png_base64,
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Screenshot error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_creates_successfully() {
        assert!(IpcServer::new(AppConfig::default()).is_ok());
    }

    #[test]
    fn dispatch_formats_notification() {
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "agent.step_progress".into(),
            params: serde_json::json!({"step_id": 1}),
        };
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("agent.step_progress"));
        assert!(!json.contains("\"id\""));
    }
}
