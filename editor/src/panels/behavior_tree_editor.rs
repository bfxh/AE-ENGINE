//! 行为树编辑器面板：可视化编辑AI行为树。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct BehaviorTreeEditorPanel {
    pub visible: bool,
    pub tree_name: String,
    pub selected_node: Option<usize>,
    pub nodes: Vec<BtNode>,
    pub blackboard: Vec<BlackboardVar>,
    pub node_type_filter: NodeType,
    pub auto_layout: bool,
    pub show_decorators: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Task,
    Decorator,
    Service,
    Root,
}

#[derive(Debug, Clone)]
pub struct BtNode {
    pub name: String,
    pub node_type: NodeType,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct BlackboardVar {
    pub key: String,
    pub var_type: String,
    pub value: String,
}

impl Default for BehaviorTreeEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            tree_name: "NewBehaviorTree".to_string(),
            selected_node: None,
            nodes: vec![
                BtNode { name: "Root".into(), node_type: NodeType::Root, enabled: true, description: "Tree root".into() },
                BtNode { name: "Patrol".into(), node_type: NodeType::Task, enabled: true, description: "Patrol behavior".into() },
                BtNode { name: "Repeat".into(), node_type: NodeType::Decorator, enabled: true, description: "Repeat child".into() },
            ],
            blackboard: vec![
                BlackboardVar { key: "Target".into(), var_type: "Object".into(), value: "None".into() },
                BlackboardVar { key: "Health".into(), var_type: "Float".into(), value: "100.0".into() },
            ],
            node_type_filter: NodeType::Task,
            auto_layout: true,
            show_decorators: true,
        }
    }
}

impl EditorPanel for BehaviorTreeEditorPanel {
    fn name(&self) -> &str { "Behavior Tree Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Tree:");
                    ui.text_edit_singleline(&mut self.tree_name);
                    ui.separator();
                    if ui.button("Add Task").clicked() {
                        self.nodes.push(BtNode { name: format!("Task {}", self.nodes.len()), node_type: NodeType::Task, enabled: true, description: String::new() });
                    }
                    if ui.button("Add Decorator").clicked() {
                        self.nodes.push(BtNode { name: format!("Decorator {}", self.nodes.len()), node_type: NodeType::Decorator, enabled: true, description: String::new() });
                    }
                    if ui.button("Add Service").clicked() {
                        self.nodes.push(BtNode { name: format!("Service {}", self.nodes.len()), node_type: NodeType::Service, enabled: true, description: String::new() });
                    }
                    ui.separator();
                    ui.checkbox(&mut self.auto_layout, "Auto Layout");
                    ui.checkbox(&mut self.show_decorators, "Show Decorators");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Node Canvas");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(400.0, 300.0));
                            ui.label("(Node graph canvas - draw nodes and connections here)");
                            for (i, node) in self.nodes.iter().enumerate() {
                                let selected = self.selected_node == Some(i);
                                let text = format!("[{:?}] {} {}", node.node_type, if node.enabled { "" } else { "(disabled) " }, node.name);
                                if ui.selectable_label(selected, text).clicked() {
                                    self.selected_node = Some(i);
                                }
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Node Properties");
                        ui.separator();
                        if let Some(idx) = self.selected_node {
                            if idx < self.nodes.len() {
                                let node = &mut self.nodes[idx];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut node.name);
                                ui.checkbox(&mut node.enabled, "Enabled");
                                ui.label("Description:");
                                ui.text_edit_multiline(&mut node.description);
                            }
                        } else {
                            ui.label("Select a node to edit");
                        }
                    });
                });
                ui.separator();
                ui.label("Blackboard:");
                ui.horizontal(|ui| {
                    if ui.button("Add Variable").clicked() {
                        self.blackboard.push(BlackboardVar { key: format!("Var{}", self.blackboard.len()), var_type: "Float".into(), value: "0.0".into() });
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, var) in self.blackboard.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut var.key);
                            ui.text_edit_singleline(&mut var.var_type);
                            ui.text_edit_singleline(&mut var.value);
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.blackboard.remove(i); }
                });
            });
    }
}
