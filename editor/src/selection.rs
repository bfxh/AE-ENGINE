//! Selection system for the editor.
//!
//! Manages the currently selected node(s) and provides ray-picking utilities.

use glam::Vec3;

/// Tracks the current selection state.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// The currently selected node id, if any.
    pub selected_id: Option<u64>,
    /// Whether the mouse is hovering over the viewport.
    pub viewport_hovered: bool,
    /// Last intersection point in world space.
    pub last_hit_point: Option<Vec3>,
}

impl Selection {
    /// Create a new empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a node.
    pub fn select(&mut self, id: u64) {
        self.selected_id = Some(id);
    }

    /// Clear the current selection.
    pub fn clear(&mut self) {
        self.selected_id = None;
    }

    /// Check if a specific node is selected.
    pub fn is_selected(&self, id: u64) -> bool {
        self.selected_id == Some(id)
    }

    /// Toggle selection of a node.
    pub fn toggle(&mut self, id: u64) {
        if self.is_selected(id) {
            self.clear();
        } else {
            self.select(id);
        }
    }
}

/// Result of a ray-picking operation.
#[derive(Debug, Clone)]
pub struct PickResult {
    /// The id of the hit node.
    pub node_id: u64,
    /// World-space hit point.
    pub hit_point: Vec3,
    /// Distance from ray origin to hit.
    pub distance: f32,
}
