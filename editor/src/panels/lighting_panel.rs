//! 光照面板：场景光源和全局光照设置。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct LightingPanel {
    pub visible: bool,
    pub lights: Vec<SceneLight>,
    pub selected_light: Option<usize>,
    pub gi_enabled: bool,
    pub gi_intensity: f32,
    pub gi_quality: GiQuality,
    pub ambient_color: egui::Color32,
    pub ambient_intensity: f32,
    pub sky_light_intensity: f32,
    pub sky_light_color: egui::Color32,
    pub shadow_quality: ShadowQuality,
    pub shadow_distance: f32,
    pub new_light_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GiQuality {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadowQuality {
    Off,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct SceneLight {
    pub name: String,
    pub light_type: LightType,
    pub color: egui::Color32,
    pub intensity: f32,
    pub range: f32,
    pub cast_shadows: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
    Area,
}

impl Default for LightingPanel {
    fn default() -> Self {
        Self {
            visible: false,
            lights: vec![
                SceneLight { name: "Sun".into(), light_type: LightType::Directional, color: egui::Color32::from_rgb(255, 240, 200), intensity: 3.0, range: 100.0, cast_shadows: true, enabled: true },
                SceneLight { name: "Lamp".into(), light_type: LightType::Point, color: egui::Color32::from_rgb(255, 200, 150), intensity: 2.0, range: 20.0, cast_shadows: true, enabled: true },
            ],
            selected_light: Some(0),
            gi_enabled: true,
            gi_intensity: 1.0,
            gi_quality: GiQuality::Medium,
            ambient_color: egui::Color32::from_rgb(50, 50, 60),
            ambient_intensity: 0.3,
            sky_light_intensity: 1.0,
            sky_light_color: egui::Color32::from_rgb(150, 180, 255),
            shadow_quality: ShadowQuality::High,
            shadow_distance: 200.0,
            new_light_name: String::new(),
        }
    }
}

impl EditorPanel for LightingPanel {
    fn name(&self) -> &str { "Lighting" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(600.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New Light:");
                    ui.text_edit_singleline(&mut self.new_light_name);
                    if ui.button("Add").clicked() && !self.new_light_name.is_empty() {
                        self.lights.push(SceneLight { name: self.new_light_name.clone(), light_type: LightType::Point, color: egui::Color32::WHITE, intensity: 1.0, range: 10.0, cast_shadows: false, enabled: true });
                        self.new_light_name.clear();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Lights");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut remove_idx: Option<usize> = None;
                            for (i, l) in self.lights.iter_mut().enumerate() {
                                let selected = self.selected_light == Some(i);
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut l.enabled, "");
                                    if ui.selectable_label(selected, format!("[{:?}] {}", l.light_type, l.name)).clicked() {
                                        self.selected_light = Some(i);
                                    }
                                    if ui.button("X").clicked() { remove_idx = Some(i); }
                                });
                            }
                            if let Some(i) = remove_idx { self.lights.remove(i); }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Light Properties");
                        ui.separator();
                        if let Some(li) = self.selected_light {
                            if li < self.lights.len() {
                                let light = &mut self.lights[li];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut light.name);
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    ui.radio_value(&mut light.light_type, LightType::Directional, "Dir");
                                    ui.radio_value(&mut light.light_type, LightType::Point, "Point");
                                    ui.radio_value(&mut light.light_type, LightType::Spot, "Spot");
                                    ui.radio_value(&mut light.light_type, LightType::Area, "Area");
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Color:");
                                    ui.color_edit_button_srgba(&mut light.color);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Intensity:");
                                    ui.add(egui::Slider::new(&mut light.intensity, 0.0..=20.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Range:");
                                    ui.add(egui::Slider::new(&mut light.range, 0.1..=500.0));
                                });
                                ui.checkbox(&mut light.cast_shadows, "Cast Shadows");
                            }
                        } else {
                            ui.label("Select a light");
                        }
                    });
                });
                ui.separator();
                ui.heading("Global Illumination");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.gi_enabled, "Enabled");
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut self.gi_intensity, 0.0..=5.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Quality:");
                    ui.radio_value(&mut self.gi_quality, GiQuality::Low, "Low");
                    ui.radio_value(&mut self.gi_quality, GiQuality::Medium, "Medium");
                    ui.radio_value(&mut self.gi_quality, GiQuality::High, "High");
                    ui.radio_value(&mut self.gi_quality, GiQuality::Ultra, "Ultra");
                });
                ui.separator();
                ui.heading("Ambient");
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_srgba(&mut self.ambient_color);
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut self.ambient_intensity, 0.0..=3.0));
                });
                ui.separator();
                ui.heading("Sky Light");
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_srgba(&mut self.sky_light_color);
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut self.sky_light_intensity, 0.0..=5.0));
                });
                ui.separator();
                ui.heading("Shadows");
                ui.horizontal(|ui| {
                    ui.label("Quality:");
                    ui.radio_value(&mut self.shadow_quality, ShadowQuality::Off, "Off");
                    ui.radio_value(&mut self.shadow_quality, ShadowQuality::Low, "Low");
                    ui.radio_value(&mut self.shadow_quality, ShadowQuality::Medium, "Med");
                    ui.radio_value(&mut self.shadow_quality, ShadowQuality::High, "High");
                    ui.label("Distance:");
                    ui.add(egui::DragValue::new(&mut self.shadow_distance).speed(1.0).range(10.0..=1000.0));
                });
            });
    }
}
