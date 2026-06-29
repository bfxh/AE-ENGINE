//! View Modes panel: control viewport rendering mode and display options.
//!
//! Provides render mode selection, display toggles, and camera settings.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// Render mode for the viewport.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderMode {
    Lit,
    Unlit,
    Wireframe,
    Depth,
    Normals,
    Albedo,
    Overdraw,
    Complexity,
    Metallic,
    Roughness,
    Emissive,
}

impl RenderMode {
    fn label(&self) -> &'static str {
        match self {
            RenderMode::Lit => "Lit",
            RenderMode::Unlit => "Unlit",
            RenderMode::Wireframe => "Wireframe",
            RenderMode::Depth => "Depth",
            RenderMode::Normals => "Normals",
            RenderMode::Albedo => "Albedo",
            RenderMode::Overdraw => "Overdraw",
            RenderMode::Complexity => "Shader Complexity",
            RenderMode::Metallic => "Metallic",
            RenderMode::Roughness => "Roughness",
            RenderMode::Emissive => "Emissive",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            RenderMode::Lit => "[L]",
            RenderMode::Unlit => "[U]",
            RenderMode::Wireframe => "[W]",
            RenderMode::Depth => "[D]",
            RenderMode::Normals => "[N]",
            RenderMode::Albedo => "[A]",
            RenderMode::Overdraw => "[O]",
            RenderMode::Complexity => "[C]",
            RenderMode::Metallic => "[M]",
            RenderMode::Roughness => "[R]",
            RenderMode::Emissive => "[E]",
        }
    }
}

/// View Modes panel state.
pub struct ViewModesPanel {
    pub visible: bool,
    pub render_mode: RenderMode,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_bounds: bool,
    pub show_stats: bool,
    pub show_fps: bool,
    pub show_origin: bool,
    pub show_light_icons: bool,
    pub show_camera_icons: bool,
    pub show_names: bool,
    pub fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub lod_bias: f32,
    pub exposure: f32,
    pub gamma: f32,
}

impl Default for ViewModesPanel {
    fn default() -> Self {
        Self {
            visible: false,
            render_mode: RenderMode::Lit,
            show_grid: true,
            show_axes: true,
            show_bounds: false,
            show_stats: true,
            show_fps: true,
            show_origin: true,
            show_light_icons: true,
            show_camera_icons: true,
            show_names: true,
            fov: 60.0,
            near_plane: 0.1,
            far_plane: 1000.0,
            lod_bias: 0.0,
            exposure: 1.0,
            gamma: 2.2,
        }
    }
}

impl EditorPanel for ViewModesPanel {
    fn name(&self) -> &str { "View Modes" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("View Modes").default_width(300.0).resizable(true).show(ctx, |ui| {
            // Render Mode section.
            ui.label(egui::RichText::new("Render Mode").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            // Render mode grid.
            let modes = [
                RenderMode::Lit, RenderMode::Unlit, RenderMode::Wireframe,
                RenderMode::Depth, RenderMode::Normals, RenderMode::Albedo,
                RenderMode::Overdraw, RenderMode::Complexity,
                RenderMode::Metallic, RenderMode::Roughness, RenderMode::Emissive,
            ];
            egui::Grid::new("render_mode_grid").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, mode) in modes.iter().enumerate() {
                    let is_active = self.render_mode == *mode;
                    let color = if is_active {
                        egui::Color32::from_rgb(100, 180, 255)
                    } else {
                        egui::Color32::from_rgb(180, 180, 180)
                    };
                    let label = format!("{} {}", mode.icon(), mode.label());
                    if ui.selectable_label(is_active, egui::RichText::new(&label).color(color).small()).clicked() {
                        self.render_mode = *mode;
                    }
                    if (i + 1) % 3 == 0 { ui.end_row(); }
                }
            });

            ui.add_space(8.0);

            // Display section.
            ui.label(egui::RichText::new("Display").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            egui::Grid::new("display_grid").num_columns(2).spacing([8.0, 3.0]).show(ui, |ui| {
                ui.checkbox(&mut self.show_grid, "Grid");
                ui.checkbox(&mut self.show_axes, "Axes");
                ui.checkbox(&mut self.show_bounds, "Bounds");
                ui.checkbox(&mut self.show_stats, "Stats Overlay");
                ui.checkbox(&mut self.show_fps, "FPS Counter");
                ui.checkbox(&mut self.show_origin, "Origin Marker");
                ui.checkbox(&mut self.show_light_icons, "Light Icons");
                ui.checkbox(&mut self.show_camera_icons, "Camera Icons");
                ui.checkbox(&mut self.show_names, "Node Names");
            });

            ui.add_space(8.0);

            // Camera section.
            ui.label(egui::RichText::new("Camera").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            egui::Grid::new("camera_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("FOV:");
                ui.add(egui::Slider::new(&mut self.fov, 10.0..=170.0).suffix(" deg"));
                ui.end_row();

                ui.label("Near Plane:");
                ui.add(egui::DragValue::new(&mut self.near_plane).speed(0.01).range(0.001..=10.0));
                ui.end_row();

                ui.label("Far Plane:");
                ui.add(egui::DragValue::new(&mut self.far_plane).speed(1.0).range(100.0..=100000.0));
                ui.end_row();
            });

            ui.add_space(8.0);

            // Post-processing section.
            ui.label(egui::RichText::new("Post-Processing").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            egui::Grid::new("post_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("LOD Bias:");
                ui.add(egui::Slider::new(&mut self.lod_bias, -2.0..=2.0));
                ui.end_row();

                ui.label("Exposure:");
                ui.add(egui::Slider::new(&mut self.exposure, 0.0..=4.0));
                ui.end_row();

                ui.label("Gamma:");
                ui.add(egui::Slider::new(&mut self.gamma, 1.0..=3.0));
                ui.end_row();
            });

            ui.add_space(8.0);

            // Quick presets.
            ui.label(egui::RichText::new("Presets").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Default").clicked() {
                    self.render_mode = RenderMode::Lit;
                    self.show_grid = true;
                    self.show_axes = true;
                    self.fov = 60.0;
                    self.exposure = 1.0;
                    self.gamma = 2.2;
                }
                if ui.small_button("Debug").clicked() {
                    self.render_mode = RenderMode::Normals;
                    self.show_bounds = true;
                    self.show_stats = true;
                }
                if ui.small_button("Wireframe").clicked() {
                    self.render_mode = RenderMode::Wireframe;
                    self.show_grid = true;
                }
                if ui.small_button("Performance").clicked() {
                    self.render_mode = RenderMode::Complexity;
                    self.show_stats = true;
                    self.show_fps = true;
                }
            });
        });
    }
}
