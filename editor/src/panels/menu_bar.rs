//! Menu bar panel: File, Edit, View, Window, and Help menus.

use crate::app::{EditorAction, EditorApp};
use crate::panels::EditorPanel;

pub struct MenuBarPanel;

impl EditorPanel for MenuBarPanel {
    fn name(&self) -> &str { "Menu Bar" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu.
                ui.menu_button("File", |ui| {
                    if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                        app.pending_action = Some(EditorAction::NewScene);
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Open...").shortcut_text("Ctrl+O")).clicked() {
                        app.pending_action = Some(EditorAction::OpenScene);
                        ui.close_menu();
                    }
                    // Open Recent submenu.
                    let recent_files: Vec<String> = app
                        .settings_panel
                        .as_ref()
                        .map(|s| s.recent_files.clone())
                        .unwrap_or_default();
                    if !recent_files.is_empty() {
                        ui.menu_button("Open Recent", |ui| {
                            for path in &recent_files {
                                let filename = std::path::Path::new(path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(path);
                                if ui.button(filename).clicked() {
                                    app.pending_action =
                                        Some(EditorAction::OpenSceneFromPath(path.clone()));
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                        app.pending_action = Some(EditorAction::SaveScene);
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Save As...").shortcut_text("Ctrl+Shift+S")).clicked() {
                        app.pending_action = Some(EditorAction::SaveSceneAs);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("Exit").shortcut_text("Alt+F4")).clicked() {
                        app.pending_action = Some(EditorAction::Exit);
                        ui.close_menu();
                    }
                });

                // Edit menu.
                ui.menu_button("Edit", |ui| {
                    let can_undo = app.command_history.can_undo();
                    let undo_label = if let Some(desc) = app.command_history.undo_description() {
                        format!("Undo {}", desc)
                    } else {
                        "Undo".to_string()
                    };
                    if ui.add_enabled(can_undo, egui::Button::new(undo_label).shortcut_text("Ctrl+Z")).clicked() {
                        app.pending_action = Some(EditorAction::Undo);
                        ui.close_menu();
                    }
                    let can_redo = app.command_history.can_redo();
                    let redo_label = if let Some(desc) = app.command_history.redo_description() {
                        format!("Redo {}", desc)
                    } else {
                        "Redo".to_string()
                    };
                    if ui.add_enabled(can_redo, egui::Button::new(redo_label).shortcut_text("Ctrl+Y")).clicked() {
                        app.pending_action = Some(EditorAction::Redo);
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_sel = app.selection.selected_id.is_some();
                    let has_clipboard = app.clipboard.is_some();
                    if ui.add_enabled(has_sel, egui::Button::new("Copy").shortcut_text("Ctrl+C")).clicked() {
                        app.pending_action = Some(EditorAction::CopySelected);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_clipboard, egui::Button::new("Paste").shortcut_text("Ctrl+V")).clicked() {
                        app.pending_action = Some(EditorAction::Paste);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Duplicate").shortcut_text("Ctrl+D")).clicked() {
                        app.pending_action = Some(EditorAction::DuplicateSelected);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Rename").shortcut_text("F2")).clicked() {
                        app.pending_action = Some(EditorAction::RenameSelected);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(has_sel, egui::Button::new("Delete").shortcut_text("Del")).clicked() {
                        app.pending_action = Some(EditorAction::DeleteSelected);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Focus Selection").shortcut_text("F")).clicked() {
                        app.pending_action = Some(EditorAction::FocusSelection);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Deselect").shortcut_text("Esc")).clicked() {
                        app.pending_action = Some(EditorAction::Deselect);
                        ui.close_menu();
                    }
                });

                // View menu.
                ui.menu_button("View", |ui| {
                    if ui.button("Focus Selection").clicked() {
                        app.pending_action = Some(EditorAction::FocusSelection);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Gizmo Mode").small().color(egui::Color32::from_rgb(150, 150, 160)));
                    if ui.add(egui::Button::new("Translate").shortcut_text("W")).clicked() {
                        app.pending_action = Some(EditorAction::GizmoTranslate);
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Rotate").shortcut_text("E")).clicked() {
                        app.pending_action = Some(EditorAction::GizmoRotate);
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Scale").shortcut_text("R")).clicked() {
                        app.pending_action = Some(EditorAction::GizmoScale);
                        ui.close_menu();
                    }
                    ui.separator();
                    let ab_label = if app.asset_browser.visible { "Asset Browser [on]" } else { "Asset Browser [off]" };
                    if ui.button(ab_label).clicked() {
                        app.asset_browser.visible = !app.asset_browser.visible;
                        ui.close_menu();
                    }
                });

                // Window menu.
                ui.menu_button("Window", |ui| {
                    ui.label(egui::RichText::new("Panels").small().color(egui::Color32::from_rgb(150, 150, 160)));
                    ui.separator();

                    let panels: Vec<(&str, &str)> = vec![
                        ("Console", "console_panel"),
                        ("Statistics", "stats_panel"),
                        ("Search", "search_panel"),
                        ("World Settings", "world_settings_panel"),
                        ("Layers", "layers_panel"),
                        ("Settings", "settings_panel"),
                        ("Material Editor", "material_editor_panel"),
                        ("Terrain Editor", "terrain_editor_panel"),
                        ("Animation Timeline", "animation_timeline_panel"),
                        ("Particle Editor", "particle_editor_panel"),
                        ("Physics Debug", "physics_debug_panel"),
                        ("Bookmarks", "bookmarks_panel"),
                        ("View Modes", "view_modes_panel"),
                        ("Measure Tool", "measure_tool_panel"),
                        ("Sequencer", "sequencer_panel"),
                        ("Behavior Tree", "behavior_tree_editor_panel"),
                        ("Shader Graph", "shader_graph_panel"),
                        ("Landscape Spline", "landscape_spline_panel"),
                        ("Foliage Editor", "foliage_editor_panel"),
                        ("Skeleton Editor", "skeleton_editor_panel"),
                        ("Anim Blueprint", "anim_blueprint_panel"),
                        ("Control Rig", "control_rig_panel"),
                        ("Variant Manager", "variant_manager_panel"),
                        ("Reference Viewer", "reference_viewer_panel"),
                        ("Visual Logger", "visual_logger_panel"),
                        ("Curve Editor", "curve_editor_panel"),
                        ("UV Editor", "uv_editor_panel"),
                        ("Node Editor", "node_editor_panel"),
                        ("Dialogue Tree", "dialogue_tree_panel"),
                        ("Quest Editor", "quest_editor_panel"),
                        ("Navmesh Editor", "navmesh_editor_panel"),
                        ("Audio Mixer", "audio_mixer_panel"),
                        ("Lighting", "lighting_panel"),
                        ("Post Process", "post_process_panel"),
                        ("Constraints", "constraints_panel"),
                        ("Modifiers", "modifiers_panel"),
                        ("Spreadsheet", "spreadsheet_panel"),
                        ("Geometry Nodes", "geometry_nodes_panel"),
                        ("Compositor", "compositor_panel"),
                    ];

                    for (name, field) in &panels {
                        let visible = get_visible(app, field);
                        let label = if visible { format!("[x] {}", name) } else { format!("[ ] {}", name) };
                        if ui.button(&label).clicked() {
                            toggle_visible(app, field);
                            ui.close_menu();
                        }
                    }
                });

                // Help menu.
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        if let Some(ref mut p) = app.about_panel { p.visible = true; }
                        ui.close_menu();
                    }
                    if ui.button("Console Commands").clicked() {
                        if let Some(ref mut p) = app.console_panel { p.visible = true; }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Keyboard Shortcuts").clicked() {
                        if let Some(ref mut p) = app.console_panel {
                            p.visible = true;
                            p.log(crate::panels::console::LogLevel::Info, "=== Keyboard Shortcuts ===", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  W/E/R    - Gizmo Translate/Rotate/Scale", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  F        - Focus Selection", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Esc      - Deselect", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Ctrl+N   - New Scene", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Ctrl+O   - Open Scene", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Ctrl+S   - Save Scene", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Ctrl+Z   - Undo", app.frame_counter);
                            p.log(crate::panels::console::LogLevel::Info, "  Ctrl+Y   - Redo", app.frame_counter);
                        }
                        ui.close_menu();
                    }
                });

                // Right side: scene info and dirty indicator.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Dirty indicator.
                    if app.dirty {
                        ui.label(egui::RichText::new("*").color(egui::Color32::from_rgb(255, 200, 80)).strong());
                    }
                    // Scene name.
                    let scene_name = app.scene_path.as_ref()
                        .and_then(|p| std::path::Path::new(p).file_stem())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Untitled".to_string());
                    ui.label(egui::RichText::new(format!("Scene: {}", scene_name)).small().color(egui::Color32::from_rgb(180, 200, 220)));

                    ui.separator();

                    // Node count.
                    ui.label(egui::RichText::new(format!("Nodes: {}", app.scene.nodes.len())).small().color(egui::Color32::from_rgb(150, 180, 150)));
                });
            });
        });
    }
}

fn get_visible(app: &EditorApp, field: &str) -> bool {
    match field {
        "console_panel" => app.console_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "stats_panel" => app.stats_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "search_panel" => app.search_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "world_settings_panel" => app.world_settings_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "layers_panel" => app.layers_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "settings_panel" => app.settings_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "material_editor_panel" => app.material_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "terrain_editor_panel" => app.terrain_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "animation_timeline_panel" => app.animation_timeline_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "particle_editor_panel" => app.particle_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "physics_debug_panel" => app.physics_debug_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "bookmarks_panel" => app.bookmarks_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "view_modes_panel" => app.view_modes_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "measure_tool_panel" => app.measure_tool_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "sequencer_panel" => app.sequencer_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "behavior_tree_editor_panel" => app.behavior_tree_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "shader_graph_panel" => app.shader_graph_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "landscape_spline_panel" => app.landscape_spline_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "foliage_editor_panel" => app.foliage_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "skeleton_editor_panel" => app.skeleton_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "anim_blueprint_panel" => app.anim_blueprint_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "control_rig_panel" => app.control_rig_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "variant_manager_panel" => app.variant_manager_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "reference_viewer_panel" => app.reference_viewer_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "visual_logger_panel" => app.visual_logger_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "curve_editor_panel" => app.curve_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "uv_editor_panel" => app.uv_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "node_editor_panel" => app.node_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "dialogue_tree_panel" => app.dialogue_tree_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "quest_editor_panel" => app.quest_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "navmesh_editor_panel" => app.navmesh_editor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "audio_mixer_panel" => app.audio_mixer_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "lighting_panel" => app.lighting_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "post_process_panel" => app.post_process_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "constraints_panel" => app.constraints_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "modifiers_panel" => app.modifiers_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "spreadsheet_panel" => app.spreadsheet_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "geometry_nodes_panel" => app.geometry_nodes_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        "compositor_panel" => app.compositor_panel.as_ref().map(|p| p.visible).unwrap_or(false),
        _ => false,
    }
}

fn toggle_visible(app: &mut EditorApp, field: &str) {
    match field {
        "console_panel" => { if let Some(ref mut p) = app.console_panel { p.visible = !p.visible; } }
        "stats_panel" => { if let Some(ref mut p) = app.stats_panel { p.visible = !p.visible; } }
        "search_panel" => { if let Some(ref mut p) = app.search_panel { p.visible = !p.visible; } }
        "world_settings_panel" => { if let Some(ref mut p) = app.world_settings_panel { p.visible = !p.visible; } }
        "layers_panel" => { if let Some(ref mut p) = app.layers_panel { p.visible = !p.visible; } }
        "settings_panel" => { if let Some(ref mut p) = app.settings_panel { p.visible = !p.visible; } }
        "material_editor_panel" => { if let Some(ref mut p) = app.material_editor_panel { p.visible = !p.visible; } }
        "terrain_editor_panel" => { if let Some(ref mut p) = app.terrain_editor_panel { p.visible = !p.visible; } }
        "animation_timeline_panel" => { if let Some(ref mut p) = app.animation_timeline_panel { p.visible = !p.visible; } }
        "particle_editor_panel" => { if let Some(ref mut p) = app.particle_editor_panel { p.visible = !p.visible; } }
        "physics_debug_panel" => { if let Some(ref mut p) = app.physics_debug_panel { p.visible = !p.visible; } }
        "bookmarks_panel" => { if let Some(ref mut p) = app.bookmarks_panel { p.visible = !p.visible; } }
        "view_modes_panel" => { if let Some(ref mut p) = app.view_modes_panel { p.visible = !p.visible; } }
        "measure_tool_panel" => { if let Some(ref mut p) = app.measure_tool_panel { p.visible = !p.visible; } }
        "sequencer_panel" => { if let Some(ref mut p) = app.sequencer_panel { p.visible = !p.visible; } }
        "behavior_tree_editor_panel" => { if let Some(ref mut p) = app.behavior_tree_editor_panel { p.visible = !p.visible; } }
        "shader_graph_panel" => { if let Some(ref mut p) = app.shader_graph_panel { p.visible = !p.visible; } }
        "landscape_spline_panel" => { if let Some(ref mut p) = app.landscape_spline_panel { p.visible = !p.visible; } }
        "foliage_editor_panel" => { if let Some(ref mut p) = app.foliage_editor_panel { p.visible = !p.visible; } }
        "skeleton_editor_panel" => { if let Some(ref mut p) = app.skeleton_editor_panel { p.visible = !p.visible; } }
        "anim_blueprint_panel" => { if let Some(ref mut p) = app.anim_blueprint_panel { p.visible = !p.visible; } }
        "control_rig_panel" => { if let Some(ref mut p) = app.control_rig_panel { p.visible = !p.visible; } }
        "variant_manager_panel" => { if let Some(ref mut p) = app.variant_manager_panel { p.visible = !p.visible; } }
        "reference_viewer_panel" => { if let Some(ref mut p) = app.reference_viewer_panel { p.visible = !p.visible; } }
        "visual_logger_panel" => { if let Some(ref mut p) = app.visual_logger_panel { p.visible = !p.visible; } }
        "curve_editor_panel" => { if let Some(ref mut p) = app.curve_editor_panel { p.visible = !p.visible; } }
        "uv_editor_panel" => { if let Some(ref mut p) = app.uv_editor_panel { p.visible = !p.visible; } }
        "node_editor_panel" => { if let Some(ref mut p) = app.node_editor_panel { p.visible = !p.visible; } }
        "dialogue_tree_panel" => { if let Some(ref mut p) = app.dialogue_tree_panel { p.visible = !p.visible; } }
        "quest_editor_panel" => { if let Some(ref mut p) = app.quest_editor_panel { p.visible = !p.visible; } }
        "navmesh_editor_panel" => { if let Some(ref mut p) = app.navmesh_editor_panel { p.visible = !p.visible; } }
        "audio_mixer_panel" => { if let Some(ref mut p) = app.audio_mixer_panel { p.visible = !p.visible; } }
        "lighting_panel" => { if let Some(ref mut p) = app.lighting_panel { p.visible = !p.visible; } }
        "post_process_panel" => { if let Some(ref mut p) = app.post_process_panel { p.visible = !p.visible; } }
        "constraints_panel" => { if let Some(ref mut p) = app.constraints_panel { p.visible = !p.visible; } }
        "modifiers_panel" => { if let Some(ref mut p) = app.modifiers_panel { p.visible = !p.visible; } }
        "spreadsheet_panel" => { if let Some(ref mut p) = app.spreadsheet_panel { p.visible = !p.visible; } }
        "geometry_nodes_panel" => { if let Some(ref mut p) = app.geometry_nodes_panel { p.visible = !p.visible; } }
        "compositor_panel" => { if let Some(ref mut p) = app.compositor_panel { p.visible = !p.visible; } }
        _ => {}
    }
}
