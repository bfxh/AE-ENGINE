//! Package manager panel: installed packages, versions, dependencies, updates.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PackageManagerPanel {
    pub visible: bool,
    pub packages: Vec<Package>,
    pub selected_package: Option<usize>,
    pub search_query: String,
    pub show_updates_only: bool,
    pub registry_url: String,
    pub auto_update: bool,
    pub active_tab: PmTab,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PmTab { Installed, Updates, Dependencies }

#[derive(Debug, Clone)]
pub struct Package { pub name: String, pub version: String, pub latest: String, pub description: String, pub has_update: bool, pub dependencies: Vec<String> }

impl Default for PackageManagerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            packages: vec![
                Package { name: "wgpu".into(), version: "24.0".into(), latest: "24.0".into(), description: "Graphics API".into(), has_update: false, dependencies: vec![] },
                Package { name: "egui".into(), version: "0.31.0".into(), latest: "0.31.2".into(), description: "Immediate mode GUI".into(), has_update: true, dependencies: vec!["epaint".into()] },
                Package { name: "winit".into(), version: "0.30.0".into(), latest: "0.30.0".into(), description: "Window creation".into(), has_update: false, dependencies: vec![] },
                Package { name: "glam".into(), version: "0.29.0".into(), latest: "0.30.0".into(), description: "Math library".into(), has_update: true, dependencies: vec![] },
                Package { name: "rfd".into(), version: "0.15.0".into(), latest: "0.15.0".into(), description: "File dialog".into(), has_update: false, dependencies: vec![] },
            ],
            selected_package: Some(0), search_query: String::new(), show_updates_only: false, registry_url: "https://crates.io".into(), auto_update: false, active_tab: PmTab::Installed,
        }
    }
}

impl EditorPanel for PackageManagerPanel {
    fn name(&self) -> &str { "Package Manager" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, PmTab::Installed, "Installed");
                ui.selectable_value(&mut self.active_tab, PmTab::Updates, "Updates");
                ui.selectable_value(&mut self.active_tab, PmTab::Dependencies, "Dependencies");
            });
            ui.separator();
            ui.horizontal(|ui| { ui.label("Search:"); ui.text_edit_singleline(&mut self.search_query); ui.checkbox(&mut self.show_updates_only, "Updates only"); });
            ui.separator();
            match self.active_tab {
                PmTab::Installed | PmTab::Updates => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.packages.len() {
                            let search_match = self.search_query.is_empty() || self.packages[i].name.contains(&self.search_query);
                            let update_match = self.active_tab != PmTab::Updates || self.packages[i].has_update;
                            if self.show_updates_only && !self.packages[i].has_update { continue; }
                            if !search_match || !update_match { continue; }
                            let selected = self.selected_package == Some(i);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.packages[i].name).clicked() { self.selected_package = Some(i); }
                                ui.label(format!("v{}", self.packages[i].version));
                                if self.packages[i].has_update { ui.label(format!("-> v{}", self.packages[i].latest)); }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if self.packages[i].has_update { if ui.button("Update").clicked() { self.packages[i].version = self.packages[i].latest.clone(); self.packages[i].has_update = false; } }
                                    if ui.button("Remove").clicked() {}
                                });
                            });
                            ui.label(format!("  {}", self.packages[i].description));
                        }
                    });
                },
                PmTab::Dependencies => {
                    if let Some(idx) = self.selected_package { if idx < self.packages.len() {
                        ui.heading(format!("Dependencies of {}", self.packages[idx].name));
                        if self.packages[idx].dependencies.is_empty() { ui.label("No dependencies"); }
                        for i in 0..self.packages[idx].dependencies.len() { ui.label(&self.packages[idx].dependencies[i]); }
                    }}
                },
            }
            ui.separator();
            ui.horizontal(|ui| { ui.label("Registry:"); ui.text_edit_singleline(&mut self.registry_url); ui.checkbox(&mut self.auto_update, "Auto Update"); });
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Check for Updates").clicked() {} if ui.button("Update All").clicked() { for i in 0..self.packages.len() { if self.packages[i].has_update { self.packages[i].version = self.packages[i].latest.clone(); self.packages[i].has_update = false; } } } });
        });
    }
}
