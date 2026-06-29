//! 对话树面板：分支对话编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct DialogueTreePanel {
    pub visible: bool,
    pub tree_name: String,
    pub nodes: Vec<DialogueNode>,
    pub selected_node: Option<usize>,
    pub variables: Vec<DialogueVariable>,
    pub voice_dir: String,
    pub auto_play_voice: bool,
    pub show_conditions: bool,
    pub start_node: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DialogueNode {
    pub name: String,
    pub speaker: String,
    pub text: String,
    pub voice_file: String,
    pub conditions: Vec<String>,
    pub branches: Vec<DialogueBranch>,
    pub node_type: DialogueNodeType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogueNodeType {
    Start,
    Dialogue,
    Choice,
    End,
}

#[derive(Debug, Clone)]
pub struct DialogueBranch {
    pub text: String,
    pub target: Option<usize>,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct DialogueVariable {
    pub name: String,
    pub var_type: String,
    pub value: String,
}

impl Default for DialogueTreePanel {
    fn default() -> Self {
        Self {
            visible: false,
            tree_name: "DialogueTree".to_string(),
            nodes: vec![
                DialogueNode {
                    name: "Start".into(),
                    speaker: "NPC".into(),
                    text: "Hello there!".into(),
                    voice_file: "voice/hello.wav".into(),
                    conditions: vec![],
                    branches: vec![DialogueBranch { text: "Continue".into(), target: Some(1), condition: String::new() }],
                    node_type: DialogueNodeType::Start,
                },
                DialogueNode {
                    name: "Response".into(),
                    speaker: "Player".into(),
                    text: "Hi, how are you?".into(),
                    voice_file: String::new(),
                    conditions: vec![],
                    branches: vec![
                        DialogueBranch { text: "Good".into(), target: None, condition: String::new() },
                        DialogueBranch { text: "Bad".into(), target: None, condition: "health < 50".into() },
                    ],
                    node_type: DialogueNodeType::Choice,
                },
            ],
            selected_node: Some(0),
            variables: vec![
                DialogueVariable { name: "health".into(), var_type: "int".into(), value: "100".into() },
                DialogueVariable { name: "met_npc".into(), var_type: "bool".into(), value: "false".into() },
            ],
            voice_dir: "voice/".to_string(),
            auto_play_voice: true,
            show_conditions: true,
            start_node: Some(0),
        }
    }
}

impl EditorPanel for DialogueTreePanel {
    fn name(&self) -> &str { "Dialogue Tree" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Tree:");
                    ui.text_edit_singleline(&mut self.tree_name);
                    ui.separator();
                    if ui.button("Add Node").clicked() {
                        self.nodes.push(DialogueNode { name: format!("Node {}", self.nodes.len()), speaker: String::new(), text: String::new(), voice_file: String::new(), conditions: vec![], branches: vec![], node_type: DialogueNodeType::Dialogue });
                    }
                    ui.separator();
                    ui.checkbox(&mut self.auto_play_voice, "Auto Voice");
                    ui.checkbox(&mut self.show_conditions, "Conditions");
                    ui.separator();
                    ui.label("Voice Dir:");
                    ui.text_edit_singleline(&mut self.voice_dir);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Canvas");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(400.0, 350.0));
                            for (i, node) in self.nodes.iter().enumerate() {
                                let selected = self.selected_node == Some(i);
                                let is_start = self.start_node == Some(i);
                                let prefix = if is_start { "[START] " } else { "" };
                                if ui.selectable_label(selected, format!("{}{} [{:?}] {} - {}", prefix, i, node.node_type, node.speaker, node.text.chars().take(30).collect::<String>())).clicked() {
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
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    ui.radio_value(&mut node.node_type, DialogueNodeType::Start, "Start");
                                    ui.radio_value(&mut node.node_type, DialogueNodeType::Dialogue, "Dialogue");
                                    ui.radio_value(&mut node.node_type, DialogueNodeType::Choice, "Choice");
                                    ui.radio_value(&mut node.node_type, DialogueNodeType::End, "End");
                                });
                                ui.label("Speaker:");
                                ui.text_edit_singleline(&mut node.speaker);
                                ui.label("Text:");
                                ui.text_edit_multiline(&mut node.text);
                                ui.label("Voice File:");
                                ui.text_edit_singleline(&mut node.voice_file);
                                ui.separator();
                                ui.label("Branches:");
                                let mut remove_idx: Option<usize> = None;
                                for (bi, b) in node.branches.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut b.text);
                                        ui.text_edit_singleline(&mut b.condition);
                                        if ui.button("X").clicked() { remove_idx = Some(bi); }
                                    });
                                }
                                if let Some(i) = remove_idx { node.branches.remove(i); }
                                if ui.button("Add Branch").clicked() {
                                    node.branches.push(DialogueBranch { text: String::new(), target: None, condition: String::new() });
                                }
                            }
                        } else {
                            ui.label("Select a node");
                        }
                    });
                });
                ui.separator();
                ui.label("Variables:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, v) in self.variables.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut v.name);
                            ui.text_edit_singleline(&mut v.var_type);
                            ui.text_edit_singleline(&mut v.value);
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.variables.remove(i); }
                });
                if ui.button("Add Variable").clicked() {
                    self.variables.push(DialogueVariable { name: format!("var{}", self.variables.len()), var_type: "int".into(), value: "0".into() });
                }
            });
    }
}
