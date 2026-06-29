//! 合成器面板：渲染层合成和后期节点。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct CompositorPanel {
    pub visible: bool,
    pub render_layers: Vec<RenderLayer>,
    pub selected_layer: Option<usize>,
    pub nodes: Vec<CompNode>,
    pub selected_node: Option<usize>,
    pub output_format: OutputFormat,
    pub resolution_scale: f32,
    pub auto_render: bool,
}

#[derive(Debug, Clone)]
pub struct RenderLayer {
    pub name: String,
    pub enabled: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
    Overlay,
}

#[derive(Debug, Clone)]
pub struct CompNode {
    pub name: String,
    pub node_type: CompNodeType,
    pub enabled: bool,
    pub params: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompNodeType {
    Filter,
    ColorCorrect,
    Mask,
    Blur,
    Output,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Png,
    Exr,
    Tiff,
    Jpg,
}

impl Default for CompositorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            render_layers: vec![
                RenderLayer { name: "Beauty".into(), enabled: true, opacity: 1.0, blend_mode: BlendMode::Normal },
                RenderLayer { name: "Depth".into(), enabled: false, opacity: 0.5, blend_mode: BlendMode::Multiply },
                RenderLayer { name: "Normal".into(), enabled: false, opacity: 1.0, blend_mode: BlendMode::Overlay },
            ],
            selected_layer: Some(0),
            nodes: vec![
                CompNode { name: "Input".into(), node_type: CompNodeType::Input, enabled: true, params: vec![] },
                CompNode { name: "ColorCorrect".into(), node_type: CompNodeType::ColorCorrect, enabled: true, params: vec![1.0, 1.0, 1.0, 0.0] },
                CompNode { name: "Blur".into(), node_type: CompNodeType::Blur, enabled: false, params: vec![2.0] },
                CompNode { name: "Output".into(), node_type: CompNodeType::Output, enabled: true, params: vec![] },
            ],
            selected_node: Some(1),
            output_format: OutputFormat::Png,
            resolution_scale: 1.0,
            auto_render: false,
        }
    }
}

impl EditorPanel for CompositorPanel {
    fn name(&self) -> &str { "Compositor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Render").clicked() {}
                    if ui.button("Clear").clicked() {}
                    ui.separator();
                    ui.checkbox(&mut self.auto_render, "Auto Render");
                    ui.separator();
                    ui.label("Format:");
                    ui.radio_value(&mut self.output_format, OutputFormat::Png, "PNG");
                    ui.radio_value(&mut self.output_format, OutputFormat::Exr, "EXR");
                    ui.radio_value(&mut self.output_format, OutputFormat::Tiff, "TIFF");
                    ui.radio_value(&mut self.output_format, OutputFormat::Jpg, "JPG");
                    ui.separator();
                    ui.label("Resolution:");
                    ui.add(egui::Slider::new(&mut self.resolution_scale, 0.1..=2.0));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Render Layers");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut remove_idx: Option<usize> = None;
                            for (i, l) in self.render_layers.iter_mut().enumerate() {
                                let selected = self.selected_layer == Some(i);
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut l.enabled, "");
                                    if ui.selectable_label(selected, &l.name).clicked() {
                                        self.selected_layer = Some(i);
                                    }
                                    ui.add(egui::Slider::new(&mut l.opacity, 0.0..=1.0));
                                    if ui.button("X").clicked() { remove_idx = Some(i); }
                                });
                            }
                            if let Some(i) = remove_idx { self.render_layers.remove(i); }
                        });
                        if ui.button("Add Layer").clicked() {
                            self.render_layers.push(RenderLayer { name: format!("Layer {}", self.render_layers.len()), enabled: true, opacity: 1.0, blend_mode: BlendMode::Normal });
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label("Node Graph");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(350.0, 200.0));
                            for (i, n) in self.nodes.iter().enumerate() {
                                let selected = self.selected_node == Some(i);
                                if ui.selectable_label(selected, format!("[{:?}] {} {}", n.node_type, if n.enabled { "" } else { "(off)" }, n.name)).clicked() {
                                    self.selected_node = Some(i);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Add Filter").clicked() {
                                self.nodes.push(CompNode { name: format!("Filter {}", self.nodes.len()), node_type: CompNodeType::Filter, enabled: true, params: vec![0.5] });
                            }
                            if ui.button("Add CC").clicked() {
                                self.nodes.push(CompNode { name: format!("CC {}", self.nodes.len()), node_type: CompNodeType::ColorCorrect, enabled: true, params: vec![1.0, 1.0, 1.0, 0.0] });
                            }
                            if ui.button("Add Mask").clicked() {
                                self.nodes.push(CompNode { name: format!("Mask {}", self.nodes.len()), node_type: CompNodeType::Mask, enabled: true, params: vec![] });
                            }
                            if ui.button("Add Blur").clicked() {
                                self.nodes.push(CompNode { name: format!("Blur {}", self.nodes.len()), node_type: CompNodeType::Blur, enabled: true, params: vec![2.0] });
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Properties");
                        ui.separator();
                        if let Some(li) = self.selected_layer {
                            if li < self.render_layers.len() {
                                let layer = &mut self.render_layers[li];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut layer.name);
                                ui.checkbox(&mut layer.enabled, "Enabled");
                                ui.horizontal(|ui| {
                                    ui.label("Opacity:");
                                    ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0));
                                });
                                ui.label("Blend Mode:");
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut layer.blend_mode, BlendMode::Normal, "Normal");
                                    ui.radio_value(&mut layer.blend_mode, BlendMode::Add, "Add");
                                    ui.radio_value(&mut layer.blend_mode, BlendMode::Multiply, "Mult");
                                    ui.radio_value(&mut layer.blend_mode, BlendMode::Screen, "Screen");
                                    ui.radio_value(&mut layer.blend_mode, BlendMode::Overlay, "Overlay");
                                });
                            }
                        }
                        ui.separator();
                        if let Some(ni) = self.selected_node {
                            if ni < self.nodes.len() {
                                let node = &mut self.nodes[ni];
                                ui.label("Node:");
                                ui.text_edit_singleline(&mut node.name);
                                ui.checkbox(&mut node.enabled, "Enabled");
                                match node.node_type {
                                    CompNodeType::ColorCorrect => {
                                        if node.params.len() >= 4 {
                                            ui.label("Brightness:");
                                            ui.add(egui::Slider::new(&mut node.params[0], 0.0..=2.0));
                                            ui.label("Contrast:");
                                            ui.add(egui::Slider::new(&mut node.params[1], 0.0..=2.0));
                                            ui.label("Saturation:");
                                            ui.add(egui::Slider::new(&mut node.params[2], 0.0..=2.0));
                                            ui.label("Hue:");
                                            ui.add(egui::Slider::new(&mut node.params[3], -180.0..=180.0));
                                        }
                                    }
                                    CompNodeType::Blur => {
                                        if node.params.len() >= 1 {
                                            ui.label("Radius:");
                                            ui.add(egui::Slider::new(&mut node.params[0], 0.0..=50.0));
                                        }
                                    }
                                    CompNodeType::Filter => {
                                        if node.params.len() >= 1 {
                                            ui.label("Strength:");
                                            ui.add(egui::Slider::new(&mut node.params[0], 0.0..=1.0));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });
                });
            });
    }
}
