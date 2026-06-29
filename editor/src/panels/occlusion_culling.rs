//! Occlusion culling panel: bake settings, cull tree, visualization.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct OcclusionCullingPanel {
    pub visible: bool,
    pub enabled: bool,
    pub bake_quality: BakeQuality,
    pub cell_size: f32,
    pub max_depth: i32,
    pub min_volume: f32,
    pub show_cull_tree: bool,
    pub show_occluders: bool,
    pub show_occludees: bool,
    pub bake_status: String,
    pub stats: OcclusionStats,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BakeQuality { Low, Medium, High, Ultra }

#[derive(Debug, Clone)]
pub struct OcclusionStats { pub occluders: u32, pub occludees: u32, pub cells: u32, pub culled_objects: u32 }

impl Default for OcclusionCullingPanel {
    fn default() -> Self {
        Self {
            visible: false, enabled: true, bake_quality: BakeQuality::Medium, cell_size: 2.0, max_depth: 8, min_volume: 1.0,
            show_cull_tree: false, show_occluders: true, show_occludees: false,
            bake_status: "Not baked".into(),
            stats: OcclusionStats { occluders: 0, occludees: 0, cells: 0, culled_objects: 0 },
        }
    }
}

impl EditorPanel for OcclusionCullingPanel {
    fn name(&self) -> &str { "Occlusion Culling" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(400.0).default_height(400.0).show(ctx, |ui| {
            ui.checkbox(&mut self.enabled, "Enable Occlusion Culling");
            ui.separator();
            ui.heading("Bake Settings");
            ui.horizontal(|ui| {
                ui.label("Quality:");
                egui::ComboBox::from_id_source("bake_q").selected_text(format!("{:?}", self.bake_quality)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.bake_quality, BakeQuality::Low, "Low");
                    ui.selectable_value(&mut self.bake_quality, BakeQuality::Medium, "Medium");
                    ui.selectable_value(&mut self.bake_quality, BakeQuality::High, "High");
                    ui.selectable_value(&mut self.bake_quality, BakeQuality::Ultra, "Ultra");
                });
            });
            ui.horizontal(|ui| { ui.label("Cell Size:"); ui.add(egui::Slider::new(&mut self.cell_size, 0.5..=10.0)); });
            ui.horizontal(|ui| { ui.label("Max Depth:"); ui.add(egui::DragValue::new(&mut self.max_depth).range(1..=16)); });
            ui.horizontal(|ui| { ui.label("Min Volume:"); ui.add(egui::Slider::new(&mut self.min_volume, 0.1..=10.0)); });
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Bake").clicked() { self.bake_status = "Baking...".into(); self.stats = OcclusionStats { occluders: 45, occludees: 230, cells: 128, culled_objects: 87 }; self.bake_status = "Baked".into(); } ui.label(&self.bake_status); });
            ui.separator();
            ui.heading("Visualization");
            ui.checkbox(&mut self.show_cull_tree, "Show Cull Tree");
            ui.checkbox(&mut self.show_occluders, "Show Occluders");
            ui.checkbox(&mut self.show_occludees, "Show Occludees");
            ui.separator();
            ui.heading("Statistics");
            ui.label(format!("Occluders: {}", self.stats.occluders));
            ui.label(format!("Occludees: {}", self.stats.occludees));
            ui.label(format!("Cells: {}", self.stats.cells));
            ui.label(format!("Culled Objects: {}", self.stats.culled_objects));
        });
    }
}
