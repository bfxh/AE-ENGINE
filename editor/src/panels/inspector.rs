//! Inspector panel: displays and edits properties of the selected node.
//!
//! Shows transform, type-specific properties, and provides edit widgets.
//! All edits go through the command system for undo/redo support.

use crate::app::EditorApp;
use crate::panels::EditorPanel;
use crate::scene::NodeType;

/// Inspector panel state.
///
/// Tracks in-progress edits to batch them into a single undo step.
#[derive(Default)]
pub struct InspectorPanel {
    /// (node_id, transform_before_edit) — captured when a drag starts, committed when it ends.
    transform_edit_start: Option<(u64, crate::scene::NodeTransform)>,
    /// (node_id, name_before_edit) — captured when name editing starts, committed on focus loss.
    name_edit_start: Option<(u64, String)>,
    /// (node_id, node_type_before_edit) — captured when type-property editing starts,
    /// committed when the drag stops (sliders/DragValues) or after a short idle period (color picker).
    node_type_edit_start: Option<(u64, crate::scene::NodeType)>,
    /// Frames since the last color-picker change (used for idle-commit of color edits).
    color_idle_frames: u32,
    /// Clipboard for Copy/Paste Transform (per-Inspector, not system clipboard).
    pub transform_clipboard: Option<crate::scene::NodeTransform>,
}

impl EditorPanel for InspectorPanel {
    fn name(&self) -> &str { "Inspector" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        // Flush pending type-property edit if the selection changed mid-edit.
        if let Some((edit_id, _)) = &self.node_type_edit_start {
            if app.selection.selected_id != Some(*edit_id) {
                self.commit_node_type_edit(app);
            }
        }

        egui::SidePanel::right("inspector_panel").resizable(true).default_width(300.0).show(
            ctx,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Inspector").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(id) = app.selection.selected_id {
                            let has_clip = self.transform_clipboard.is_some();
                            // Paste Transform (right-to-left layout, so rightmost first).
                            if ui.add_enabled(has_clip, egui::Button::new("Paste T")).clicked() {
                                if let Some(t) = self.transform_clipboard.clone() {
                                    let old = app.scene.find_node(id).map(|n| n.transform.clone());
                                    if let Some(old_t) = old {
                                        let cmd = crate::commands::SetTransformCommand::new(id, old_t, t);
                                        let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                                        app.dirty = true;
                                    }
                                }
                            }
                            if ui.button("Copy T").clicked() {
                                if let Some(n) = app.scene.find_node(id) {
                                    self.transform_clipboard = Some(n.transform.clone());
                                }
                            }
                            // Snap to Grid.
                            if ui.button("Snap").clicked() {
                                let (snap, dist) = match app.settings_panel.as_ref() {
                                    Some(s) => (s.grid_snapping, s.snap_distance.max(0.0001)),
                                    None => (false, 0.25),
                                };
                                if snap {
                                    let old = app.scene.find_node(id).map(|n| n.transform.clone());
                                    if let Some(old_t) = old {
                                        let p = old_t.translation;
                                        let new_t = crate::scene::NodeTransform {
                                            translation: glam::Vec3::new(
                                                (p.x / dist).round() * dist,
                                                (p.y / dist).round() * dist,
                                                (p.z / dist).round() * dist,
                                            ),
                                            rotation: old_t.rotation,
                                            scale: old_t.scale,
                                        };
                                        if new_t != old_t {
                                            let cmd = crate::commands::SetTransformCommand::new(id, old_t, new_t);
                                            let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                                            app.dirty = true;
                                        }
                                    }
                                }
                            }
                            // Reset dropdown.
                            ui.menu_button("Reset", |ui| {
                                if ui.button("Translation").clicked() {
                                    Self::reset_partial(app, id, ResetKind::Translation);
                                    ui.close_menu();
                                }
                                if ui.button("Rotation").clicked() {
                                    Self::reset_partial(app, id, ResetKind::Rotation);
                                    ui.close_menu();
                                }
                                if ui.button("Scale").clicked() {
                                    Self::reset_partial(app, id, ResetKind::Scale);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("All").clicked() {
                                    Self::reset_partial(app, id, ResetKind::All);
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                });
                ui.separator();

                let selected_id = match app.selection.selected_id {
                    Some(id) => id,
                    None => {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No object selected").color(egui::Color32::from_rgb(120, 120, 120)));
                            ui.label(egui::RichText::new("Click a node in the viewport or hierarchy").small().color(egui::Color32::from_rgb(100, 100, 100)));
                        });
                        return;
                    },
                };

                let node = match app.scene.find_node(selected_id) {
                    Some(n) => n.clone(),
                    None => {
                        ui.label("Selected node not found.");
                        return;
                    },
                };

                // Node header with type icon.
                ui.horizontal(|ui| {
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
                    ui.label(egui::RichText::new(type_icon).color(type_color).strong());
                    ui.label(format!("ID: {}", node.id));
                });

                // Name editor with undo support.
                self.render_name_editor(ui, app, selected_id, &node.name);

                ui.add_space(4.0);

                // Transform section (collapsing).
                ui.collapsing("Transform", |ui| {
                    self.render_transform(ui, app, selected_id, &node);
                });

                ui.add_space(2.0);

                // Type-specific properties.
                ui.collapsing("Properties", |ui| {
                    self.render_type_properties(ui, app, selected_id, &node);
                });

                ui.add_space(2.0);

                // Engine properties.
                ui.collapsing("Engine Properties", |ui| {
                    render_engine_properties(ui, app, selected_id);
                });
            },
        );
    }
}

/// Which part of the transform to reset.
enum ResetKind {
    Translation,
    Rotation,
    Scale,
    All,
}

impl InspectorPanel {
    /// Reset part (or all) of a node'"'"'s transform, recording a SetTransformCommand.
    fn reset_partial(app: &mut EditorApp, id: u64, kind: ResetKind) {
        let old = match app.scene.find_node(id) {
            Some(n) => n.transform.clone(),
            None => return,
        };
        let new_t = match kind {
            ResetKind::All => crate::scene::NodeTransform::default(),
            ResetKind::Translation => crate::scene::NodeTransform {
                translation: glam::Vec3::ZERO,
                rotation: old.rotation,
                scale: old.scale,
            },
            ResetKind::Rotation => crate::scene::NodeTransform {
                translation: old.translation,
                rotation: glam::Quat::IDENTITY,
                scale: old.scale,
            },
            ResetKind::Scale => crate::scene::NodeTransform {
                translation: old.translation,
                rotation: old.rotation,
                scale: glam::Vec3::ONE,
            },
        };
        if new_t != old {
            let cmd = crate::commands::SetTransformCommand::new(id, old, new_t);
            let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
            app.dirty = true;
        }
    }

    fn render_name_editor(&mut self, ui: &mut egui::Ui, app: &mut EditorApp, selected_id: u64, current_name: &str) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            let mut name = current_name.to_string();
            let resp = ui.add(egui::TextEdit::singleline(&mut name).desired_width(180.0));

            // Capture start of editing.
            if resp.has_focus() && self.name_edit_start.is_none() {
                self.name_edit_start = Some((selected_id, current_name.to_string()));
            }

            // Live-update the node for immediate feedback.
            if name != current_name {
                if let Some(n) = app.scene.find_node_mut(selected_id) {
                    n.name = name.clone();
                    app.dirty = true;
                }
            }

            // Commit on focus loss or Enter.
            let should_commit = (resp.lost_focus() && self.name_edit_start.is_some())
                || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if should_commit {
                if let Some((id, old_name)) = self.name_edit_start.take() {
                    let new_name = name.trim().to_string();
                    if !new_name.is_empty() && new_name != old_name {
                        let cmd = crate::commands::RenameNodeCommand::new(id, old_name, new_name);
                        let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                    }
                }
            }
        });
    }

    fn render_transform(&mut self, ui: &mut egui::Ui, app: &mut EditorApp, selected_id: u64, node: &crate::scene::SceneNode) {
        // Position.
        let mut translation = [
            node.transform.translation.x,
            node.transform.translation.y,
            node.transform.translation.z,
        ];
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Position").color(egui::Color32::from_rgb(180, 180, 190)));
        });
        let mut pos_dragged = false;
        let mut pos_drag_stopped = false;
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").color(egui::Color32::from_rgb(255, 100, 100)).strong());
                    let r0 = ui.add(egui::DragValue::new(&mut translation[0]).speed(0.1).range(-10000.0..=10000.0));
                    ui.label(egui::RichText::new("Y").color(egui::Color32::from_rgb(100, 255, 100)).strong());
                    let r1 = ui.add(egui::DragValue::new(&mut translation[1]).speed(0.1).range(-10000.0..=10000.0));
                    ui.label(egui::RichText::new("Z").color(egui::Color32::from_rgb(100, 150, 255)).strong());
                    let r2 = ui.add(egui::DragValue::new(&mut translation[2]).speed(0.1).range(-10000.0..=10000.0));
                    pos_dragged = r0.dragged() || r1.dragged() || r2.dragged();
                    pos_drag_stopped = r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped();
                });
            });
        });
        self.handle_transform_drag(ui, app, selected_id, node, pos_dragged, pos_drag_stopped, |n| {
            n.transform.translation = glam::Vec3::new(translation[0], translation[1], translation[2]);
        }, translation[0] != node.transform.translation.x
            || translation[1] != node.transform.translation.y
            || translation[2] != node.transform.translation.z);

        ui.add_space(4.0);

        // Rotation.
        let (roll, pitch_euler, yaw_euler) = node.transform.rotation.to_euler(glam::EulerRot::YXZ);
        let mut euler = [yaw_euler.to_degrees(), pitch_euler.to_degrees(), roll.to_degrees()];
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(180, 180, 190)));
        });
        let mut rot_dragged = false;
        let mut rot_drag_stopped = false;
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Y").color(egui::Color32::from_rgb(255, 200, 100)).strong());
                    let r0 = ui.add(egui::DragValue::new(&mut euler[0]).speed(1.0).suffix(" deg").range(-360.0..=360.0));
                    ui.label(egui::RichText::new("P").color(egui::Color32::from_rgb(200, 255, 100)).strong());
                    let r1 = ui.add(egui::DragValue::new(&mut euler[1]).speed(1.0).suffix(" deg").range(-360.0..=360.0));
                    ui.label(egui::RichText::new("R").color(egui::Color32::from_rgb(100, 200, 255)).strong());
                    let r2 = ui.add(egui::DragValue::new(&mut euler[2]).speed(1.0).suffix(" deg").range(-360.0..=360.0));
                    rot_dragged = r0.dragged() || r1.dragged() || r2.dragged();
                    rot_drag_stopped = r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped();
                });
            });
        });
        let new_rotation = glam::Quat::from_euler(
            glam::EulerRot::YXZ,
            euler[0].to_radians(),
            euler[1].to_radians(),
            euler[2].to_radians(),
        );
        let rot_changed = new_rotation != node.transform.rotation;
        self.handle_transform_drag(ui, app, selected_id, node, rot_dragged, rot_drag_stopped, |n| {
            n.transform.rotation = new_rotation;
        }, rot_changed);

        ui.add_space(4.0);

        // Scale.
        let mut scale = [node.transform.scale.x, node.transform.scale.y, node.transform.scale.z];
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Scale").color(egui::Color32::from_rgb(180, 180, 190)));
        });
        let mut scale_dragged = false;
        let mut scale_drag_stopped = false;
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").color(egui::Color32::from_rgb(255, 100, 100)).strong());
                    let r0 = ui.add(egui::DragValue::new(&mut scale[0]).speed(0.05).range(0.01..=100.0));
                    ui.label(egui::RichText::new("Y").color(egui::Color32::from_rgb(100, 255, 100)).strong());
                    let r1 = ui.add(egui::DragValue::new(&mut scale[1]).speed(0.05).range(0.01..=100.0));
                    ui.label(egui::RichText::new("Z").color(egui::Color32::from_rgb(100, 150, 255)).strong());
                    let r2 = ui.add(egui::DragValue::new(&mut scale[2]).speed(0.05).range(0.01..=100.0));
                    scale_dragged = r0.dragged() || r1.dragged() || r2.dragged();
                    scale_drag_stopped = r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped();
                });
            });
        });
        let scale_changed = scale[0] != node.transform.scale.x || scale[1] != node.transform.scale.y || scale[2] != node.transform.scale.z;
        self.handle_transform_drag(ui, app, selected_id, node, scale_dragged, scale_drag_stopped, |n| {
            n.transform.scale = glam::Vec3::new(scale[0], scale[1], scale[2]);
        }, scale_changed);
    }

    /// Helper: manages drag-start capture, live mutation, and drag-stop commit for one transform field group.
    ///
    /// - On drag start: records the node's pre-edit transform.
    /// - During drag: applies live mutation for immediate feedback.
    /// - On drag stop: commits a single `SetTransformCommand` covering the whole drag.
    fn handle_transform_drag(
        &mut self,
        _ui: &mut egui::Ui,
        app: &mut EditorApp,
        selected_id: u64,
        node: &crate::scene::SceneNode,
        dragged: bool,
        drag_stopped: bool,
        mutate: impl FnOnce(&mut crate::scene::SceneNode),
        changed: bool,
    ) {
        // Capture pre-edit transform when a drag begins.
        if dragged && self.transform_edit_start.is_none() {
            self.transform_edit_start = Some((selected_id, node.transform.clone()));
        }

        // Live mutation for immediate visual feedback.
        if changed {
            if let Some(n) = app.scene.find_node_mut(selected_id) {
                mutate(n);
                app.dirty = true;
            }
        }

        // Commit one undo command when the drag ends.
        if drag_stopped {
            if let Some((id, old_t)) = self.transform_edit_start.take() {
                if let Some(n) = app.scene.find_node(id) {
                    let new_t = n.transform.clone();
                    if new_t != old_t {
                        let cmd = crate::commands::SetTransformCommand::new(id, old_t, new_t);
                        let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                    }
                }
            }
        }
    }

    fn render_type_properties(&mut self, ui: &mut egui::Ui, app: &mut EditorApp, selected_id: u64, node: &crate::scene::SceneNode) {
        match &node.node_type {
            NodeType::Empty => {
                ui.label(egui::RichText::new("Empty node - no properties").color(egui::Color32::from_rgb(120, 120, 120)));
            },
            NodeType::Mesh { path } => {
                ui.horizontal(|ui| {
                    ui.label("Mesh Path:");
                    ui.label(egui::RichText::new(path).color(egui::Color32::from_rgb(100, 180, 255)).family(egui::FontFamily::Monospace));
                });
            },
            NodeType::Light { light_type, color, intensity } => {
                ui.horizontal(|ui| {
                    ui.label("Light Type:");
                    let lt_str = format!("{:?}", light_type);
                    ui.label(egui::RichText::new(lt_str).color(egui::Color32::from_rgb(255, 220, 100)));
                });

                ui.add_space(4.0);

                // Color editor — uses idle-frame commit (color picker has no drag_stopped).
                let mut col = [color.x, color.y, color.z];
                let col_resp = ui.horizontal(|ui| {
                    ui.label("Color:");
                    let r = ui.color_edit_button_rgb(&mut col);
                    ui.label(egui::RichText::new(format!("({:.2}, {:.2}, {:.2})", color.x, color.y, color.z)).small().color(egui::Color32::from_rgb(150, 150, 150)).family(egui::FontFamily::Monospace));
                    r
                }).inner;
                if col_resp.changed() {
                    if self.node_type_edit_start.is_none() {
                        self.node_type_edit_start = Some((selected_id, node.node_type.clone()));
                    }
                    if let Some(n) = app.scene.find_node_mut(selected_id) {
                        if let NodeType::Light { color: ref mut c, .. } = &mut n.node_type {
                            *c = glam::Vec3::new(col[0], col[1], col[2]);
                        }
                        app.dirty = true;
                    }
                    self.color_idle_frames = 0;
                } else if self.node_type_edit_start.is_some() {
                    self.color_idle_frames += 1;
                    if self.color_idle_frames > 10 {
                        self.commit_node_type_edit(app);
                    }
                }

                ui.add_space(4.0);

                // Intensity slider — uses drag_stopped commit.
                let mut int_val = *intensity;
                let int_resp = ui.horizontal(|ui| {
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut int_val, 0.0..=100.0).text(""))
                }).inner;
                if int_resp.dragged() && self.node_type_edit_start.is_none() {
                    self.node_type_edit_start = Some((selected_id, node.node_type.clone()));
                }
                if int_resp.changed() && int_val != *intensity {
                    if let Some(n) = app.scene.find_node_mut(selected_id) {
                        if let NodeType::Light { intensity: ref mut i, .. } = &mut n.node_type {
                            *i = int_val;
                        }
                        app.dirty = true;
                    }
                }
                if int_resp.drag_stopped() {
                    self.commit_node_type_edit(app);
                }
            },
            NodeType::Camera { fov, near, far } => {
                // FOV slider — drag_stopped commit.
                let mut fov_val = *fov;
                let fov_resp = ui.horizontal(|ui| {
                    ui.label("FOV:");
                    ui.add(egui::Slider::new(&mut fov_val, 1.0..=179.0).suffix(" deg"))
                }).inner;
                if fov_resp.dragged() && self.node_type_edit_start.is_none() {
                    self.node_type_edit_start = Some((selected_id, node.node_type.clone()));
                }
                if fov_resp.changed() && fov_val != *fov {
                    if let Some(n) = app.scene.find_node_mut(selected_id) {
                        if let NodeType::Camera { fov: ref mut f, .. } = &mut n.node_type {
                            *f = fov_val;
                        }
                        app.dirty = true;
                    }
                }
                if fov_resp.drag_stopped() {
                    self.commit_node_type_edit(app);
                }

                ui.add_space(4.0);

                // Near plane DragValue — drag_stopped commit.
                let mut near_val = *near;
                let near_resp = ui.horizontal(|ui| {
                    ui.label("Near Plane:");
                    ui.add(egui::DragValue::new(&mut near_val).speed(0.01).range(0.001..=10.0))
                }).inner;
                if near_resp.dragged() && self.node_type_edit_start.is_none() {
                    self.node_type_edit_start = Some((selected_id, node.node_type.clone()));
                }
                if near_resp.changed() && near_val != *near {
                    if let Some(n) = app.scene.find_node_mut(selected_id) {
                        if let NodeType::Camera { near: ref mut nr, .. } = &mut n.node_type {
                            *nr = near_val;
                        }
                        app.dirty = true;
                    }
                }
                if near_resp.drag_stopped() {
                    self.commit_node_type_edit(app);
                }

                ui.add_space(4.0);

                // Far plane DragValue — drag_stopped commit.
                let mut far_val = *far;
                let far_resp = ui.horizontal(|ui| {
                    ui.label("Far Plane:");
                    ui.add(egui::DragValue::new(&mut far_val).speed(10.0).range(100.0..=100000.0))
                }).inner;
                if far_resp.dragged() && self.node_type_edit_start.is_none() {
                    self.node_type_edit_start = Some((selected_id, node.node_type.clone()));
                }
                if far_resp.changed() && far_val != *far {
                    if let Some(n) = app.scene.find_node_mut(selected_id) {
                        if let NodeType::Camera { far: ref mut fr, .. } = &mut n.node_type {
                            *fr = far_val;
                        }
                        app.dirty = true;
                    }
                }
                if far_resp.drag_stopped() {
                    self.commit_node_type_edit(app);
                }
            },
        }
    }

    /// Commit a pending type-property edit as a single `SetNodeTypeCommand`.
    fn commit_node_type_edit(&mut self, app: &mut EditorApp) {
        self.color_idle_frames = 0;
        if let Some((id, old_type)) = self.node_type_edit_start.take() {
            if let Some(n) = app.scene.find_node(id) {
                let new_type = n.node_type.clone();
                if new_type != old_type {
                    let cmd = crate::commands::SetNodeTypeCommand::new(id, old_type, new_type);
                    let _ = app.command_history.execute(Box::new(cmd), &mut app.scene);
                }
            }
        }
    }
}

/// Render engine-level properties for the selected node.
fn render_engine_properties(ui: &mut egui::Ui, app: &mut EditorApp, selected_id: u64) {
    let links = app.engine_bridge.links_for_node(selected_id);
    if !links.is_empty() {
        ui.label(egui::RichText::new("Linked Engine Entities:").color(egui::Color32::from_rgb(180, 180, 190)));
        for link in links {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(format!("[{}] {}", link.engine_type, link.engine_id));
            });
        }
        ui.separator();
    }

    if ui.button("Link to Engine Entity").clicked() {
        let fake_id = format!("entity_{}", selected_id);
        app.engine_bridge.link_entity(selected_id, &fake_id, "Simulated");
    }

    if let Some(ref snap) = app.engine_bridge.snapshot {
        ui.separator();
        ui.label(egui::RichText::new("Engine World Snapshot:").color(egui::Color32::from_rgb(180, 180, 190)));
        ui.add_space(4.0);

        egui::Grid::new("engine_snapshot_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            ui.label("Sim Time:");
            ui.label(format!("{:.2}s", snap.sim_time));
            ui.end_row();

            ui.label("Tick:");
            ui.label(format!("{}", snap.tick_count));
            ui.end_row();

            ui.label("Temperature:");
            ui.label(format!("{:.1}C", snap.global_temperature));
            ui.end_row();

            ui.label("Radiation:");
            ui.label(format!("{:.4}", snap.global_radiation));
            ui.end_row();
        });

        ui.separator();
        ui.label(egui::RichText::new("Entity Counts:").color(egui::Color32::from_rgb(180, 180, 190)));
        let ec = &snap.entity_counts;
        egui::Grid::new("entity_counts_grid").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
            ui.label("Physics:"); ui.label(format!("{}", ec.physics_bodies)); ui.end_row();
            ui.label("Chemistry:"); ui.label(format!("{}", ec.chemistry_entities)); ui.end_row();
            ui.label("Ecosystems:"); ui.label(format!("{}", ec.ecosystems)); ui.end_row();
            ui.label("Particles:"); ui.label(format!("{}", ec.particles)); ui.end_row();
            ui.label("Meta:"); ui.label(format!("{}", ec.meta_entities)); ui.end_row();
            ui.label("NPCs:"); ui.label(format!("{}", ec.npcs)); ui.end_row();
            ui.label("Audio:"); ui.label(format!("{}", ec.audio_sources)); ui.end_row();
            ui.label("Weather:"); ui.label(format!("{}", ec.weather_systems)); ui.end_row();
        });

        ui.separator();
        ui.label(egui::RichText::new("World Bounds:").color(egui::Color32::from_rgb(180, 180, 190)));
        ui.label(format!(
            "({:.0}, {:.0}, {:.0}) .. ({:.0}, {:.0}, {:.0})",
            snap.world_bounds.min.x, snap.world_bounds.min.y, snap.world_bounds.min.z,
            snap.world_bounds.max.x, snap.world_bounds.max.y, snap.world_bounds.max.z,
        ));
    } else {
        ui.label(egui::RichText::new("(No engine snapshot yet)").color(egui::Color32::from_rgb(120, 120, 120)));
    }
}

