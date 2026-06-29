//! 电子表格面板：网格数据属性编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SpreadsheetPanel {
    pub visible: bool,
    pub data_type: DataType,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub filter_text: String,
    pub selected_row: Option<usize>,
    pub show_index: bool,
    pub edit_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataType {
    Vertices,
    Edges,
    Faces,
    Points,
}

impl Default for SpreadsheetPanel {
    fn default() -> Self {
        Self {
            visible: false,
            data_type: DataType::Vertices,
            columns: vec!["Index".into(), "X".into(), "Y".into(), "Z".into(), "Selected".into()],
            rows: vec![
                vec!["0".into(), "0.0".into(), "0.0".into(), "0.0".into(), "true".into()],
                vec!["1".into(), "1.0".into(), "0.0".into(), "0.0".into(), "false".into()],
                vec!["2".into(), "1.0".into(), "1.0".into(), "0.0".into(), "false".into()],
                vec!["3".into(), "0.0".into(), "1.0".into(), "0.0".into(), "true".into()],
            ],
            filter_text: String::new(),
            selected_row: None,
            show_index: true,
            edit_mode: false,
        }
    }
}

impl EditorPanel for SpreadsheetPanel {
    fn name(&self) -> &str { "Spreadsheet" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Data:");
                    ui.radio_value(&mut self.data_type, DataType::Vertices, "Vertices");
                    ui.radio_value(&mut self.data_type, DataType::Edges, "Edges");
                    ui.radio_value(&mut self.data_type, DataType::Faces, "Faces");
                    ui.radio_value(&mut self.data_type, DataType::Points, "Points");
                    ui.separator();
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.filter_text);
                    ui.separator();
                    ui.checkbox(&mut self.show_index, "Index");
                    ui.checkbox(&mut self.edit_mode, "Edit");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Add Row").clicked() {
                        let mut new_row = vec![self.rows.len().to_string()];
                        for _ in 1..self.columns.len() {
                            new_row.push("0.0".into());
                        }
                        self.rows.push(new_row);
                    }
                    if ui.button("Add Column").clicked() {
                        self.columns.push(format!("Col{}", self.columns.len()));
                        for row in self.rows.iter_mut() {
                            row.push("0.0".into());
                        }
                    }
                    if ui.button("Clear").clicked() {
                        self.rows.clear();
                    }
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.set_min_width(600.0);
                    ui.horizontal(|ui| {
                        for col in &self.columns {
                            ui.label(egui::RichText::new(col).strong());
                            ui.separator();
                        }
                    });
                    ui.separator();
                    let mut remove_idx: Option<usize> = None;
                    for (ri, row) in self.rows.iter_mut().enumerate() {
                        let matches = self.filter_text.is_empty() || row.iter().any(|c| c.contains(&self.filter_text));
                        if !matches { continue; }
                        let selected = self.selected_row == Some(ri);
                        ui.horizontal(|ui| {
                            for (ci, cell) in row.iter_mut().enumerate() {
                                if self.edit_mode {
                                    ui.text_edit_singleline(cell);
                                } else {
                                    if ui.selectable_label(selected, cell.as_str()).clicked() {
                                        self.selected_row = Some(ri);
                                    }
                                }
                                ui.separator();
                                let _ = ci;
                            }
                            if ui.button("X").clicked() { remove_idx = Some(ri); }
                        });
                    }
                    if let Some(i) = remove_idx { self.rows.remove(i); }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Rows: {}", self.rows.len()));
                    ui.label(format!("Columns: {}", self.columns.len()));
                    if let Some(ri) = self.selected_row {
                        ui.label(format!("Selected: Row {}", ri));
                    }
                });
            });
    }
}
