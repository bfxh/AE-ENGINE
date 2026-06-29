//! Bookmarks panel: save and manage camera positions.
//!
//! Allows saving camera viewpoints for quick navigation.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// A saved camera bookmark.
#[derive(Clone, Debug)]
pub struct Bookmark {
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub group: String,
    pub notes: String,
}

/// Bookmarks panel state.
pub struct BookmarksPanel {
    pub visible: bool,
    pub bookmarks: Vec<Bookmark>,
    pub selected: Option<usize>,
    pub new_name: String,
    pub new_group: String,
    pub filter_text: String,
    pub rename_mode: Option<usize>,
    pub rename_text: String,
}

impl Default for BookmarksPanel {
    fn default() -> Self {
        Self {
            visible: false,
            bookmarks: vec![
                Bookmark {
                    name: "Origin".into(),
                    position: [0.0, 5.0, 10.0],
                    target: [0.0, 0.0, 0.0],
                    group: "Default".into(),
                    notes: "World origin view".into(),
                },
                Bookmark {
                    name: "Top Down".into(),
                    position: [0.0, 20.0, 0.0],
                    target: [0.0, 0.0, 0.0],
                    group: "Default".into(),
                    notes: "Top-down view".into(),
                },
            ],
            selected: Some(0),
            new_name: String::new(),
            new_group: "Default".into(),
            filter_text: String::new(),
            rename_mode: None,
            rename_text: String::new(),
        }
    }
}

impl EditorPanel for BookmarksPanel {
    fn name(&self) -> &str { "Bookmarks" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("Bookmarks").default_width(380.0).default_height(400.0).resizable(true).show(ctx, |ui| {
            // Add bookmark bar.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Name:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                ui.add(egui::TextEdit::singleline(&mut self.new_name).hint_text("Bookmark name...").desired_width(100.0));
                ui.label(egui::RichText::new("Group:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                ui.add(egui::TextEdit::singleline(&mut self.new_group).desired_width(60.0));
                if ui.button("+ Add Current View").clicked() {
                    let pos = app.camera.position;
                    let tgt = app.camera.target;
                    let name = if self.new_name.is_empty() {
                        format!("BM {}", self.bookmarks.len())
                    } else {
                        self.new_name.clone()
                    };
                    self.bookmarks.push(Bookmark {
                        name,
                        position: [pos.x, pos.y, pos.z],
                        target: [tgt.x, tgt.y, tgt.z],
                        group: self.new_group.clone(),
                        notes: String::new(),
                    });
                    self.new_name.clear();
                }
            });

            // Filter.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Filter:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search bookmarks...").desired_width(150.0));
                if ui.button("Clear").clicked() { self.filter_text.clear(); }
            });

            ui.separator();

            // Bookmark list - clone data to avoid borrow conflicts.
            let filter = self.filter_text.to_lowercase();
            let mut to_remove: Option<usize> = None;
            let mut to_go: Option<usize> = None;
            let mut new_selected: Option<usize> = None;
            let mut new_rename: Option<usize> = None;
            let mut rename_done = false;

            let bm_data: Vec<(usize, String, String, [f32; 3], String)> = self.bookmarks.iter().enumerate().filter_map(|(i, bm)| {
                let matches = filter.is_empty()
                    || bm.name.to_lowercase().contains(&filter)
                    || bm.group.to_lowercase().contains(&filter);
                if matches {
                    Some((i, bm.name.clone(), bm.group.clone(), bm.position, bm.notes.clone()))
                } else {
                    None
                }
            }).collect();

            let cur_selected = self.selected;
            let cur_rename = self.rename_mode;
            let mut rename_text = self.rename_text.clone();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if bm_data.is_empty() {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("No bookmarks match filter").color(egui::Color32::from_rgb(120, 120, 120)));
                    return;
                }

                for (i, name, group, position, notes) in &bm_data {
                    let is_sel = cur_selected == Some(*i);
                    let is_renaming = cur_rename == Some(*i);

                    ui.horizontal(|ui| {
                        let group_color = match group.as_str() {
                            "Default" => egui::Color32::from_rgb(100, 180, 255),
                            "Combat" => egui::Color32::from_rgb(255, 100, 100),
                            "Cinematic" => egui::Color32::from_rgb(255, 200, 100),
                            _ => egui::Color32::from_rgb(180, 180, 180),
                        };
                        ui.label(egui::RichText::new("[G]").color(group_color).small());

                        if is_renaming {
                            let response = ui.add(egui::TextEdit::singleline(&mut rename_text).desired_width(100.0));
                            if response.lost_focus() {
                                rename_done = true;
                            }
                        } else {
                            let response = ui.selectable_label(is_sel, egui::RichText::new(name).strong());
                            if response.clicked() { new_selected = Some(*i); }
                            if response.double_clicked() {
                                new_rename = Some(*i);
                                rename_text = name.clone();
                            }
                        }

                        ui.label(egui::RichText::new(format!("({:.1},{:.1},{:.1})", position[0], position[1], position[2])).small().color(egui::Color32::from_rgb(120, 120, 120)).family(egui::FontFamily::Monospace));

                        if ui.small_button("Go").clicked() { to_go = Some(*i); }
                        if ui.small_button("Ren").clicked() { new_rename = Some(*i); rename_text = name.clone(); }
                        if ui.small_button("X").clicked() { to_remove = Some(*i); }
                    });

                    if is_sel && !is_renaming {
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            let notes_str = if notes.is_empty() { "(none)" } else { notes.as_str() };
                            ui.label(egui::RichText::new(format!("Group: {} | Notes: {}", group, notes_str)).small().color(egui::Color32::from_rgb(140, 140, 150)));
                        });
                    }
                }
            });

            // Apply changes after rendering.
            if let Some(idx) = new_selected { self.selected = Some(idx); }
            if let Some(idx) = new_rename { self.rename_mode = Some(idx); }
            if rename_done {
                if let Some(idx) = cur_rename {
                    if let Some(bm) = self.bookmarks.get_mut(idx) {
                        bm.name = rename_text.clone();
                    }
                }
                self.rename_mode = None;
            }
            self.rename_text = rename_text;

            ui.separator();

            // Bottom actions.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} bookmarks", self.bookmarks.len())).small().color(egui::Color32::from_rgb(120, 120, 120)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Export").clicked() {
                        // Export bookmarks as JSON to clipboard.
                        let mut json = String::from("[\n");
                        for (i, bm) in self.bookmarks.iter().enumerate() {
                            json.push_str(&format!(
                                "  {{\"name\":\"{}\",\"pos\":[{},{},{}],\"target\":[{},{},{}],\"group\":\"{}\"}}{}\n",
                                bm.name, bm.position[0], bm.position[1], bm.position[2],
                                bm.target[0], bm.target[1], bm.target[2], bm.group,
                                if i + 1 < self.bookmarks.len() { "," } else { "" }
                            ));
                        }
                        json.push(']');
                        ui.ctx().copy_text(json);
                    }
                    if ui.button("Clear All").clicked() {
                        self.bookmarks.clear();
                        self.selected = None;
                    }
                });
            });

            // Apply actions after iteration.
            if let Some(i) = to_remove {
                self.bookmarks.remove(i);
                if self.selected == Some(i) { self.selected = None; }
                if self.rename_mode == Some(i) { self.rename_mode = None; }
            }
            if let Some(i) = to_go {
                if let Some(bm) = self.bookmarks.get(i) {
                    app.camera.position = glam::Vec3::new(bm.position[0], bm.position[1], bm.position[2]);
                    app.camera.target = glam::Vec3::new(bm.target[0], bm.target[1], bm.target[2]);
                }
            }
        });
    }
}
