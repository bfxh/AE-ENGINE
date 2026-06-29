//! 几何节点面板：程序化几何体节点编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct GeometryNodesPanel {
    pub visible: bool,
    pub graph_name: String,
    pub nodes: Vec<GeoNode>,
    pub selected_node: Option<usize>,
    pub show_grid: bool,
    pub preview_geometry: bool,
    pub input_mesh: String,
    pub output_name: String,
}

#[derive(Debug, Clone)]
pub struct GeoNode {
    pub name: String,
    pub node_type: GeoNodeType,
    pub pos: [f32; 2],
    pub enabled: bool,
    pub params: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeoNodeType {
    Input,
    Transform,
    Instance,
    Attribute,
    Math,
    Output,
}

impl Default for GeometryNodesPanel {
    fn default() -> Self {
        Self {
            visible: false,
            graph_name: "GeometryNodes".to_string(),
            nodes: vec![
                GeoNode { name: "Input".into(), node_type: GeoNodeType::Input, pos: [50.0, 100.0], enabled: true, params: vec![] },
                GeoNode { name: "Transform".into(), node_type: GeoNodeType::Transform, pos: [250.0, 100.0], enabled: true, params: vec![0.0, 0.0, 0.0, 1.0] },
                GeoNode { name: "Instance".into(), node_type: GeoNodeType::Instance, pos: [450.0, 100.0], enabled: true, params: vec![10.0] },
                GeoNode { name: "Output".into(), node_type: GeoNodeType::Output, pos: [650.0, 100.0], enabled: true, params: vec![] },
            ],
            selected_node: Some(1),
            show_grid: true,
            preview_geometry: true,
            input_mesh: "Cube".to_string(),
            output_name: "Result".to_string(),
        }
    }
}

impl EditorPanel for GeometryNodesPanel {
    fn name(&self) -> &str { "Geometry Nodes" }

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
                    if ui.button("Add Input").clicked() {
                        self.nodes.push(GeoNode { name: format!("Input {}", self.nodes.len()), node_type: GeoNodeType::Input, pos: [50.0, 200.0], enabled: true, params: vec![] });
                    }
                    if ui.button("Add Transform").clicked() {
                        self.nodes.push(GeoNode { name: format!("Transform {}", self.nodes.len()), node_type: GeoNodeType::Transform, pos: [250.0, 200.0], enabled: true, params: vec![0.0, 0.0, 0.0, 1.0] });
                    }
                    if ui.button("Add Instance").clicked() {
                        self.nodes.push(GeoNode { name: format!("Instance {}", self.nodes.len()), node_type: GeoNodeType::Instance, pos: [450.0, 200.0], enabled: true, params: vec![10.0] });
                    }
                    if ui.button("Add Attribute").clicked() {
                        self.nodes.push(GeoNode { name: format!("Attribute {}", self.nodes.len()), node_type: GeoNodeType::Attribute, pos: [350.0, 300.0], enabled: true, params: vec![] });
                    }
                    if ui.button("Add Math").clicked() {
                        self.nodes.push(GeoNode { name: format!("Math {}", self.nodes.len()), node_type: GeoNodeType::Math, pos: [350.0, 400.0], enabled: true, params: vec![0.0] });
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.preview_geometry, "Preview");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Input Mesh:");
                    ui.text_edit_singleline(&mut self.input_mesh);
                    ui.separator();
                    ui.label("Output:");
                    ui.text_edit_singleline(&mut self.output_name);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Canvas");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(500.0, 350.0));
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(500.0, 350.0), egui::Sense::hover());
                            let painter = ui.painter();
                            if self.show_grid {
                                for x in (0..500).step_by(25) {
                                    painter.line_segment([egui::pos2(rect.left() + x as f32, rect.top()), egui::pos2(rect.left() + x as f32, rect.bottom())], egui::Stroke::new(0.3, egui::Color32::DARK_GRAY));
                                }
                                for y in (0..350).step_by(25) {
                                    painter.line_segment([egui::pos2(rect.left(), rect.top() + y as f32), egui::pos2(rect.right(), rect.top() + y as f32)], egui::Stroke::new(0.3, egui::Color32::DARK_GRAY));
                                }
                            }
                            for (i, node) in self.nodes.iter().enumerate() {
                                let selected = self.selected_node == Some(i);
                                let color = match node.node_type {
                                    GeoNodeType::Input => egui::Color32::LIGHT_BLUE,
                                    GeoNodeType::Transform => egui::Color32::LIGHT_GREEN,
                                    GeoNodeType::Instance => egui::Color32::LIGHT_YELLOW,
                                    GeoNodeType::Attribute => egui::Color32::from_rgb(255, 150, 200),
                                    GeoNodeType::Math => egui::Color32::from_rgb(200, 150, 255),
                                    GeoNodeType::Output => egui::Color32::LIGHT_RED,
                                };
                                let border = if selected { egui::Color32::WHITE } else { color };
                                let p = egui::pos2(rect.left() + node.pos[0], rect.top() + node.pos[1]);
                                painter.rect_filled(egui::Rect::from_min_size(p, egui::vec2(130.0, 50.0)), 3.0, egui::Color32::from_rgb(40, 40, 50));
                                painter.rect_stroke(egui::Rect::from_min_size(p, egui::vec2(130.0, 50.0)), 3.0, egui::Stroke::new(2.0, border), egui::StrokeKind::Middle);
                                painter.circle_filled(egui::pos2(p.x + 10.0, p.y + 25.0), 4.0, color);
                                painter.text(egui::pos2(p.x + 65.0, p.y + 25.0), egui::Align2::CENTER_CENTER, &node.name, egui::FontId::default(), egui::Color32::WHITE);
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Properties");
                        ui.separator();
                        if let Some(idx) = self.selected_node {
                            if idx < self.nodes.len() {
                                let node = &mut self.nodes[idx];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut node.name);
                                ui.checkbox(&mut node.enabled, "Enabled");
                                ui.label(format!("Type: {:?}", node.node_type));
                                ui.label("Position:");
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut node.pos[0]).speed(1.0));
                                    ui.add(egui::DragValue::new(&mut node.pos[1]).speed(1.0));
                                });
                                match node.node_type {
                                    GeoNodeType::Transform => {
                                        ui.label("Translation:");
                                        if node.params.len() >= 3 {
                                            ui.horizontal(|ui| {
                                                ui.add(egui::DragValue::new(&mut node.params[0]).speed(0.1));
                                                ui.add(egui::DragValue::new(&mut node.params[1]).speed(0.1));
                                                ui.add(egui::DragValue::new(&mut node.params[2]).speed(0.1));
                                            });
                                        }
                                        if node.params.len() >= 4 {
                                            ui.label("Scale:");
                                            ui.add(egui::Slider::new(&mut node.params[3], 0.01..=10.0));
                                        }
                                    }
                                    GeoNodeType::Instance => {
                                        if node.params.len() >= 1 {
                                            ui.label("Count:");
                                            ui.add(egui::DragValue::new(&mut node.params[0]).range(1.0..=10000.0));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            ui.label("Select a node");
                        }
                    });
                });
            });
    }
}
