//! Modeling mode panel: extrude, bevel, boolean, cut, symmetry.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ModelingModePanel {
    pub visible: bool,
    pub active_tool: ModelingTool,
    pub extrude_distance: f32,
    pub bevel_width: f32,
    pub bevel_segments: i32,
    pub boolean_op: BooleanOp,
    pub cut_plane_axis: Axis,
    pub cut_position: f32,
    pub symmetry_axis: Axis,
    pub symmetry_enabled: bool,
    pub selected_vertices: u32,
    pub selected_edges: u32,
    pub selected_faces: u32,
    pub auto_merge: bool,
    pub snap_to_vertices: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelingTool { Select, Extrude, Bevel, Boolean, Cut, Symmetry, Move, Scale, Rotate }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BooleanOp { Union, Subtract, Intersect }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis { X, Y, Z }

impl Default for ModelingModePanel {
    fn default() -> Self {
        Self {
            visible: false, active_tool: ModelingTool::Select,
            extrude_distance: 1.0, bevel_width: 0.2, bevel_segments: 2,
            boolean_op: BooleanOp::Union, cut_plane_axis: Axis::Y, cut_position: 0.0,
            symmetry_axis: Axis::X, symmetry_enabled: false,
            selected_vertices: 0, selected_edges: 0, selected_faces: 0,
            auto_merge: true, snap_to_vertices: false,
        }
    }
}

impl EditorPanel for ModelingModePanel {
    fn name(&self) -> &str { "Modeling Mode" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(350.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tool, ModelingTool::Select, "Select");
                ui.selectable_value(&mut self.active_tool, ModelingTool::Extrude, "Extrude");
                ui.selectable_value(&mut self.active_tool, ModelingTool::Bevel, "Bevel");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tool, ModelingTool::Boolean, "Boolean");
                ui.selectable_value(&mut self.active_tool, ModelingTool::Cut, "Cut");
                ui.selectable_value(&mut self.active_tool, ModelingTool::Symmetry, "Symmetry");
            });
            ui.separator();
            match self.active_tool {
                ModelingTool::Extrude => {
                    ui.heading("Extrude");
                    ui.horizontal(|ui| { ui.label("Distance:"); ui.add(egui::Slider::new(&mut self.extrude_distance, -10.0..=10.0)); });
                },
                ModelingTool::Bevel => {
                    ui.heading("Bevel");
                    ui.horizontal(|ui| { ui.label("Width:"); ui.add(egui::Slider::new(&mut self.bevel_width, 0.0..=5.0)); });
                    ui.horizontal(|ui| { ui.label("Segments:"); ui.add(egui::DragValue::new(&mut self.bevel_segments).range(1..=10)); });
                },
                ModelingTool::Boolean => {
                    ui.heading("Boolean");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.boolean_op, BooleanOp::Union, "Union");
                        ui.selectable_value(&mut self.boolean_op, BooleanOp::Subtract, "Subtract");
                        ui.selectable_value(&mut self.boolean_op, BooleanOp::Intersect, "Intersect");
                    });
                    if ui.button("Apply Boolean").clicked() {}
                },
                ModelingTool::Cut => {
                    ui.heading("Cut");
                    ui.horizontal(|ui| { ui.label("Axis:"); ui.selectable_value(&mut self.cut_plane_axis, Axis::X, "X"); ui.selectable_value(&mut self.cut_plane_axis, Axis::Y, "Y"); ui.selectable_value(&mut self.cut_plane_axis, Axis::Z, "Z"); });
                    ui.horizontal(|ui| { ui.label("Position:"); ui.add(egui::Slider::new(&mut self.cut_position, -10.0..=10.0)); });
                    if ui.button("Cut").clicked() {}
                },
                ModelingTool::Symmetry => {
                    ui.heading("Symmetry");
                    ui.checkbox(&mut self.symmetry_enabled, "Enable Symmetry");
                    ui.horizontal(|ui| { ui.label("Axis:"); ui.selectable_value(&mut self.symmetry_axis, Axis::X, "X"); ui.selectable_value(&mut self.symmetry_axis, Axis::Y, "Y"); ui.selectable_value(&mut self.symmetry_axis, Axis::Z, "Z"); });
                },
                _ => {
                    ui.label("Select a tool to configure");
                },
            }
            ui.separator();
            ui.heading("Selection");
            ui.label(format!("Vertices: {}", self.selected_vertices));
            ui.label(format!("Edges: {}", self.selected_edges));
            ui.label(format!("Faces: {}", self.selected_faces));
            ui.separator();
            ui.heading("Options");
            ui.checkbox(&mut self.auto_merge, "Auto Merge");
            ui.checkbox(&mut self.snap_to_vertices, "Snap to Vertices");
        });
    }
}
