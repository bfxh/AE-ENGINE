//! 可视化日志面板：时间轴日志查看。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct VisualLoggerPanel {
    pub visible: bool,
    pub log_entries: Vec<LogEntry>,
    pub selected_object: String,
    pub object_filter: String,
    pub category_filter: LogCategory,
    pub current_time: f32,
    pub time_range: f32,
    pub show_timestamps: bool,
    pub auto_scroll: bool,
    pub snapshot_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogCategory {
    All,
    Info,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub time: f32,
    pub object: String,
    pub category: LogCategory,
    pub message: String,
}

impl Default for VisualLoggerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            log_entries: vec![
                LogEntry { time: 0.0, object: "Player".into(), category: LogCategory::Info, message: "Spawned".into() },
                LogEntry { time: 1.2, object: "Player".into(), category: LogCategory::Warning, message: "Low health".into() },
                LogEntry { time: 2.5, object: "Enemy".into(), category: LogCategory::Error, message: "AI stuck".into() },
                LogEntry { time: 3.0, object: "Player".into(), category: LogCategory::Debug, message: "Position update".into() },
            ],
            selected_object: "Player".to_string(),
            object_filter: String::new(),
            category_filter: LogCategory::All,
            current_time: 3.0,
            time_range: 10.0,
            show_timestamps: true,
            auto_scroll: true,
            snapshot_index: None,
        }
    }
}

impl EditorPanel for VisualLoggerPanel {
    fn name(&self) -> &str { "Visual Logger" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Object:");
                    ui.text_edit_singleline(&mut self.selected_object);
                    ui.separator();
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.object_filter);
                    ui.separator();
                    ui.radio_value(&mut self.category_filter, LogCategory::All, "All");
                    ui.radio_value(&mut self.category_filter, LogCategory::Info, "Info");
                    ui.radio_value(&mut self.category_filter, LogCategory::Warning, "Warn");
                    ui.radio_value(&mut self.category_filter, LogCategory::Error, "Error");
                    ui.radio_value(&mut self.category_filter, LogCategory::Debug, "Debug");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Time:");
                    ui.add(egui::Slider::new(&mut self.current_time, 0.0..=self.time_range));
                    ui.label("Range:");
                    ui.add(egui::DragValue::new(&mut self.time_range).speed(1.0).range(1.0..=300.0));
                    ui.separator();
                    ui.checkbox(&mut self.show_timestamps, "Timestamps");
                    ui.checkbox(&mut self.auto_scroll, "Auto Scroll");
                    if ui.button("Snapshot").clicked() {
                        self.snapshot_index = Some(self.log_entries.len());
                    }
                });
                ui.separator();
                ui.label("Timeline:");
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(650.0, 40.0));
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(650.0, 40.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, egui::Color32::GRAY));
                    for entry in &self.log_entries {
                        if entry.time > self.time_range { continue; }
                        let x = rect.left() + (entry.time / self.time_range) * rect.width();
                        let color = match entry.category {
                            LogCategory::Info => egui::Color32::LIGHT_BLUE,
                            LogCategory::Warning => egui::Color32::YELLOW,
                            LogCategory::Error => egui::Color32::RED,
                            LogCategory::Debug => egui::Color32::LIGHT_GREEN,
                            LogCategory::All => egui::Color32::WHITE,
                        };
                        painter.circle_filled(egui::pos2(x, rect.center().y), 3.0, color);
                    }
                    let cur_x = rect.left() + (self.current_time / self.time_range) * rect.width();
                    painter.line_segment([egui::pos2(cur_x, rect.top()), egui::pos2(cur_x, rect.bottom())], egui::Stroke::new(2.0, egui::Color32::WHITE));
                });
                ui.separator();
                ui.label("Log Entries:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in &self.log_entries {
                        if !self.selected_object.is_empty() && entry.object != self.selected_object { continue; }
                        if self.category_filter != LogCategory::All && entry.category != self.category_filter { continue; }
                        if !self.object_filter.is_empty() && !entry.object.contains(&self.object_filter) { continue; }
                        let color = match entry.category {
                            LogCategory::Info => egui::Color32::LIGHT_BLUE,
                            LogCategory::Warning => egui::Color32::YELLOW,
                            LogCategory::Error => egui::Color32::RED,
                            LogCategory::Debug => egui::Color32::LIGHT_GREEN,
                            LogCategory::All => egui::Color32::WHITE,
                        };
                        ui.horizontal(|ui| {
                            if self.show_timestamps {
                                ui.label(format!("[{:.2}s]", entry.time));
                            }
                            ui.colored_label(color, format!("[{:?}]", entry.category));
                            ui.label(&entry.object);
                            ui.label(":");
                            ui.label(&entry.message);
                        });
                    }
                });
            });
    }
}
