//! 导航网格编辑器面板：AI导航网格烘焙和编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct NavmeshEditorPanel {
    pub visible: bool,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub cell_size: f32,
    pub cell_height: f32,
    pub slope_limit: f32,
    pub step_height: f32,
    pub regions: Vec<NavRegion>,
    pub selected_region: Option<usize>,
    pub obstacles: Vec<NavObstacle>,
    pub show_navmesh: bool,
    pub show_obstacles: bool,
    pub bake_status: String,
    pub auto_rebuild: bool,
}

#[derive(Debug, Clone)]
pub struct NavRegion {
    pub name: String,
    pub area_type: AreaType,
    pub cost: f32,
    pub color: egui::Color32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AreaType {
    Walkable,
    NotWalkable,
    Door,
    Jump,
    Water,
}

#[derive(Debug, Clone)]
pub struct NavObstacle {
    pub name: String,
    pub shape: ObstacleShape,
    pub radius: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObstacleShape {
    Box,
    Cylinder,
    Capsule,
}

impl Default for NavmeshEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            agent_radius: 0.5,
            agent_height: 2.0,
            cell_size: 0.2,
            cell_height: 0.2,
            slope_limit: 45.0,
            step_height: 0.4,
            regions: vec![
                NavRegion { name: "Default".into(), area_type: AreaType::Walkable, cost: 1.0, color: egui::Color32::from_rgb(0, 255, 0) },
                NavRegion { name: "Door".into(), area_type: AreaType::Door, cost: 3.0, color: egui::Color32::from_rgb(255, 255, 0) },
                NavRegion { name: "Water".into(), area_type: AreaType::Water, cost: 5.0, color: egui::Color32::from_rgb(0, 0, 255) },
            ],
            selected_region: Some(0),
            obstacles: vec![
                NavObstacle { name: "Wall".into(), shape: ObstacleShape::Box, radius: 1.0, height: 3.0 },
            ],
            show_navmesh: true,
            show_obstacles: true,
            bake_status: "Not baked".to_string(),
            auto_rebuild: false,
        }
    }
}

impl EditorPanel for NavmeshEditorPanel {
    fn name(&self) -> &str { "Navmesh Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Bake").clicked() {
                        self.bake_status = "Baking...".to_string();
                    }
                    if ui.button("Clear").clicked() {
                        self.bake_status = "Cleared".to_string();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_navmesh, "Show Navmesh");
                    ui.checkbox(&mut self.show_obstacles, "Show Obstacles");
                    ui.checkbox(&mut self.auto_rebuild, "Auto Rebuild");
                });
                ui.separator();
                ui.label(format!("Status: {}", self.bake_status));
                ui.separator();
                ui.heading("Agent Settings");
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(egui::Slider::new(&mut self.agent_radius, 0.05..=5.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Height:");
                    ui.add(egui::Slider::new(&mut self.agent_height, 0.5..=10.0));
                });
                ui.separator();
                ui.heading("Bake Settings");
                ui.horizontal(|ui| {
                    ui.label("Cell Size:");
                    ui.add(egui::Slider::new(&mut self.cell_size, 0.05..=2.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Cell Height:");
                    ui.add(egui::Slider::new(&mut self.cell_height, 0.05..=2.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Slope Limit:");
                    ui.add(egui::Slider::new(&mut self.slope_limit, 0.0..=90.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Step Height:");
                    ui.add(egui::Slider::new(&mut self.step_height, 0.0..=2.0));
                });
                ui.separator();
                ui.heading("Regions");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, r) in self.regions.iter_mut().enumerate() {
                        let selected = self.selected_region == Some(i);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &r.name).clicked() {
                                self.selected_region = Some(i);
                            }
                            ui.color_edit_button_srgba(&mut r.color);
                            ui.add(egui::DragValue::new(&mut r.cost).speed(0.1).range(0.0..=100.0));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.regions.remove(i); }
                });
                if ui.button("Add Region").clicked() {
                    self.regions.push(NavRegion { name: format!("Region {}", self.regions.len()), area_type: AreaType::Walkable, cost: 1.0, color: egui::Color32::WHITE });
                }
                ui.separator();
                ui.heading("Obstacles");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, o) in self.obstacles.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut o.name);
                            ui.radio_value(&mut o.shape, ObstacleShape::Box, "Box");
                            ui.radio_value(&mut o.shape, ObstacleShape::Cylinder, "Cyl");
                            ui.radio_value(&mut o.shape, ObstacleShape::Capsule, "Cap");
                            ui.add(egui::DragValue::new(&mut o.radius).speed(0.1));
                            ui.add(egui::DragValue::new(&mut o.height).speed(0.1));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.obstacles.remove(i); }
                });
                if ui.button("Add Obstacle").clicked() {
                    self.obstacles.push(NavObstacle { name: format!("Obstacle {}", self.obstacles.len()), shape: ObstacleShape::Box, radius: 1.0, height: 2.0 });
                }
            });
    }
}
