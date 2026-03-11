/// IPC server module for JSON-RPC 2.0 communication with the GUI.
///
/// Listens on a TCP port and handles newline-delimited JSON-RPC messages.
/// Phase 7: supports LLM reasoning configuration and async instruction handling.
use common::{LapisError, LapisResult};
use config::AppConfig;
use perception::VlmConfig;
use serde::{Deserialize, Serialize};
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

// ── IPC Server ──

pub struct IpcServer {
    port: u16,
    config: Arc<Mutex<AppConfig>>,
}

impl IpcServer {
    pub fn new(config: AppConfig) -> LapisResult<Self> {
        let port = config.ipc_port;
        Ok(Self {
            port,
            config: Arc::new(Mutex::new(config)),
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
            let config = Arc::clone(&self.config);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, &config).await {
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
    config: &Arc<Mutex<AppConfig>>,
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

        let response = dispatch_request(&request, config, &writer).await;

        send_message(&writer, &response).await;
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
async fn dispatch_request(
    req: &JsonRpcRequest,
    config: &Arc<Mutex<AppConfig>>,
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> JsonRpcResponse {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "agent.instruct" => {
            let cfg = config.lock().unwrap_or_else(|e| e.into_inner()).clone();
            handle_instruct(id, &req.params, &cfg, writer).await
        }
        "agent.ping" => JsonRpcResponse::success(id, serde_json::json!({"pong": true})),
        "agent.status" => {
            let cfg = config.lock().unwrap_or_else(|e| e.into_inner());
            JsonRpcResponse::success(
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
            )
        }
        "agent.screenshot" => handle_screenshot(id),
        "agent.configure" => handle_configure(id, &req.params, config),
        "agent.configure_reasoning" => handle_configure_reasoning(id, &req.params, config),
        "agent.analyze_screen" => handle_analyze_screen(id, &req.params, config).await,
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method)),
    }
}

/// Handle the `agent.configure` method — update VLM/simulation config from GUI settings.
fn handle_configure(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    config: &Arc<Mutex<AppConfig>>,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, -32602, "Missing parameters".into()),
    };

    let mut cfg = config.lock().unwrap_or_else(|e| e.into_inner());

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

    JsonRpcResponse::success(id, serde_json::json!({"configured": true}))
}

/// Handle the `agent.configure_reasoning` method — update LLM reasoning config.
fn handle_configure_reasoning(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    config: &Arc<Mutex<AppConfig>>,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, -32602, "Missing parameters".into()),
    };

    let mut cfg = config.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(provider) = params.get("provider").and_then(|v| v.as_str()) {
        cfg.reasoning_provider = provider.to_lowercase();
        // Auto-set endpoint per provider
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
    config: &Arc<Mutex<AppConfig>>,
) -> JsonRpcResponse {
    let prompt = params
        .as_ref()
        .and_then(|p| p.get("prompt"))
        .and_then(|v| v.as_str())
        .unwrap_or("Describe what you see on the screen in detail. Include any visible windows, text, buttons, and UI elements.");

    let vlm_cfg = {
        let cfg = config.lock().unwrap_or_else(|e| e.into_inner());
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

/// Handle the `agent.instruct` method, sending step notifications.
async fn handle_instruct(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    config: &AppConfig,
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

    // Optionally get screen context from params (GUI can send VLM description)
    let screen_context = params
        .as_ref()
        .and_then(|p| p.get("screen_context"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let writer_clone = Arc::clone(writer);

    match orchestrator::handle_instruction(
        text,
        config,
        screen_context.as_deref(),
        move |step_notif| {
            let notification = JsonRpcNotification {
                jsonrpc: "2.0".into(),
                method: "agent.step_progress".into(),
                params: serde_json::to_value(&step_notif).unwrap_or_default(),
            };
            let mut out = serde_json::to_string(&notification).unwrap_or_default();
            out.push('\n');
            let mut w = writer_clone.lock().unwrap_or_else(|e| e.into_inner());
            let _ = std::io::Write::write_all(
                &mut WriterAdapter(&mut *w),
                out.as_bytes(),
            );
        },
    )
    .await
    {
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
