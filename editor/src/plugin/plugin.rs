//! EditorPlugin trait: the extension point for all editor plugins.
//!
//! A plugin owns its UI state (panels, tools), can contribute menu items,
//! react to viewport input, and run per-frame logic. Plugins are registered
//! once at startup via `PluginRegistry::register` and live for the lifetime
//! of the editor.

use crate::app::EditorApp;
use crate::plugin::tool::ToolContext;
use egui::Context;

/// A menu item contributed by a plugin.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: MenuAction,
    pub enabled: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>, action: MenuAction) -> Self {
        MenuItem { label: label.into(), shortcut: None, action, enabled: true }
    }

    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }

    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
}

/// Actions a plugin menu item can trigger.
#[derive(Debug, Clone)]
pub enum MenuAction {
    /// Custom action identified by a string id; the plugin receives it in `on_menu_action`.
    Custom(String),
    /// Toggle a panel's visibility (by panel id string).
    TogglePanel(String),
    /// Activate a tool (by tool id string).
    ActivateTool(String),
}

/// Kind of viewport input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventKind {
    PointerDown,
    PointerUp,
    PointerMove,
    Drag,
    Scroll,
    KeyDown,
    KeyUp,
}

/// A viewport input event handed to plugins.
#[derive(Debug, Clone)]
pub struct ViewportInputEvent {
    pub kind: InputEventKind,
    pub pointer_pos: Option<egui::Pos2>,
    pub pointer_delta: egui::Vec2,
    pub scroll_delta: egui::Vec2,
    pub key: Option<egui::Key>,
    pub modifiers: egui::Modifiers,
    pub button: Option<egui::PointerButton>,
}

/// Context for viewport hooks (read-only view of editor state).
pub struct ViewportContext<'a> {
    pub app: &'a EditorApp,
    pub viewport_rect: egui::Rect,
}

/// Context for dock panel registration.
pub struct DockContext<'a> {
    pub registry: &'a mut Vec<Box<dyn crate::plugin::dock::DockPanel>>,
}

/// The editor plugin trait.
///
/// Implementors are stored as `Box<dyn EditorPlugin>` in the `PluginRegistry`.
/// All methods have default no-op implementations so plugins only override
/// what they need.
pub trait EditorPlugin: Send + Sync {
    /// Human-readable plugin name (shown in Plugin Browser).
    fn name(&self) -> &str;

    /// Plugin version string.
    fn version(&self) -> &str {
        "0.1.0"
    }

    /// Called once after registration, before the first frame.
    fn on_register(&mut self, _ctx: &PluginContext) {}

    /// Register dockable panels (called once during startup).
    fn register_panels(&mut self, _dock: &mut DockContext) {}

    /// Register editor tools (called once during startup).
    fn register_tools(&mut self, _tools: &mut Vec<Box<dyn crate::plugin::tool::EditorTool>>) {}

    /// Contribute menu items to the menu bar (called each frame the menu is open).
    fn menu_items(&self, _menu: &mut Vec<MenuItem>) {}

    /// Handle a custom menu action.
    fn on_menu_action(&mut self, _action: &str, _app: &mut EditorApp) {}

    /// Per-frame update (called before any UI is rendered).
    fn update(&mut self, _app: &mut EditorApp, _ctx: &Context) {}

    /// Per-frame viewport hook (called inside the viewport panel).
    fn on_viewport_event(&mut self, _event: &ViewportInputEvent, _ctx: &ViewportContext, _app: &mut EditorApp) {}

    /// Called when the active tool changes (new_tool_id is None if no tool active).
    fn on_tool_changed(&mut self, _new_tool_id: Option<&str>, _app: &mut EditorApp) {}

    /// Called when the selection changes.
    fn on_selection_changed(&mut self, _app: &mut EditorApp) {}

    /// Called when the scene is loaded or reset.
    fn on_scene_changed(&mut self, _app: &mut EditorApp) {}

    /// Render any custom overlay UI inside the viewport (after the 3D scene).
    fn render_viewport_overlay(&mut self, _ui: &mut egui::Ui, _app: &mut EditorApp) {}

    /// Hook called when a tool is active and the user clicks in the viewport.
    fn on_tool_apply(&mut self, _tool_ctx: &mut ToolContext, _app: &mut EditorApp) {}
}

/// Context handed to `on_register`. Provides access to plugin metadata storage.
pub struct PluginContext<'a> {
    pub app: &'a mut EditorApp,
}
