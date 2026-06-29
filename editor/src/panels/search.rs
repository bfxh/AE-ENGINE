//! Search panel: find nodes by name, type, or property.
//!
//! Provides a search bar with filters for finding scene nodes quickly.

use crate::app::EditorApp;
use crate::panels::EditorPanel;
use crate::scene::NodeType;

/// Search panel state.
pub struct SearchPanel {
    pub visible: bool,
    pub query: String,
    pub filter_type: Option<NodeTypeFilter>,
    pub results: Vec<SearchResult>,
    pub selected: Option<usize>,
    pub case_sensitive: bool,
    pub recent_searches: Vec<String>,
}

/// Node type filter options.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeTypeFilter {
    All,
    Empty,
    Mesh,
    Light,
    Camera,
}

impl NodeTypeFilter {
    fn label(&self) -> &'static str {
        match self {
            NodeTypeFilter::All => "All",
            NodeTypeFilter::Empty => "Empty",
            NodeTypeFilter::Mesh => "Mesh",
            NodeTypeFilter::Light => "Light",
            NodeTypeFilter::Camera => "Camera",
        }
    }
}

/// A single search result.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub node_id: u64,
    pub node_name: String,
    pub node_type: String,
    pub depth: usize,
    pub path: String,
}

impl Default for SearchPanel {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            filter_type: Some(NodeTypeFilter::All),
            results: Vec::new(),
            selected: None,
            case_sensitive: false,
            recent_searches: Vec::new(),
        }
    }
}

impl SearchPanel {
    fn search(&mut self, app: &EditorApp) {
        self.results.clear();
        self.selected = None;

        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        if query.is_empty() && self.filter_type == Some(NodeTypeFilter::All) {
            return;
        }

        // Add to recent searches.
        if !self.query.is_empty() {
            if !self.recent_searches.contains(&self.query) {
                self.recent_searches.insert(0, self.query.clone());
                if self.recent_searches.len() > 10 {
                    self.recent_searches.pop();
                }
            }
        }

        for node in &app.scene.nodes {
            if let Some(filter) = self.filter_type {
                let matches_type = match (&node.node_type, filter) {
                    (_, NodeTypeFilter::All) => true,
                    (NodeType::Empty { .. }, NodeTypeFilter::Empty) => true,
                    (NodeType::Mesh { .. }, NodeTypeFilter::Mesh) => true,
                    (NodeType::Light { .. }, NodeTypeFilter::Light) => true,
                    (NodeType::Camera { .. }, NodeTypeFilter::Camera) => true,
                    _ => false,
                };
                if !matches_type { continue; }
            }

            let name_to_check = if self.case_sensitive {
                node.name.clone()
            } else {
                node.name.to_lowercase()
            };

            if !query.is_empty() && !name_to_check.contains(&query) { continue; }

            let type_name = match &node.node_type {
                NodeType::Empty => "Empty",
                NodeType::Mesh { .. } => "Mesh",
                NodeType::Light { .. } => "Light",
                NodeType::Camera { .. } => "Camera",
            };

            let path = self.build_path(app, node.id);

            self.results.push(SearchResult {
                node_id: node.id,
                node_name: node.name.clone(),
                node_type: type_name.to_string(),
                depth: 0usize,
                path,
            });
        }

        self.results.sort_by(|a, b| a.node_name.cmp(&b.node_name));
    }

    fn build_path(&self, app: &EditorApp, node_id: u64) -> String {
        let mut path = Vec::new();
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node) = app.scene.find_node(id) {
                path.insert(0, node.name.clone());
                current = node.parent;
            } else {
                break;
            }
        }
        path.join(" / ")
    }

    fn highlight_match(&self, text: &str, query: &str) -> Vec<egui::RichText> {
        if query.is_empty() {
            return vec![egui::RichText::new(text.to_string())];
        }

        let lower_text = if self.case_sensitive { text.to_string() } else { text.to_lowercase() };
        let lower_query = if self.case_sensitive { query.to_string() } else { query.to_lowercase() };

        let mut parts = Vec::new();
        let mut last_end = 0;

        for (idx, _) in lower_text.match_indices(&lower_query) {
            if idx > last_end {
                parts.push(egui::RichText::new(&text[last_end..idx]));
            }
            let end = idx + query.len();
            parts.push(egui::RichText::new(&text[idx..end]).color(egui::Color32::from_rgb(255, 220, 100)).strong());
            last_end = end;
        }

        if last_end < text.len() {
            parts.push(egui::RichText::new(&text[last_end..]));
        }

        if parts.is_empty() {
            vec![egui::RichText::new(text.to_string())]
        } else {
            parts
        }
    }
}

impl EditorPanel for SearchPanel {
    fn name(&self) -> &str { "Search" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }

        egui::Window::new("Search")
            .default_width(550.0).default_height(350.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Search bar.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Search:").strong());
                    let response = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Enter node name...").desired_width(250.0));
                    if response.changed() { self.search(app); }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.search(app); }
                    if ui.button("Search").clicked() { self.search(app); }
                    if ui.button("Clear").clicked() {
                        self.query.clear();
                        self.results.clear();
                        self.selected = None;
                    }
                    ui.checkbox(&mut self.case_sensitive, "Aa");
                });

                ui.separator();

                // Filters row.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Type:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                    let mut current = self.filter_type.unwrap_or(NodeTypeFilter::All);
                    egui::ComboBox::from_id_salt("type_filter")
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut current, NodeTypeFilter::All, "All");
                            ui.selectable_value(&mut current, NodeTypeFilter::Empty, "Empty");
                            ui.selectable_value(&mut current, NodeTypeFilter::Mesh, "Mesh");
                            ui.selectable_value(&mut current, NodeTypeFilter::Light, "Light");
                            ui.selectable_value(&mut current, NodeTypeFilter::Camera, "Camera");
                        });
                    if current != self.filter_type.unwrap_or(NodeTypeFilter::All) {
                        self.filter_type = Some(current);
                        self.search(app);
                    }

                    ui.separator();

                    // Recent searches.
                    if !self.recent_searches.is_empty() {
                        ui.label(egui::RichText::new("Recent:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                        let recents: Vec<String> = self.recent_searches.iter().take(5).cloned().collect();
                        for recent in &recents {
                            if ui.small_button(recent).clicked() {
                                self.query = recent.clone();
                                self.search(app);
                            }
                        }
                    }
                });

                ui.separator();

                // Results count.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{} results", self.results.len())).color(egui::Color32::from_rgb(180, 220, 255)));
                    if !self.results.is_empty() {
                        ui.label(egui::RichText::new(format!("(showing {})", self.results.len())).small().color(egui::Color32::from_rgb(120, 120, 120)));
                    }
                });

                ui.separator();

                // Results list.
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let query = self.query.clone();
                    let results_clone: Vec<(usize, SearchResult)> = self.results.iter().enumerate().map(|(i, r)| (i, r.clone())).collect();
                    for (i, result) in &results_clone {
                        let is_selected = self.selected == Some(*i);
                        let type_icon = match result.node_type.as_str() {
                            "Empty" => "[E]",
                            "Mesh" => "[M]",
                            "Light" => "[L]",
                            "Camera" => "[C]",
                            _ => "[?]",
                        };
                        let type_color = match result.node_type.as_str() {
                            "Empty" => egui::Color32::from_rgb(150, 150, 150),
                            "Mesh" => egui::Color32::from_rgb(100, 180, 255),
                            "Light" => egui::Color32::from_rgb(255, 220, 100),
                            "Camera" => egui::Color32::from_rgb(100, 255, 100),
                            _ => egui::Color32::from_rgb(180, 180, 180),
                        };

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(type_icon).color(type_color).strong());

                            // Highlighted name.
                            let highlights = self.highlight_match(&result.node_name, &query);
                            let mut label_response = None;
                            ui.horizontal(|ui| {
                                for part in highlights {
                                    let rich = part.color(if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(220, 220, 230) });
                                    let r = ui.add(egui::Label::new(rich).sense(egui::Sense::click()));
                                    if r.clicked() {
                                        label_response = Some(r.clone());
                                    }
                                }
                            });

                            ui.label(egui::RichText::new(format!("#{}", result.node_id)).small().color(egui::Color32::from_rgb(120, 120, 120)));
                            ui.label(egui::RichText::new(&result.node_type).small().color(type_color));

                            // Select on click.
                            if ui.add(egui::Label::new("").sense(egui::Sense::click())).clicked() || label_response.is_some() {
                                self.selected = Some(*i);
                                app.selection.select(result.node_id);
                            }
                        });

                        if is_selected {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(egui::RichText::new(&result.path).small().color(egui::Color32::from_rgb(150, 150, 160)));
                            });
                        }

                        ui.separator();
                    }
                });

                ui.separator();

                // Actions.
                ui.horizontal(|ui| {
                    if ui.button("Select First").clicked() {
                        if let Some(first) = self.results.first() {
                            app.selection.select(first.node_id);
                        }
                    }
                    if ui.button("Focus Selected").clicked() {
                        app.pending_action = Some(crate::app::EditorAction::FocusSelection);
                    }
                    if ui.button("Close").clicked() {
                        self.visible = false;
                    }
                });
            });
    }
}



