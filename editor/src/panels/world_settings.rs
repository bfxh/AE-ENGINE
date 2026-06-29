//! World Settings panel: configures the current scene's global parameters.
//!
//! Allows editing world bounds, gravity, ambient light, fog, and other
//! scene-level settings.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// World settings panel state.
pub struct WorldSettingsPanel {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Gravity vector.
    pub gravity: [f32; 3],
    /// Ambient light color.
    pub ambient_color: [f32; 3],
    /// Ambient light intensity.
    pub ambient_intensity: f32,
    /// Fog color.
    pub fog_color: [f32; 3],
    /// Fog density.
    pub fog_density: f32,
    /// Fog start distance.
    pub fog_start: f32,
    /// Fog end distance.
    pub fog_end: f32,
    /// World bounds min.
    pub world_min: [f32; 3],
    /// World bounds max.
    pub world_max: [f32; 3],
    /// Time of day (0-24).
    pub time_of_day: f32,
    /// Day length (seconds).
    pub day_length: f32,
    /// Wind direction.
    pub wind_direction: [f32; 3],
    /// Wind strength.
    pub wind_strength: f32,
    /// Temperature (Celsius).
    pub temperature: f32,
    /// Humidity (0-1).
    pub humidity: f32,
    /// Enable physics simulation.
    pub enable_physics: bool,
    /// Show collision shapes.
    pub show_collision_shapes: bool,
    /// Show velocity vectors.
    pub show_velocity_vectors: bool,
    /// Enable shadows.
    pub enable_shadows: bool,
    /// Enable global illumination.
    pub enable_gi: bool,
    /// Enable fog.
    pub enable_fog: bool,
}

impl Default for WorldSettingsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            gravity: [0.0, -9.81, 0.0],
            ambient_color: [0.2, 0.2, 0.3],
            ambient_intensity: 0.3,
            fog_color: [0.5, 0.5, 0.6],
            fog_density: 0.01,
            fog_start: 50.0,
            fog_end: 500.0,
            world_min: [-1000.0, -1000.0, -1000.0],
            world_max: [1000.0, 1000.0, 1000.0],
            time_of_day: 12.0,
            day_length: 1200.0,
            wind_direction: [1.0, 0.0, 0.0],
            wind_strength: 0.5,
            temperature: 20.0,
            humidity: 0.5,
            enable_physics: true,
            show_collision_shapes: false,
            show_velocity_vectors: false,
            enable_shadows: true,
            enable_gi: false,
            enable_fog: false,
        }
    }
}

impl EditorPanel for WorldSettingsPanel {
    fn name(&self) -> &str {
        "World Settings"
    }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible {
            return;
        }

        egui::Window::new("World Settings")
            .default_width(380.0)
            .default_height(500.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Physics.
                ui.collapsing("Physics", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Gravity:");
                        ui.add(egui::DragValue::new(&mut self.gravity[0]).speed(0.1).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut self.gravity[1]).speed(0.1).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut self.gravity[2]).speed(0.1).prefix("z: "));
                    });
                    ui.checkbox(&mut self.enable_physics, "Enable physics simulation");
                    ui.checkbox(&mut self.show_collision_shapes, "Show collision shapes");
                    ui.checkbox(&mut self.show_velocity_vectors, "Show velocity vectors");
                });

                ui.separator();

                // Lighting.
                ui.collapsing("Lighting", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Ambient Color:");
                        ui.color_edit_button_rgb(&mut self.ambient_color);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Ambient Intensity:");
                        ui.add(egui::Slider::new(&mut self.ambient_intensity, 0.0..=2.0));
                    });
                    ui.checkbox(&mut self.enable_shadows, "Enable shadows");
                    ui.checkbox(&mut self.enable_gi, "Enable global illumination");
                });

                ui.separator();

                // Fog.
                ui.collapsing("Fog", |ui| {
                    ui.checkbox(&mut self.enable_fog, "Enable fog");
                    ui.horizontal(|ui| {
                        ui.label("Fog Color:");
                        ui.color_edit_button_rgb(&mut self.fog_color);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Density:");
                        ui.add(egui::DragValue::new(&mut self.fog_density).speed(0.001).range(0.0..=0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Start:");
                        ui.add(egui::DragValue::new(&mut self.fog_start).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("End:");
                        ui.add(egui::DragValue::new(&mut self.fog_end).speed(1.0));
                    });
                });

                ui.separator();

                // World Bounds.
                ui.collapsing("World Bounds", |ui| {
                    ui.label("Min:");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.world_min[0]).speed(1.0).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut self.world_min[1]).speed(1.0).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut self.world_min[2]).speed(1.0).prefix("z: "));
                    });
                    ui.label("Max:");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.world_max[0]).speed(1.0).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut self.world_max[1]).speed(1.0).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut self.world_max[2]).speed(1.0).prefix("z: "));
                    });
                });

                ui.separator();

                // Environment.
                ui.collapsing("Environment", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Time of Day:");
                        ui.add(egui::Slider::new(&mut self.time_of_day, 0.0..=24.0).suffix("h"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Day Length:");
                        ui.add(egui::DragValue::new(&mut self.day_length).speed(10.0).suffix("s"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Temperature:");
                        ui.add(egui::DragValue::new(&mut self.temperature).speed(0.5).suffix("°C"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Humidity:");
                        ui.add(egui::Slider::new(&mut self.humidity, 0.0..=1.0));
                    });
                });

                ui.separator();

                // Wind.
                ui.collapsing("Wind", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Direction:");
                        ui.add(egui::DragValue::new(&mut self.wind_direction[0]).speed(0.1).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut self.wind_direction[1]).speed(0.1).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut self.wind_direction[2]).speed(0.1).prefix("z: "));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(&mut self.wind_strength, 0.0..=20.0));
                    });
                });

                ui.separator();

                // Scene info.
                ui.heading("Scene Info");
                ui.separator();
                ui.label(format!("Name: {}", app.scene.name));
                ui.label(format!("Nodes: {}", app.scene.nodes.len()));
                ui.label(format!("Dirty: {}", app.dirty));
                if let Some(ref path) = app.scene_path {
                    ui.label(format!("Path: {}", path));
                }
            });
    }
}
