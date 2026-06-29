//! 任务编辑器面板：游戏任务系统编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct QuestEditorPanel {
    pub visible: bool,
    pub quests: Vec<Quest>,
    pub selected_quest: Option<usize>,
    pub selected_objective: Option<usize>,
    pub quest_chains: Vec<QuestChain>,
    pub new_quest_name: String,
    pub show_completed: bool,
}

#[derive(Debug, Clone)]
pub struct Quest {
    pub name: String,
    pub description: String,
    pub quest_type: QuestType,
    pub objectives: Vec<Objective>,
    pub rewards: Vec<Reward>,
    pub prerequisites: Vec<String>,
    pub auto_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestType {
    Main,
    Side,
    Daily,
    Event,
}

#[derive(Debug, Clone)]
pub struct Objective {
    pub description: String,
    pub target_count: u32,
    pub condition: String,
    pub optional: bool,
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct Reward {
    pub item: String,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct QuestChain {
    pub name: String,
    pub quest_order: Vec<String>,
}

impl Default for QuestEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            quests: vec![
                Quest {
                    name: "Find the Sword".into(),
                    description: "Locate the legendary sword".into(),
                    quest_type: QuestType::Main,
                    objectives: vec![
                        Objective { description: "Talk to NPC".into(), target_count: 1, condition: "talked_npc == true".into(), optional: false, completed: false },
                        Objective { description: "Find sword".into(), target_count: 1, condition: "has_sword == true".into(), optional: false, completed: false },
                    ],
                    rewards: vec![Reward { item: "Gold".into(), count: 100 }],
                    prerequisites: vec![],
                    auto_complete: false,
                },
            ],
            selected_quest: Some(0),
            selected_objective: Some(0),
            quest_chains: vec![QuestChain { name: "Main Story".into(), quest_order: vec!["Find the Sword".into()] }],
            new_quest_name: String::new(),
            show_completed: true,
        }
    }
}

impl EditorPanel for QuestEditorPanel {
    fn name(&self) -> &str { "Quest Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New Quest:");
                    ui.text_edit_singleline(&mut self.new_quest_name);
                    if ui.button("Add Quest").clicked() && !self.new_quest_name.is_empty() {
                        self.quests.push(Quest { name: self.new_quest_name.clone(), description: String::new(), quest_type: QuestType::Side, objectives: vec![], rewards: vec![], prerequisites: vec![], auto_complete: false });
                        self.new_quest_name.clear();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_completed, "Show Completed");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Quests");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, q) in self.quests.iter().enumerate() {
                                let selected = self.selected_quest == Some(i);
                                if ui.selectable_label(selected, format!("[{:?}] {}", q.quest_type, q.name)).clicked() {
                                    self.selected_quest = Some(i);
                                    self.selected_objective = None;
                                }
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Quest Details");
                        ui.separator();
                        if let Some(qi) = self.selected_quest {
                            if qi < self.quests.len() {
                                let quest = &mut self.quests[qi];
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut quest.name);
                                ui.label("Description:");
                                ui.text_edit_multiline(&mut quest.description);
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    ui.radio_value(&mut quest.quest_type, QuestType::Main, "Main");
                                    ui.radio_value(&mut quest.quest_type, QuestType::Side, "Side");
                                    ui.radio_value(&mut quest.quest_type, QuestType::Daily, "Daily");
                                    ui.radio_value(&mut quest.quest_type, QuestType::Event, "Event");
                                });
                                ui.checkbox(&mut quest.auto_complete, "Auto Complete");
                                ui.separator();
                                ui.label("Objectives:");
                                let mut remove_idx: Option<usize> = None;
                                for (oi, obj) in quest.objectives.iter_mut().enumerate() {
                                    let selected = self.selected_objective == Some(oi);
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut obj.completed, "");
                                        if ui.selectable_label(selected, &obj.description).clicked() {
                                            self.selected_objective = Some(oi);
                                        }
                                        ui.add(egui::DragValue::new(&mut obj.target_count).range(1..=1000));
                                        ui.checkbox(&mut obj.optional, "Opt");
                                        if ui.button("X").clicked() { remove_idx = Some(oi); }
                                    });
                                }
                                if let Some(i) = remove_idx { quest.objectives.remove(i); }
                                if ui.button("Add Objective").clicked() {
                                    quest.objectives.push(Objective { description: String::new(), target_count: 1, condition: String::new(), optional: false, completed: false });
                                }
                                ui.separator();
                                ui.label("Rewards:");
                                let mut remove_r: Option<usize> = None;
                                for (ri, r) in quest.rewards.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut r.item);
                                        ui.add(egui::DragValue::new(&mut r.count).range(1..=10000));
                                        if ui.button("X").clicked() { remove_r = Some(ri); }
                                    });
                                }
                                if let Some(i) = remove_r { quest.rewards.remove(i); }
                                if ui.button("Add Reward").clicked() {
                                    quest.rewards.push(Reward { item: String::new(), count: 1 });
                                }
                            }
                        } else {
                            ui.label("Select a quest");
                        }
                    });
                });
                ui.separator();
                ui.label("Quest Chains:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for chain in &self.quest_chains {
                        ui.label(format!("{}: {}", chain.name, chain.quest_order.join(" -> ")));
                    }
                });
            });
    }
}
