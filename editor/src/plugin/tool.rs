//! EditorTool trait: interactive viewport tools (brush, place, select, etc.).

use crate::app::EditorApp;
use crate::scene::Scene;
use crate::selection::Selection;

/// Context passed to tools. Provides mutable access to scene and selection
/// plus read-only viewport info.
pub struct ToolContext<'a> {
    pub scene: &'a mut Scene,
    pub selection: &'a mut Selection,
    /// Screen-space pointer position (None if not in viewport).
    pub pointer_pos: Option<egui::Pos2>,
    /// World-space ray origin/direction from camera (None if not computed).
    pub ray_origin: Option<glam::Vec3>,
    pub ray_dir: Option<glam::Vec3>,
    /// Viewport rectangle in screen coordinates.
    pub viewport_rect: egui::Rect,
    /// Current modifiers.
    pub modifiers: egui::Modifiers,
}

impl<'a> ToolContext<'a> {
    /// Convenience: compute world position at ray hit with ground plane y=0.
    pub fn ground_hit(&self) -> Option<glam::Vec3> {
        let (origin, dir) = (self.ray_origin?, self.ray_dir?);
        if dir.y.abs() < 1e-6 {
            return None;
        }
        let t = -origin.y / dir.y;
        if t < 0.0 {
            return None;
        }
        Some(origin + dir * t)
    }
}

/// Editor tool trait. Tools are activated via the toolbar or plugin menu.
pub trait EditorTool: Send + Sync {
    /// Unique tool id (e.g., "brush", "place_node").
    fn id(&self) -> &str;

    /// Human-readable name shown in the toolbar.
    fn name(&self) -> &str;

    /// Icon (optional, e.g., emoji or icon font glyph).
    fn icon(&self) -> Option<&str> {
        None
    }

    /// Tooltip shown on hover.
    fn tooltip(&self) -> &str {
        self.name()
    }

    /// Called when the tool is activated.
    fn on_activate(&mut self, _app: &mut EditorApp) {}

    /// Called when the tool is deactivated.
    fn on_deactivate(&mut self, _app: &mut EditorApp) {}

    /// Called each frame the tool is active (before viewport overlay).
    fn update(&mut self, _app: &mut EditorApp) {}

    /// Called when the user clicks/applies the tool in the viewport.
    fn apply(&mut self, _ctx: &mut ToolContext, _app: &mut EditorApp) {}

    /// Render tool-specific overlay UI inside the viewport.
    fn render_overlay(&mut self, _ui: &mut egui::Ui, _app: &mut EditorApp) {}

    /// Render tool options panel (shown in a side panel when tool active).
    fn render_options(&mut self, _ui: &mut egui::Ui, _app: &mut EditorApp) {}

    /// Whether the tool consumes the viewport input (blocks camera control).
    fn consumes_input(&self) -> bool {
        false
    }
}
