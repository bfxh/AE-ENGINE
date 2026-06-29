//! 动画蓝图面板：状态机和动画混合编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct AnimBlueprintPanel {
    pub visible: bool,
    pub blueprint_name: String,
    pub states: Vec<AnimState>,
    pub selected_state: Option<usize>,
    pub transitions: Vec<StateTransition>,
    pub blend_time: f32,
    pub current_state: Option<usize>,
    pub show_blend_spaces: bool,
    pub slot_name: String,
    pub slot_weight: f32,
    pub preview_anim: String,
    pub loop_preview: bool,
}

#[derive(Debug, Clone)]
pub struct AnimState {
    pub name: String,
    pub anim_path: String,
    pub play_rate: f32,
    pub looping: bool,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: usize,
    pub to: usize,
    pub rule: String,
    pub blend_duration: f32,
}

impl Default for AnimBlueprintPanel {
    fn default() -> Self {
        Self {
            visible: false,
            blueprint_name: "AnimBP".to_string(),
            states: vec![
                AnimState { name: "Idle".into(), anim_path: "anims/idle.fbx".into(), play_rate: 1.0, looping: true },
                AnimState { name: "Walk".into(), anim_path: "anims/walk.fbx".into(), play_rate: 1.0, looping: true },
                AnimState { name: "Run".into(), anim_path: "anims/run.fbx".into(), play_rate: 1.0, looping: true },
            ],
            selected_state: Some(0),
            transitions: vec![
                StateTransition { from: 0, to: 1, rule: "Speed > 0".into(), blend_duration: 0.2 },
                StateTransition { from: 1, to: 2, rule: "Speed > 5".into(), blend_duration: 0.15 },
            ],
            blend_time: 0.2,
            current_state: Some(0),
            show_blend_spaces: true,
            slot_name: "DefaultSlot".to_string(),
            slot_weight: 1.0,
            preview_anim: String::new(),
            loop_preview: true,
        }
    }
}

impl EditorPanel for AnimBlueprintPanel {
    fn name(&self) -> &str { "Anim Blueprint" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Blueprint:");
                    ui.text_edit_singleline(&mut self.blueprint_name);
                    ui.separator();
                    if ui.button("Add State").clicked() {
                        self.states.push(AnimState { name: format!("State {}", self.states.len()), anim_path: String::new(), play_rate: 1.0, looping: true });
                    }
                    if ui.button("Add Transition").clicked() {
                        if self.states.len() >= 2 {
                            self.transitions.push(StateTransition { from: 0, to: 1, rule: String::new(), blend_duration: self.blend_time });
                        }
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_blend_spaces, "Blend Spaces");
                    ui.checkbox(&mut self.loop_preview, "Loop Preview");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("State Machine");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(350.0, 250.0));
                            for (i, state) in self.states.iter().enumerate() {
                                let selected = self.selected_state == Some(i);
                                let current = self.current_state == Some(i);
                                let prefix = if current { "[*] " } else { "" };
                                if ui.selectable_label(selected, format!("{}{}", prefix, state.name)).clicked() {
                                    self.selected_state = Some(i);
                                }
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("State Properties");
                        ui.separator();
                        if let Some(idx) = self.selected_state {
                            if idx < self.states.len() {
                                let state = &mut self.states[idx];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut state.name);
                                ui.label("Animation:");
                                ui.text_edit_singleline(&mut state.anim_path);
                                ui.horizontal(|ui| {
                                    ui.label("Play Rate:");
                                    ui.add(egui::Slider::new(&mut state.play_rate, 0.0..=5.0));
                                });
                                ui.checkbox(&mut state.looping, "Looping");
                            }
                        } else {
                            ui.label("Select a state");
                        }
                    });
                });
                ui.separator();
                ui.heading("Transitions");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, t) in self.transitions.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let from_name = self.states.get(t.from).map(|s| s.name.as_str()).unwrap_or("?");
                            let to_name = self.states.get(t.to).map(|s| s.name.as_str()).unwrap_or("?");
                            ui.label(format!("{} -> {}", from_name, to_name));
                            ui.text_edit_singleline(&mut t.rule);
                            ui.add(egui::DragValue::new(&mut t.blend_duration).speed(0.01).range(0.0..=2.0));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.transitions.remove(i); }
                });
                ui.separator();
                ui.heading("Animation Slots");
                ui.horizontal(|ui| {
                    ui.label("Slot:");
                    ui.text_edit_singleline(&mut self.slot_name);
                    ui.label("Weight:");
                    ui.add(egui::Slider::new(&mut self.slot_weight, 0.0..=1.0));
                });
            });
    }
}
