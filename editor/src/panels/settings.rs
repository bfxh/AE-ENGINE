//! Settings panel: editor preferences and configuration.
//!
//! Allows the user to configure editor behavior, appearance, and shortcuts.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// Settings panel state.
pub struct SettingsPanel {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Current settings tab.
    pub tab: SettingsTab,
    // General
    pub auto_save_interval: u32,
    pub max_recent_files: u32,
    pub show_splash: bool,
    pub check_updates: bool,
    pub send_usage_data: bool,
    // Editor
    pub grid_size: f32,
    pub grid_divisions: u32,
    pub enable_grid_snapping: bool,
    pub enable_vertex_snapping: bool,
    pub enable_rotation_snapping: bool,
    pub rotation_snap_angle: f32,
    pub camera_move_speed: f32,
    pub invert_y: bool,
    pub smooth_camera: bool,
    pub gizmo_size: f32,
    pub show_gizmo_labels: bool,
    // Theme
    pub theme_dark: bool,
    pub accent_color: [f32; 3],
    pub font_size: f32,
    pub use_system_accent: bool,
    pub high_contrast: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Editor,
    Shortcuts,
    Theme,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            tab: SettingsTab::General,
            auto_save_interval: 300,
            max_recent_files: 10,
            show_splash: true,
            check_updates: true,
            send_usage_data: false,
            grid_size: 1.0,
            grid_divisions: 10,
            enable_grid_snapping: false,
            enable_vertex_snapping: false,
            enable_rotation_snapping: false,
            rotation_snap_angle: 15.0,
            camera_move_speed: 1.0,
            invert_y: true,
            smooth_camera: true,
            gizmo_size: 60.0,
            show_gizmo_labels: true,
            theme_dark: true,
            accent_color: [0.2, 0.5, 0.8],
            font_size: 14.0,
            use_system_accent: true,
            high_contrast: false,
        }
    }
}

impl EditorPanel for SettingsPanel {
    fn name(&self) -> &str {
        "Settings"
    }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible {
            return;
        }

        egui::Window::new("Settings")
            .default_width(450.0)
            .default_height(400.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, SettingsTab::General, "General");
                    ui.selectable_value(&mut self.tab, SettingsTab::Editor, "Editor");
                    ui.selectable_value(&mut self.tab, SettingsTab::Shortcuts, "Shortcuts");
                    ui.selectable_value(&mut self.tab, SettingsTab::Theme, "Theme");
                });

                ui.separator();

                match self.tab {
                    SettingsTab::General => self.render_general(ui, app),
                    SettingsTab::Editor => self.render_editor(ui, app),
                    SettingsTab::Shortcuts => self.render_shortcuts(ui, app),
                    SettingsTab::Theme => self.render_theme(ui, app),
                }
            });
    }
}

impl SettingsPanel {
    fn render_general(&mut self, ui: &mut egui::Ui, app: &mut EditorApp) {
        ui.heading("General Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Scene Name:");
            let mut name = app.scene.name.clone();
            if ui.text_edit_singleline(&mut name).changed() {
                app.scene.name = name;
                app.dirty = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Auto-save Interval (s):");
            ui.add(egui::DragValue::new(&mut self.auto_save_interval).range(30..=3600));
        });

        ui.horizontal(|ui| {
            ui.label("Max Recent Files:");
            ui.add(egui::DragValue::new(&mut self.max_recent_files).range(1..=50));
        });

        ui.separator();

        ui.checkbox(&mut self.show_splash, "Show splash screen on startup");
        ui.checkbox(&mut self.check_updates, "Check for updates");
        ui.checkbox(&mut self.send_usage_data, "Send anonymous usage data");
    }

    fn render_editor(&mut self, ui: &mut egui::Ui, app: &mut EditorApp) {
        ui.heading("Editor Settings");
        ui.separator();

        ui.label("Grid:");
        ui.horizontal(|ui| {
            ui.label("  Grid Size:");
            ui.add(egui::DragValue::new(&mut self.grid_size).range(0.1..=10.0).speed(0.1));
        });
        ui.horizontal(|ui| {
            ui.label("  Grid Divisions:");
            ui.add(egui::DragValue::new(&mut self.grid_divisions).range(1..=100));
        });

        ui.separator();
        ui.label("Snapping:");
        ui.checkbox(&mut self.enable_grid_snapping, "Enable grid snapping");
        ui.checkbox(&mut self.enable_vertex_snapping, "Enable vertex snapping");
        ui.checkbox(&mut self.enable_rotation_snapping, "Enable rotation snapping");
        ui.horizontal(|ui| {
            ui.label("  Rotation Snap Angle:");
            ui.add(egui::DragValue::new(&mut self.rotation_snap_angle).range(1.0..=90.0).suffix("°"));
        });

        ui.separator();
        ui.label("Camera:");
        ui.horizontal(|ui| {
            ui.label("  Move Speed:");
            ui.add(egui::DragValue::new(&mut self.camera_move_speed).range(0.1..=10.0).speed(0.1));
        });
        ui.checkbox(&mut self.invert_y, "Invert Y axis");
        ui.checkbox(&mut self.smooth_camera, "Smooth camera movement");

        ui.separator();
        ui.label("Gizmo:");
        ui.horizontal(|ui| {
            ui.label("  Size:");
            ui.add(egui::DragValue::new(&mut self.gizmo_size).range(20.0..=200.0));
        });
        ui.checkbox(&mut self.show_gizmo_labels, "Show gizmo labels");

        let _ = app;
    }

    fn render_shortcuts(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        ui.heading("Keyboard Shortcuts");
        ui.separator();

        let shortcuts = [
            ("File", vec![
                ("Ctrl+N", "New Scene"),
                ("Ctrl+O", "Open Scene"),
                ("Ctrl+S", "Save Scene"),
                ("Ctrl+Shift+S", "Save As..."),
                ("Alt+F4", "Exit"),
            ]),
            ("Edit", vec![
                ("Ctrl+Z", "Undo"),
                ("Ctrl+Y", "Redo"),
                ("Delete", "Delete Selected"),
                ("Esc", "Deselect"),
                ("F", "Focus Selection"),
            ]),
            ("View", vec![
                ("W", "Translate Gizmo"),
                ("E", "Rotate Gizmo"),
                ("R", "Scale Gizmo"),
            ]),
            ("Console", vec![
                ("`", "Toggle Console"),
                ("Enter", "Execute Command"),
            ]),
        ];

        for (category, items) in &shortcuts {
            ui.collapsing(*category, |ui| {
                egui::Grid::new(format!("shortcuts_{}", category)).striped(true).show(ui, |ui| {
                    for (key, action) in items {
                        ui.label(*key);
                        ui.label(*action);
                        ui.end_row();
                    }
                });
            });
        }
    }

    fn render_theme(&mut self, ui: &mut egui::Ui, app: &mut EditorApp) {
        ui.heading("Theme");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Theme:");
            if ui.radio_value(&mut self.theme_dark, true, "Dark").changed() {
                app.frame_counter = app.frame_counter.wrapping_add(1);
            }
            ui.radio_value(&mut self.theme_dark, false, "Light");
        });

        ui.separator();
        ui.label("Accent Color:");
        ui.horizontal(|ui| {
            ui.color_edit_button_rgb(&mut self.accent_color);
        });

        ui.separator();
        ui.label("Font Size:");
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0));
        });

        ui.separator();
        ui.checkbox(&mut self.use_system_accent, "Use system accent color");
        ui.checkbox(&mut self.high_contrast, "High contrast mode");
    }
}
