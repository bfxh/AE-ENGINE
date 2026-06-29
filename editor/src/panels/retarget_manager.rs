//! Retarget manager panel: source/target skeleton, mapping, preview.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct RetargetManagerPanel {
    pub visible: bool,
    pub source_skeleton: String,
    pub target_skeleton: String,
    pub bone_mappings: Vec<BoneMapping>,
    pub selected_mapping: Option<usize>,
    pub retarget_mode: RetargetMode,
    pub auto_map: bool,
    pub preserve_proportions: bool,
    pub preview_animation: String,
    pub preview_playing: bool,
    pub preview_time: f32,
}

#[derive(Debug, Clone)]
pub struct BoneMapping { pub source_bone: String, pub target_bone: String, pub scale: f32, pub enabled: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetargetMode { Skeleton, Animation, Both }

impl Default for RetargetManagerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            source_skeleton: "Humanoid_Base".into(), target_skeleton: "Humanoid_Tall".into(),
            bone_mappings: vec![
                BoneMapping { source_bone: "Hips".into(), target_bone: "Hips".into(), scale: 1.0, enabled: true },
                BoneMapping { source_bone: "Spine".into(), target_bone: "Spine".into(), scale: 1.0, enabled: true },
                BoneMapping { source_bone: "LeftArm".into(), target_bone: "LeftArm".into(), scale: 0.9, enabled: true },
                BoneMapping { source_bone: "RightArm".into(), target_bone: "RightArm".into(), scale: 0.9, enabled: true },
            ],
            selected_mapping: Some(0), retarget_mode: RetargetMode::Both, auto_map: true, preserve_proportions: true,
            preview_animation: "walk.anim".into(), preview_playing: false, preview_time: 0.0,
        }
    }
}

impl EditorPanel for RetargetManagerPanel {
    fn name(&self) -> &str { "Retarget Manager" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Skeletons");
            ui.horizontal(|ui| { ui.label("Source:"); ui.text_edit_singleline(&mut self.source_skeleton); });
            ui.horizontal(|ui| { ui.label("Target:"); ui.text_edit_singleline(&mut self.target_skeleton); });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.retarget_mode, RetargetMode::Skeleton, "Skeleton");
                ui.selectable_value(&mut self.retarget_mode, RetargetMode::Animation, "Animation");
                ui.selectable_value(&mut self.retarget_mode, RetargetMode::Both, "Both");
            });
            ui.checkbox(&mut self.auto_map, "Auto Map");
            ui.checkbox(&mut self.preserve_proportions, "Preserve Proportions");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Auto Map Bones").clicked() {}
                if ui.button("Add Mapping").clicked() { self.bone_mappings.push(BoneMapping { source_bone: "New".into(), target_bone: "New".into(), scale: 1.0, enabled: true }); }
                if ui.button("Remove Selected").clicked() { if let Some(idx) = self.selected_mapping { if idx < self.bone_mappings.len() { self.bone_mappings.remove(idx); self.selected_mapping = None; } } }
            });
            ui.separator();
            ui.heading("Bone Mappings");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.bone_mappings.len() {
                    let selected = self.selected_mapping == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.bone_mappings[i].source_bone).clicked() { self.selected_mapping = Some(i); }
                        ui.label("->");
                        ui.label(&self.bone_mappings[i].target_bone);
                        ui.checkbox(&mut self.bone_mappings[i].enabled, "");
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_mapping { if idx < self.bone_mappings.len() {
                ui.heading("Mapping Properties");
                ui.horizontal(|ui| { ui.label("Scale:"); ui.add(egui::Slider::new(&mut self.bone_mappings[idx].scale, 0.1..=3.0)); });
            }}
            ui.separator();
            ui.heading("Preview");
            ui.horizontal(|ui| { ui.label("Animation:"); ui.text_edit_singleline(&mut self.preview_animation); });
            ui.horizontal(|ui| { if ui.button(if self.preview_playing { "Pause" } else { "Play" }).clicked() { self.preview_playing = !self.preview_playing; } ui.add(egui::Slider::new(&mut self.preview_time, 0.0..=10.0)); });
        });
    }
}
