//! Plugin registry: owns all registered plugins and dispatches lifecycle hooks.

use crate::plugin::dock::DockPanel;
use crate::plugin::plugin::{
    DockContext, EditorPlugin, MenuItem, PluginContext, ViewportContext, ViewportInputEvent,
};
use crate::plugin::tool::EditorTool;
use egui::Context;
use std::collections::HashMap;

/// Optional gizmo rendering hook for plugins that draw custom gizmos.
pub trait GizmoRenderer: Send + Sync {
    fn render_gizmos(&self, ui: &mut egui::Ui, viewport_rect: egui::Rect);
}

/// Registry of all loaded plugins, their contributed panels/tools/menus.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn EditorPlugin>>,
    dock_panels: Vec<Box<dyn DockPanel>>,
    tools: Vec<Box<dyn EditorTool>>,
    active_tool_id: Option<String>,
    menu_items_cache: Vec<MenuItem>,
    gizmo_renderers: HashMap<String, Box<dyn GizmoRenderer>>,
    registered: bool,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry {
            plugins: Vec::new(),
            dock_panels: Vec::new(),
            tools: Vec::new(),
            active_tool_id: None,
            menu_items_cache: Vec::new(),
            gizmo_renderers: HashMap::new(),
            registered: false,
        }
    }

    /// Add a plugin. Must be called before `finish_registration`.
    pub fn register(&mut self, plugin: Box<dyn EditorPlugin>) {
        if self.registered {
            log::warn!("Cannot register plugin after finish_registration");
            return;
        }
        log::info!("Registered plugin: {}", plugin.name());
        self.plugins.push(plugin);
    }

    /// Register a gizmo renderer.
    pub fn register_gizmo_renderer(&mut self, name: String, renderer: Box<dyn GizmoRenderer>) {
        self.gizmo_renderers.insert(name, renderer);
    }

    /// Finalize registration: call on_register on each plugin, collect panels/tools.
    pub fn finish_registration(&mut self, app: &mut crate::app::EditorApp) {
        if self.registered {
            return;
        }

        // Two-phase: first on_register, then collect panels/tools.
        // We use a temp list because we cannot borrow self.plugins mutably
        // while also passing &mut PluginContext (which borrows app).
        // Instead, iterate by index.
        let plugin_count = self.plugins.len();
        for i in 0..plugin_count {
            let ctx = PluginContext { app };
            self.plugins[i].on_register(&ctx);
        }

        // Collect dock panels.
        let plugin_count2 = self.plugins.len();
        for i in 0..plugin_count2 {
            let mut dock = DockContext { registry: &mut self.dock_panels };
            self.plugins[i].register_panels(&mut dock);
        }

        // Collect tools.
        let plugin_count3 = self.plugins.len();
        for i in 0..plugin_count3 {
            self.plugins[i].register_tools(&mut self.tools);
        }

        self.registered = true;
        log::info!(
            "Plugin registration complete: {} plugins, {} panels, {} tools",
            self.plugins.len(),
            self.dock_panels.len(),
            self.tools.len()
        );
    }

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// List plugin names (for Plugin Browser).
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }

    /// All contributed menu items (refreshed each call).
    pub fn collect_menu_items(&mut self) -> Vec<MenuItem> {
        self.menu_items_cache.clear();
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].menu_items(&mut self.menu_items_cache);
        }
        self.menu_items_cache.clone()
    }

    /// Dispatch a custom menu action.
    pub fn dispatch_menu_action(&mut self, action: &str, app: &mut crate::app::EditorApp) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].on_menu_action(action, app);
        }
    }

    /// Per-frame update for all plugins.
    pub fn update(&mut self, app: &mut crate::app::EditorApp, ctx: &Context) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].update(app, ctx);
        }
    }

    /// Dispatch a viewport input event to all plugins.
    pub fn dispatch_viewport_event(
        &mut self,
        event: &ViewportInputEvent,
        viewport_ctx: &ViewportContext,
        app: &mut crate::app::EditorApp,
    ) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].on_viewport_event(event, viewport_ctx, app);
        }
    }

    /// Notify all plugins of a selection change.
    pub fn notify_selection_changed(&mut self, app: &mut crate::app::EditorApp) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].on_selection_changed(app);
        }
    }

    /// Notify all plugins of a scene change.
    pub fn notify_scene_changed(&mut self, app: &mut crate::app::EditorApp) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].on_scene_changed(app);
        }
    }

    /// Render viewport overlay for all plugins.
    pub fn render_viewport_overlays(&mut self, ui: &mut egui::Ui, app: &mut crate::app::EditorApp) {
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].render_viewport_overlay(ui, app);
        }
    }

    /// Set the active tool by id. Triggers `on_tool_changed` on all plugins.
    pub fn set_active_tool(&mut self, tool_id: Option<String>, app: &mut crate::app::EditorApp) {
        if self.active_tool_id == tool_id {
            return;
        }
        self.active_tool_id = tool_id.clone();
        let count = self.plugins.len();
        for i in 0..count {
            self.plugins[i].on_tool_changed(tool_id.as_deref(), app);
        }
    }

    /// Get the currently active tool id.
    pub fn active_tool_id(&self) -> Option<&str> {
        self.active_tool_id.as_deref()
    }

    /// Get the list of registered tools (read-only).
    pub fn tools(&self) -> &[Box<dyn EditorTool>] {
        &self.tools
    }

    /// Get the list of registered dock panels (read-only).
    pub fn dock_panels(&self) -> &[Box<dyn DockPanel>] {
        &self.dock_panels
    }

    /// Get mutable access to dock panels (for rendering).
    pub fn dock_panels_mut(&mut self) -> &mut [Box<dyn DockPanel>] {
        &mut self.dock_panels
    }

    /// Render all gizmo renderers.
    pub fn render_gizmos(&self, ui: &mut egui::Ui, viewport_rect: egui::Rect) {
        for renderer in self.gizmo_renderers.values() {
            renderer.render_gizmos(ui, viewport_rect);
        }
    }

    /// Apply the active tool (called when user clicks in viewport with a tool active).
    pub fn apply_active_tool(
        &mut self,
        tool_ctx: &mut crate::plugin::tool::ToolContext,
        app: &mut crate::app::EditorApp,
    ) {
        let active = self.active_tool_id.clone();
        if let Some(id) = active {
            for tool in &mut self.tools {
                if tool.id() == id {
                    tool.apply(tool_ctx, app);
                    break;
                }
            }
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
