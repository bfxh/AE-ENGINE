//! 约束面板：对象约束管理。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ConstraintsPanel {
    pub visible: bool,
    pub constraints: Vec<Constraint>,
    pub selected_constraint: Option<usize>,
    pub target_object: String,
    pub new_constraint_name: String,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub target: String,
    pub weight: f32,
    pub enabled: bool,
    pub axis_lock: [bool; 3],
    pub influence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintType {
    Track,
    IK,
    Copy,
    Limit,
    Follow,
    Look,
}

impl Default for ConstraintsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            constraints: vec![
                Constraint { name: "Track Camera".into(), constraint_type: ConstraintType::Track, target: "Camera".into(), weight: 1.0, enabled: true, axis_lock: [true, true, false], influence: 1.0 },
                Constraint { name: "Limit Rotation".into(), constraint_type: ConstraintType::Limit, target: String::new(), weight: 1.0, enabled: true, axis_lock: [false, false, true], influence: 1.0 },
            ],
            selected_constraint: Some(0),
            target_object: String::new(),
            new_constraint_name: String::new(),
        }
    }
}

impl EditorPanel for ConstraintsPanel {
    fn name(&self) -> &str { "Constraints" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New:");
                    ui.text_edit_singleline(&mut self.new_constraint_name);
                    if ui.button("Add Track").clicked() && !self.new_constraint_name.is_empty() {
                        self.constraints.push(Constraint { name: self.new_constraint_name.clone(), constraint_type: ConstraintType::Track, target: String::new(), weight: 1.0, enabled: true, axis_lock: [true, true, true], influence: 1.0 });
                        self.new_constraint_name.clear();
                    }
                    if ui.button("Add IK").clicked() && !self.new_constraint_name.is_empty() {
                        self.constraints.push(Constraint { name: self.new_constraint_name.clone(), constraint_type: ConstraintType::IK, target: String::new(), weight: 1.0, enabled: true, axis_lock: [true, true, true], influence: 1.0 });
                        self.new_constraint_name.clear();
                    }
                    if ui.button("Add Copy").clicked() && !self.new_constraint_name.is_empty() {
                        self.constraints.push(Constraint { name: self.new_constraint_name.clone(), constraint_type: ConstraintType::Copy, target: String::new(), weight: 1.0, enabled: true, axis_lock: [true, true, true], influence: 1.0 });
                        self.new_constraint_name.clear();
                    }
                    if ui.button("Add Limit").clicked() && !self.new_constraint_name.is_empty() {
                        self.constraints.push(Constraint { name: self.new_constraint_name.clone(), constraint_type: ConstraintType::Limit, target: String::new(), weight: 1.0, enabled: true, axis_lock: [true, true, true], influence: 1.0 });
                        self.new_constraint_name.clear();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Constraints");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut remove_idx: Option<usize> = None;
                            for (i, c) in self.constraints.iter_mut().enumerate() {
                                let selected = self.selected_constraint == Some(i);
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut c.enabled, "");
                                    if ui.selectable_label(selected, format!("[{:?}] {} -> {}", c.constraint_type, c.name, c.target)).clicked() {
                                        self.selected_constraint = Some(i);
                                    }
                                    if ui.button("X").clicked() { remove_idx = Some(i); }
                                });
                            }
                            if let Some(i) = remove_idx { self.constraints.remove(i); }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Properties");
                        ui.separator();
                        if let Some(ci) = self.selected_constraint {
                            if ci < self.constraints.len() {
                                let c = &mut self.constraints[ci];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut c.name);
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::Track, "Track");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::IK, "IK");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::Copy, "Copy");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::Limit, "Limit");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::Follow, "Follow");
                                    ui.radio_value(&mut c.constraint_type, ConstraintType::Look, "Look");
                                });
                                ui.label("Target:");
                                ui.text_edit_singleline(&mut c.target);
                                ui.horizontal(|ui| {
                                    ui.label("Weight:");
                                    ui.add(egui::Slider::new(&mut c.weight, 0.0..=1.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Influence:");
                                    ui.add(egui::Slider::new(&mut c.influence, 0.0..=1.0));
                                });
                                ui.label("Axis Lock:");
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut c.axis_lock[0], "X");
                                    ui.checkbox(&mut c.axis_lock[1], "Y");
                                    ui.checkbox(&mut c.axis_lock[2], "Z");
                                });
                            }
                        } else {
                            ui.label("Select a constraint");
                        }
                    });
                });
            });
    }
}
