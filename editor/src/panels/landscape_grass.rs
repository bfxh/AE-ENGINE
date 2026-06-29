//! Landscape grass panel: grass types, density, scale, material-driven.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct LandscapeGrassPanel {
    pub visible: bool,
    pub grass_types: Vec<GrassType>,
    pub selected_type: Option<usize>,
    pub global_density: f32,
    pub global_scale: f32,
    pub wind_strength: f32,
    pub wind_speed: f32,
    pub cull_distance: f32,
    pub use_material_driven: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub lod_count: i32,
}

#[derive(Debug, Clone)]
pub struct GrassType { pub name: String, pub mesh: String, pub density: f32, pub min_scale: f32, pub max_scale: f32, pub enabled: bool, pub color: egui::Color32 }

impl Default for LandscapeGrassPanel {
    fn default() -> Self {
        Self {
            visible: false,
            grass_types: vec![
                GrassType { name: "Grass_A".into(), mesh: "grass_a.fbx".into(), density: 1.0, min_scale: 0.5, max_scale: 1.5, enabled: true, color: egui::Color32::from_rgb(100, 200, 50) },
                GrassType { name: "Bush_B".into(), mesh: "bush_b.fbx".into(), density: 0.3, min_scale: 0.8, max_scale: 2.0, enabled: true, color: egui::Color32::from_rgb(80, 150, 40) },
                GrassType { name: "Flower_C".into(), mesh: "flower_c.fbx".into(), density: 0.1, min_scale: 0.6, max_scale: 1.2, enabled: false, color: egui::Color32::from_rgb(255, 100, 100) },
            ],
            selected_type: Some(0), global_density: 1.0, global_scale: 1.0, wind_strength: 0.5, wind_speed: 1.0, cull_distance: 200.0,
            use_material_driven: true, cast_shadows: false, receive_shadows: true, lod_count: 3,
        }
    }
}

impl EditorPanel for LandscapeGrassPanel {
    fn name(&self) -> &str { "Landscape Grass" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(450.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Global Settings");
            ui.separator();
            ui.horizontal(|ui| { ui.label("Density:"); ui.add(egui::Slider::new(&mut self.global_density, 0.0..=5.0)); });
            ui.horizontal(|ui| { ui.label("Scale:"); ui.add(egui::Slider::new(&mut self.global_scale, 0.1..=5.0)); });
            ui.horizontal(|ui| { ui.label("Wind Strength:"); ui.add(egui::Slider::new(&mut self.wind_strength, 0.0..=2.0)); });
            ui.horizontal(|ui| { ui.label("Wind Speed:"); ui.add(egui::Slider::new(&mut self.wind_speed, 0.0..=5.0)); });
            ui.horizontal(|ui| { ui.label("Cull Distance:"); ui.add(egui::DragValue::new(&mut self.cull_distance).range(10.0..=1000.0)); });
            ui.checkbox(&mut self.use_material_driven, "Material Driven");
            ui.checkbox(&mut self.cast_shadows, "Cast Shadows");
            ui.checkbox(&mut self.receive_shadows, "Receive Shadows");
            ui.horizontal(|ui| { ui.label("LOD Count:"); ui.add(egui::DragValue::new(&mut self.lod_count).range(1..=5)); });
            ui.separator();
            ui.heading("Grass Types");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.grass_types.len() {
                    let selected = self.selected_type == Some(i);
                    let gc = self.grass_types[i].color;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.grass_types[i].name).clicked() { self.selected_type = Some(i); }
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, gc);
                        ui.checkbox(&mut self.grass_types[i].enabled, "Enabled");
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_type { if idx < self.grass_types.len() {
                ui.heading("Type Properties");
                ui.label(format!("Mesh: {}", self.grass_types[idx].mesh));
                ui.horizontal(|ui| { ui.label("Density:"); ui.add(egui::Slider::new(&mut self.grass_types[idx].density, 0.0..=5.0)); });
                ui.horizontal(|ui| { ui.label("Min Scale:"); ui.add(egui::Slider::new(&mut self.grass_types[idx].min_scale, 0.1..=5.0)); });
                ui.horizontal(|ui| { ui.label("Max Scale:"); ui.add(egui::Slider::new(&mut self.grass_types[idx].max_scale, 0.1..=10.0)); });
            }}
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Add Type").clicked() { self.grass_types.push(GrassType { name: "NewGrass".into(), mesh: "".into(), density: 1.0, min_scale: 0.5, max_scale: 1.5, enabled: true, color: egui::Color32::from_rgb(100, 200, 50) }); } if ui.button("Remove").clicked() { if let Some(idx) = self.selected_type { if idx < self.grass_types.len() { self.grass_types.remove(idx); self.selected_type = None; } } } });
        });
    }
}
