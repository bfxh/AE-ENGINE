//! Integration tests for the MCP HTTP bridge.
//!
//! Part 1: HTTP transport tests — start the bridge on an ephemeral port and
//! exercise the GET endpoints (status, responses, index, 404).
//! Part 2: End-to-end pipeline tests — push JSON-RPC requests through the
//! MemoryTransport, poll the McpServer, and verify responses come back.

use slime_editor::mcp::bridge::McpHttpBridge;
use slime_editor::mcp::server::{McpServer, ToolContext};
use slime_editor::mcp::transport::MemoryTransport;
use slime_editor::scene::Scene;
use slime_editor::selection::Selection;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

/// Read the full HTTP response body from a TCP stream (blocking, 5s timeout).
fn http_get(url_path: &str, host: &str, port: u16) -> (u16, String) {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().expect("valid addr"),
        Duration::from_secs(2),
    )
    .expect("connect to bridge");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url_path, addr
    );
    use std::io::Write;
    stream.write_all(request.as_bytes()).unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf).into_owned();

    let status_line = response.lines().next().unwrap_or("");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .to_string();
    (status_code, body)
}

/// Extract the port from a "host:port" string.
fn parse_port(bound_addr: &str) -> u16 {
    bound_addr
        .rsplit(':')
        .next()
        .and_then(|s| s.parse().ok())
        .expect("valid port in bound_addr")
}

#[test]
fn test_bridge_starts_with_valid_bound_addr() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");

    let bound = bridge.bound_addr().to_string();
    assert!(bound.starts_with("127.0.0.1:"), "bound_addr = {}", bound);

    let port = parse_port(&bound);
    assert!(port > 0, "port should be non-zero, got {}", port);

    // Drop transport handle (kept alive by bridge via internal clone).
    drop(transport);

    bridge.stop();
}

#[test]
fn test_bridge_status_endpoint() {
    let (_transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");
    let port = parse_port(bridge.bound_addr());

    // Give the bridge a moment to start accepting.
    std::thread::sleep(Duration::from_millis(50));

    let (code, body) = http_get("/mcp/status", "127.0.0.1", port);
    assert_eq!(code, 200, "status endpoint should return 200, got {}: {}", code, body);
    assert!(
        body.contains("pending_requests"),
        "status body should contain pending_requests: {}",
        body
    );
    assert!(
        body.contains("pending_responses"),
        "status body should contain pending_responses: {}",
        body
    );

    bridge.stop();
}

#[test]
fn test_bridge_responses_endpoint_returns_array() {
    let (_transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");
    let port = parse_port(bridge.bound_addr());

    std::thread::sleep(Duration::from_millis(50));

    let (code, body) = http_get("/mcp/responses", "127.0.0.1", port);
    assert_eq!(code, 200, "responses endpoint should return 200: {}", body);
    assert!(
        body.trim_start().starts_with('['),
        "responses body should be a JSON array: {}",
        body
    );

    bridge.stop();
}

#[test]
fn test_bridge_index_returns_html() {
    let (_transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");
    let port = parse_port(bridge.bound_addr());

    std::thread::sleep(Duration::from_millis(50));

    let (code, body) = http_get("/", "127.0.0.1", port);
    assert_eq!(code, 200, "index endpoint should return 200: {}", body);
    assert!(
        body.contains("<html") || body.contains("<h1>"),
        "index body should be HTML: {}",
        body
    );
    assert!(
        body.contains("MCP HTTP Bridge"),
        "index body should mention 'MCP HTTP Bridge': {}",
        body
    );

    bridge.stop();
}

#[test]
fn test_bridge_404_for_unknown_path() {
    let (_transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");
    let port = parse_port(bridge.bound_addr());

    std::thread::sleep(Duration::from_millis(50));

    let (code, _body) = http_get("/nonexistent", "127.0.0.1", port);
    assert_eq!(code, 404, "unknown path should return 404");

    bridge.stop();
}

#[test]
fn test_bridge_mcp_url_accessor() {
    let (_transport, handle) = MemoryTransport::new_with_handle();
    let mut bridge = McpHttpBridge::start(handle, "127.0.0.1:0").expect("bridge starts");

    let url = bridge.mcp_url();
    assert!(
        url.starts_with("http://127.0.0.1:") && url.ends_with("/mcp"),
        "mcp_url should be http://127.0.0.1:<port>/mcp, got {}",
        url
    );

    bridge.stop();
}

// ============================================================
// End-to-end pipeline tests: transport → server.poll() → tool → response
// ============================================================

#[test]
fn test_transport_sharing() {
    // Verify that MemoryTransport and MemoryTransportHandle share state.
    let (transport, handle) = MemoryTransport::new_with_handle();
    handle.push_message("test_message");

    let mut boxed: Box<dyn slime_editor::mcp::transport::McpTransport> = Box::new(transport);
    let msg = boxed.receive();
    assert_eq!(msg.as_deref(), Some("test_message"));

    boxed.send("test_response").unwrap();
    let resp = handle.pop_response();
    assert_eq!(resp.as_deref(), Some("test_response"));
}

#[test]
fn test_server_poll_consumes_and_responds() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    handle.push_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    assert_eq!(handle.pending_request_count(), 1);

    let mut server = McpServer::new();
    server.set_transport(Box::new(transport));

    let mut scene = Scene::new_empty();
    let mut selection = Selection::new();
    let mut path: Option<String> = None;
    let mut dirty = false;
    server.poll(ToolContext {
        scene: &mut scene,
        selection: &mut selection,
        scene_path: &mut path,
        dirty: &mut dirty,
    });

    assert_eq!(handle.pending_request_count(), 0, "request should be consumed");
    assert_eq!(
        handle.pending_response_count(),
        1,
        "should have 1 pending response"
    );
}

#[test]
fn test_end_to_end_get_scene_tree() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    let mut server = McpServer::new();
    server.set_transport(Box::new(transport));

    // Push a JSON-RPC request directly via the handle (bypassing HTTP).
    handle.push_message(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_scene_tree","arguments":{}}}"#);

    let mut scene = Scene::new_empty();
    let mut selection = Selection::new();
    let mut path: Option<String> = None;
    let mut dirty = false;

    // Poll the server once — it should process the request and queue a response.
    server.poll(ToolContext {
        scene: &mut scene,
        selection: &mut selection,
        scene_path: &mut path,
        dirty: &mut dirty,
    });

    // The response should now be available via the handle.
    let response = handle.pop_response();
    assert!(response.is_some(), "server should have queued a response");
    let resp = response.unwrap();
    assert!(
        resp.contains("\"result\""),
        "response should contain result: {}",
        resp
    );
    assert!(
        !resp.contains("\"error\""),
        "response should not contain error: {}",
        resp
    );
}

#[test]
fn test_end_to_end_create_node() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    let mut server = McpServer::new();
    server.set_transport(Box::new(transport));

    handle.push_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_node","arguments":{"parent_id":0,"name":"TestCube","node_type":"mesh","path":"cube.glb"}}}"#);

    let mut scene = Scene::new_empty();
    let mut selection = Selection::new();
    let mut path: Option<String> = None;
    let mut dirty = false;

    server.poll(ToolContext {
        scene: &mut scene,
        selection: &mut selection,
        scene_path: &mut path,
        dirty: &mut dirty,
    });

    let response = handle.pop_response();
    assert!(response.is_some(), "server should have queued a response");
    let resp = response.unwrap();
    assert!(
        resp.contains("TestCube"),
        "response should contain node name: {}",
        resp
    );
    assert!(dirty, "dirty flag should be set after create_node");
    assert_eq!(
        scene.nodes.len(),
        2,
        "scene should have root + 1 child after create_node"
    );
}

#[test]
fn test_end_to_end_initialize_handshake() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    let mut server = McpServer::new();
    server.set_transport(Box::new(transport));

    handle.push_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);

    let mut scene = Scene::new_empty();
    let mut selection = Selection::new();
    let mut path: Option<String> = None;
    let mut dirty = false;

    server.poll(ToolContext {
        scene: &mut scene,
        selection: &mut selection,
        scene_path: &mut path,
        dirty: &mut dirty,
    });

    let response = handle.pop_response();
    assert!(response.is_some(), "server should have queued a response");
    let resp = response.unwrap();
    assert!(
        resp.contains("protocolVersion"),
        "initialize response should contain protocolVersion: {}",
        resp
    );
    assert!(
        resp.contains("wasteland-editor-mcp"),
        "initialize response should contain server name: {}",
        resp
    );
}

#[test]
fn test_end_to_end_multiple_requests_in_one_poll() {
    let (transport, handle) = MemoryTransport::new_with_handle();
    let mut server = McpServer::new();
    server.set_transport(Box::new(transport));

    // Push 3 requests before polling.
    handle.push_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    handle.push_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    handle.push_message(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_scene_tree","arguments":{}}}"#);

    let mut scene = Scene::new_empty();
    let mut selection = Selection::new();
    let mut path: Option<String> = None;
    let mut dirty = false;

    server.poll(ToolContext {
        scene: &mut scene,
        selection: &mut selection,
        scene_path: &mut path,
        dirty: &mut dirty,
    });

    // All 3 responses should be available, in order.
    let r1 = handle.pop_response().expect("response 1");
    let r2 = handle.pop_response().expect("response 2");
    let r3 = handle.pop_response().expect("response 3");
    assert!(r1.contains("protocolVersion"), "r1 should be initialize: {}", r1);
    assert!(r2.contains("tools"), "r2 should be tools/list: {}", r2);
    assert!(r3.contains("result"), "r3 should be get_scene_tree: {}", r3);
    assert!(handle.pop_response().is_none(), "no more responses expected");
}

