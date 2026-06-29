//! Layers panel: manages scene layers for organizing and toggling node visibility.
//!
//! Each node can belong to a layer, and layers can be toggled visible/hidden
//! or locked/unlocked for editing.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// A scene layer.
#[derive(Clone, Debug)]
pub struct SceneLayer {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub color: [f32; 3],
    pub node_count: u32,
}

/// Layers panel state.
pub struct LayersPanel {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Scene layers.
    pub layers: Vec<SceneLayer>,
    /// Selected layer.
    pub selected: Option<u32>,
    /// New layer name input.
    pub new_layer_name: String,
}

impl Default for LayersPanel {
    fn default() -> Self {
        Self {
            visible: false,
            layers: vec![
                SceneLayer {
                    id: 0,
                    name: "Default".to_string(),
                    visible: true,
                    locked: false,
                    color: [0.8, 0.8, 0.8],
                    node_count: 0,
                },
                SceneLayer {
                    id: 1,
                    name: "Environment".to_string(),
                    visible: true,
                    locked: false,
                    color: [0.2, 0.8, 0.2],
                    node_count: 0,
                },
                SceneLayer {
                    id: 2,
                    name: "Entities".to_string(),
                    visible: true,
                    locked: false,
                    color: [0.8, 0.4, 0.2],
                    node_count: 0,
                },
                SceneLayer {
                    id: 3,
                    name: "UI".to_string(),
                    visible: true,
                    locked: true,
                    color: [0.2, 0.4, 0.8],
                    node_count: 0,
                },
            ],
            selected: Some(0),
            new_layer_name: String::new(),
        }
    }
}

impl LayersPanel {
    /// Add a new layer.
    fn add_layer(&mut self) {
        let id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let name = if self.new_layer_name.is_empty() {
            format!("Layer {}", id)
        } else {
            self.new_layer_name.clone()
        };
        self.layers.push(SceneLayer {
            id,
            name,
            visible: true,
            locked: false,
            color: [0.5, 0.5, 0.5],
            node_count: 0,
        });
        self.new_layer_name.clear();
        self.selected = Some(id);
    }

    /// Remove a layer by ID.
    fn remove_layer(&mut self, id: u32) {
        if id == 0 {
            return; // Can't remove default layer.
        }
        self.layers.retain(|l| l.id != id);
        if self.selected == Some(id) {
            self.selected = Some(0);
        }
    }
}

impl EditorPanel for LayersPanel {
    fn name(&self) -> &str {
        "Layers"
    }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible {
            return;
        }

        egui::Window::new("Layers")
            .default_width(280.0)
            .default_height(350.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar.
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_layer_name);
                    if ui.button("+ Add").clicked() {
                        self.add_layer();
                    }
                    if ui.button("- Remove").clicked() {
                        if let Some(id) = self.selected {
                            self.remove_layer(id);
                        }
                    }
                });

                ui.separator();

                // Column headers.
                ui.horizontal(|ui| {
                    ui.label("V");
                    ui.label("L");
                    ui.label("Color");
                    ui.label("Name");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Nodes");
                    });
                });

                ui.separator();

                // Layer list.
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut to_remove = None;
                    for layer in &mut self.layers {
                        let is_selected = self.selected == Some(layer.id);
                        let bg = if is_selected {
                            egui::Color32::from_rgb(60, 90, 130)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let frame = egui::Frame::group(ui.style())
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(4, 2));
                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Visibility toggle.
                                let vis_icon = if layer.visible { "👁" } else { "—" };
                                if ui.selectable_label(false, vis_icon).clicked() {
                                    layer.visible = !layer.visible;
                                }

                                // Lock toggle.
                                let lock_icon = if layer.locked { "🔒" } else { "🔓" };
                                if ui.selectable_label(false, lock_icon).clicked() {
                                    layer.locked = !layer.locked;
                                }

                                // Color.
                                ui.color_edit_button_rgb(&mut layer.color);

                                // Name (selectable).
                                let name_label = format!("{}", layer.name);
                                if ui.selectable_label(is_selected, &name_label).clicked() {
                                    self.selected = Some(layer.id);
                                }

                                // Node count.
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(format!("{}", layer.node_count));
                                });
                            });
                        });

                        // Context menu.
                        if ui.input(|i| i.pointer.secondary_clicked()) {
                            if let Some(id) = self.selected {
                                if id != 0 {
                                    to_remove = Some(id);
                                }
                            }
                        }
                    }

                    if let Some(id) = to_remove {
                        self.remove_layer(id);
                    }
                });

                ui.separator();

                // Selected layer details.
                if let Some(id) = self.selected {
                    if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
                        ui.heading("Layer Properties");
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut layer.name);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            ui.color_edit_button_rgb(&mut layer.color);
                        });
                        ui.checkbox(&mut layer.visible, "Visible");
                        ui.checkbox(&mut layer.locked, "Locked");
                    }
                }

                let _ = app;
            });
    }
}
