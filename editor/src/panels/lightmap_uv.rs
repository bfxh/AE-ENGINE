//! Lightmap UV panel: UV generation, packing, density, inspection.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct LightmapUvPanel {
    pub visible: bool,
    pub uv_generation: UvGenMethod,
    pub packing_quality: PackingQuality,
    pub texel_density: f32,
    pub lightmap_resolution: u32,
    pub padding: f32,
    pub max_charts: u32,
    pub angle_error: f32,
    pub area_error: f32,
    pub stats: UvStats,
    pub show_overlaps: bool,
    pub show_charts: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UvGenMethod { Automatic, Xatlas, Custom }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackingQuality { Fast, Balanced, Best }

#[derive(Debug, Clone)]
pub struct UvStats { pub charts: u32, pub overlaps: u32, pub coverage: f32, pub waste: f32 }

impl Default for LightmapUvPanel {
    fn default() -> Self {
        Self {
            visible: false, uv_generation: UvGenMethod::Automatic, packing_quality: PackingQuality::Balanced,
            texel_density: 10.0, lightmap_resolution: 1024, padding: 4.0, max_charts: 256, angle_error: 0.1, area_error: 0.1,
            stats: UvStats { charts: 0, overlaps: 0, coverage: 0.0, waste: 0.0 },
            show_overlaps: false, show_charts: true,
        }
    }
}

impl EditorPanel for LightmapUvPanel {
    fn name(&self) -> &str { "Lightmap UV" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(400.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("UV Generation");
            ui.horizontal(|ui| {
                ui.label("Method:");
                egui::ComboBox::from_id_source("uv_gen").selected_text(format!("{:?}", self.uv_generation)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.uv_generation, UvGenMethod::Automatic, "Automatic");
                    ui.selectable_value(&mut self.uv_generation, UvGenMethod::Xatlas, "Xatlas");
                    ui.selectable_value(&mut self.uv_generation, UvGenMethod::Custom, "Custom");
                });
            });
            ui.horizontal(|ui| { ui.label("Max Charts:"); ui.add(egui::DragValue::new(&mut self.max_charts).range(1..=1024)); });
            ui.horizontal(|ui| { ui.label("Angle Error:"); ui.add(egui::Slider::new(&mut self.angle_error, 0.0..=1.0)); });
            ui.horizontal(|ui| { ui.label("Area Error:"); ui.add(egui::Slider::new(&mut self.area_error, 0.0..=1.0)); });
            ui.separator();
            ui.heading("Packing");
            ui.horizontal(|ui| {
                ui.label("Quality:");
                egui::ComboBox::from_id_source("pack_q").selected_text(format!("{:?}", self.packing_quality)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.packing_quality, PackingQuality::Fast, "Fast");
                    ui.selectable_value(&mut self.packing_quality, PackingQuality::Balanced, "Balanced");
                    ui.selectable_value(&mut self.packing_quality, PackingQuality::Best, "Best");
                });
            });
            ui.horizontal(|ui| { ui.label("Texel Density:"); ui.add(egui::Slider::new(&mut self.texel_density, 1.0..=50.0)); });
            ui.horizontal(|ui| {
                ui.label("Lightmap Res:");
                egui::ComboBox::from_id_source("lm_res").selected_text(format!("{}px", self.lightmap_resolution)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.lightmap_resolution, 256, "256");
                    ui.selectable_value(&mut self.lightmap_resolution, 512, "512");
                    ui.selectable_value(&mut self.lightmap_resolution, 1024, "1024");
                    ui.selectable_value(&mut self.lightmap_resolution, 2048, "2048");
                    ui.selectable_value(&mut self.lightmap_resolution, 4096, "4096");
                });
            });
            ui.horizontal(|ui| { ui.label("Padding:"); ui.add(egui::Slider::new(&mut self.padding, 0.0..=16.0)); });
            ui.separator();
            ui.checkbox(&mut self.show_overlaps, "Show Overlaps");
            ui.checkbox(&mut self.show_charts, "Show Charts");
            ui.separator();
            if ui.button("Generate UVs").clicked() { self.stats = UvStats { charts: 45, overlaps: 0, coverage: 0.82, waste: 0.18 }; }
            ui.separator();
            ui.heading("Statistics");
            ui.label(format!("Charts: {}", self.stats.charts));
            ui.label(format!("Overlaps: {}", self.stats.overlaps));
            ui.label(format!("Coverage: {:.1}%", self.stats.coverage * 100.0));
            ui.label(format!("Waste: {:.1}%", self.stats.waste * 100.0));
        });
    }
}
