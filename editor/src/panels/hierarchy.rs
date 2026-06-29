//! Hierarchy panel: displays the scene graph as a tree.
//!
//! Shows all nodes in the scene in a hierarchical tree view on the left side.
//! Supports expand/collapse, context menu, search filter, and drag-and-drop reparenting.

use crate::app::{EditorAction, EditorApp};
use crate::panels::EditorPanel;
use crate::scene::NodeType;
use std::collections::HashSet;

#[derive(Default)]
pub struct HierarchyPanel {
    /// Set of collapsed node IDs.
    collapsed: HashSet<u64>,
    /// Search filter text.
    pub filter_text: String,
    /// Whether to show node IDs.
    pub show_ids: bool,
    /// Set of nodes visible in viewport (toggled by eye icon).
    pub hidden_nodes: HashSet<u64>,
    /// Node currently being renamed (inline edit), if any.
    renaming_node: Option<u64>,
    /// Buffer for the inline rename text edit.
    rename_buffer: String,
    /// Node currently being dragged (drag-and-drop reparenting), if any.
    drag_source: Option<u64>,
    /// Current drop target under the cursor while dragging, if any.
    /// Reset at the start of every frame; recomputed during rendering.
    drop_target: Option<u64>,
}

impl EditorPanel for HierarchyPanel {
    fn name(&self) -> &str { "Hierarchy" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        // Consume pending rename request (from F2 shortcut).
        if let Some(rename_id) = app.pending_rename.take() {
            if let Some(node) = app.scene.find_node(rename_id) {
                self.renaming_node = Some(rename_id);
                self.rename_buffer = node.name.clone();
            }
        }

        // Reset drop target at the start of each frame; it is recomputed during
        // rendering as the user hovers over nodes while dragging.
        self.drop_target = None;

        egui::SidePanel::left("hierarchy_panel").resizable(true).default_width(240.0).show(
            ctx,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Hierarchy").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+").clicked() {
                            let root_id = app.scene.nodes.first().map(|n| n.id).unwrap_or(0);
                            if let Some(new_id) = app.add_child_with_undo(root_id, "Empty", NodeType::Empty) {
                                app.selection.select(new_id);
                            }
                        }
                    });
                });
                ui.separator();

                // Search filter.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Filter:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                    ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search nodes...").desired_width(120.0));
                    if ui.button("x").clicked() {
                        self.filter_text.clear();
                    }
                });

                // Quick add buttons.
                ui.horizontal(|ui| {
                    if ui.small_button("Light").clicked() {
                        let root_id = app.scene.nodes.first().map(|n| n.id).unwrap_or(0);
                        if let Some(new_id) = app.add_child_with_undo(root_id, "Light", NodeType::Light {
                            light_type: crate::scene::LightType::Point,
                            color: glam::Vec3::ONE,
                            intensity: 1.0,
                        }) {
                            app.selection.select(new_id);
                        }
                    }
                    if ui.small_button("Camera").clicked() {
                        let root_id = app.scene.nodes.first().map(|n| n.id).unwrap_or(0);
                        if let Some(new_id) = app.add_child_with_undo(root_id, "Camera", NodeType::Camera { fov: 60.0, near: 0.1, far: 1000.0 }) {
                            app.selection.select(new_id);
                        }
                    }
                    if ui.small_button("Empty").clicked() {
                        let root_id = app.scene.nodes.first().map(|n| n.id).unwrap_or(0);
                        if let Some(new_id) = app.add_child_with_undo(root_id, "Empty", NodeType::Empty) {
                            app.selection.select(new_id);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut self.show_ids, "IDs");
                    });
                });

                ui.separator();

                // Render hierarchy tree.
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let order = app.scene.hierarchy_order();
                    let filter = self.filter_text.to_lowercase();
                    let mut visible_count = 0u32;
                    for node_id in order {
                        if self.render_node(ui, app, node_id, 0, &filter) {
                            visible_count += 1;
                        }
                    }
                    if !filter.is_empty() && visible_count == 0 {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("No matching nodes").color(egui::Color32::from_rgb(120, 120, 120)));
                    }
                });

                // Handle drag release: if a drag is in progress and the primary
                // button was released, attempt to reparent to drop_target.
                let pointer_released = ui.input(|i| i.pointer.primary_released());
                if pointer_released {
                    if let (Some(src), Some(target)) = (self.drag_source.take(), self.drop_target.take()) {
                        self.commit_reparent(app, src, target);
                    } else {
                        // Drag ended outside any valid target — just clear state.
                        self.drag_source = None;
                        self.drop_target = None;
                    }
                }
            },
        );
    }
}

impl HierarchyPanel {
    /// Attempt to reparent `src` under `new_parent`, recording a command for undo.
    /// Silently rejects invalid reparents (cycles, self, same parent, root).
    fn commit_reparent(&mut self, app: &mut EditorApp, src: u64, new_parent: u64) {
        // Snapshot old parent before mutation.
        let old_parent = match app.scene.find_node(src) {
            Some(n) => n.parent,
            None => return,
        };
        let old_parent_id = match old_parent {
            Some(p) => p,
            None => return, // root cannot be reparented
        };
        // Attempt the reparent; reparent_node validates cycles/self/same-parent.
        if app.scene.reparent_node(src, new_parent).is_some() {
            let cmd = crate::commands::ReparentNodeCommand::new(src, old_parent_id, new_parent);
            let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
            app.dirty = true;
            // Select the reparented node so the user sees it move.
            app.selection.select(src);
        }
    }
}

impl HierarchyPanel {
    /// Render a single node and its children recursively.
    /// Returns true if the node (or any child) matched the filter.
    fn render_node(&mut self, ui: &mut egui::Ui, app: &mut EditorApp, node_id: u64, depth: usize, filter: &str) -> bool {
        let node = match app.scene.find_node(node_id) {
            Some(n) => n.clone(),
            None => return false,
        };

        // Check filter match.
        let name_matches = filter.is_empty() || node.name.to_lowercase().contains(filter);
        let is_hidden = self.hidden_nodes.contains(&node_id);

        // Check if any children match (for filtering).
        let children: Vec<u64> = node.children.clone();
        let _any_child_matches = false;
        if !filter.is_empty() && !name_matches {
            // Need to check children recursively.
            // We'll render children first to determine visibility.
        }

        let is_selected = app.selection.is_selected(node_id);
        let indent = depth as f32 * 16.0;
        let has_children = !node.children.is_empty();
        let is_collapsed = self.collapsed.contains(&node_id);

        // Only render if matches filter or has matching children.
        let should_render = filter.is_empty() || name_matches;

        if should_render {
            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Expand/collapse arrow.
                let arrow_text = if has_children {
                    if is_collapsed { ">" } else { "v" }
                } else {
                    " "
                };
                let arrow_color = if has_children {
                    egui::Color32::from_rgb(200, 200, 210)
                } else {
                    egui::Color32::from_rgb(80, 80, 80)
                };
                if ui.add(egui::Label::new(egui::RichText::new(arrow_text).color(arrow_color)).sense(egui::Sense::click())).clicked() && has_children {
                    if is_collapsed {
                        self.collapsed.remove(&node_id);
                    } else {
                        self.collapsed.insert(node_id);
                    }
                }

                // Visibility toggle (eye icon).
                let eye_text = if is_hidden { "-" } else { "o" };
                let eye_color = if is_hidden {
                    egui::Color32::from_rgb(120, 120, 120)
                } else {
                    egui::Color32::from_rgb(180, 220, 180)
                };
                if ui.add(egui::Label::new(egui::RichText::new(eye_text).color(eye_color)).sense(egui::Sense::click())).clicked() {
                    if is_hidden {
                        self.hidden_nodes.remove(&node_id);
                    } else {
                        self.hidden_nodes.insert(node_id);
                    }
                }

                // Type icon.
                let type_icon = match &node.node_type {
                    NodeType::Empty => "[E]",
                    NodeType::Mesh { .. } => "[M]",
                    NodeType::Light { .. } => "[L]",
                    NodeType::Camera { .. } => "[C]",
                };
                let type_color = match &node.node_type {
                    NodeType::Empty => egui::Color32::from_rgb(150, 150, 150),
                    NodeType::Mesh { .. } => egui::Color32::from_rgb(100, 180, 255),
                    NodeType::Light { .. } => egui::Color32::from_rgb(255, 220, 100),
                    NodeType::Camera { .. } => egui::Color32::from_rgb(100, 255, 100),
                };

                // Build label.
                let label = if self.show_ids {
                    format!("{} {} #{}", type_icon, node.name, node.id)
                } else {
                    format!("{} {}", type_icon, node.name)
                };

                // Inline rename or normal label.
                let is_renaming = self.renaming_node == Some(node_id);

                let response = if is_renaming {
                    let edit_response = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_buffer)
                            .desired_width(120.0)
                            .clip_text(true),
                    );
                    if !edit_response.has_focus() {
                        edit_response.request_focus();
                    }

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                    if escape_pressed {
                        self.renaming_node = None;
                    } else if enter_pressed || (edit_response.lost_focus() && edit_response.changed()) {
                        let new_name = self.rename_buffer.trim().to_string();
                        if !new_name.is_empty() && new_name != node.name {
                            let cmd = crate::commands::RenameNodeCommand::new(
                                node_id,
                                node.name.clone(),
                                new_name,
                            );
                            let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                            app.dirty = true;
                        }
                        self.renaming_node = None;
                    } else if edit_response.lost_focus() {
                        // Lost focus without changes - cancel.
                        self.renaming_node = None;
                    }

                    edit_response
                } else {
                    // click_and_drag preserves click selection / double-click focus
                    // while also enabling drag-and-drop reparenting.
                    ui.add(
                        egui::Label::new(egui::RichText::new(&label).color(type_color).strong())
                            .sense(egui::Sense::click_and_drag())
                    )
                };

                // Track drag start (root cannot be dragged).
                if response.dragged() && self.drag_source.is_none() && node.parent.is_some() {
                    self.drag_source = Some(node_id);
                }

                // Detect drop target while dragging.
                if let Some(src) = self.drag_source {
                    if src != node_id && response.hovered() {
                        // Reject if node_id is a descendant of src (would cycle).
                        let is_descendant_of_src = app
                            .scene
                            .collect_subtree_nodes(src)
                            .iter()
                            .any(|n| n.id == node_id);
                        if !is_descendant_of_src {
                            self.drop_target = Some(node_id);
                        }
                    }
                }

                // Visual feedback.
                let is_drag_src = self.drag_source == Some(node_id);
                let is_drop_tgt = self.drop_target == Some(node_id);

                if is_drop_tgt {
                    ui.painter().rect_filled(
                        response.rect,
                        2,
                        egui::Color32::from_rgba_premultiplied(80, 220, 120, 90),
                    );
                    ui.painter().rect_stroke(
                        response.rect,
                        2,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 240, 140)),
                        egui::StrokeKind::Middle,
                    );
                } else if is_selected {
                    ui.painter().rect_filled(
                        response.rect,
                        2,
                        egui::Color32::from_rgba_premultiplied(80, 120, 200, 60),
                    );
                }

                if is_drag_src && !is_drop_tgt {
                    ui.painter().rect_stroke(
                        response.rect,
                        2,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 180, 100)),
                        egui::StrokeKind::Middle,
                    );
                }

                // Right-click context menu.
                response.context_menu(|ui| {
                    if ui.button("Add Child (Empty)").clicked() {
                        if let Some(new_id) = app.add_child_with_undo(node_id, "Child", NodeType::Empty) {
                            app.selection.select(new_id);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Add Child (Light)").clicked() {
                        if let Some(new_id) = app.add_child_with_undo(node_id, "Light", NodeType::Light {
                            light_type: crate::scene::LightType::Point,
                            color: glam::Vec3::ONE,
                            intensity: 1.0,
                        }) {
                            app.selection.select(new_id);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Rename").clicked() {
                        self.renaming_node = Some(node_id);
                        self.rename_buffer = node.name.clone();
                        ui.close_menu();
                    }
                    if ui.button("Duplicate").clicked() {
                        if let Some(new_id) = app.duplicate_node_with_undo(node_id) {
                            // Selection handled inside duplicate_node_with_undo when auto_select is on.
                            let _ = new_id;
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Toggle Visibility").clicked() {
                        if self.hidden_nodes.contains(&node_id) {
                            self.hidden_nodes.remove(&node_id);
                        } else {
                            self.hidden_nodes.insert(node_id);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if node.parent.is_some() {
                        if ui.button("Delete").clicked() {
                            app.pending_action = Some(EditorAction::DeleteSelected);
                            app.selection.select(node_id);
                            ui.close_menu();
                        }
                    }
                });

                if response.clicked() {
                    app.selection.select(node_id);
                    if has_children {
                        // Don't toggle on single click, only on arrow click.
                    }
                }

                if response.double_clicked() {
                    app.pending_action = Some(EditorAction::FocusSelection);
                    app.selection.select(node_id);
                }
            });
        }

        // Render children if not collapsed.
        let mut child_matched = false;
        if !is_collapsed {
            for child_id in children {
                if self.render_node(ui, app, child_id, depth + 1, filter) {
                    child_matched = true;
                }
            }
        }

        should_render || child_matched
    }
}
