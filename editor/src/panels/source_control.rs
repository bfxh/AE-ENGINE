//! Source control panel: commit, branches, history, diff, merge.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SourceControlPanel {
    pub visible: bool,
    pub active_tab: ScTab,
    pub commit_message: String,
    pub staged_files: Vec<ScFile>,
    pub unstaged_files: Vec<ScFile>,
    pub branches: Vec<String>,
    pub current_branch: usize,
    pub history: Vec<CommitEntry>,
    pub author_name: String,
    pub author_email: String,
    pub auto_fetch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScTab { Changes, Branches, History, Settings }

#[derive(Debug, Clone)]
pub struct ScFile { pub path: String, pub status: FileStatus, pub selected: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileStatus { Modified, Added, Deleted, Renamed, Untracked }

#[derive(Debug, Clone)]
pub struct CommitEntry { pub hash: String, pub message: String, pub author: String, pub date: String }

impl Default for SourceControlPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: ScTab::Changes, commit_message: String::new(),
            staged_files: vec![ScFile { path: "src/main.rs".into(), status: FileStatus::Modified, selected: true }],
            unstaged_files: vec![
                ScFile { path: "src/scene.rs".into(), status: FileStatus::Modified, selected: false },
                ScFile { path: "assets/new.png".into(), status: FileStatus::Untracked, selected: false },
            ],
            branches: vec!["main".into(), "feature/physics".into(), "bugfix/leak".into()],
            current_branch: 0,
            history: vec![
                CommitEntry { hash: "a1b2c3d".into(), message: "Add physics system".into(), author: "Dev".into(), date: "2026-06-22".into() },
                CommitEntry { hash: "e4f5g6h".into(), message: "Fix memory leak".into(), author: "Dev".into(), date: "2026-06-21".into() },
            ],
            author_name: "Developer".into(), author_email: "dev@example.com".into(), auto_fetch: true,
        }
    }
}

impl EditorPanel for SourceControlPanel {
    fn name(&self) -> &str { "Source Control" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ScTab::Changes, "Changes");
                ui.selectable_value(&mut self.active_tab, ScTab::Branches, "Branches");
                ui.selectable_value(&mut self.active_tab, ScTab::History, "History");
                ui.selectable_value(&mut self.active_tab, ScTab::Settings, "Settings");
            });
            ui.separator();
            match self.active_tab {
                ScTab::Changes => {
                    ui.label("Commit Message:");
                    ui.text_edit_multiline(&mut self.commit_message);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Commit").clicked() {
                            if !self.commit_message.is_empty() && !self.staged_files.is_empty() {
                                self.history.insert(0, CommitEntry { hash: "new".into(), message: self.commit_message.clone(), author: self.author_name.clone(), date: "2026-06-23".into() });
                                self.commit_message.clear(); self.staged_files.clear();
                            }
                        }
                        if ui.button("Stage All").clicked() { while let Some(mut f) = self.unstaged_files.pop() { f.selected = true; self.staged_files.push(f); } }
                        if ui.button("Unstage All").clicked() { while let Some(mut f) = self.staged_files.pop() { f.selected = false; self.unstaged_files.push(f); } }
                    });
                    ui.separator();
                    ui.label("Staged Files");
                    for i in 0..self.staged_files.len() { ui.horizontal(|ui| { ui.checkbox(&mut self.staged_files[i].selected, ""); ui.label(self.staged_files[i].path.as_str()); ui.label(format!("{:?}", self.staged_files[i].status)); }); }
                    ui.separator();
                    ui.label("Unstaged Files");
                    for i in 0..self.unstaged_files.len() { ui.horizontal(|ui| { ui.checkbox(&mut self.unstaged_files[i].selected, ""); ui.label(self.unstaged_files[i].path.as_str()); ui.label(format!("{:?}", self.unstaged_files[i].status)); }); }
                },
                ScTab::Branches => {
                    ui.heading("Branches");
                    ui.horizontal(|ui| { ui.label("Current:"); if self.current_branch < self.branches.len() { ui.label(self.branches[self.current_branch].as_str()); } });
                    ui.separator();
                    for i in 0..self.branches.len() {
                        let selected = self.current_branch == i;
                        ui.horizontal(|ui| { if ui.selectable_label(selected, self.branches[i].as_str()).clicked() { self.current_branch = i; } ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if i != self.current_branch && ui.button("Checkout").clicked() { self.current_branch = i; } }); });
                    }
                },
                ScTab::History => {
                    ui.heading("Commit History");
                    ui.separator();
                    for i in 0..self.history.len() {
                        ui.horizontal(|ui| { ui.label(&self.history[i].hash[..7]); ui.label(self.history[i].message.as_str()); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(self.history[i].date.as_str()); }); });
                        ui.label(format!("  by {}", self.history[i].author));
                        ui.separator();
                    }
                },
                ScTab::Settings => {
                    ui.heading("Settings");
                    ui.horizontal(|ui| { ui.label("Author Name:"); ui.text_edit_singleline(&mut self.author_name); });
                    ui.horizontal(|ui| { ui.label("Author Email:"); ui.text_edit_singleline(&mut self.author_email); });
                    ui.checkbox(&mut self.auto_fetch, "Auto Fetch");
                },
            }
        });
    }
}
