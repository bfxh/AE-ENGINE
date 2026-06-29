//! Script editor panel: code editing, syntax highlighting, autocomplete, debugging.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ScriptEditorPanel {
    pub visible: bool,
    pub open_files: Vec<ScriptFile>,
    pub active_file: Option<usize>,
    pub code: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub font_size: f32,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub auto_indent: bool,
    pub breakpoints: Vec<usize>,
    pub is_debugging: bool,
    pub console_output: String,
}

#[derive(Debug, Clone)]
pub struct ScriptFile { pub name: String, pub path: String, pub modified: bool }

impl Default for ScriptEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            open_files: vec![
                ScriptFile { name: "main.rs".into(), path: "scripts/main.rs".into(), modified: false },
                ScriptFile { name: "player.rs".into(), path: "scripts/player.rs".into(), modified: true },
            ],
            active_file: Some(0),
            code: "fn main() {\n    println!(\"Hello, World!\");\n}\n".into(),
            cursor_line: 1, cursor_col: 1, font_size: 14.0,
            show_line_numbers: true, word_wrap: false, auto_indent: true,
            breakpoints: vec![3], is_debugging: false, console_output: String::new(),
        }
    }
}

impl EditorPanel for ScriptEditorPanel {
    fn name(&self) -> &str { "Script Editor" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(650.0).default_height(500.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {}
                if ui.button("Open").clicked() {}
                if ui.button("Save").clicked() { if let Some(idx) = self.active_file { if idx < self.open_files.len() { self.open_files[idx].modified = false; } } }
                if ui.button("Save All").clicked() { for i in 0..self.open_files.len() { self.open_files[i].modified = false; } }
                ui.separator();
                if ui.button(if self.is_debugging { "Stop" } else { "Debug" }).clicked() { self.is_debugging = !self.is_debugging; }
                if ui.button("Run").clicked() { self.console_output = "Running...\n".into(); }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Open Files");
                    for i in 0..self.open_files.len() {
                        let selected = self.active_file == Some(i);
                        let label = if self.open_files[i].modified { format!("* {}", self.open_files[i].name) } else { self.open_files[i].name.clone() };
                        if ui.selectable_label(selected, &label).clicked() { self.active_file = Some(i); }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_line_numbers, "Line#");
                        ui.checkbox(&mut self.word_wrap, "Wrap");
                        ui.checkbox(&mut self.auto_indent, "Auto-indent");
                        ui.label("Font:");
                        ui.add(egui::Slider::new(&mut self.font_size, 8.0..=24.0));
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let style = egui::TextStyle::Monospace;
                        ui.add(egui::TextEdit::multiline(&mut self.code).font(style).code_editor().desired_width(500.0));
                    });
                    ui.separator();
                    ui.horizontal(|ui| { ui.label(format!("Ln {}, Col {}", self.cursor_line, self.cursor_col)); });
                });
            });
            ui.separator();
            ui.label("Console");
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(self.console_output.as_str());
            });
        });
    }
}
