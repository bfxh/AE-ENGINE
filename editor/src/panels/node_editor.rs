//! 通用节点编辑器面板：可复用的节点图编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct NodeEditorPanel {
    pub visible: bool,
    pub graph_name: String,
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<NodeConnection>,
    pub selected_node: Option<usize>,
    pub search_query: String,
    pub group_filter: NodeGroup,
    pub show_grid: bool,
    pub minimap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeGroup {
    All,
    Input,
    Transform,
    Output,
    Logic,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub name: String,
    pub group: NodeGroup,
    pub pos: [f32; 2],
    pub enabled: bool,
    pub inputs: u32,
    pub outputs: u32,
}

#[derive(Debug, Clone)]
pub struct NodeConnection {
    pub from_node: usize,
    pub from_port: u32,
    pub to_node: usize,
    pub to_port: u32,
}

impl Default for NodeEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            graph_name: "NodeGraph".to_string(),
            nodes: vec![
                GraphNode { name: "Input1".into(), group: NodeGroup::Input, pos: [50.0, 50.0], enabled: true, inputs: 0, outputs: 1 },
                GraphNode { name: "Transform".into(), group: NodeGroup::Transform, pos: [250.0, 100.0], enabled: true, inputs: 2, outputs: 1 },
                GraphNode { name: "Output".into(), group: NodeGroup::Output, pos: [500.0, 150.0], enabled: true, inputs: 1, outputs: 0 },
            ],
            connections: vec![
                NodeConnection { from_node: 0, from_port: 0, to_node: 1, to_port: 0 },
                NodeConnection { from_node: 1, from_port: 0, to_node: 2, to_port: 0 },
            ],
            selected_node: Some(1),
            search_query: String::new(),
            group_filter: NodeGroup::All,
            show_grid: true,
            minimap: false,
        }
    }
}

impl EditorPanel for NodeEditorPanel {
    fn name(&self) -> &str { "Node Editor" }

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
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                    ui.separator();
                    ui.radio_value(&mut self.group_filter, NodeGroup::All, "All");
                    ui.radio_value(&mut self.group_filter, NodeGroup::Input, "Input");
                    ui.radio_value(&mut self.group_filter, NodeGroup::Transform, "Transform");
                    ui.radio_value(&mut self.group_filter, NodeGroup::Output, "Output");
                    ui.radio_value(&mut self.group_filter, NodeGroup::Logic, "Logic");
                    ui.separator();
                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.minimap, "Minimap");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Add Input").clicked() {
                        self.nodes.push(GraphNode { name: format!("Input{}", self.nodes.len()), group: NodeGroup::Input, pos: [50.0, 200.0], enabled: true, inputs: 0, outputs: 1 });
                    }
                    if ui.button("Add Transform").clicked() {
                        self.nodes.push(GraphNode { name: format!("Transform{}", self.nodes.len()), group: NodeGroup::Transform, pos: [250.0, 200.0], enabled: true, inputs: 2, outputs: 1 });
                    }
                    if ui.button("Add Output").clicked() {
                        self.nodes.push(GraphNode { name: format!("Output{}", self.nodes.len()), group: NodeGroup::Output, pos: [500.0, 200.0], enabled: true, inputs: 1, outputs: 0 });
                    }
                    if ui.button("Add Logic").clicked() {
                        self.nodes.push(GraphNode { name: format!("Logic{}", self.nodes.len()), group: NodeGroup::Logic, pos: [350.0, 300.0], enabled: true, inputs: 2, outputs: 1 });
                    }
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
                                for x in (0..500).step_by(20) {
                                    painter.line_segment([egui::pos2(rect.left() + x as f32, rect.top()), egui::pos2(rect.left() + x as f32, rect.bottom())], egui::Stroke::new(0.3, egui::Color32::DARK_GRAY));
                                }
                                for y in (0..350).step_by(20) {
                                    painter.line_segment([egui::pos2(rect.left(), rect.top() + y as f32), egui::pos2(rect.right(), rect.top() + y as f32)], egui::Stroke::new(0.3, egui::Color32::DARK_GRAY));
                                }
                            }
                            for (i, node) in self.nodes.iter().enumerate() {
                                if self.group_filter != NodeGroup::All && node.group != self.group_filter { continue; }
                                if !self.search_query.is_empty() && !node.name.contains(&self.search_query) { continue; }
                                let selected = self.selected_node == Some(i);
                                let color = if selected { egui::Color32::YELLOW } else { egui::Color32::LIGHT_BLUE };
                                let p = egui::pos2(rect.left() + node.pos[0], rect.top() + node.pos[1]);
                                painter.rect_filled(egui::Rect::from_min_size(p, egui::vec2(120.0, 40.0)), 3.0, egui::Color32::from_rgb(40, 40, 50));
                                painter.rect_stroke(egui::Rect::from_min_size(p, egui::vec2(120.0, 40.0)), 3.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Middle);
                                painter.text(egui::pos2(p.x + 60.0, p.y + 20.0), egui::Align2::CENTER_CENTER, &node.name, egui::FontId::default(), egui::Color32::WHITE);
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
                                ui.label(format!("Group: {:?}", node.group));
                                ui.label("Position:");
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut node.pos[0]).speed(1.0));
                                    ui.add(egui::DragValue::new(&mut node.pos[1]).speed(1.0));
                                });
                                ui.label(format!("Inputs: {} / Outputs: {}", node.inputs, node.outputs));
                            }
                        } else {
                            ui.label("Select a node");
                        }
                    });
                });
                ui.separator();
                ui.label(format!("Nodes: {} | Connections: {}", self.nodes.len(), self.connections.len()));
            });
    }
}
