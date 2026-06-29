//! Settings panel: editor configuration and preferences.
//!
//! Provides tabs for General, Editor, Shortcuts, and Theme settings.

use crate::app::EditorApp;
use crate::panels::EditorPanel;
use serde::{Deserialize, Serialize};

/// Settings panel state with persistent editor preferences.
#[derive(Serialize, Deserialize)]
pub struct SettingsPanel {
    #[serde(skip)]
    pub visible: bool,
    #[serde(skip)]
    pub tab: u32,
    // General settings
    pub auto_save_interval: f32,
    pub auto_save_enabled: bool,
    pub max_recent_files: u32,
    pub project_path: String,
    /// Recently opened scene file paths (most recent first).
    pub recent_files: Vec<String>,
    // Editor settings
    pub grid_size: f32,
    pub grid_snapping: bool,
    pub snap_distance: f32,
    pub rotation_snapping: bool,
    pub snap_angle_deg: f32,
    pub camera_speed: f32,
    pub camera_sensitivity: f32,
    pub gizmo_size: f32,
    pub undo_history_limit: u32,
    pub show_node_ids: bool,
    pub auto_select_new_nodes: bool,
    pub confirm_deletes: bool,
    // Theme settings
    pub font_size: f32,
    pub ui_scale: f32,
    pub theme_mode: u32,
    pub accent_color: [f32; 3],
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            tab: 0,
            auto_save_interval: 300.0,
            auto_save_enabled: false,
            max_recent_files: 10,
            project_path: String::new(),
            recent_files: Vec::new(),
            grid_size: 1.0,
            grid_snapping: true,
            snap_distance: 0.25,
            rotation_snapping: false,
            snap_angle_deg: 15.0,
            camera_speed: 1.0,
            camera_sensitivity: 1.0,
            gizmo_size: 60.0,
            undo_history_limit: 100,
            show_node_ids: false,
            auto_select_new_nodes: true,
            confirm_deletes: true,
            font_size: 14.0,
            ui_scale: 1.0,
            theme_mode: 0,
            accent_color: [0.3, 0.6, 1.0],
        }
    }
}

impl SettingsPanel {
    /// Add a file path to the recent files list (most recent first).
    /// Deduplicates and trims to `max_recent_files` entries.
    pub fn add_recent_file(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        // Remove existing entry if present (dedup).
        self.recent_files.retain(|p| p != path);
        // Insert at front.
        self.recent_files.insert(0, path.to_string());
        // Trim to max.
        let max = self.max_recent_files as usize;
        if self.recent_files.len() > max {
            self.recent_files.truncate(max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_recent_file_dedup_and_order() {
        let mut s = SettingsPanel::default();
        s.add_recent_file("/a/scene1.json");
        s.add_recent_file("/b/scene2.json");
        s.add_recent_file("/a/scene1.json"); // re-open, should move to front

        assert_eq!(s.recent_files.len(), 2);
        assert_eq!(s.recent_files[0], "/a/scene1.json");
        assert_eq!(s.recent_files[1], "/b/scene2.json");
    }

    #[test]
    fn test_add_recent_file_trims_to_max() {
        let mut s = SettingsPanel::default();
        s.max_recent_files = 3;
        s.add_recent_file("/1.json");
        s.add_recent_file("/2.json");
        s.add_recent_file("/3.json");
        s.add_recent_file("/4.json");
        s.add_recent_file("/5.json");

        assert_eq!(s.recent_files.len(), 3);
        assert_eq!(s.recent_files[0], "/5.json");
        assert_eq!(s.recent_files[2], "/3.json");
    }

    #[test]
    fn test_add_recent_file_ignores_empty() {
        let mut s = SettingsPanel::default();
        s.add_recent_file("");
        assert!(s.recent_files.is_empty());
    }
}

impl EditorPanel for SettingsPanel {
    fn name(&self) -> &str { "Settings" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("Settings").default_width(500.0).default_height(400.0).resizable(true).show(ctx, |ui| {
            // Tab bar.
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, 0u32, "General");
                ui.selectable_value(&mut self.tab, 1u32, "Editor");
                ui.selectable_value(&mut self.tab, 2u32, "Shortcuts");
                ui.selectable_value(&mut self.tab, 3u32, "Theme");
                ui.selectable_value(&mut self.tab, 4u32, "About");
            });
            ui.separator();

            match self.tab {
                0 => self.render_general(ui, app),
                1 => self.render_editor(ui),
                2 => self.render_shortcuts(ui),
                3 => self.render_theme(ui),
                _ => self.render_about(ui),
            }
        });
    }
}

impl SettingsPanel {
    fn render_general(&mut self, ui: &mut egui::Ui, app: &mut EditorApp) {
        ui.heading("General Settings");
        ui.separator();

        egui::Grid::new("general_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Scene Name:");
            ui.label(egui::RichText::new(&app.scene.name).color(egui::Color32::from_rgb(200, 220, 255)).strong());
            ui.end_row();

            ui.label("Scene Path:");
            let path = app.scene_path.as_ref().map(|s| s.as_str()).unwrap_or("(unsaved)");
            ui.label(egui::RichText::new(path).color(egui::Color32::from_rgb(180, 180, 180)).family(egui::FontFamily::Monospace).small());
            ui.end_row();

            ui.label("Node Count:");
            ui.label(format!("{}", app.scene.nodes.len()));
            ui.end_row();

            ui.label("Dirty:");
            let dirty_text = if app.dirty { "Yes (unsaved changes)" } else { "No" };
            let dirty_color = if app.dirty { egui::Color32::from_rgb(255, 200, 80) } else { egui::Color32::from_rgb(100, 255, 100) };
            ui.label(egui::RichText::new(dirty_text).color(dirty_color));
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.heading("Auto-Save");
        ui.separator();

        
        ui.checkbox(&mut self.auto_save_enabled, "Enable Auto-Save");
        // Note: can't mutate self in closure, so we just display.

        ui.horizontal(|ui| {
            ui.label("Interval (seconds):");
            ui.add(egui::Slider::new(&mut self.auto_save_interval, 30.0..=3600.0).suffix(" s"));
        });

        ui.add_space(8.0);
        ui.heading("Recent Files");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Max recent files:");
            ui.add(egui::DragValue::new(&mut self.max_recent_files).range(1..=50));
        });

        if self.recent_files.is_empty() {
            ui.label(egui::RichText::new("(No recent files yet)")
                .small()
                .color(egui::Color32::from_rgb(120, 120, 120)));
        } else {
            ui.add_space(4.0);
            let mut open_path: Option<String> = None;
            for (i, path) in self.recent_files.iter().enumerate() {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", i + 1));
                    if ui.button(egui::RichText::new(filename).color(egui::Color32::from_rgb(180, 220, 255))).clicked() {
                        open_path = Some(path.clone());
                    }
                    ui.label(egui::RichText::new(path)
                        .small()
                        .color(egui::Color32::from_rgb(120, 120, 120))
                        .family(egui::FontFamily::Monospace));
                });
            }
            if let Some(p) = open_path {
                app.pending_action = Some(crate::app::EditorAction::OpenSceneFromPath(p));
            }
        }

        ui.add_space(8.0);
        ui.heading("Preferences Persistence");
        ui.separator();
        if let Some(path) = crate::settings::settings_path() {
            ui.label(egui::RichText::new(format!("Settings file: {}", path.display()))
                .small()
                .color(egui::Color32::from_rgb(150, 150, 160))
                .family(egui::FontFamily::Monospace));
        }
        ui.label(egui::RichText::new("(Auto-saves every ~10s and on exit)")
            .small()
            .color(egui::Color32::from_rgb(120, 120, 120)));
        ui.horizontal(|ui| {
            if ui.button("Save Now").clicked() {
                if let Err(e) = crate::settings::save_settings(self) {
                    log::warn!("Failed to save settings: {}", e);
                }
            }
            if ui.button("Reset to Defaults").clicked() {
                let visible = self.visible;
                let tab = self.tab;
                *self = SettingsPanel::default();
                self.visible = visible;
                self.tab = tab;
            }
        });
    }

    fn render_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Editor Preferences");
        ui.separator();

        ui.label(egui::RichText::new("Grid & Snapping").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        egui::Grid::new("editor_grid1").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Grid Size:");
            ui.add(egui::Slider::new(&mut self.grid_size, 0.1..=10.0).suffix(" u"));
            ui.end_row();

            ui.label("Snap Distance:");
            ui.add(egui::Slider::new(&mut self.snap_distance, 0.01..=2.0));
            ui.end_row();
        });
        ui.checkbox(&mut self.grid_snapping, "Enable Grid Snapping");

        ui.horizontal(|ui| {
            ui.label("Rotation Snap Angle:");
            ui.add(egui::Slider::new(&mut self.snap_angle_deg, 1.0..=90.0).suffix("°"));
        });
        ui.checkbox(&mut self.rotation_snapping, "Enable Rotation Snapping");

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Camera").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        egui::Grid::new("editor_grid2").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Camera Speed:");
            ui.add(egui::Slider::new(&mut self.camera_speed, 0.1..=10.0));
            ui.end_row();

            ui.label("Camera Sensitivity:");
            ui.add(egui::Slider::new(&mut self.camera_sensitivity, 0.1..=5.0));
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Gizmo & Selection").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        egui::Grid::new("editor_grid3").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Gizmo Size:");
            ui.add(egui::Slider::new(&mut self.gizmo_size, 20.0..=150.0).suffix(" px"));
            ui.end_row();

            ui.label("Undo History Limit:");
            ui.add(egui::Slider::new(&mut self.undo_history_limit, 10..=1000));
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Behavior").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        ui.checkbox(&mut self.show_node_ids, "Show Node IDs in Hierarchy");
        ui.checkbox(&mut self.auto_select_new_nodes, "Auto-select newly created nodes");
        ui.checkbox(&mut self.confirm_deletes, "Confirm before deleting nodes");
    }

    fn render_shortcuts(&self, ui: &mut egui::Ui) {
        ui.heading("Keyboard Shortcuts");
        ui.separator();

        let sections: &[(&str, &[(&str, &str)])] = &[
            ("File", &[
                ("Ctrl+N", "New Scene"),
                ("Ctrl+O", "Open Scene"),
                ("Ctrl+S", "Save Scene"),
                ("Ctrl+Shift+S", "Save As..."),
            ]),
            ("Edit", &[
                ("Ctrl+Z", "Undo"),
                ("Ctrl+Y", "Redo"),
                ("Del", "Delete Selected"),
                ("Ctrl+D", "Duplicate Selected"),
                ("Ctrl+C", "Copy Selected"),
                ("Ctrl+V", "Paste"),
                ("F2", "Rename Selected"),
                ("Esc", "Deselect"),
            ]),
            ("View", &[
                ("W", "Gizmo: Translate"),
                ("E", "Gizmo: Rotate"),
                ("R", "Gizmo: Scale"),
                ("F", "Focus Selection"),
            ]),
            ("Camera", &[
                ("Middle Drag", "Orbit Camera"),
                ("Shift+Middle", "Pan Camera"),
                ("Scroll", "Zoom"),
                ("Right Drag", "Orbit (alt)"),
            ]),
        ];

        for (section_name, shortcuts) in sections {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(*section_name).strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();
            egui::Grid::new(format!("shortcuts_{}", section_name)).num_columns(2).spacing([16.0, 3.0]).show(ui, |ui| {
                for (key, desc) in *shortcuts {
                    ui.label(egui::RichText::new(*key).color(egui::Color32::from_rgb(255, 220, 100)).family(egui::FontFamily::Monospace).strong());
                    ui.label(*desc);
                    ui.end_row();
                }
            });
        }
    }

    fn render_theme(&mut self, ui: &mut egui::Ui) {
        ui.heading("Theme");
        ui.separator();

        ui.label(egui::RichText::new("Appearance").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);

        egui::Grid::new("theme_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Theme Mode:");
            egui::ComboBox::from_id_salt("theme_mode")
                .selected_text(match self.theme_mode { 0 => "Dark", 1 => "Light", 2 => "System", _ => "?" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.theme_mode, 0u32, "Dark");
                    ui.selectable_value(&mut self.theme_mode, 1u32, "Light");
                    ui.selectable_value(&mut self.theme_mode, 2u32, "System");
                });
            ui.end_row();

            ui.label("Font Size:");
            ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0).suffix(" px"));
            ui.end_row();

            ui.label("UI Scale:");
            ui.add(egui::Slider::new(&mut self.ui_scale, 0.5..=2.0));
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Accent Color").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.color_edit_button_rgb(&mut self.accent_color);
            let r = (self.accent_color[0] * 255.0) as u8;
            let g = (self.accent_color[1] * 255.0) as u8;
            let b = (self.accent_color[2] * 255.0) as u8;
            ui.label(egui::RichText::new(format!("#{:02X}{:02X}{:02X}", r, g, b)).family(egui::FontFamily::Monospace).color(egui::Color32::from_rgb(r, g, b)));
        });

        // Preset colors.
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Presets:").small().color(egui::Color32::from_rgb(150, 150, 160)));
        ui.horizontal_wrapped(|ui| {
            let presets: [([f32; 3], &str); 6] = [
                ([0.3, 0.6, 1.0], "Blue"),
                ([0.2, 0.8, 0.4], "Green"),
                ([1.0, 0.4, 0.3], "Red"),
                ([0.9, 0.7, 0.2], "Orange"),
                ([0.7, 0.3, 0.9], "Purple"),
                ([0.2, 0.8, 0.9], "Cyan"),
            ];
            for (color, name) in &presets {
                let r = (color[0] * 255.0) as u8;
                let g = (color[1] * 255.0) as u8;
                let b = (color[2] * 255.0) as u8;
                if ui.add(egui::Button::new(egui::RichText::new(*name).small()).fill(egui::Color32::from_rgb(r, g, b))).clicked() {
                    self.accent_color = *color;
                }
            }
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("(Theme changes apply on restart)").small().color(egui::Color32::from_rgb(120, 120, 120)));
    }

    fn render_about(&self, ui: &mut egui::Ui) {
        ui.heading("About");
        ui.separator();

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Wasteland Editor").strong().size(24.0).color(egui::Color32::from_rgb(100, 180, 255)));
            ui.label(egui::RichText::new("Rust Native 3D Editor").size(14.0).color(egui::Color32::from_rgb(180, 180, 190)));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("v0.1.0").color(egui::Color32::from_rgb(150, 150, 160)));
            ui.add_space(20.0);
        });

        ui.separator();
        egui::Grid::new("about_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Engine:");
            ui.label("wgpu 24 + egui 0.31");
            ui.end_row();

            ui.label("Language:");
            ui.label("Rust (stable)");
            ui.end_row();

            ui.label("Platform:");
            ui.label(std::env::consts::OS);
            ui.end_row();

            ui.label("Arch:");
            ui.label(std::env::consts::ARCH);
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("Technologies:").strong().color(egui::Color32::from_rgb(180, 200, 220)));
        ui.add_space(4.0);
        let techs = ["winit 0.30", "wgpu 24", "egui 0.31", "egui-wgpu 0.31", "glam", "rfd 0.15", "serde", "bytemuck"];
        ui.horizontal_wrapped(|ui| {
            for tech in &techs {
                ui.label(egui::RichText::new(*tech).small().color(egui::Color32::from_rgb(150, 180, 220)).family(egui::FontFamily::Monospace));
            }
        });
    }
}
