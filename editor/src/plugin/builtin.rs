//! Built-in editor plugins shipped with the editor itself.
//!
//! These plugins demonstrate the plugin API and provide baseline features
//! (scene statistics, MCP status indicator). They are registered during
//! `EditorApp::new()` so every editor instance has them available.

use crate::app::EditorApp;
use crate::plugin::plugin::{EditorPlugin, MenuItem, MenuAction, PluginContext};
use crate::plugin::plugin::DockContext;
use crate::plugin::tool::EditorTool;
use crate::plugin::tools::{TranslateTool, RotateTool, ScaleTool, MeshPlacerTool};
use egui::Context;

/// A minimal built-in plugin that logs scene statistics each second and
/// contributes a "Scene > Statistics" menu item.
///
/// Purpose: verify the plugin lifecycle (on_register → update → menu_items)
/// works end-to-end and give the user visible feedback that the plugin
/// system is alive.
pub struct SceneStatsPlugin {
    /// Frame counter modulo 60 — used to throttle the per-second log.
    frame_tick: u64,
    /// Last computed node count (for change detection).
    last_node_count: usize,
}

impl SceneStatsPlugin {
    pub fn new() -> Self {
        SceneStatsPlugin { frame_tick: 0, last_node_count: usize::MAX }
    }

    fn compute_stats(app: &EditorApp) -> SceneStats {
        let mut stats = SceneStats::default();
        for node in &app.scene.nodes {
            stats.total_nodes += 1;
            match &node.node_type {
                crate::scene::NodeType::Empty => stats.empty_nodes += 1,
                crate::scene::NodeType::Mesh { .. } => stats.mesh_nodes += 1,
                crate::scene::NodeType::Light { .. } => stats.light_nodes += 1,
                crate::scene::NodeType::Camera { .. } => stats.camera_nodes += 1,
            }
        }
        stats
    }
}

impl Default for SceneStatsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone)]
struct SceneStats {
    total_nodes: usize,
    empty_nodes: usize,
    mesh_nodes: usize,
    light_nodes: usize,
    camera_nodes: usize,
}

impl EditorPlugin for SceneStatsPlugin {
    fn name(&self) -> &str {
        "scene-stats"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn on_register(&mut self, _ctx: &PluginContext) {
        log::info!("[plugin:scene-stats] registered");
    }

    fn register_panels(&mut self, _dock: &mut DockContext) {}

    fn register_tools(&mut self, _tools: &mut Vec<Box<dyn EditorTool>>) {}

    fn menu_items(&self, menu: &mut Vec<MenuItem>) {
        menu.push(MenuItem::new("Scene Statistics", MenuAction::Custom("scene_stats".into())));
    }

    fn on_menu_action(&mut self, action: &str, app: &mut EditorApp) {
        if action == "scene_stats" {
            let stats = Self::compute_stats(app);
            log::info!(
                "[scene-stats] total={} empty={} mesh={} light={} camera={}",
                stats.total_nodes, stats.empty_nodes, stats.mesh_nodes,
                stats.light_nodes, stats.camera_nodes
            );
        }
    }

    fn update(&mut self, app: &mut EditorApp, _ctx: &Context) {
        self.frame_tick = self.frame_tick.wrapping_add(1);
        if self.frame_tick % 60 != 0 {
            return;
        }
        let stats = Self::compute_stats(app);
        if stats.total_nodes != self.last_node_count {
            log::info!(
                "[scene-stats] tick: total={} mesh={} light={} camera={}",
                stats.total_nodes, stats.mesh_nodes, stats.light_nodes, stats.camera_nodes
            );
            self.last_node_count = stats.total_nodes;
        }
    }

    fn on_scene_changed(&mut self, _app: &mut EditorApp) {
        log::info!("[scene-stats] scene changed");
        self.last_node_count = usize::MAX; // force refresh on next update
    }
}

/// Plugin that surfaces MCP server status and connection state.
pub struct McpStatusPlugin {
    /// Last reported connected state (for change logging).
    last_connected: bool,
}

impl McpStatusPlugin {
    pub fn new() -> Self {
        McpStatusPlugin { last_connected: false }
    }
}

impl Default for McpStatusPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorPlugin for McpStatusPlugin {
    fn name(&self) -> &str {
        "mcp-status"
    }

    fn on_register(&mut self, _ctx: &PluginContext) {
        log::info!("[plugin:mcp-status] registered");
    }

    fn update(&mut self, app: &mut EditorApp, _ctx: &Context) {
        let connected = app.mcp_server.is_connected();
        if connected != self.last_connected {
            log::info!("[mcp-status] connected={}", connected);
            self.last_connected = connected;
        }
    }
}

/// Register all built-in plugins with the given registry.
///
/// Called from `EditorApp::new()` after the registry is created.
pub struct BuiltinToolsPlugin;

impl Default for BuiltinToolsPlugin {
    fn default() -> Self { Self }
}

impl EditorPlugin for BuiltinToolsPlugin {
    fn name(&self) -> &str { "builtin-tools" }

    fn on_register(&mut self, _ctx: &PluginContext) {
        log::info!("[plugin:builtin-tools] registered");
    }

    fn register_tools(&mut self, tools: &mut Vec<Box<dyn EditorTool>>) {
        tools.push(Box::new(TranslateTool::new()));
        tools.push(Box::new(RotateTool::new()));
        tools.push(Box::new(ScaleTool::new()));
        tools.push(Box::new(MeshPlacerTool::new()));
    }
}

/// Register all built-in plugins with the given registry.
///
/// Called from EditorApp::new() after the registry is created.
pub fn register_all(registry: &mut crate::plugin::registry::PluginRegistry) {
    registry.register(Box::new(SceneStatsPlugin::new()));
    registry.register(Box::new(McpStatusPlugin::new()));
    registry.register(Box::new(BuiltinToolsPlugin));
}
