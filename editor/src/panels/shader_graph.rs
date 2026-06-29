//! 着色器图编辑器面板：基于节点的着色器编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ShaderGraphPanel {
    pub visible: bool,
    pub graph_name: String,
    pub selected_node: Option<usize>,
    pub nodes: Vec<ShaderNode>,
    pub category: NodeCategory,
    pub preview_enabled: bool,
    pub preview_size: f32,
    pub show_properties: bool,
    pub compile_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeCategory {
    Input,
    Math,
    Texture,
    Color,
    Output,
}

#[derive(Debug, Clone)]
pub struct ShaderNode {
    pub name: String,
    pub category: NodeCategory,
    pub pos_x: f32,
    pub pos_y: f32,
    pub enabled: bool,
}

impl Default for ShaderGraphPanel {
    fn default() -> Self {
        Self {
            visible: false,
            graph_name: "NewShader".to_string(),
            selected_node: None,
            nodes: vec![
                ShaderNode { name: "TexCoord".into(), category: NodeCategory::Input, pos_x: 50.0, pos_y: 50.0, enabled: true },
                ShaderNode { name: "Multiply".into(), category: NodeCategory::Math, pos_x: 250.0, pos_y: 100.0, enabled: true },
                ShaderNode { name: "TextureSample".into(), category: NodeCategory::Texture, pos_x: 250.0, pos_y: 200.0, enabled: true },
                ShaderNode { name: "MasterOutput".into(), category: NodeCategory::Output, pos_x: 500.0, pos_y: 150.0, enabled: true },
            ],
            category: NodeCategory::Math,
            preview_enabled: true,
            preview_size: 128.0,
            show_properties: true,
            compile_status: "Not compiled".to_string(),
        }
    }
}

impl EditorPanel for ShaderGraphPanel {
    fn name(&self) -> &str { "Shader Graph" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Graph:");
                    ui.text_edit_singleline(&mut self.graph_name);
                    ui.separator();
                    if ui.button("Compile").clicked() {
                        self.compile_status = "Compiled OK".to_string();
                    }
                    if ui.button("Save").clicked() {}
                    ui.separator();
                    ui.checkbox(&mut self.preview_enabled, "Preview");
                    ui.checkbox(&mut self.show_properties, "Properties");
                    ui.separator();
                    ui.label("Preview Size:");
                    ui.add(egui::Slider::new(&mut self.preview_size, 64.0..=512.0));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Node Library");
                        ui.separator();
                        ui.radio_value(&mut self.category, NodeCategory::Input, "Input");
                        ui.radio_value(&mut self.category, NodeCategory::Math, "Math");
                        ui.radio_value(&mut self.category, NodeCategory::Texture, "Texture");
                        ui.radio_value(&mut self.category, NodeCategory::Color, "Color");
                        ui.radio_value(&mut self.category, NodeCategory::Output, "Output");
                        ui.separator();
                        if ui.button("Add Node").clicked() {
                            self.nodes.push(ShaderNode {
                                name: format!("{:?}Node", self.category),
                                category: self.category,
                                pos_x: 300.0,
                                pos_y: 300.0,
                                enabled: true,
                            });
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label("Canvas");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(400.0, 350.0));
                            for (i, node) in self.nodes.iter().enumerate() {
                                let selected = self.selected_node == Some(i);
                                let label = format!("[{:?}] {} {}", node.category, if node.enabled { "" } else { "(off)" }, node.name);
                                if ui.selectable_label(selected, label).clicked() {
                                    self.selected_node = Some(i);
                                }
                            }
                        });
                    });
                    if self.show_properties {
                        ui.vertical(|ui| {
                            ui.label("Properties");
                            ui.separator();
                            if let Some(idx) = self.selected_node {
                                if idx < self.nodes.len() {
                                    let node = &mut self.nodes[idx];
                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut node.name);
                                    ui.checkbox(&mut node.enabled, "Enabled");
                                    ui.label("Position:");
                                    ui.horizontal(|ui| {
                                        ui.add(egui::DragValue::new(&mut node.pos_x).speed(1.0));
                                        ui.add(egui::DragValue::new(&mut node.pos_y).speed(1.0));
                                    });
                                }
                            } else {
                                ui.label("Select a node");
                            }
                        });
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Status: {}", self.compile_status));
                    ui.label(format!("Nodes: {}", self.nodes.len()));
                });
            });
    }
}
