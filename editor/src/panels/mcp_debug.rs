//! MCP debug panel: inspect and drive the editor's MCP (JSON-RPC) server.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// Debug panel for manually sending JSON-RPC requests to the MCP server
/// and inspecting responses.
pub struct McpDebugPanel {
    /// Whether the window is currently shown.
    pub visible: bool,
    /// User-editable JSON-RPC request buffer (multi-line).
    pub request_buffer: String,
    /// Accumulated response text (read-only display).
    pub response_display: String,
    /// When true, drain responses into the display every frame.
    pub auto_drain: bool,
}

impl Default for McpDebugPanel {
    fn default() -> Self {
        Self {
            visible: false,
            request_buffer: String::from(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
            ),
            response_display: String::new(),
            auto_drain: true,
        }
    }
}

impl McpDebugPanel {
    fn fill(&mut self, payload: &str) {
        self.request_buffer.clear();
        self.request_buffer.push_str(payload);
    }

    fn append_responses(&mut self, responses: Vec<String>) {
        for r in responses {
            self.response_display.push_str(&r);
            self.response_display.push('\n');
        }
    }
}

impl EditorPanel for McpDebugPanel {
    fn name(&self) -> &str {
        "MCP Debug"
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible {
            return;
        }

        // Snapshot status before draining so the indicators reflect this frame.
        let connected = app.mcp_server.is_connected();
        let (pending_req, pending_resp) = match &app.mcp_transport_handle {
            Some(h) => (h.pending_request_count(), h.pending_response_count()),
            None => (0usize, 0usize),
        };
        // Snapshot the HTTP bridge URL (if running) so it can be shown in the
        // status bar without borrowing `app` inside the egui closure.
        let bridge_url: Option<String> =
            app.mcp_http_bridge.as_ref().map(|b| b.mcp_url());
        let port_file_path = std::env::temp_dir()
            .join("ae_editor_mcp_port.txt")
            .to_string_lossy()
            .into_owned();

        if self.auto_drain {
            let responses = app.drain_mcp_responses();
            if !responses.is_empty() {
                self.append_responses(responses);
            }
        }

        egui::Window::new("MCP Debug")
            .default_width(640.0)
            .default_height(420.0)
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                // --- Status bar ---
                let conn_color = if connected {
                    egui::Color32::from_rgb(100, 220, 100)
                } else {
                    egui::Color32::from_rgb(220, 100, 100)
                };
                let conn_label = if connected { "Connected" } else { "Disconnected" };

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("\u{25CF}").color(conn_color));
                    ui.label(egui::RichText::new(conn_label).color(conn_color).strong());
                    ui.separator();
                    ui.label(format!("Pending requests: {}", pending_req));
                    ui.label(format!("Pending responses: {}", pending_resp));
                    ui.separator();
                    ui.checkbox(&mut self.auto_drain, "Auto-drain");
                    if ui.button("Drain Responses").clicked() {
                        let responses = app.drain_mcp_responses();
                        if responses.is_empty() {
                            self.response_display.push_str("(no responses)\n");
                        } else {
                            self.append_responses(responses);
                        }
                    }
                    if ui.button("Clear Responses").clicked() {
                        self.response_display.clear();
                    }
                });

                // --- HTTP bridge endpoint (for external AI clients) ---
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Bridge:")
                            .small()
                            .color(egui::Color32::from_rgb(150, 150, 160)),
                    );
                    match &bridge_url {
                        Some(url) => {
                            ui.label(
                                egui::RichText::new(url)
                                    .small()
                                    .color(egui::Color32::from_rgb(100, 220, 100))
                                    .family(egui::FontFamily::Monospace),
                            );
                            ui.separator();
                            ui.label(
                                egui::RichText::new(format!("Port file: {}", port_file_path))
                                    .small()
                                    .color(egui::Color32::from_rgb(120, 120, 130))
                                    .family(egui::FontFamily::Monospace),
                            );
                        }
                        None => {
                            ui.label(
                                egui::RichText::new("not running")
                                    .small()
                                    .color(egui::Color32::from_rgb(220, 100, 100)),
                            );
                        }
                    }
                });

                ui.separator();

                // --- Quick action buttons ---
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Quick actions:")
                            .small()
                            .color(egui::Color32::from_rgb(150, 150, 160)),
                    );
                    if ui.button("Initialize").clicked() {
                        self.fill(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
                    }
                    if ui.button("List Tools").clicked() {
                        self.fill(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
                    }
                    if ui.button("Get Scene Tree").clicked() {
                        self.fill(
                            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_scene_tree","arguments":{}}}"#,
                        );
                    }
                    if ui.button("Create Node").clicked() {
                        self.fill(
                            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"create_node","arguments":{"parent_id":0,"name":"NewNode","node_type":"empty"}}}"#,
                        );
                    }
                    if ui.button("Get Selection").clicked() {
                        self.fill(
                            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_selection","arguments":{}}}"#,
                        );
                    }
                });

                ui.separator();

                // --- Request editor ---
                ui.label(
                    egui::RichText::new("Request")
                        .small()
                        .color(egui::Color32::from_rgb(150, 150, 160)),
                );
                egui::ScrollArea::vertical()
                    .id_salt("mcp_debug_request_scroll")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.request_buffer)
                                .font(egui::FontId::monospace(13.0))
                                .desired_width(f32::INFINITY)
                                .code_editor(),
                        );
                    });

                ui.horizontal(|ui| {
                    if ui.button("Send Request").clicked() {
                        let req = self.request_buffer.trim().to_string();
                        if req.is_empty() {
                            self.response_display
                                .push_str("(empty request - not sent)\n");
                        } else {
                            app.push_mcp_request(&req);
                            self.response_display
                                .push_str(&format!(">> sent: {}\n", req));
                        }
                    }
                    if ui.button("Clear Request").clicked() {
                        self.request_buffer.clear();
                    }
                });

                ui.separator();

                // --- Response display (read-only) ---
                ui.label(
                    egui::RichText::new("Responses")
                        .small()
                        .color(egui::Color32::from_rgb(150, 150, 160)),
                );
                egui::ScrollArea::vertical()
                    .id_salt("mcp_debug_response_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.response_display)
                                .font(egui::FontId::monospace(13.0))
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
            });
    }
}
