//! 项目设置面板：渲染、输入、物理、音频标签页。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ProjectSettingsPanel {
    pub visible: bool,
    pub active_tab: SettingsTab,
    pub project_name: String,
    pub project_version: String,
    pub render_quality: RenderQuality,
    pub vsync: bool,
    pub max_fps: i32,
    pub shadow_resolution: u32,
    pub msaa_samples: u32,
    pub input_sensitivity: f32,
    pub invert_y: bool,
    pub gravity: f32,
    pub fixed_timestep: f32,
    pub max_substeps: i32,
    pub master_volume: f32,
    pub audio_device: String,
    pub spatial_audio: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsTab {
    General,
    Rendering,
    Input,
    Physics,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl Default for ProjectSettingsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            active_tab: SettingsTab::General,
            project_name: "Wasteland Project".into(),
            project_version: "0.1.0".into(),
            render_quality: RenderQuality::High,
            vsync: true,
            max_fps: 60,
            shadow_resolution: 2048,
            msaa_samples: 4,
            input_sensitivity: 1.0,
            invert_y: false,
            gravity: -9.81,
            fixed_timestep: 0.016,
            max_substeps: 4,
            master_volume: 0.8,
            audio_device: "Default".into(),
            spatial_audio: true,
        }
    }
}

impl EditorPanel for ProjectSettingsPanel {
    fn name(&self) -> &str { "Project Settings" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, SettingsTab::General, "General");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Rendering, "Rendering");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Input, "Input");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Physics, "Physics");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Audio, "Audio");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        SettingsTab::General => {
                            ui.heading("General");
                            ui.horizontal(|ui| { ui.label("Project Name:"); ui.text_edit_singleline(&mut self.project_name); });
                            ui.horizontal(|ui| { ui.label("Version:"); ui.text_edit_singleline(&mut self.project_version); });
                        },
                        SettingsTab::Rendering => {
                            ui.heading("Rendering");
                            ui.horizontal(|ui| {
                                ui.label("Quality:");
                                egui::ComboBox::from_id_source("rq_combo")
                                    .selected_text(format!("{:?}", self.render_quality))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.render_quality, RenderQuality::Low, "Low");
                                        ui.selectable_value(&mut self.render_quality, RenderQuality::Medium, "Medium");
                                        ui.selectable_value(&mut self.render_quality, RenderQuality::High, "High");
                                        ui.selectable_value(&mut self.render_quality, RenderQuality::Ultra, "Ultra");
                                    });
                            });
                            ui.checkbox(&mut self.vsync, "V-Sync");
                            ui.horizontal(|ui| { ui.label("Max FPS:"); ui.add(egui::DragValue::new(&mut self.max_fps).range(30..=240)); });
                            ui.horizontal(|ui| {
                                ui.label("Shadow Resolution:");
                                egui::ComboBox::from_id_source("shadow_combo")
                                    .selected_text(format!("{}px", self.shadow_resolution))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.shadow_resolution, 512, "512");
                                        ui.selectable_value(&mut self.shadow_resolution, 1024, "1024");
                                        ui.selectable_value(&mut self.shadow_resolution, 2048, "2048");
                                        ui.selectable_value(&mut self.shadow_resolution, 4096, "4096");
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("MSAA:");
                                egui::ComboBox::from_id_source("msaa_combo")
                                    .selected_text(format!("{}x", self.msaa_samples))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.msaa_samples, 1, "1x");
                                        ui.selectable_value(&mut self.msaa_samples, 2, "2x");
                                        ui.selectable_value(&mut self.msaa_samples, 4, "4x");
                                        ui.selectable_value(&mut self.msaa_samples, 8, "8x");
                                    });
                            });
                        },
                        SettingsTab::Input => {
                            ui.heading("Input");
                            ui.horizontal(|ui| { ui.label("Sensitivity:"); ui.add(egui::Slider::new(&mut self.input_sensitivity, 0.1..=3.0)); });
                            ui.checkbox(&mut self.invert_y, "Invert Y Axis");
                        },
                        SettingsTab::Physics => {
                            ui.heading("Physics");
                            ui.horizontal(|ui| { ui.label("Gravity:"); ui.add(egui::DragValue::new(&mut self.gravity).speed(0.1).range(-50.0..=0.0)); });
                            ui.horizontal(|ui| { ui.label("Fixed Timestep:"); ui.add(egui::DragValue::new(&mut self.fixed_timestep).speed(0.001).range(0.001..=0.1)); });
                            ui.horizontal(|ui| { ui.label("Max Substeps:"); ui.add(egui::DragValue::new(&mut self.max_substeps).range(1..=10)); });
                        },
                        SettingsTab::Audio => {
                            ui.heading("Audio");
                            ui.horizontal(|ui| { ui.label("Master Volume:"); ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=1.0)); });
                            ui.horizontal(|ui| { ui.label("Device:"); ui.text_edit_singleline(&mut self.audio_device); });
                            ui.checkbox(&mut self.spatial_audio, "Spatial Audio");
                        },
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {}
                    if ui.button("Reset to Defaults").clicked() {}
                    if ui.button("Close").clicked() { self.visible = false; }
                });
            });
    }
}
