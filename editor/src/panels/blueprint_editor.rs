//! Blueprint editor panel: event graph, functions, variables, components.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct BlueprintEditorPanel {
    pub visible: bool,
    pub active_tab: BpTab,
    pub blueprint_name: String,
    pub parent_class: String,
    pub variables: Vec<BpVariable>,
    pub functions: Vec<BpFunction>,
    pub events: Vec<String>,
    pub selected_var: Option<usize>,
    pub new_var_name: String,
    pub compile_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BpTab { EventGraph, Functions, Variables, Components }

#[derive(Debug, Clone)]
pub struct BpVariable { pub name: String, pub var_type: String, pub default_value: String, pub is_exposed: bool }

#[derive(Debug, Clone)]
pub struct BpFunction { pub name: String, pub inputs: u32, pub outputs: u32, pub is_pure: bool }

impl Default for BlueprintEditorPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: BpTab::Variables,
            blueprint_name: "MyActor".into(), parent_class: "Actor".into(),
            variables: vec![
                BpVariable { name: "Health".into(), var_type: "Float".into(), default_value: "100.0".into(), is_exposed: true },
                BpVariable { name: "PlayerName".into(), var_type: "String".into(), default_value: "Player".into(), is_exposed: true },
                BpVariable { name: "IsAlive".into(), var_type: "Bool".into(), default_value: "true".into(), is_exposed: false },
            ],
            functions: vec![
                BpFunction { name: "TakeDamage".into(), inputs: 2, outputs: 1, is_pure: false },
                BpFunction { name: "GetHealth".into(), inputs: 0, outputs: 1, is_pure: true },
            ],
            events: vec!["BeginPlay".into(), "Tick".into(), "OnHit".into(), "Destroyed".into()],
            selected_var: Some(0), new_var_name: String::new(), compile_status: "Compiled".into(),
        }
    }
}

impl EditorPanel for BlueprintEditorPanel {
    fn name(&self) -> &str { "Blueprint Editor" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(600.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.label("Blueprint:"); ui.label(&self.blueprint_name); ui.label("Parent:"); ui.label(&self.parent_class); });
            ui.horizontal(|ui| { if ui.button("Compile").clicked() { self.compile_status = "Compiled".into(); } ui.label(&self.compile_status); if ui.button("Save").clicked() {} });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, BpTab::EventGraph, "Event Graph");
                ui.selectable_value(&mut self.active_tab, BpTab::Functions, "Functions");
                ui.selectable_value(&mut self.active_tab, BpTab::Variables, "Variables");
                ui.selectable_value(&mut self.active_tab, BpTab::Components, "Components");
            });
            ui.separator();
            match self.active_tab {
                BpTab::EventGraph => {
                    ui.heading("Events");
                    for i in 0..self.events.len() { ui.horizontal(|ui| { ui.label(&self.events[i]); if ui.button("Add Node").clicked() {} }); }
                },
                BpTab::Functions => {
                    ui.heading("Functions");
                    for i in 0..self.functions.len() { ui.horizontal(|ui| { ui.label(&self.functions[i].name); ui.label(format!("({} in, {} out)", self.functions[i].inputs, self.functions[i].outputs)); if self.functions[i].is_pure { ui.label("Pure"); } }); }
                    if ui.button("Add Function").clicked() { self.functions.push(BpFunction { name: "NewFunction".into(), inputs: 0, outputs: 0, is_pure: false }); }
                },
                BpTab::Variables => {
                    ui.heading("Variables");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.variables.len() {
                            let selected = self.selected_var == Some(i);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.variables[i].name).clicked() { self.selected_var = Some(i); }
                                ui.label(&self.variables[i].var_type);
                                ui.checkbox(&mut self.variables[i].is_exposed, "Exposed");
                            });
                        }
                    });
                    ui.separator();
                    if let Some(idx) = self.selected_var { if idx < self.variables.len() {
                        ui.horizontal(|ui| { ui.label("Default:"); ui.text_edit_singleline(&mut self.variables[idx].default_value); });
                    }}
                    ui.separator();
                    ui.horizontal(|ui| { ui.label("New:"); ui.text_edit_singleline(&mut self.new_var_name); if ui.button("Add").clicked() && !self.new_var_name.is_empty() { self.variables.push(BpVariable { name: self.new_var_name.clone(), var_type: "Float".into(), default_value: "0.0".into(), is_exposed: false }); self.new_var_name.clear(); } });
                },
                BpTab::Components => {
                    ui.heading("Components");
                    ui.label("No components added");
                    if ui.button("Add Component").clicked() {}
                },
            }
        });
    }
}
