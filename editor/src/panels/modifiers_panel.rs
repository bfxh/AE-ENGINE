//! 修改器面板：几何体修改器堆栈。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ModifiersPanel {
    pub visible: bool,
    pub modifiers: Vec<Modifier>,
    pub selected_modifier: Option<usize>,
    pub target_object: String,
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub name: String,
    pub modifier_type: ModifierType,
    pub enabled: bool,
    pub params: ModifierParams,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModifierType {
    Subdivision,
    Mirror,
    Boolean,
    Array,
    Deform,
    Smooth,
}

#[derive(Debug, Clone)]
pub struct ModifierParams {
    pub level: u32,
    pub axis_x: bool,
    pub axis_y: bool,
    pub axis_z: bool,
    pub count: u32,
    pub offset: f32,
    pub strength: f32,
}

impl Default for ModifiersPanel {
    fn default() -> Self {
        Self {
            visible: false,
            modifiers: vec![
                Modifier { name: "Subsurf".into(), modifier_type: ModifierType::Subdivision, enabled: true, params: ModifierParams { level: 2, axis_x: true, axis_y: false, axis_z: false, count: 1, offset: 0.0, strength: 0.5 } },
                Modifier { name: "Mirror".into(), modifier_type: ModifierType::Mirror, enabled: true, params: ModifierParams { level: 0, axis_x: true, axis_y: false, axis_z: false, count: 1, offset: 0.0, strength: 0.0 } },
            ],
            selected_modifier: Some(0),
            target_object: "Cube".to_string(),
        }
    }
}

impl EditorPanel for ModifiersPanel {
    fn name(&self) -> &str { "Modifiers" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(450.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Object:");
                    ui.text_edit_singleline(&mut self.target_object);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Add Subdivision").clicked() {
                        self.modifiers.push(Modifier { name: format!("Subsurf {}", self.modifiers.len()), modifier_type: ModifierType::Subdivision, enabled: true, params: ModifierParams { level: 1, axis_x: true, axis_y: false, axis_z: false, count: 1, offset: 0.0, strength: 0.0 } });
                    }
                    if ui.button("Add Mirror").clicked() {
                        self.modifiers.push(Modifier { name: format!("Mirror {}", self.modifiers.len()), modifier_type: ModifierType::Mirror, enabled: true, params: ModifierParams { level: 0, axis_x: true, axis_y: false, axis_z: false, count: 1, offset: 0.0, strength: 0.0 } });
                    }
                    if ui.button("Add Boolean").clicked() {
                        self.modifiers.push(Modifier { name: format!("Boolean {}", self.modifiers.len()), modifier_type: ModifierType::Boolean, enabled: true, params: ModifierParams { level: 0, axis_x: true, axis_y: true, axis_z: true, count: 1, offset: 0.0, strength: 0.0 } });
                    }
                    if ui.button("Add Array").clicked() {
                        self.modifiers.push(Modifier { name: format!("Array {}", self.modifiers.len()), modifier_type: ModifierType::Array, enabled: true, params: ModifierParams { level: 0, axis_x: true, axis_y: false, axis_z: false, count: 5, offset: 1.0, strength: 0.0 } });
                    }
                    if ui.button("Add Deform").clicked() {
                        self.modifiers.push(Modifier { name: format!("Deform {}", self.modifiers.len()), modifier_type: ModifierType::Deform, enabled: true, params: ModifierParams { level: 0, axis_x: true, axis_y: true, axis_z: true, count: 1, offset: 0.0, strength: 0.5 } });
                    }
                });
                ui.separator();
                ui.label("Modifier Stack:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    let mut move_up: Option<usize> = None;
                    let mut move_down: Option<usize> = None;
                    let mcount = self.modifiers.len();
                    for (i, m) in self.modifiers.iter_mut().enumerate() {
                        let selected = self.selected_modifier == Some(i);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut m.enabled, "");
                            if ui.selectable_label(selected, format!("[{:?}] {}", m.modifier_type, m.name)).clicked() {
                                self.selected_modifier = Some(i);
                            }
                            if ui.button("^").clicked() { if i > 0 { move_up = Some(i); } }
                            if ui.button("v").clicked() { if i + 1 < mcount { move_down = Some(i); } }
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.modifiers.remove(i); }
                    if let Some(i) = move_up { self.modifiers.swap(i, i - 1); }
                    if let Some(i) = move_down { self.modifiers.swap(i, i + 1); }
                });
                ui.separator();
                if let Some(mi) = self.selected_modifier {
                    if mi < self.modifiers.len() {
                        let m = &mut self.modifiers[mi];
                        ui.heading("Modifier Properties");
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut m.name);
                        match m.modifier_type {
                            ModifierType::Subdivision => {
                                ui.horizontal(|ui| {
                                    ui.label("Level:");
                                    ui.add(egui::Slider::new(&mut m.params.level, 0..=6).integer());
                                });
                            }
                            ModifierType::Mirror => {
                                ui.label("Axis:");
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut m.params.axis_x, "X");
                                    ui.checkbox(&mut m.params.axis_y, "Y");
                                    ui.checkbox(&mut m.params.axis_z, "Z");
                                });
                            }
                            ModifierType::Boolean => {
                                ui.label("Operation:");
                                ui.label("Union/Diff/Intersect");
                            }
                            ModifierType::Array => {
                                ui.horizontal(|ui| {
                                    ui.label("Count:");
                                    ui.add(egui::DragValue::new(&mut m.params.count).range(1..=1000));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Offset:");
                                    ui.add(egui::Slider::new(&mut m.params.offset, 0.0..=10.0));
                                });
                            }
                            ModifierType::Deform | ModifierType::Smooth => {
                                ui.horizontal(|ui| {
                                    ui.label("Strength:");
                                    ui.add(egui::Slider::new(&mut m.params.strength, 0.0..=1.0));
                                });
                            }
                        }
                    }
                }
            });
    }
}
