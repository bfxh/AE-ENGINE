//! Prefab mode panel: prefab editing, variants, overrides, nesting.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PrefabModePanel {
    pub visible: bool,
    pub prefab_name: String,
    pub prefab_path: String,
    pub is_variant: bool,
    pub parent_prefab: String,
    pub overrides: Vec<PrefabOverride>,
    pub selected_override: Option<usize>,
    pub nested_prefabs: Vec<String>,
    pub auto_save: bool,
    pub show_overrides: bool,
}

#[derive(Debug, Clone)]
pub struct PrefabOverride { pub property: String, pub original: String, pub current: String, pub applied: bool }

impl Default for PrefabModePanel {
    fn default() -> Self {
        Self {
            visible: false, prefab_name: "Enemy_Base".into(), prefab_path: "prefabs/enemy_base.pfb".into(),
            is_variant: true, parent_prefab: "Character_Base".into(),
            overrides: vec![
                PrefabOverride { property: "health".into(), original: "100".into(), current: "50".into(), applied: false },
                PrefabOverride { property: "speed".into(), original: "5.0".into(), current: "7.5".into(), applied: false },
                PrefabOverride { property: "color".into(), original: "white".into(), current: "red".into(), applied: true },
            ],
            selected_override: Some(0),
            nested_prefabs: vec!["Weapon_Sword".into(), "Armor_Light".into()],
            auto_save: false, show_overrides: true,
        }
    }
}

impl EditorPanel for PrefabModePanel {
    fn name(&self) -> &str { "Prefab Mode" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Prefab Info");
            ui.horizontal(|ui| { ui.label("Name:"); ui.label(&self.prefab_name); });
            ui.horizontal(|ui| { ui.label("Path:"); ui.label(&self.prefab_path); });
            if self.is_variant {
                ui.horizontal(|ui| { ui.label("Variant of:"); ui.label(&self.parent_prefab); });
            }
            ui.checkbox(&mut self.auto_save, "Auto Save");
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.show_overrides, true, "Overrides");
                if ui.button("Nested Prefabs").clicked() {}
            });
            ui.separator();
            if self.show_overrides {
                ui.heading("Overrides");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for i in 0..self.overrides.len() {
                        let selected = self.selected_override == Some(i);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &self.overrides[i].property).clicked() { self.selected_override = Some(i); }
                            ui.label(&self.overrides[i].original);
                            ui.label("->");
                            ui.label(&self.overrides[i].current);
                            if self.overrides[i].applied { ui.label("(applied)"); }
                        });
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply Selected").clicked() { if let Some(idx) = self.selected_override { if idx < self.overrides.len() { self.overrides[idx].applied = true; } } }
                    if ui.button("Apply All").clicked() { for i in 0..self.overrides.len() { self.overrides[i].applied = true; } }
                    if ui.button("Revert Selected").clicked() { if let Some(idx) = self.selected_override { if idx < self.overrides.len() { self.overrides[idx].current = self.overrides[idx].original.clone(); self.overrides[idx].applied = true; } } }
                });
            }
            ui.separator();
            ui.heading("Nested Prefabs");
            for i in 0..self.nested_prefabs.len() { ui.horizontal(|ui| { ui.label(&self.nested_prefabs[i]); if ui.button("Edit").clicked() {} }); }
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Save Prefab").clicked() {} if ui.button("Save As Variant").clicked() {} if ui.button("Close").clicked() { self.visible = false; } });
        });
    }
}
