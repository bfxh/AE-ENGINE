//! Gameplay debugger panel: runtime data, AI behavior, property viewer.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct GameplayDebuggerPanel {
    pub visible: bool,
    pub active_tab: GdTab,
    pub entities: Vec<GdEntity>,
    pub selected_entity: Option<usize>,
    pub ai_behaviors: Vec<AiBehavior>,
    pub watch_vars: Vec<WatchVar>,
    pub paused: bool,
    pub time_scale: f32,
    pub show_ai_paths: bool,
    pub show_collision: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GdTab { Entities, AiBehavior, WatchVars }

#[derive(Debug, Clone)]
pub struct GdEntity { pub id: u32, pub name: String, pub pos: [f32; 3], pub health: f32, pub max_health: f32, pub active: bool }

#[derive(Debug, Clone)]
pub struct AiBehavior { pub entity: String, pub state: String, pub target: String, pub priority: i32 }

#[derive(Debug, Clone)]
pub struct WatchVar { pub name: String, pub value: String, pub var_type: String }

impl Default for GameplayDebuggerPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: GdTab::Entities,
            entities: vec![
                GdEntity { id: 1, name: "Player".into(), pos: [0.0, 0.0, 0.0], health: 100.0, max_health: 100.0, active: true },
                GdEntity { id: 2, name: "Enemy_1".into(), pos: [10.0, 0.0, 5.0], health: 50.0, max_health: 50.0, active: true },
                GdEntity { id: 3, name: "NPC_Villager".into(), pos: [-5.0, 0.0, 3.0], health: 30.0, max_health: 30.0, active: false },
            ],
            selected_entity: Some(0),
            ai_behaviors: vec![
                AiBehavior { entity: "Enemy_1".into(), state: "Chase".into(), target: "Player".into(), priority: 5 },
                AiBehavior { entity: "NPC_Villager".into(), state: "Idle".into(), target: "None".into(), priority: 1 },
            ],
            watch_vars: vec![
                WatchVar { name: "game_time".into(), value: "123.45".into(), var_type: "f32".into() },
                WatchVar { name: "player_score".into(), value: "1500".into(), var_type: "i32".into() },
                WatchVar { name: "is_night".into(), value: "true".into(), var_type: "bool".into() },
            ],
            paused: false, time_scale: 1.0, show_ai_paths: true, show_collision: false,
        }
    }
}

impl EditorPanel for GameplayDebuggerPanel {
    fn name(&self) -> &str { "Gameplay Debugger" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(400.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.paused { "Resume" } else { "Pause" }).clicked() { self.paused = !self.paused; }
                ui.label("Time Scale:");
                ui.add(egui::Slider::new(&mut self.time_scale, 0.0..=3.0));
                ui.checkbox(&mut self.show_ai_paths, "AI Paths");
                ui.checkbox(&mut self.show_collision, "Collision");
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, GdTab::Entities, "Entities");
                ui.selectable_value(&mut self.active_tab, GdTab::AiBehavior, "AI Behavior");
                ui.selectable_value(&mut self.active_tab, GdTab::WatchVars, "Watch Vars");
            });
            ui.separator();
            match self.active_tab {
                GdTab::Entities => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.entities.len() {
                            let selected = self.selected_entity == Some(i);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.entities[i].name).clicked() { self.selected_entity = Some(i); }
                                ui.label(format!("#{}", self.entities[i].id));
                                ui.label(format!("({:.1},{:.1},{:.1})", self.entities[i].pos[0], self.entities[i].pos[1], self.entities[i].pos[2]));
                                ui.checkbox(&mut self.entities[i].active, "Active");
                            });
                            if selected {
                                ui.indent("ent_detail", |ui| {
                                    ui.horizontal(|ui| { ui.label("Health:"); ui.add(egui::ProgressBar::new(self.entities[i].health / self.entities[i].max_health)); ui.label(format!("{:.0}/{:.0}", self.entities[i].health, self.entities[i].max_health)); });
                                });
                            }
                        }
                    });
                },
                GdTab::AiBehavior => {
                    for i in 0..self.ai_behaviors.len() {
                        ui.horizontal(|ui| { ui.label(&self.ai_behaviors[i].entity); ui.label("->"); ui.label(&self.ai_behaviors[i].state); ui.label(&self.ai_behaviors[i].target); ui.label(format!("P{}", self.ai_behaviors[i].priority)); });
                    }
                },
                GdTab::WatchVars => {
                    for i in 0..self.watch_vars.len() {
                        ui.horizontal(|ui| { ui.label(&self.watch_vars[i].name); ui.label(":"); ui.label(&self.watch_vars[i].value); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(&self.watch_vars[i].var_type); }); });
                    }
                },
            }
        });
    }
}
