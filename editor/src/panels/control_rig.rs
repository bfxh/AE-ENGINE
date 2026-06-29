//! 控制绑定面板：程序化骨骼控制。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ControlRigPanel {
    pub visible: bool,
    pub rig_name: String,
    pub control_type: ControlType,
    pub ik_chain: Vec<String>,
    pub ik_target: String,
    pub ik_pole: String,
    pub ik_weight: f32,
    pub fk_bone: String,
    pub fk_rotation: [f32; 3],
    pub fk_translation: [f32; 3],
    pub constraint_type: ConstraintType,
    pub constraint_target: String,
    pub constraint_weight: f32,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub mirror_z: bool,
    pub selected_control: Option<usize>,
    pub controls: Vec<RigControl>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlType {
    IK,
    FK,
    Constraint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintType {
    Track,
    Aim,
    Copy,
    Limit,
}

#[derive(Debug, Clone)]
pub struct RigControl {
    pub name: String,
    pub control_type: ControlType,
    pub enabled: bool,
}

impl Default for ControlRigPanel {
    fn default() -> Self {
        Self {
            visible: false,
            rig_name: "ControlRig".to_string(),
            control_type: ControlType::IK,
            ik_chain: vec!["UpperArm".into(), "LowerArm".into(), "Hand".into()],
            ik_target: "Hand_Target".into(),
            ik_pole: "Elbow_Pole".into(),
            ik_weight: 1.0,
            fk_bone: "Spine".into(),
            fk_rotation: [0.0, 0.0, 0.0],
            fk_translation: [0.0, 0.0, 0.0],
            constraint_type: ConstraintType::Track,
            constraint_target: String::new(),
            constraint_weight: 1.0,
            mirror_x: true,
            mirror_y: false,
            mirror_z: false,
            selected_control: None,
            controls: vec![
                RigControl { name: "Hand_IK".into(), control_type: ControlType::IK, enabled: true },
                RigControl { name: "Foot_IK".into(), control_type: ControlType::IK, enabled: true },
                RigControl { name: "Spine_FK".into(), control_type: ControlType::FK, enabled: true },
            ],
        }
    }
}

impl EditorPanel for ControlRigPanel {
    fn name(&self) -> &str { "Control Rig" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rig:");
                    ui.text_edit_singleline(&mut self.rig_name);
                    ui.separator();
                    if ui.button("Add Control").clicked() {
                        self.controls.push(RigControl { name: format!("Control {}", self.controls.len()), control_type: self.control_type, enabled: true });
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.control_type, ControlType::IK, "IK");
                    ui.radio_value(&mut self.control_type, ControlType::FK, "FK");
                    ui.radio_value(&mut self.control_type, ControlType::Constraint, "Constraint");
                });
                ui.separator();
                match self.control_type {
                    ControlType::IK => {
                        ui.heading("IK Chain");
                        ui.horizontal(|ui| {
                            ui.label("Target:");
                            ui.text_edit_singleline(&mut self.ik_target);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Pole:");
                            ui.text_edit_singleline(&mut self.ik_pole);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Weight:");
                            ui.add(egui::Slider::new(&mut self.ik_weight, 0.0..=1.0));
                        });
                        ui.label("Chain Bones:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut remove_idx: Option<usize> = None;
                            for (i, bone) in self.ik_chain.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(bone);
                                    if ui.button("X").clicked() { remove_idx = Some(i); }
                                });
                            }
                            if let Some(i) = remove_idx { self.ik_chain.remove(i); }
                        });
                        if ui.button("Add Bone").clicked() {
                            self.ik_chain.push(String::new());
                        }
                    }
                    ControlType::FK => {
                        ui.heading("FK Control");
                        ui.horizontal(|ui| {
                            ui.label("Bone:");
                            ui.text_edit_singleline(&mut self.fk_bone);
                        });
                        ui.label("Translation:");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut self.fk_translation[0]).speed(0.01));
                            ui.add(egui::DragValue::new(&mut self.fk_translation[1]).speed(0.01));
                            ui.add(egui::DragValue::new(&mut self.fk_translation[2]).speed(0.01));
                        });
                        ui.label("Rotation (deg):");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut self.fk_rotation[0]).speed(1.0));
                            ui.add(egui::DragValue::new(&mut self.fk_rotation[1]).speed(1.0));
                            ui.add(egui::DragValue::new(&mut self.fk_rotation[2]).speed(1.0));
                        });
                    }
                    ControlType::Constraint => {
                        ui.heading("Constraint");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.constraint_type, ConstraintType::Track, "Track");
                            ui.radio_value(&mut self.constraint_type, ConstraintType::Aim, "Aim");
                            ui.radio_value(&mut self.constraint_type, ConstraintType::Copy, "Copy");
                            ui.radio_value(&mut self.constraint_type, ConstraintType::Limit, "Limit");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Target:");
                            ui.text_edit_singleline(&mut self.constraint_target);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Weight:");
                            ui.add(egui::Slider::new(&mut self.constraint_weight, 0.0..=1.0));
                        });
                    }
                }
                ui.separator();
                ui.heading("Mirror");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.mirror_x, "Mirror X");
                    ui.checkbox(&mut self.mirror_y, "Mirror Y");
                    ui.checkbox(&mut self.mirror_z, "Mirror Z");
                });
                ui.separator();
                ui.label("Controls:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let count = self.controls.len();
                    for i in 0..count {
                        let selected = self.selected_control == Some(i);
                        let name = self.controls[i].name.clone();
                        let ct = self.controls[i].control_type;
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &name).clicked() {
                                self.selected_control = Some(i);
                            }
                            ui.checkbox(&mut self.controls[i].enabled, "On");
                            ui.label(format!("{:?}", ct));
                        });
                    }
                });
            });
    }
}
