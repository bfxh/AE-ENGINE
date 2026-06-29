//! Console panel: displays log messages and accepts command input.
//!
//! Captures log output and provides a command-line interface for editor commands.

use crate::app::EditorApp;
use crate::panels::EditorPanel;
use std::collections::VecDeque;

/// A single log entry in the console.
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub frame: u64,
}

/// Log severity level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
            LogLevel::Warn => egui::Color32::from_rgb(255, 200, 80),
            LogLevel::Info => egui::Color32::from_rgb(180, 220, 255),
            LogLevel::Debug => egui::Color32::from_rgb(160, 160, 170),
            LogLevel::Trace => egui::Color32::from_rgb(110, 110, 120),
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Error => "[ERR] ",
            LogLevel::Warn => "[WRN] ",
            LogLevel::Info => "[INF] ",
            LogLevel::Debug => "[DBG] ",
            LogLevel::Trace => "[TRC] ",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        }
    }
}

/// Console panel state.
pub struct ConsolePanel {
    pub entries: VecDeque<LogEntry>,
    pub max_entries: usize,
    pub input: String,
    pub command_history: Vec<String>,
    pub history_pos: Option<usize>,
    pub auto_scroll: bool,
    pub filter_level: LogLevel,
    pub filter_text: String,
    pub visible: bool,
    pub show_timestamps: bool,
    pub show_level_filter: bool,
}

impl Default for ConsolePanel {
    fn default() -> Self {
        let mut entries = VecDeque::new();
        entries.push_back(LogEntry {
            level: LogLevel::Info,
            message: "Console initialized. Type 'help' for available commands.".to_string(),
            frame: 0,
        });
        Self {
            entries,
            max_entries: 1000,
            input: String::new(),
            command_history: Vec::new(),
            history_pos: None,
            auto_scroll: true,
            filter_level: LogLevel::Trace,
            filter_text: String::new(),
            visible: false,
            show_timestamps: true,
            show_level_filter: true,
        }
    }
}

impl ConsolePanel {
    pub fn log(&mut self, level: LogLevel, message: &str, frame: u64) {
        self.entries.push_back(LogEntry {
            level,
            message: message.to_string(),
            frame,
        });
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn execute_command(&mut self, cmd: &str, app: &mut EditorApp) {
        self.command_history.push(cmd.to_string());
        self.history_pos = None;

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() { return; }

        let command = parts[0];
        let args = &parts[1..];

        match command {
            "help" => {
                self.log(LogLevel::Info, "Available commands:", app.frame_counter);
                self.log(LogLevel::Info, "  help          - Show this help", app.frame_counter);
                self.log(LogLevel::Info, "  clear         - Clear console", app.frame_counter);
                self.log(LogLevel::Info, "  stats         - Show scene statistics", app.frame_counter);
                self.log(LogLevel::Info, "  list          - List all scene nodes", app.frame_counter);
                self.log(LogLevel::Info, "  select <id>   - Select a node by ID", app.frame_counter);
                self.log(LogLevel::Info, "  delete <id>   - Delete a node by ID", app.frame_counter);
                self.log(LogLevel::Info, "  add <type>    - Add a node (empty/light/camera)", app.frame_counter);
                self.log(LogLevel::Info, "  fps           - Toggle FPS display", app.frame_counter);
                self.log(LogLevel::Info, "  save          - Save the scene", app.frame_counter);
                self.log(LogLevel::Info, "  quit          - Exit the editor", app.frame_counter);
            },
            "clear" => { self.clear(); },
            "stats" => {
                self.log(LogLevel::Info, &format!("Scene: {}", app.scene.name), app.frame_counter);
                self.log(LogLevel::Info, &format!("Nodes: {}", app.scene.nodes.len()), app.frame_counter);
                self.log(LogLevel::Info, &format!("Dirty: {}", app.dirty), app.frame_counter);
                self.log(LogLevel::Info, &format!("Frame: {}", app.frame_counter), app.frame_counter);
            },
            "list" => {
                for node in &app.scene.nodes {
                    self.log(LogLevel::Info, &format!("  #{} {} {:?}", node.id, node.name, node.node_type), app.frame_counter);
                }
            },
            "select" => {
                if let Some(id_str) = args.get(0) {
                    if let Ok(id) = id_str.parse::<u64>() {
                        app.selection.select(id);
                        self.log(LogLevel::Info, &format!("Selected node {}", id), app.frame_counter);
                    } else {
                        self.log(LogLevel::Error, "Invalid ID", app.frame_counter);
                    }
                } else {
                    self.log(LogLevel::Warn, "Usage: select <id>", app.frame_counter);
                }
            },
            "delete" => {
                if let Some(id_str) = args.get(0) {
                    if let Ok(id) = id_str.parse::<u64>() {
                        app.selection.select(id);
                        app.delete_selected();
                        self.log(LogLevel::Info, &format!("Deleted node {}", id), app.frame_counter);
                    } else {
                        self.log(LogLevel::Error, "Invalid ID", app.frame_counter);
                    }
                } else {
                    self.log(LogLevel::Warn, "Usage: delete <id>", app.frame_counter);
                }
            },
            "add" => {
                let node_type = args.get(0).unwrap_or(&"empty");
                let root_id = app.scene.nodes.first().map(|n| n.id).unwrap_or(0);
                let name = format!("{}_{}", node_type, app.scene.nodes.len());
                let nt = match *node_type {
                    "light" => crate::scene::NodeType::Light {
                        light_type: crate::scene::LightType::Point,
                        color: glam::Vec3::ONE,
                        intensity: 1.0,
                    },
                    "camera" => crate::scene::NodeType::Camera { fov: 60.0, near: 0.1, far: 1000.0 },
                    _ => crate::scene::NodeType::Empty,
                };
                if let Some(id) = app.add_child_with_undo(root_id, &name, nt) {
                    app.selection.select(id);
                    self.log(LogLevel::Info, &format!("Added {} as node {}", node_type, id), app.frame_counter);
                }
            },
            "fps" => { self.log(LogLevel::Info, "FPS display toggled (see status bar)", app.frame_counter); },
            "save" => {
                app.pending_action = Some(crate::app::EditorAction::SaveScene);
                self.log(LogLevel::Info, "Save triggered", app.frame_counter);
            },
            "quit" => {
                app.pending_action = Some(crate::app::EditorAction::Exit);
                self.log(LogLevel::Info, "Exit triggered", app.frame_counter);
            },
            _ => {
                self.log(LogLevel::Error, &format!("Unknown command: '{}'. Type 'help' for commands.", command), app.frame_counter);
            },
        }
    }
}

impl EditorPanel for ConsolePanel {
    fn name(&self) -> &str { "Console" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }

        egui::Window::new("Console")
            .default_width(700.0).default_height(220.0)
            .resizable(true).collapsible(true)
            .show(ctx, |ui| {
                // Toolbar with level filter buttons.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Filter:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                    ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search...").desired_width(120.0));

                    ui.separator();

                    // Level filter buttons.
                    let levels = [LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug, LogLevel::Trace];
                    for level in levels {
                        let is_active = self.filter_level >= level;
                        let color = if is_active { level.color() } else { egui::Color32::from_rgb(80, 80, 80) };
                        if ui.add(egui::Button::new(egui::RichText::new(level.label()).color(color).small())).clicked() {
                            self.filter_level = level;
                        }
                    }

                    ui.separator();

                    ui.checkbox(&mut self.show_timestamps, "Time");
                    ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

                    ui.separator();

                    if ui.button("Clear").clicked() { self.clear(); }
                    if ui.button("Copy All").clicked() {
                        let mut text = String::new();
                        for entry in &self.entries {
                            text.push_str(&format!("{} {}\n", entry.level.prefix(), entry.message));
                        }
                        ui.ctx().copy_text(text);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{} entries", self.entries.len())).small().color(egui::Color32::from_rgb(120, 120, 120)));
                    });
                });

                ui.separator();

                // Log output area.
                egui::ScrollArea::vertical()
                    .stick_to_bottom(self.auto_scroll)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let filter = self.filter_text.to_lowercase();
                        let mut visible_count = 0u32;
                        for entry in &self.entries {
                            if entry.level > self.filter_level { continue; }
                            if !filter.is_empty() && !entry.message.to_lowercase().contains(&filter) { continue; }
                            visible_count += 1;

                            let color = entry.level.color();
                            let timestamp = if self.show_timestamps {
                                format!("[F{}] ", entry.frame)
                            } else {
                                String::new()
                            };
                            let text = format!("{}{}{}", timestamp, entry.level.prefix(), entry.message);

                            // Highlight errors with background.
                            if entry.level == LogLevel::Error {
                                let rect = ui.available_rect_before_wrap();
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 16.0)),
                                    0,
                                    egui::Color32::from_rgba_premultiplied(80, 20, 20, 60),
                                );
                            }

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&text).color(color).family(egui::FontFamily::Monospace).small());
                            });
                        }
                        if visible_count == 0 && !self.entries.is_empty() {
                            ui.label(egui::RichText::new("No entries match filter").color(egui::Color32::from_rgb(120, 120, 120)));
                        }
                    });

                ui.separator();

                // Command input with history navigation.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(">").color(egui::Color32::from_rgb(100, 255, 100)).strong());
                    let response = ui.add(egui::TextEdit::singleline(&mut self.input).hint_text("Enter command (type 'help')").desired_width(500.0));

                    // History navigation with Up/Down arrows.
                    if response.has_focus() {
                        ui.input(|i| {
                            if i.key_pressed(egui::Key::ArrowUp) {
                                if self.command_history.is_empty() {
                                    return;
                                }
                                self.history_pos = match self.history_pos {
                                    None => Some(self.command_history.len() - 1),
                                    Some(0) => Some(0),
                                    Some(pos) => Some(pos - 1),
                                };
                                if let Some(pos) = self.history_pos {
                                    self.input = self.command_history[pos].clone();
                                }
                            }
                            if i.key_pressed(egui::Key::ArrowDown) {
                                if let Some(pos) = self.history_pos {
                                    if pos + 1 < self.command_history.len() {
                                        self.history_pos = Some(pos + 1);
                                        self.input = self.command_history[pos + 1].clone();
                                    } else {
                                        self.history_pos = None;
                                        self.input.clear();
                                    }
                                }
                            }
                        });
                    }

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let cmd = self.input.clone();
                        self.input.clear();
                        self.execute_command(&cmd, app);
                    }
                    if ui.button("Run").clicked() {
                        let cmd = self.input.clone();
                        self.input.clear();
                        self.execute_command(&cmd, app);
                    }
                });
            });
    }
}

