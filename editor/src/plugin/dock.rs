//! Dockable panel system: panels contributed by plugins that can be docked
//! into the editor's main window layout.

use crate::app::EditorApp;
use egui::Context;

/// Context for dock panels (currently a thin wrapper; may grow).
pub struct DockPanelContext<'a> {
    pub app: &'a mut EditorApp,
    pub ctx: &'a Context,
}

/// Trait for dockable panels. Plugins implement this to add custom panels
/// that appear in the editor's dock area and the Window menu.
pub trait DockPanel: Send + Sync {
    /// Unique panel id (e.g., "plugin:test_panel").
    fn id(&self) -> &str;

    /// Human-readable title shown in the panel tab.
    fn title(&self) -> &str;

    /// Optional icon glyph.
    fn icon(&self) -> Option<&str> {
        None
    }

    /// Whether the panel is initially visible.
    fn default_visible(&self) -> bool {
        false
    }

    /// Whether the panel can be closed by the user.
    fn closeable(&self) -> bool {
        true
    }

    /// Render the panel contents.
    fn render(&mut self, ctx: &DockPanelContext);

    /// Called when the panel is opened.
    fn on_open(&mut self) {}

    /// Called when the panel is closed.
    fn on_close(&mut self) {}

    /// Per-frame update (called even when panel is not visible).
    fn update(&mut self, _app: &mut EditorApp) {}
}
