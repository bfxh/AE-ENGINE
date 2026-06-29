//! VFX graph panel: node graph, particle context, output, properties.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct VfxGraphPanel {
    pub visible: bool,
    pub vfx_name: String,
    pub active_context: VfxContext,
    pub nodes: Vec<VfxNode>,
    pub selected_node: Option<usize>,
    pub max_particles: u32,
    pub simulation_space: SimSpace,
    pub play_on_start: bool,
    pub looping: bool,
    pub duration: f32,
    pub new_node_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfxContext { Spawn, Initialize, Update, Output }

#[derive(Debug, Clone)]
pub struct VfxNode { pub name: String, pub node_type: NodeType, pub context: VfxContext, pub enabled: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType { Spawn, SetPosition, SetVelocity, SetColor, SetSize, Kill, Output }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimSpace { Local, World }

impl Default for VfxGraphPanel {
    fn default() -> Self {
        Self {
            visible: false, vfx_name: "FireVFX".into(), active_context: VfxContext::Spawn,
            nodes: vec![
                VfxNode { name: "SpawnBurst".into(), node_type: NodeType::Spawn, context: VfxContext::Spawn, enabled: true },
                VfxNode { name: "InitPos".into(), node_type: NodeType::SetPosition, context: VfxContext::Initialize, enabled: true },
                VfxNode { name: "InitVel".into(), node_type: NodeType::SetVelocity, context: VfxContext::Initialize, enabled: true },
                VfxNode { name: "UpdateColor".into(), node_type: NodeType::SetColor, context: VfxContext::Update, enabled: true },
                VfxNode { name: "UpdateSize".into(), node_type: NodeType::SetSize, context: VfxContext::Update, enabled: true },
                VfxNode { name: "Output".into(), node_type: NodeType::Output, context: VfxContext::Output, enabled: true },
            ],
            selected_node: Some(0), max_particles: 10000, simulation_space: SimSpace::World, play_on_start: true, looping: true, duration: 5.0, new_node_name: String::new(),
        }
    }
}

impl EditorPanel for VfxGraphPanel {
    fn name(&self) -> &str { "VFX Graph" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.label("VFX:"); ui.label(&self.vfx_name); if ui.button("Compile").clicked() {} });
            ui.separator();
            ui.heading("System Settings");
            ui.horizontal(|ui| { ui.label("Max Particles:"); ui.add(egui::DragValue::new(&mut self.max_particles).range(100..=1000000)); });
            ui.horizontal(|ui| { ui.label("Sim Space:"); ui.selectable_value(&mut self.simulation_space, SimSpace::Local, "Local"); ui.selectable_value(&mut self.simulation_space, SimSpace::World, "World"); });
            ui.checkbox(&mut self.play_on_start, "Play on Start");
            ui.checkbox(&mut self.looping, "Looping");
            ui.horizontal(|ui| { ui.label("Duration:"); ui.add(egui::Slider::new(&mut self.duration, 0.1..=30.0)); });
            ui.separator();
            ui.heading("Context");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_context, VfxContext::Spawn, "Spawn");
                ui.selectable_value(&mut self.active_context, VfxContext::Initialize, "Initialize");
                ui.selectable_value(&mut self.active_context, VfxContext::Update, "Update");
                ui.selectable_value(&mut self.active_context, VfxContext::Output, "Output");
            });
            ui.separator();
            ui.heading("Nodes");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.nodes.len() {
                    if self.nodes[i].context != self.active_context { continue; }
                    let selected = self.selected_node == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.nodes[i].name).clicked() { self.selected_node = Some(i); }
                        ui.label(format!("{:?}", self.nodes[i].node_type));
                        ui.checkbox(&mut self.nodes[i].enabled, "");
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| { ui.text_edit_singleline(&mut self.new_node_name); if ui.button("Add Node").clicked() && !self.new_node_name.is_empty() { self.nodes.push(VfxNode { name: self.new_node_name.clone(), node_type: NodeType::Spawn, context: self.active_context, enabled: true }); self.new_node_name.clear(); } });
        });
    }
}
