//! Addressables panel: addresses, groups, tags, loading.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct AddressablesPanel {
    pub visible: bool,
    pub groups: Vec<AddressableGroup>,
    pub selected_group: Option<usize>,
    pub assets: Vec<AddressableAsset>,
    pub selected_asset: Option<usize>,
    pub tags: Vec<String>,
    pub new_group_name: String,
    pub build_status: String,
    pub active_tab: AddrTab,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddrTab { Groups, Assets, Tags }

#[derive(Debug, Clone)]
pub struct AddressableGroup { pub name: String, pub asset_count: u32, pub compressed: bool, pub remote: bool }

#[derive(Debug, Clone)]
pub struct AddressableAsset { pub address: String, pub path: String, pub group: String, pub labels: Vec<String> }

impl Default for AddressablesPanel {
    fn default() -> Self {
        Self {
            visible: false,
            groups: vec![
                AddressableGroup { name: "Default".into(), asset_count: 25, compressed: false, remote: false },
                AddressableGroup { name: "Textures".into(), asset_count: 120, compressed: true, remote: false },
                AddressableGroup { name: "Audio".into(), asset_count: 45, compressed: true, remote: true },
            ],
            selected_group: Some(0),
            assets: vec![
                AddressableAsset { address: "player/tex".into(), path: "textures/player.png".into(), group: "Textures".into(), labels: vec!["character".into()] },
                AddressableAsset { address: "ui/bg".into(), path: "textures/bg.png".into(), group: "Textures".into(), labels: vec!["ui".into()] },
                AddressableAsset { address: "sfx/hit".into(), path: "audio/hit.wav".into(), group: "Audio".into(), labels: vec!["combat".into()] },
            ],
            selected_asset: Some(0),
            tags: vec!["character".into(), "ui".into(), "combat".into(), "environment".into()],
            new_group_name: String::new(), build_status: "Not built".into(), active_tab: AddrTab::Assets,
        }
    }
}

impl EditorPanel for AddressablesPanel {
    fn name(&self) -> &str { "Addressables" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, AddrTab::Assets, "Assets");
                ui.selectable_value(&mut self.active_tab, AddrTab::Groups, "Groups");
                ui.selectable_value(&mut self.active_tab, AddrTab::Tags, "Tags");
            });
            ui.separator();
            match self.active_tab {
                AddrTab::Assets => {
                    ui.heading("Addressable Assets");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.assets.len() {
                            let selected = self.selected_asset == Some(i);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.assets[i].address).clicked() { self.selected_asset = Some(i); }
                                ui.label(&self.assets[i].group);
                                ui.label(&self.assets[i].path);
                            });
                        }
                    });
                    ui.separator();
                    if let Some(idx) = self.selected_asset { if idx < self.assets.len() {
                        ui.heading("Asset Properties");
                        ui.label(format!("Address: {}", self.assets[idx].address));
                        ui.label(format!("Path: {}", self.assets[idx].path));
                        ui.label(format!("Group: {}", self.assets[idx].group));
                        ui.label(format!("Labels: {}", self.assets[idx].labels.join(", ")));
                    }}
                },
                AddrTab::Groups => {
                    ui.heading("Groups");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.groups.len() {
                            let selected = self.selected_group == Some(i);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.groups[i].name).clicked() { self.selected_group = Some(i); }
                                ui.label(format!("{} assets", self.groups[i].asset_count));
                                ui.checkbox(&mut self.groups[i].compressed, "Compressed");
                                ui.checkbox(&mut self.groups[i].remote, "Remote");
                            });
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| { ui.text_edit_singleline(&mut self.new_group_name); if ui.button("Add Group").clicked() && !self.new_group_name.is_empty() { self.groups.push(AddressableGroup { name: self.new_group_name.clone(), asset_count: 0, compressed: false, remote: false }); self.new_group_name.clear(); } });
                },
                AddrTab::Tags => {
                    ui.heading("Tags");
                    for i in 0..self.tags.len() { ui.label(&self.tags[i]); }
                },
            }
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Build").clicked() { self.build_status = "Built".into(); } ui.label(&self.build_status); });
        });
    }
}
