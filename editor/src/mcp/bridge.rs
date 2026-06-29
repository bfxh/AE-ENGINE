#![allow(dead_code)]

//! Lightweight HTTP bridge that exposes the editor's MCP `MemoryTransport`
//! over HTTP so an external MCP skill (or any HTTP client) can drive the
//! editor remotely. Uses only the standard library — no extra dependencies.
//!
//! Spawn with `McpHttpBridge::start(handle, addr)`. Drop or `stop()` to shut
//! it down. This is a debug-grade bridge: connections are handled sequentially.

use super::transport::MemoryTransportHandle;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct McpHttpBridge {
    stop_flag: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    bound_addr: String,
}

impl McpHttpBridge {
    pub fn start(handle: MemoryTransportHandle, addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        // Non-blocking listener so the accept loop can poll the stop flag.
        listener.set_nonblocking(true)?;
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();
        let bound_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| addr.to_string());
        let thread = thread::spawn(move || run(listener, handle, stop_flag_clone));
        log::info!("MCP HTTP bridge listening on {}", bound_addr);
        Ok(McpHttpBridge {
            stop_flag,
            thread: Some(thread),
            bound_addr,
        })
    }

    /// Returns the address the bridge is bound to (e.g. `127.0.0.1:12345`).
    pub fn bound_addr(&self) -> &str {
        &self.bound_addr
    }

    /// Returns the full URL for the MCP endpoint (e.g. `http://127.0.0.1:12345/mcp`).
    pub fn mcp_url(&self) -> String {
        format!("http://{}/mcp", self.bound_addr)
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for McpHttpBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(listener: TcpListener, handle: MemoryTransportHandle, stop_flag: Arc<AtomicBool>) {
    while !stop_flag.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, addr)) => {
                log::info!("MCP HTTP bridge: connection from {}", addr);
                handle_connection(stream, &handle);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                log::error!("MCP HTTP bridge: accept error: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    log::info!("MCP HTTP bridge: stopped");
}

fn handle_connection(stream: TcpStream, handle: &MemoryTransportHandle) {
    // Ensure blocking I/O with a timeout so a malformed client can't hang us.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            log::error!("MCP HTTP bridge: clone stream failed: {}", e);
            return;
        }
    };
    let mut reader = BufReader::new(stream);

    // --- Request line ---
    let mut request_line = String::new();
    match reader.read_line(&mut request_line) {
        Ok(0) => return, // client closed before sending anything
        Ok(_) => {}
        Err(_) => {
            let _ = write_response(&write_stream, 400, "text/plain", b"Bad Request");
            return;
        }
    }
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        let _ = write_response(&write_stream, 400, "text/plain", b"Bad Request");
        return;
    }
    let method = parts[0];
    let path = parts[1];

    // --- Headers (just enough to find Content-Length) ---
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = header.trim_end_matches(['\r', '\n']);
                if trimmed.is_empty() {
                    break;
                }
                if let Some((key, value)) = trimmed.split_once(':') {
                    if key.trim().eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
            }
            Err(_) => break,
        }
    }

    // --- Body ---
    let mut body = Vec::new();
    if content_length > 0 {
        let _ = reader.take(content_length as u64).read_to_end(&mut body);
    }
    let body_str = String::from_utf8_lossy(&body).into_owned();

    // --- Routing ---
    match (method, path) {
        ("POST", "/mcp") => {
            handle.push_message(&body_str);
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut response: Option<String> = None;
            while Instant::now() < deadline {
                if let Some(r) = handle.pop_response() {
                    response = Some(r);
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
            match response {
                Some(r) => write_response(&write_stream, 200, "text/plain", r.as_bytes()),
                None => write_response(&write_stream, 504, "text/plain", b"Gateway Timeout"),
            }
        }
        ("GET", "/mcp/responses") => {
            let drained = handle.drain_responses();
            let json = to_json_array(&drained);
            write_response(&write_stream, 200, "application/json", json.as_bytes());
        }
        ("GET", "/mcp/status") => {
            let json = format!(
                "{{\"pending_requests\":{},\"pending_responses\":{}}}",
                handle.pending_request_count(),
                handle.pending_response_count()
            );
            write_response(&write_stream, 200, "application/json", json.as_bytes());
        }
        ("GET", "/") => {
            let html = html_status_page();
            write_response(&write_stream, 200, "text/html", html.as_bytes());
        }
        _ => {
            write_response(&write_stream, 404, "text/plain", b"Not Found");
        }
    }
}

fn write_response(stream: &TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        504 => "Gateway Timeout",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        reason,
        content_type,
        body.len()
    );
    let mut s = stream;
    if let Err(e) = s.write_all(header.as_bytes()) {
        log::error!("MCP HTTP bridge: write header failed: {}", e);
        return;
    }
    if let Err(e) = s.write_all(body) {
        log::error!("MCP HTTP bridge: write body failed: {}", e);
        return;
    }
    let _ = s.flush();
}

fn to_json_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&json_escape_string(item));
    }
    out.push(']');
    out
}

fn json_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn html_status_page() -> String {
    "<!DOCTYPE html>\n\
     <html>\n\
     <head><meta charset=\"utf-8\"><title>MCP HTTP Bridge</title></head>\n\
     <body>\n\
     <h1>MCP HTTP Bridge</h1>\n\
     <p>The editor's MCP transport is reachable over HTTP.</p>\n\
     <h2>Endpoints</h2>\n\
     <ul>\n\
     <li><code>POST /mcp</code> — body is a JSON-RPC request; returns one response (504 on timeout).</li>\n\
     <li><code>GET /mcp/responses</code> — drain all pending responses as a JSON array.</li>\n\
     <li><code>GET /mcp/status</code> — pending request/response counts as JSON.</li>\n\
     </ul>\n\
     </body>\n\
     </html>"
        .to_string()
}
