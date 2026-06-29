//! Procedural foliage panel: distribution rules, density map, LOD, collision.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ProceduralFoliagePanel {
    pub visible: bool,
    pub spawners: Vec<FoliageSpawner>,
    pub selected_spawner: Option<usize>,
    pub area_size: f32,
    pub seed: u32,
    pub density_scale: f32,
    pub min_distance: f32,
    pub max_lod: i32,
    pub enable_collision: bool,
    pub collision_simplify: bool,
    pub show_density_map: bool,
    pub regenerate: bool,
}

#[derive(Debug, Clone)]
pub struct FoliageSpawner { pub name: String, pub mesh: String, pub density: f32, pub radius: f32, pub slope_max: f32, pub height_min: f32, pub height_max: f32, pub enabled: bool }

impl Default for ProceduralFoliagePanel {
    fn default() -> Self {
        Self {
            visible: false,
            spawners: vec![
                FoliageSpawner { name: "Tree_Oak".into(), mesh: "oak.fbx".into(), density: 0.5, radius: 3.0, slope_max: 45.0, height_min: 0.0, height_max: 50.0, enabled: true },
                FoliageSpawner { name: "Tree_Pine".into(), mesh: "pine.fbx".into(), density: 0.8, radius: 2.5, slope_max: 60.0, height_min: 10.0, height_max: 80.0, enabled: true },
                FoliageSpawner { name: "Rock_Large".into(), mesh: "rock.fbx".into(), density: 0.2, radius: 1.0, slope_max: 90.0, height_min: 0.0, height_max: 100.0, enabled: false },
            ],
            selected_spawner: Some(0), area_size: 100.0, seed: 12345, density_scale: 1.0, min_distance: 1.0, max_lod: 3, enable_collision: true, collision_simplify: true, show_density_map: false, regenerate: false,
        }
    }
}

impl EditorPanel for ProceduralFoliagePanel {
    fn name(&self) -> &str { "Procedural Foliage" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(450.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Generation Settings");
            ui.horizontal(|ui| { ui.label("Area Size:"); ui.add(egui::DragValue::new(&mut self.area_size).range(10.0..=1000.0)); });
            ui.horizontal(|ui| { ui.label("Seed:"); ui.add(egui::DragValue::new(&mut self.seed).range(0..=999999)); });
            ui.horizontal(|ui| { ui.label("Density Scale:"); ui.add(egui::Slider::new(&mut self.density_scale, 0.0..=5.0)); });
            ui.horizontal(|ui| { ui.label("Min Distance:"); ui.add(egui::Slider::new(&mut self.min_distance, 0.1..=10.0)); });
            ui.separator();
            ui.checkbox(&mut self.show_density_map, "Show Density Map");
            ui.checkbox(&mut self.enable_collision, "Enable Collision");
            ui.checkbox(&mut self.collision_simplify, "Simplify Collision");
            ui.horizontal(|ui| { ui.label("Max LOD:"); ui.add(egui::DragValue::new(&mut self.max_lod).range(1..=5)); });
            ui.separator();
            if ui.button("Regenerate").clicked() { self.regenerate = true; }
            ui.separator();
            ui.heading("Spawners");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.spawners.len() {
                    let selected = self.selected_spawner == Some(i);
                    ui.horizontal(|ui| { if ui.selectable_label(selected, &self.spawners[i].name).clicked() { self.selected_spawner = Some(i); } ui.checkbox(&mut self.spawners[i].enabled, "Enabled"); });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_spawner { if idx < self.spawners.len() {
                ui.heading("Spawner Properties");
                ui.label(format!("Mesh: {}", self.spawners[idx].mesh));
                ui.horizontal(|ui| { ui.label("Density:"); ui.add(egui::Slider::new(&mut self.spawners[idx].density, 0.0..=5.0)); });
                ui.horizontal(|ui| { ui.label("Radius:"); ui.add(egui::Slider::new(&mut self.spawners[idx].radius, 0.1..=20.0)); });
                ui.horizontal(|ui| { ui.label("Max Slope:"); ui.add(egui::Slider::new(&mut self.spawners[idx].slope_max, 0.0..=90.0)); });
                ui.horizontal(|ui| { ui.label("Height Min:"); ui.add(egui::DragValue::new(&mut self.spawners[idx].height_min).range(0.0..=500.0)); });
                ui.horizontal(|ui| { ui.label("Height Max:"); ui.add(egui::DragValue::new(&mut self.spawners[idx].height_max).range(0.0..=500.0)); });
            }}
        });
    }
}
