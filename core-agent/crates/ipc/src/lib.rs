/// IPC server module for JSON-RPC 2.0 communication with the GUI.
///
/// Listens on a TCP port and handles newline-delimited JSON-RPC messages.
use common::{LapisError, LapisResult};
use config::AppConfig;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    let (reader, mut writer) = stream.into_split();
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
                let mut out = serde_json::to_string(&err_resp).unwrap_or_default();
                out.push('\n');
                let _ = writer.write_all(out.as_bytes()).await;
                continue;
            }
        };

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);
        let response = dispatch_request(&request, config);

        let mut out = serde_json::to_string(&response).unwrap_or_default();
        out.push('\n');
        writer
            .write_all(out.as_bytes())
            .await
            .map_err(|e| LapisError::Ipc(format!("write error: {e}")))?;

        // If this was a notification (no id), don't expect ack
        let _ = id;
    }

    Ok(())
}

/// Route a JSON-RPC request to the appropriate handler.
fn dispatch_request(req: &JsonRpcRequest, config: &AppConfig) -> JsonRpcResponse {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "agent.instruct" => handle_instruct(id, &req.params, config),
        "agent.ping" => JsonRpcResponse::success(id, serde_json::json!({"pong": true})),
        "agent.status" => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "simulation_mode": config.simulation_mode,
                "version": "0.1.0"
            }),
        ),
        _ => JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method)),
    }
}

/// Handle the `agent.instruct` method.
fn handle_instruct(
    id: serde_json::Value,
    params: &Option<serde_json::Value>,
    config: &AppConfig,
) -> JsonRpcResponse {
    let text = params
        .as_ref()
        .and_then(|p| p.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if text.is_empty() {
        return JsonRpcResponse::error(id, -32602, "Missing 'text' parameter".into());
    }

    match orchestrator::handle_instruction(text, config) {
        Ok(summary) => JsonRpcResponse::success(id, serde_json::json!({"summary": summary})),
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Agent error: {e}")),
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
    fn dispatch_ping() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "agent.ping".into(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let resp = dispatch_request(&req, &AppConfig::default());
        assert!(resp.result.is_some());
    }

    #[test]
    fn dispatch_unknown_method() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "unknown".into(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let resp = dispatch_request(&req, &AppConfig::default());
        assert!(resp.error.is_some());
    }

    #[test]
    fn dispatch_instruct() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "agent.instruct".into(),
            params: Some(serde_json::json!({"text": "hello"})),
            id: Some(serde_json::json!(1)),
        };
        let resp = dispatch_request(&req, &AppConfig::default());
        assert!(resp.result.is_some());
    }
}
