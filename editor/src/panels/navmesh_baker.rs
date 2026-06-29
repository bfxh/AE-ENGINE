//! Navmesh baker panel: area settings, bake parameters, LOD.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct NavmeshBakerPanel {
    pub visible: bool,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub max_slope: f32,
    pub step_height: f32,
    pub cell_size: f32,
    pub cell_height: f32,
    pub tile_size: f32,
    pub min_region_area: f32,
    pub merge_regions: bool,
    pub lod_levels: i32,
    pub bake_status: String,
    pub stats: NavmeshStats,
    pub auto_rebuild: bool,
}

#[derive(Debug, Clone)]
pub struct NavmeshStats { pub tiles: u32, pub polygons: u32, pub vertices: u32, pub build_time_ms: f32 }

impl Default for NavmeshBakerPanel {
    fn default() -> Self {
        Self {
            visible: false, agent_radius: 0.5, agent_height: 2.0, max_slope: 45.0, step_height: 0.4,
            cell_size: 0.2, cell_height: 0.2, tile_size: 32.0, min_region_area: 3.0, merge_regions: true,
            lod_levels: 2, bake_status: "Not baked".into(),
            stats: NavmeshStats { tiles: 0, polygons: 0, vertices: 0, build_time_ms: 0.0 },
            auto_rebuild: false,
        }
    }
}

impl EditorPanel for NavmeshBakerPanel {
    fn name(&self) -> &str { "Navmesh Baker" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(400.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Agent Settings");
            ui.horizontal(|ui| { ui.label("Agent Radius:"); ui.add(egui::Slider::new(&mut self.agent_radius, 0.1..=5.0)); });
            ui.horizontal(|ui| { ui.label("Agent Height:"); ui.add(egui::Slider::new(&mut self.agent_height, 0.5..=10.0)); });
            ui.horizontal(|ui| { ui.label("Max Slope:"); ui.add(egui::Slider::new(&mut self.max_slope, 0.0..=90.0)); });
            ui.horizontal(|ui| { ui.label("Step Height:"); ui.add(egui::Slider::new(&mut self.step_height, 0.0..=2.0)); });
            ui.separator();
            ui.heading("Bake Parameters");
            ui.horizontal(|ui| { ui.label("Cell Size:"); ui.add(egui::Slider::new(&mut self.cell_size, 0.05..=1.0)); });
            ui.horizontal(|ui| { ui.label("Cell Height:"); ui.add(egui::Slider::new(&mut self.cell_height, 0.05..=1.0)); });
            ui.horizontal(|ui| { ui.label("Tile Size:"); ui.add(egui::Slider::new(&mut self.tile_size, 8.0..=128.0)); });
            ui.horizontal(|ui| { ui.label("Min Region Area:"); ui.add(egui::Slider::new(&mut self.min_region_area, 0.0..=20.0)); });
            ui.checkbox(&mut self.merge_regions, "Merge Regions");
            ui.separator();
            ui.heading("LOD");
            ui.horizontal(|ui| { ui.label("LOD Levels:"); ui.add(egui::DragValue::new(&mut self.lod_levels).range(1..=5)); });
            ui.separator();
            ui.checkbox(&mut self.auto_rebuild, "Auto Rebuild");
            ui.horizontal(|ui| { if ui.button("Bake").clicked() { self.bake_status = "Baking...".into(); self.stats = NavmeshStats { tiles: 64, polygons: 12500, vertices: 8200, build_time_ms: 342.5 }; self.bake_status = "Baked".into(); } ui.label(&self.bake_status); });
            ui.separator();
            ui.heading("Statistics");
            ui.label(format!("Tiles: {}", self.stats.tiles));
            ui.label(format!("Polygons: {}", self.stats.polygons));
            ui.label(format!("Vertices: {}", self.stats.vertices));
            ui.label(format!("Build Time: {:.1} ms", self.stats.build_time_ms));
        });
    }
}
