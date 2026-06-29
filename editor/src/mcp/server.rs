//! MCP server core: JSON-RPC 2.0 dispatcher and tool execution.
//!
//! Implements the Model Context Protocol server-side lifecycle:
//!   1. `initialize` — protocol handshake, returns server capabilities
//!   2. `tools/list` — returns the static tool schemas from `tools::list_all_tools`
//!   3. `tools/call` — dispatches to `tools::execute_tool` against the live Scene
//!
//! The server is transport-agnostic: it accepts `McpTransport` instances and
//! processes one request per `poll()` call. The editor's main loop drives
//! `poll()` once per frame so AI requests are serviced without blocking the UI.

use super::scene_snapshot::SceneSnapshot;
use super::tools::{list_all_tools, McpTool};
use super::transport::McpTransport;
use crate::scene::{LightType, NodeType, Scene, SceneNode};
use crate::selection::Selection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Protocol version advertised during `initialize`.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server information returned to the client during handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

impl Default for ServerInfo {
    fn default() -> Self {
        ServerInfo { name: "wasteland-editor-mcp".to_string(), version: "0.1.0".to_string() }
    }
}

/// Capabilities advertised during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub list_changed: bool,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        ServerCapabilities { tools: ToolsCapability { list_changed: false } }
    }
}

/// JSON-RPC 2.0 envelope for requests/responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error() -> Self {
        JsonRpcError { code: -32700, message: "Parse error".into(), data: None }
    }
    pub fn invalid_request() -> Self {
        JsonRpcError { code: -32600, message: "Invalid Request".into(), data: None }
    }
    pub fn method_not_found(method: &str) -> Self {
        JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }
    pub fn invalid_params(msg: &str) -> Self {
        JsonRpcError { code: -32602, message: format!("Invalid params: {}", msg), data: None }
    }
    pub fn internal_error(msg: &str) -> Self {
        JsonRpcError { code: -32603, message: format!("Internal error: {}", msg), data: None }
    }
}

/// Context handed to tool executors so they can mutate editor state safely.
pub struct ToolContext<'a> {
    pub scene: &'a mut Scene,
    pub selection: &'a mut Selection,
    pub scene_path: &'a mut Option<String>,
    pub dirty: &'a mut bool,
}

/// The MCP server. Owns the transport and a buffered response queue.
pub struct McpServer {
    transport: Option<Box<dyn McpTransport>>,
    server_info: ServerInfo,
    capabilities: ServerCapabilities,
    initialized: bool,
    /// Pending responses waiting to be flushed to the transport.
    outbox: Vec<String>,
}

impl McpServer {
    /// Create a server with no transport (AI cannot connect until one is set).
    pub fn new() -> Self {
        McpServer {
            transport: None,
            server_info: ServerInfo::default(),
            capabilities: ServerCapabilities::default(),
            initialized: false,
            outbox: Vec::new(),
        }
    }

    /// Attach a transport (e.g., StdioTransport for CLI, ChannelTransport for in-process).
    pub fn set_transport(&mut self, transport: Box<dyn McpTransport>) {
        self.transport = Some(transport);
        log::info!("MCP transport attached");
    }

    /// Returns true if a transport is attached and the server is initialized.
    pub fn is_connected(&self) -> bool {
        self.transport.is_some() && self.initialized
    }

    /// List the static tool schemas (used by `tools/list`).
    pub fn list_tools(&self) -> Vec<McpTool> {
        list_all_tools()
    }

    /// Drive the server: read incoming requests, dispatch, queue responses.
    /// Call this once per frame from the editor main loop.
    pub fn poll(&mut self, ctx: ToolContext<'_>) {
        // Collect incoming messages in a scoped borrow so `transport` is
        // released before we call `self.handle_message`.
        let inbox: Vec<String> = {
            let transport = match self.transport.as_mut() {
                Some(t) => t,
                None => return,
            };
            let mut inbox = Vec::new();
            while let Some(line) = transport.receive() {
                inbox.push(line);
            }
            inbox
        };

        for line in inbox {
            self.handle_message(&line, ctx.scene, ctx.selection, ctx.scene_path, ctx.dirty);
        }

        // Flush outbox.
        let outbox = std::mem::take(&mut self.outbox);
        for msg in outbox {
            if let Some(transport) = self.transport.as_mut() {
                if let Err(e) = transport.send(&msg) {
                    log::warn!("MCP send failed: {}", e);
                }
            }
        }
    }

    fn handle_message(
        &mut self,
        line: &str,
        scene: &mut Scene,
        selection: &mut Selection,
        scene_path: &mut Option<String>,
        dirty: &mut bool,
    ) {
        let msg: JsonRpcMessage = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(e) => {
                self.queue_error(Value::Null, JsonRpcError::parse_error(), Some(e.to_string()));
                return;
            },
        };

        // Notifications (no id) — silently accept.
        let id = msg.id.clone().unwrap_or(Value::Null);
        let method = match msg.method.as_deref() {
            Some(m) => m,
            None => {
                self.queue_error(id, JsonRpcError::invalid_request(), None);
                return;
            },
        };

        match method {
            "initialize" => self.handle_initialize(id),
            "initialized" => {
                // Notification, no response.
                self.initialized = true;
            },
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, msg.params, scene, selection, scene_path, dirty),
            "ping" => self.queue_result(id, json!({})),
            _ => self.queue_error(id, JsonRpcError::method_not_found(method), None),
        }
    }

    fn handle_initialize(&mut self, id: Value) {
        let result = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": self.capabilities,
            "serverInfo": self.server_info,
        });
        self.queue_result(id, result);
    }

    fn handle_tools_list(&mut self, id: Value) {
        let tools: Vec<Value> = self
            .list_tools()
            .into_iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();
        self.queue_result(id, json!({ "tools": tools }));
    }

    fn handle_tools_call(
        &mut self,
        id: Value,
        params: Option<Value>,
        scene: &mut Scene,
        selection: &mut Selection,
        scene_path: &mut Option<String>,
        dirty: &mut bool,
    ) {
        let params = match params {
            Some(p) => p,
            None => {
                self.queue_error(id, JsonRpcError::invalid_params("missing params"), None);
                return;
            },
        };

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = super::tools::execute_tool(&tool_name, &args, scene, selection, scene_path, dirty);
        let payload = if result.success {
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result.data).unwrap_or_default()
                }],
                "isError": false,
            })
        } else {
            json!({
                "content": [{
                    "type": "text",
                    "text": result.error.unwrap_or_else(|| "Unknown error".into())
                }],
                "isError": true,
            })
        };
        self.queue_result(id, payload);
    }

    fn queue_result(&mut self, id: Value, result: Value) {
        let msg = JsonRpcMessage {
            jsonrpc: "2.0".into(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
            error: None,
        };
        if let Ok(s) = serde_json::to_string(&msg) {
            self.outbox.push(s);
        }
    }

    fn queue_error(&mut self, id: Value, err: JsonRpcError, data: Option<String>) {
        let err = match data {
            Some(d) => JsonRpcError { data: Some(json!({"detail": d})), ..err },
            None => err,
        };
        let msg = JsonRpcMessage {
            jsonrpc: "2.0".into(),
            id: Some(id),
            method: None,
            params: None,
            result: None,
            error: Some(err),
        };
        if let Ok(s) = serde_json::to_string(&msg) {
            self.outbox.push(s);
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a `SceneSnapshot` for `get_scene_tree` responses.
pub fn build_scene_snapshot(scene: &Scene, selection: &Selection) -> SceneSnapshot {
    SceneSnapshot::from_scene(scene, selection.selected_id)
}

/// Helper used by `tools::execute_tool` to construct a node from JSON args.
pub fn node_type_from_args(node_type_str: &str, args: &Value) -> NodeType {
    match node_type_str {
        "mesh" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            NodeType::Mesh { path }
        },
        "light" => {
            let light_kind = args.get("light_type").and_then(|v| v.as_str()).unwrap_or("point");
            let light_type = match light_kind {
                "directional" => LightType::Directional,
                "spot" => LightType::Spot,
                _ => LightType::Point,
            };
            let color = parse_vec3(args.get("color"), [1.0, 1.0, 1.0]);
            let intensity = args.get("intensity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            NodeType::Light { light_type, color, intensity }
        },
        "camera" => {
            let fov = args.get("fov").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
            let near = args.get("near").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
            let far = args.get("far").and_then(|v| v.as_f64()).unwrap_or(1000.0) as f32;
            NodeType::Camera { fov, near, far }
        },
        _ => NodeType::Empty,
    }
}

fn parse_vec3(v: Option<&Value>, default: [f32; 3]) -> glam::Vec3 {
    match v {
        Some(Value::Array(arr)) if arr.len() >= 3 => {
            let x = arr[0].as_f64().unwrap_or(default[0] as f64) as f32;
            let y = arr[1].as_f64().unwrap_or(default[1] as f64) as f32;
            let z = arr[2].as_f64().unwrap_or(default[2] as f64) as f32;
            glam::Vec3::new(x, y, z)
        },
        _ => glam::Vec3::from(default),
    }
}

#[allow(dead_code)]
fn unused_scene_node_marker(_n: &SceneNode) {}
