//! Collections panel: collection list, asset grouping, color tags, sharing.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct CollectionsPanel {
    pub visible: bool,
    pub collections: Vec<Collection>,
    pub selected_collection: Option<usize>,
    pub new_collection_name: String,
    pub share_mode: ShareMode,
    pub sort_by: SortBy,
    pub show_empty: bool,
    pub auto_color: bool,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub asset_count: u32,
    pub color: egui::Color32,
    pub shared: bool,
    pub owner: String,
    pub modified: String,
    pub assets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShareMode { Private, Shared, Public }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortBy { Name, Date, Size, Color }

impl Default for CollectionsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            collections: vec![
                Collection { name: "Environment".into(), asset_count: 42, color: egui::Color32::from_rgb(80, 160, 80), shared: true, owner: "team".into(), modified: "2026-06-20".into(), assets: vec!["tree.fbx".into(), "rock.fbx".into()] },
                Collection { name: "Characters".into(), asset_count: 18, color: egui::Color32::from_rgb(160, 80, 80), shared: true, owner: "team".into(), modified: "2026-06-18".into(), assets: vec!["hero.fbx".into()] },
                Collection { name: "Audio".into(), asset_count: 7, color: egui::Color32::from_rgb(80, 80, 160), shared: false, owner: "me".into(), modified: "2026-06-15".into(), assets: vec![] },
                Collection { name: "VFX".into(), asset_count: 25, color: egui::Color32::from_rgb(200, 120, 40), shared: false, owner: "me".into(), modified: "2026-06-22".into(), assets: vec!["fire.ns".into()] },
            ],
            selected_collection: Some(0),
            new_collection_name: String::new(),
            share_mode: ShareMode::Private,
            sort_by: SortBy::Name,
            show_empty: true,
            auto_color: true,
        }
    }
}

impl EditorPanel for CollectionsPanel {
    fn name(&self) -> &str { "Collections" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_collection_name);
                if ui.button("Add Collection").clicked() && !self.new_collection_name.is_empty() {
                    self.collections.push(Collection {
                        name: self.new_collection_name.clone(),
                        asset_count: 0,
                        color: egui::Color32::from_rgb(120, 120, 120),
                        shared: false,
                        owner: "me".into(),
                        modified: "2026-06-24".into(),
                        assets: vec![],
                    });
                    self.new_collection_name.clear();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Sort:");
                ui.selectable_value(&mut self.sort_by, SortBy::Name, "Name");
                ui.selectable_value(&mut self.sort_by, SortBy::Date, "Date");
                ui.selectable_value(&mut self.sort_by, SortBy::Size, "Size");
                ui.selectable_value(&mut self.sort_by, SortBy::Color, "Color");
                ui.checkbox(&mut self.show_empty, "Show Empty");
                ui.checkbox(&mut self.auto_color, "Auto Color");
            });
            ui.separator();
            ui.heading("Collections");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.collections.len() {
                    if !self.show_empty && self.collections[i].asset_count == 0 { continue; }
                    let selected = self.selected_collection == Some(i);
                    let cc = self.collections[i].color;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.collections[i].name).clicked() { self.selected_collection = Some(i); }
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, cc);
                        ui.label(format!("({})", self.collections[i].asset_count));
                        ui.checkbox(&mut self.collections[i].shared, "Shared");
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_collection { if idx < self.collections.len() {
                ui.heading("Collection Properties");
                ui.horizontal(|ui| { ui.label("Name:"); ui.text_edit_singleline(&mut self.collections[idx].name); });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_srgba(&mut self.collections[idx].color);
                });
                ui.label(format!("Owner: {}", self.collections[idx].owner));
                ui.label(format!("Modified: {}", self.collections[idx].modified));
                ui.horizontal(|ui| {
                    ui.label("Share:");
                    ui.selectable_value(&mut self.share_mode, ShareMode::Private, "Private");
                    ui.selectable_value(&mut self.share_mode, ShareMode::Shared, "Shared");
                    ui.selectable_value(&mut self.share_mode, ShareMode::Public, "Public");
                });
                ui.separator();
                ui.label("Assets:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for j in 0..self.collections[idx].assets.len() {
                        ui.label(&self.collections[idx].assets[j]);
                    }
                    if self.collections[idx].assets.is_empty() { ui.label("No assets in this collection"); }
                });
                ui.separator();
                if ui.button("Remove Collection").clicked() {
                    self.collections.remove(idx);
                    self.selected_collection = None;
                }
            }}
        });
    }
}
