//! 编辑器偏好面板：外观、行为、快捷键、主题。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct EditorPreferencesPanel {
    pub visible: bool,
    pub active_tab: PrefTab,
    pub theme: Theme,
    pub font_size: f32,
    pub ui_scale: f32,
    pub show_grid: bool,
    pub show_gizmo_labels: bool,
    pub auto_save_interval: f32,
    pub auto_save_enabled: bool,
    pub undo_history_size: i32,
    pub snap_to_grid: bool,
    pub grid_size: f32,
    pub rotation_snap: f32,
    pub scale_snap: f32,
    pub shortcuts: Vec<(String, String)>,
    pub shortcut_filter: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefTab {
    Appearance,
    Behavior,
    Shortcuts,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Dark,
    Light,
    Midnight,
    Custom,
}

impl Default for EditorPreferencesPanel {
    fn default() -> Self {
        Self {
            visible: false,
            active_tab: PrefTab::Appearance,
            theme: Theme::Dark,
            font_size: 14.0,
            ui_scale: 1.0,
            show_grid: true,
            show_gizmo_labels: true,
            auto_save_interval: 300.0,
            auto_save_enabled: true,
            undo_history_size: 100,
            snap_to_grid: true,
            grid_size: 0.5,
            rotation_snap: 15.0,
            scale_snap: 0.25,
            shortcuts: vec![
                ("Save".into(), "Ctrl+S".into()),
                ("Open".into(), "Ctrl+O".into()),
                ("Undo".into(), "Ctrl+Z".into()),
                ("Redo".into(), "Ctrl+Y".into()),
                ("Delete".into(), "Del".into()),
                ("Duplicate".into(), "Ctrl+D".into()),
                ("Focus".into(), "F".into()),
                ("Search".into(), "Ctrl+F".into()),
            ],
            shortcut_filter: String::new(),
        }
    }
}

impl EditorPanel for EditorPreferencesPanel {
    fn name(&self) -> &str { "Editor Preferences" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, PrefTab::Appearance, "Appearance");
                    ui.selectable_value(&mut self.active_tab, PrefTab::Behavior, "Behavior");
                    ui.selectable_value(&mut self.active_tab, PrefTab::Shortcuts, "Shortcuts");
                    ui.selectable_value(&mut self.active_tab, PrefTab::Theme, "Theme");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        PrefTab::Appearance => {
                            ui.heading("Appearance");
                            ui.horizontal(|ui| { ui.label("Font Size:"); ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0)); });
                            ui.horizontal(|ui| { ui.label("UI Scale:"); ui.add(egui::Slider::new(&mut self.ui_scale, 0.5..=2.0)); });
                            ui.checkbox(&mut self.show_grid, "Show Grid");
                            ui.checkbox(&mut self.show_gizmo_labels, "Show Gizmo Labels");
                        },
                        PrefTab::Behavior => {
                            ui.heading("Behavior");
                            ui.checkbox(&mut self.auto_save_enabled, "Auto Save");
                            ui.add_enabled(self.auto_save_enabled, egui::Slider::new(&mut self.auto_save_interval, 30.0..=1800.0).text("sec"));
                            ui.horizontal(|ui| { ui.label("Undo History:"); ui.add(egui::DragValue::new(&mut self.undo_history_size).range(10..=1000)); });
                            ui.separator();
                            ui.checkbox(&mut self.snap_to_grid, "Snap to Grid");
                            ui.horizontal(|ui| { ui.label("Grid Size:"); ui.add(egui::DragValue::new(&mut self.grid_size).speed(0.1).range(0.01..=10.0)); });
                            ui.horizontal(|ui| { ui.label("Rotation Snap:"); ui.add(egui::DragValue::new(&mut self.rotation_snap).speed(1.0).range(1.0..=90.0)); });
                            ui.horizontal(|ui| { ui.label("Scale Snap:"); ui.add(egui::DragValue::new(&mut self.scale_snap).speed(0.05).range(0.01..=5.0)); });
                        },
                        PrefTab::Shortcuts => {
                            ui.heading("Shortcuts");
                            ui.horizontal(|ui| {
                                ui.label("Filter:");
                                ui.text_edit_singleline(&mut self.shortcut_filter);
                            });
                            ui.separator();
                            let filter = self.shortcut_filter.to_lowercase();
                            for i in 0..self.shortcuts.len() {
                                let matches = filter.is_empty()
                                    || self.shortcuts[i].0.to_lowercase().contains(&filter)
                                    || self.shortcuts[i].1.to_lowercase().contains(&filter);
                                if !matches { continue; }
                                ui.horizontal(|ui| {
                                    ui.label(&self.shortcuts[i].0.as_str());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(&self.shortcuts[i].1.as_str());
                                    });
                                });
                            }
                        },
                        PrefTab::Theme => {
                            ui.heading("Theme");
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.theme, Theme::Dark, "Dark");
                                ui.selectable_value(&mut self.theme, Theme::Light, "Light");
                                ui.selectable_value(&mut self.theme, Theme::Midnight, "Midnight");
                                ui.selectable_value(&mut self.theme, Theme::Custom, "Custom");
                            });
                            ui.separator();
                            ui.label("Preview");
                            let preview_color = match self.theme {
                                Theme::Dark => egui::Color32::from_rgb(40, 40, 45),
                                Theme::Light => egui::Color32::from_rgb(230, 230, 235),
                                Theme::Midnight => egui::Color32::from_rgb(15, 15, 30),
                                Theme::Custom => egui::Color32::from_rgb(60, 30, 60),
                            };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 80.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 4.0, preview_color);
                        },
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        // Settings are live in the struct; Apply is a visual confirmation.
                    }
                    if ui.button("Reset").clicked() {
                        let visible = self.visible;
                        let tab = self.active_tab;
                        *self = Self::default();
                        self.visible = visible;
                        self.active_tab = tab;
                    }
                    if ui.button("Close").clicked() { self.visible = false; }
                });
            });
    }
}
