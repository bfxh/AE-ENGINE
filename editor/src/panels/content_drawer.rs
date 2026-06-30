//! Content drawer panel: quick access, recent, favorites, search.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ContentDrawerPanel {
    pub visible: bool,
    pub active_tab: CdTab,
    pub recent_items: Vec<ContentItem>,
    pub favorites: Vec<ContentItem>,
    pub search_query: String,
    pub search_results: Vec<ContentItem>,
    pub max_recent: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CdTab { Recent, Favorites, Search }

#[derive(Debug, Clone)]
pub struct ContentItem { pub name: String, pub path: String, pub item_type: String, pub last_used: String }

impl Default for ContentDrawerPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: CdTab::Recent,
            recent_items: vec![
                ContentItem { name: "main_scene.ae".into(), path: "scenes/".into(), item_type: "Scene".into(), last_used: "2 min ago".into() },
                ContentItem { name: "player.fbx".into(), path: "models/".into(), item_type: "Mesh".into(), last_used: "10 min ago".into() },
                ContentItem { name: "terrain.mat".into(), path: "materials/".into(), item_type: "Material".into(), last_used: "1 hour ago".into() },
            ],
            favorites: vec![
                ContentItem { name: "skybox.mat".into(), path: "materials/".into(), item_type: "Material".into(), last_used: "".into() },
                ContentItem { name: "sun_light.ent".into(), path: "entities/".into(), item_type: "Entity".into(), last_used: "".into() },
            ],
            search_query: String::new(), search_results: vec![], max_recent: 20,
        }
    }
}

impl EditorPanel for ContentDrawerPanel {
    fn name(&self) -> &str { "Content Drawer" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(350.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, CdTab::Recent, "Recent");
                ui.selectable_value(&mut self.active_tab, CdTab::Favorites, "Favorites");
                ui.selectable_value(&mut self.active_tab, CdTab::Search, "Search");
            });
            ui.separator();
            match self.active_tab {
                CdTab::Recent => {
                    ui.heading("Recent");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.recent_items.len() {
                            ui.horizontal(|ui| {
                                ui.label(&self.recent_items[i].name);
                                ui.label(&self.recent_items[i].item_type);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(&self.recent_items[i].last_used); });
                            });
                            ui.label(format!("  {}", self.recent_items[i].path));
                        }
                    });
                },
                CdTab::Favorites => {
                    ui.heading("Favorites");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.favorites.len() {
                            ui.horizontal(|ui| {
                                ui.label(&self.favorites[i].name);
                                ui.label(&self.favorites[i].item_type);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("X").clicked() { self.favorites.remove(i); } });
                            });
                        }
                    });
                },
                CdTab::Search => {
                    ui.horizontal(|ui| { ui.label("Search:"); ui.text_edit_singleline(&mut self.search_query); if ui.button("Search").clicked() {} });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.search_results.len() {
                            ui.horizontal(|ui| { ui.label(&self.search_results[i].name); ui.label(&self.search_results[i].path); });
                        }
                        if self.search_results.is_empty() && !self.search_query.is_empty() { ui.label("No results found"); }
                    });
                },
            }
            ui.separator();
            ui.horizontal(|ui| { ui.label(format!("Recent: {}/{}", self.recent_items.len(), self.max_recent)); });
        });
    }
}
