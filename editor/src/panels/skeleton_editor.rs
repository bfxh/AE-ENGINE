//! 骨骼编辑器面板：骨骼层级和变换编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SkeletonEditorPanel {
    pub visible: bool,
    pub skeleton_name: String,
    pub bones: Vec<Bone>,
    pub selected_bone: Option<usize>,
    pub show_skeleton: bool,
    pub show_names: bool,
    pub retarget_source: String,
    pub retarget_mode: RetargetMode,
    pub rotation_offset: [f32; 3],
    pub scale_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetargetMode {
    None,
    Skeleton,
    Animation,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
    pub socket: bool,
}

impl Default for SkeletonEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            skeleton_name: "Skeleton".to_string(),
            bones: vec![
                Bone { name: "Root".into(), parent: None, position: [0.0, 0.0, 0.0], rotation: [0.0, 0.0, 0.0], scale: 1.0, socket: false },
                Bone { name: "Spine".into(), parent: Some(0), position: [0.0, 1.0, 0.0], rotation: [0.0, 0.0, 0.0], scale: 1.0, socket: false },
                Bone { name: "Head".into(), parent: Some(1), position: [0.0, 1.5, 0.0], rotation: [0.0, 0.0, 0.0], scale: 1.0, socket: true },
            ],
            selected_bone: Some(0),
            show_skeleton: true,
            show_names: false,
            retarget_source: String::new(),
            retarget_mode: RetargetMode::None,
            rotation_offset: [0.0, 0.0, 0.0],
            scale_offset: 1.0,
        }
    }
}

impl EditorPanel for SkeletonEditorPanel {
    fn name(&self) -> &str { "Skeleton Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Skeleton:");
                    ui.text_edit_singleline(&mut self.skeleton_name);
                    ui.separator();
                    ui.checkbox(&mut self.show_skeleton, "Show Skeleton");
                    ui.checkbox(&mut self.show_names, "Show Names");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Bone Tree");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, bone) in self.bones.iter().enumerate() {
                                let selected = self.selected_bone == Some(i);
                                let depth = if bone.parent.is_some() { 1 } else { 0 };
                                let indent = "  ".repeat(depth);
                                if ui.selectable_label(selected, format!("{}{}", indent, bone.name)).clicked() {
                                    self.selected_bone = Some(i);
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Add Bone").clicked() {
                            self.bones.push(Bone { name: format!("Bone {}", self.bones.len()), parent: self.selected_bone, position: [0.0, 0.0, 0.0], rotation: [0.0, 0.0, 0.0], scale: 1.0, socket: false });
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label("Bone Properties");
                        ui.separator();
                        if let Some(idx) = self.selected_bone {
                            if idx < self.bones.len() {
                                let bone = &mut self.bones[idx];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut bone.name);
                                ui.checkbox(&mut bone.socket, "Is Socket");
                                ui.label("Position:");
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut bone.position[0]).speed(0.01));
                                    ui.add(egui::DragValue::new(&mut bone.position[1]).speed(0.01));
                                    ui.add(egui::DragValue::new(&mut bone.position[2]).speed(0.01));
                                });
                                ui.label("Rotation (deg):");
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut bone.rotation[0]).speed(1.0));
                                    ui.add(egui::DragValue::new(&mut bone.rotation[1]).speed(1.0));
                                    ui.add(egui::DragValue::new(&mut bone.rotation[2]).speed(1.0));
                                });
                                ui.label("Scale:");
                                ui.add(egui::Slider::new(&mut bone.scale, 0.01..=10.0));
                            }
                        } else {
                            ui.label("Select a bone");
                        }
                    });
                });
                ui.separator();
                ui.heading("Retargeting");
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    ui.text_edit_singleline(&mut self.retarget_source);
                });
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.radio_value(&mut self.retarget_mode, RetargetMode::None, "None");
                    ui.radio_value(&mut self.retarget_mode, RetargetMode::Skeleton, "Skeleton");
                    ui.radio_value(&mut self.retarget_mode, RetargetMode::Animation, "Animation");
                });
                ui.horizontal(|ui| {
                    ui.label("Offset Rot:");
                    ui.add(egui::DragValue::new(&mut self.rotation_offset[0]).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.rotation_offset[1]).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.rotation_offset[2]).speed(1.0));
                    ui.label("Offset Scale:");
                    ui.add(egui::DragValue::new(&mut self.scale_offset).speed(0.01));
                });
            });
    }
}
