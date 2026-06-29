//! Collection manager panel: collection list, asset grouping, tags.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct CollectionManagerPanel {
    pub visible: bool,
    pub collections: Vec<Collection>,
    pub selected_collection: Option<usize>,
    pub tags: Vec<String>,
    pub new_collection_name: String,
    pub new_tag_name: String,
    pub filter_tag: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Collection { pub name: String, pub asset_count: u32, pub tags: Vec<String>, pub color: egui::Color32 }

impl Default for CollectionManagerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            collections: vec![
                Collection { name: "Environment".into(), asset_count: 45, tags: vec!["outdoor".into(), "nature".into()], color: egui::Color32::from_rgb(100, 200, 100) },
                Collection { name: "Characters".into(), asset_count: 12, tags: vec!["npc".into(), "player".into()], color: egui::Color32::from_rgb(200, 100, 100) },
                Collection { name: "UI".into(), asset_count: 30, tags: vec!["ui".into(), "icons".into()], color: egui::Color32::from_rgb(100, 100, 200) },
            ],
            selected_collection: Some(0),
            tags: vec!["outdoor".into(), "nature".into(), "npc".into(), "player".into(), "ui".into(), "icons".into(), "indoor".into()],
            new_collection_name: String::new(), new_tag_name: String::new(), filter_tag: None,
        }
    }
}

impl EditorPanel for CollectionManagerPanel {
    fn name(&self) -> &str { "Collection Manager" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(400.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("New Collection:");
                ui.text_edit_singleline(&mut self.new_collection_name);
                if ui.button("Create").clicked() && !self.new_collection_name.is_empty() {
                    self.collections.push(Collection { name: self.new_collection_name.clone(), asset_count: 0, tags: vec![], color: egui::Color32::from_rgb(150, 150, 150) });
                    self.new_collection_name.clear();
                }
            });
            ui.separator();
            ui.heading("Tags");
            ui.horizontal_wrapped(|ui| {
                for i in 0..self.tags.len() {
                    let selected = self.filter_tag == Some(i);
                    if ui.selectable_label(selected, &self.tags[i]).clicked() {
                        self.filter_tag = if selected { None } else { Some(i) };
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_tag_name);
                if ui.button("Add Tag").clicked() && !self.new_tag_name.is_empty() {
                    self.tags.push(self.new_tag_name.clone());
                    self.new_tag_name.clear();
                }
            });
            ui.separator();
            ui.heading("Collections");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.collections.len() {
                    let tag_match = match self.filter_tag {
                        Some(t) if t < self.tags.len() => self.collections[i].tags.contains(&self.tags[t]),
                        _ => true,
                    };
                    if !tag_match { continue; }
                    let selected = self.selected_collection == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.collections[i].name).clicked() { self.selected_collection = Some(i); }
                        let cc = self.collections[i].color;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, cc);
                        ui.label(format!("{} assets", self.collections[i].asset_count));
                    });
                    if !self.collections[i].tags.is_empty() {
                        ui.label(format!("  Tags: {}", self.collections[i].tags.join(", ")));
                    }
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_collection { if idx < self.collections.len() {
                ui.heading("Properties");
                ui.label(format!("Name: {}", self.collections[idx].name));
                ui.label(format!("Assets: {}", self.collections[idx].asset_count));
                ui.horizontal(|ui| { ui.label("Color:"); ui.color_button(self.collections[idx].color, egui::Color32::from_rgb(80,80,80)); });
                if ui.button("Delete Collection").clicked() { self.collections.remove(idx); self.selected_collection = None; }
            }}
        });
    }
}
