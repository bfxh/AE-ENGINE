//! 地形样条面板：道路和河流样条编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct LandscapeSplinePanel {
    pub visible: bool,
    pub spline_name: String,
    pub control_points: Vec<SplinePoint>,
    pub width: f32,
    pub segments: u32,
    pub road_type: RoadType,
    pub material_path: String,
    pub auto_tangent: bool,
    pub closed_loop: bool,
    pub selected_point: Option<usize>,
    pub thickness: f32,
    pub uv_tiling: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoadType {
    Road,
    River,
    Path,
    Fence,
}

#[derive(Debug, Clone)]
pub struct SplinePoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub roll: f32,
}

impl Default for LandscapeSplinePanel {
    fn default() -> Self {
        Self {
            visible: false,
            spline_name: "Spline01".to_string(),
            control_points: vec![
                SplinePoint { x: 0.0, y: 0.0, z: 0.0, roll: 0.0 },
                SplinePoint { x: 10.0, y: 0.0, z: 5.0, roll: 0.0 },
                SplinePoint { x: 20.0, y: 1.0, z: 10.0, roll: 5.0 },
            ],
            width: 4.0,
            segments: 32,
            road_type: RoadType::Road,
            material_path: "materials/road.mat".to_string(),
            auto_tangent: true,
            closed_loop: false,
            selected_point: None,
            thickness: 0.2,
            uv_tiling: 1.0,
        }
    }
}

impl EditorPanel for LandscapeSplinePanel {
    fn name(&self) -> &str { "Landscape Spline" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.spline_name);
                    ui.separator();
                    if ui.button("Add Point").clicked() {
                        self.control_points.push(SplinePoint { x: 0.0, y: 0.0, z: 0.0, roll: 0.0 });
                    }
                    if ui.button("Remove Last").clicked() {
                        self.control_points.pop();
                    }
                });
                ui.separator();
                ui.heading("Spline Properties");
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ui.radio_value(&mut self.road_type, RoadType::Road, "Road");
                    ui.radio_value(&mut self.road_type, RoadType::River, "River");
                    ui.radio_value(&mut self.road_type, RoadType::Path, "Path");
                    ui.radio_value(&mut self.road_type, RoadType::Fence, "Fence");
                });
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::Slider::new(&mut self.width, 0.1..=50.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Segments:");
                    ui.add(egui::DragValue::new(&mut self.segments).range(1..=256));
                });
                ui.horizontal(|ui| {
                    ui.label("Thickness:");
                    ui.add(egui::Slider::new(&mut self.thickness, 0.0..=5.0));
                });
                ui.horizontal(|ui| {
                    ui.label("UV Tiling:");
                    ui.add(egui::Slider::new(&mut self.uv_tiling, 0.01..=10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Material:");
                    ui.text_edit_singleline(&mut self.material_path);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.auto_tangent, "Auto Tangent");
                    ui.checkbox(&mut self.closed_loop, "Closed Loop");
                });
                ui.separator();
                ui.heading("Control Points");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, p) in self.control_points.iter_mut().enumerate() {
                        let selected = self.selected_point == Some(i);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, format!("Point {}", i)).clicked() {
                                self.selected_point = Some(i);
                            }
                            ui.add(egui::DragValue::new(&mut p.x).speed(0.1));
                            ui.add(egui::DragValue::new(&mut p.y).speed(0.1));
                            ui.add(egui::DragValue::new(&mut p.z).speed(0.1));
                            ui.add(egui::DragValue::new(&mut p.roll).speed(1.0));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.control_points.remove(i); }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Points: {}", self.control_points.len()));
                    ui.label(format!("Type: {:?}", self.road_type));
                });
            });
    }
}
