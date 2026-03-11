/// IPC server module for JSON-RPC 2.0 communication with the GUI.
///
/// Listens on a TCP port and handles newline-delimited JSON-RPC messages.
/// Phase 4: supports pushing step notifications during instruction execution.
use common::{LapisError, LapisResult};
use config::AppConfig;
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
    config: AppConfig,
}

impl IpcServer {
    pub fn new(config: AppConfig) -> LapisResult<Self> {
        let port = config.ipc_port;
        Ok(Self { port, config })
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
            let config = self.config.clone();

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
    config: &AppConfig,
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
    // Use a blocking write since we hold the mutex briefly
    let bytes = out.as_bytes();
    let _ = std::io::Write::write_all(&mut WriterAdapter(&mut *w), bytes);
}

/// Adapter to use tokio OwnedWriteHalf with std::io::Write.
/// We buffer into a Vec and then write synchronously since we're in a mutex.
struct WriterAdapter<'a>(&'a mut tokio::net::tcp::OwnedWriteHalf);

impl<'a> std::io::Write for WriterAdapter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Use try_write for non-blocking TCP write
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
    config: &AppConfig,
    writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> JsonRpcResponse {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "agent.instruct" => handle_instruct(id, &req.params, config, writer).await,
        "agent.ping" => JsonRpcResponse::success(id, serde_json::json!({"pong": true})),
        "agent.status" => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "simulation_mode": config.simulation_mode,
                "version": "0.1.0"
            }),
        ),
        "agent.screenshot" => handle_screenshot(id),
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method)),
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

    // Collect notifications to send after each step
    let writer_clone = Arc::clone(writer);

    match orchestrator::handle_instruction(text, config, move |step_notif| {
        // Send notification immediately
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
    }) {
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
