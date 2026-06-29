//! 后期处理面板：渲染后期效果设置。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PostProcessPanel {
    pub visible: bool,
    pub dof_enabled: bool,
    pub dof_focus_distance: f32,
    pub dof_aperture: f32,
    pub dof_focal_length: f32,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_radius: f32,
    pub bloom_tint: egui::Color32,
    pub vignette_enabled: bool,
    pub vignette_intensity: f32,
    pub vignette_color: egui::Color32,
    pub color_grading_enabled: bool,
    pub color_temperature: f32,
    pub color_tint: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub ssr_enabled: bool,
    pub ssr_intensity: f32,
    pub ssr_max_roughness: f32,
    pub tonemapper: Tonemapper,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tonemapper {
    None,
    ACES,
    Filmic,
    Reinhard,
    Linear,
}

impl Default for PostProcessPanel {
    fn default() -> Self {
        Self {
            visible: false,
            dof_enabled: false,
            dof_focus_distance: 10.0,
            dof_aperture: 2.8,
            dof_focal_length: 50.0,
            bloom_enabled: true,
            bloom_intensity: 1.0,
            bloom_threshold: 1.0,
            bloom_radius: 0.5,
            bloom_tint: egui::Color32::from_rgb(255, 255, 255),
            vignette_enabled: false,
            vignette_intensity: 0.5,
            vignette_color: egui::Color32::from_rgb(0, 0, 0),
            color_grading_enabled: true,
            color_temperature: 6500.0,
            color_tint: 0.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 2.2,
            ssr_enabled: false,
            ssr_intensity: 0.5,
            ssr_max_roughness: 0.6,
            tonemapper: Tonemapper::ACES,
        }
    }
}

impl EditorPanel for PostProcessPanel {
    fn name(&self) -> &str { "Post Process" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(450.0)
            .default_height(650.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Tonemapper:");
                    ui.radio_value(&mut self.tonemapper, Tonemapper::None, "None");
                    ui.radio_value(&mut self.tonemapper, Tonemapper::ACES, "ACES");
                    ui.radio_value(&mut self.tonemapper, Tonemapper::Filmic, "Filmic");
                    ui.radio_value(&mut self.tonemapper, Tonemapper::Reinhard, "Reinhard");
                });
                ui.separator();
                ui.heading("Depth of Field");
                ui.checkbox(&mut self.dof_enabled, "Enabled");
                if self.dof_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Focus Distance:");
                        ui.add(egui::Slider::new(&mut self.dof_focus_distance, 0.1..=1000.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Aperture:");
                        ui.add(egui::Slider::new(&mut self.dof_aperture, 0.5..=32.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Focal Length:");
                        ui.add(egui::Slider::new(&mut self.dof_focal_length, 10.0..=300.0));
                    });
                }
                ui.separator();
                ui.heading("Bloom");
                ui.checkbox(&mut self.bloom_enabled, "Enabled");
                if self.bloom_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        ui.add(egui::Slider::new(&mut self.bloom_intensity, 0.0..=10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Threshold:");
                        ui.add(egui::Slider::new(&mut self.bloom_threshold, 0.0..=5.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        ui.add(egui::Slider::new(&mut self.bloom_radius, 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tint:");
                        ui.color_edit_button_srgba(&mut self.bloom_tint);
                    });
                }
                ui.separator();
                ui.heading("Vignette");
                ui.checkbox(&mut self.vignette_enabled, "Enabled");
                if self.vignette_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        ui.add(egui::Slider::new(&mut self.vignette_intensity, 0.0..=2.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        ui.color_edit_button_srgba(&mut self.vignette_color);
                    });
                }
                ui.separator();
                ui.heading("Color Grading");
                ui.checkbox(&mut self.color_grading_enabled, "Enabled");
                if self.color_grading_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Temperature:");
                        ui.add(egui::Slider::new(&mut self.color_temperature, 1000.0..=20000.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tint:");
                        ui.add(egui::Slider::new(&mut self.color_tint, -100.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Saturation:");
                        ui.add(egui::Slider::new(&mut self.saturation, 0.0..=2.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Contrast:");
                        ui.add(egui::Slider::new(&mut self.contrast, 0.0..=2.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Gamma:");
                        ui.add(egui::Slider::new(&mut self.gamma, 0.1..=4.0));
                    });
                }
                ui.separator();
                ui.heading("Screen Space Reflections");
                ui.checkbox(&mut self.ssr_enabled, "Enabled");
                if self.ssr_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        ui.add(egui::Slider::new(&mut self.ssr_intensity, 0.0..=2.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max Roughness:");
                        ui.add(egui::Slider::new(&mut self.ssr_max_roughness, 0.0..=1.0));
                    });
                }
            });
    }
}
