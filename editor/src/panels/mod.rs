//! Panel system for the editor UI.

use crate::app::EditorApp;

pub trait EditorPanel {
    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp);
    fn name(&self) -> &str;
    fn visible(&self) -> bool { true }
}

pub mod about;
pub mod animation_timeline;
pub mod asset_browser;
pub mod bookmarks;
pub mod console;
pub mod hierarchy;
pub mod inspector;
pub mod layers;
pub mod material_editor;
pub mod mcp_debug;
pub mod measure_tool;
pub mod menu_bar;
pub mod particle_editor;
pub mod physics_debug;
pub mod search;
pub mod settings_panel;
pub mod stats;
pub mod status_bar;
pub mod terrain_editor;
pub mod view_modes;
pub mod viewport;
pub mod world_settings;
pub mod anim_blueprint;
pub mod audio_mixer;
pub mod behavior_tree_editor;
pub mod compositor;
pub mod constraints_panel;
pub mod control_rig;
pub mod curve_editor;
pub mod dialogue_tree;
pub mod foliage_editor;
pub mod geometry_nodes;
pub mod landscape_spline;
pub mod lighting_panel;
pub mod modifiers_panel;
pub mod navmesh_editor;
pub mod node_editor;
pub mod post_process;
pub mod quest_editor;
pub mod reference_viewer;
pub mod sequencer;
pub mod shader_graph;
pub mod skeleton_editor;
pub mod spreadsheet;
pub mod uv_editor;
pub mod variant_manager;
pub mod visual_logger;

pub fn render_all_panels(ctx: &egui::Context, app: &mut EditorApp) {
    let mut status = status_bar::StatusBarPanel;
    status.render(ctx, app);

    { let mut hp = std::mem::take(&mut app.hierarchy_panel); hp.render(ctx, app); app.hierarchy_panel = hp; }

    { let mut ip = std::mem::take(&mut app.inspector_panel); ip.render(ctx, app); app.inspector_panel = ip; }

    { let mut vp = std::mem::take(&mut app.viewport_panel); vp.render(ctx, app); app.viewport_panel = vp; }

    { let mut ab = std::mem::take(&mut app.asset_browser); asset_browser::render_asset_browser_panel(ctx, app, &mut ab); app.asset_browser = ab; }

    let mut menu = menu_bar::MenuBarPanel;
    menu.render(ctx, app);

    // Floating panels.
    if let Some(mut p) = app.console_panel.take() { p.render(ctx, app); app.console_panel = Some(p); }
    if let Some(mut p) = app.stats_panel.take() { p.render(ctx, app); app.stats_panel = Some(p); }
    if let Some(mut p) = app.about_panel.take() { p.render(ctx, app); app.about_panel = Some(p); }
    if let Some(mut p) = app.search_panel.take() { p.render(ctx, app); app.search_panel = Some(p); }
    if let Some(mut p) = app.world_settings_panel.take() { p.render(ctx, app); app.world_settings_panel = Some(p); }
    if let Some(mut p) = app.layers_panel.take() { p.render(ctx, app); app.layers_panel = Some(p); }
    if let Some(mut p) = app.settings_panel.take() { p.render(ctx, app); app.settings_panel = Some(p); }
    if let Some(mut p) = app.material_editor_panel.take() { p.render(ctx, app); app.material_editor_panel = Some(p); }
    if let Some(mut p) = app.terrain_editor_panel.take() { p.render(ctx, app); app.terrain_editor_panel = Some(p); }
    if let Some(mut p) = app.animation_timeline_panel.take() { p.render(ctx, app); app.animation_timeline_panel = Some(p); }
    if let Some(mut p) = app.particle_editor_panel.take() { p.render(ctx, app); app.particle_editor_panel = Some(p); }
    if let Some(mut p) = app.physics_debug_panel.take() { p.render(ctx, app); app.physics_debug_panel = Some(p); }
    if let Some(mut p) = app.bookmarks_panel.take() { p.render(ctx, app); app.bookmarks_panel = Some(p); }
    if let Some(mut p) = app.view_modes_panel.take() { p.render(ctx, app); app.view_modes_panel = Some(p); }
    if let Some(mut p) = app.measure_tool_panel.take() { p.render(ctx, app); app.measure_tool_panel = Some(p); }
    if let Some(mut p) = app.sequencer_panel.take() { p.render(ctx, app); app.sequencer_panel = Some(p); }
    if let Some(mut p) = app.behavior_tree_editor_panel.take() { p.render(ctx, app); app.behavior_tree_editor_panel = Some(p); }
    if let Some(mut p) = app.shader_graph_panel.take() { p.render(ctx, app); app.shader_graph_panel = Some(p); }
    if let Some(mut p) = app.landscape_spline_panel.take() { p.render(ctx, app); app.landscape_spline_panel = Some(p); }
    if let Some(mut p) = app.foliage_editor_panel.take() { p.render(ctx, app); app.foliage_editor_panel = Some(p); }
    if let Some(mut p) = app.skeleton_editor_panel.take() { p.render(ctx, app); app.skeleton_editor_panel = Some(p); }
    if let Some(mut p) = app.anim_blueprint_panel.take() { p.render(ctx, app); app.anim_blueprint_panel = Some(p); }
    if let Some(mut p) = app.control_rig_panel.take() { p.render(ctx, app); app.control_rig_panel = Some(p); }
    if let Some(mut p) = app.variant_manager_panel.take() { p.render(ctx, app); app.variant_manager_panel = Some(p); }
    if let Some(mut p) = app.reference_viewer_panel.take() { p.render(ctx, app); app.reference_viewer_panel = Some(p); }
    if let Some(mut p) = app.visual_logger_panel.take() { p.render(ctx, app); app.visual_logger_panel = Some(p); }
    if let Some(mut p) = app.curve_editor_panel.take() { p.render(ctx, app); app.curve_editor_panel = Some(p); }
    if let Some(mut p) = app.uv_editor_panel.take() { p.render(ctx, app); app.uv_editor_panel = Some(p); }
    if let Some(mut p) = app.node_editor_panel.take() { p.render(ctx, app); app.node_editor_panel = Some(p); }
    if let Some(mut p) = app.dialogue_tree_panel.take() { p.render(ctx, app); app.dialogue_tree_panel = Some(p); }
    if let Some(mut p) = app.quest_editor_panel.take() { p.render(ctx, app); app.quest_editor_panel = Some(p); }
    if let Some(mut p) = app.navmesh_editor_panel.take() { p.render(ctx, app); app.navmesh_editor_panel = Some(p); }
    if let Some(mut p) = app.audio_mixer_panel.take() { p.render(ctx, app); app.audio_mixer_panel = Some(p); }
    if let Some(mut p) = app.lighting_panel.take() { p.render(ctx, app); app.lighting_panel = Some(p); }
    if let Some(mut p) = app.post_process_panel.take() { p.render(ctx, app); app.post_process_panel = Some(p); }
    if let Some(mut p) = app.constraints_panel.take() { p.render(ctx, app); app.constraints_panel = Some(p); }
    if let Some(mut p) = app.modifiers_panel.take() { p.render(ctx, app); app.modifiers_panel = Some(p); }
    if let Some(mut p) = app.spreadsheet_panel.take() { p.render(ctx, app); app.spreadsheet_panel = Some(p); }
    if let Some(mut p) = app.geometry_nodes_panel.take() { p.render(ctx, app); app.geometry_nodes_panel = Some(p); }
    if let Some(mut p) = app.compositor_panel.take() { p.render(ctx, app); app.compositor_panel = Some(p); }
    if let Some(mut p) = app.mcp_debug_panel.take() { p.render(ctx, app); app.mcp_debug_panel = Some(p); }
}

