//! Built-in editor tools: gizmo mode switchers (Translate/Rotate/Scale) and
//! a click-to-place mesh placer that spawns NodeType::Mesh nodes on the
//! y=0 ground plane.

#![allow(dead_code)]

use crate::app::EditorApp;
use crate::gizmo::GizmoMode;
use crate::plugin::tool::{EditorTool, ToolContext};
use crate::scene::NodeType;

/// Hint text color (warm yellow) for the gizmo-mode tools.
const HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 220, 120);
/// Hint text color (soft green) for the mesh placer tool.
const PLACE_HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 255, 180);
/// Secondary text color (muted gray) for metadata lines.
const META_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);

/// Draw a single line of hint text at an offset from the viewport's top-left.
fn draw_hint(ui: &mut egui::Ui, offset_y: f32, text: &str, color: egui::Color32) {
    let pos = ui.max_rect().left_top() + egui::vec2(8.0, offset_y);
    ui.painter().text(
        pos,
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(14.0),
        color,
    );
}

/// Tool that switches the active gizmo to Translate mode.
///
/// Lets the existing gizmo system handle the actual transformation; this
/// tool only flips the mode and renders a viewport hint.
pub struct TranslateTool;

impl TranslateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranslateTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTool for TranslateTool {
    fn id(&self) -> &str {
        "tool.translate"
    }

    fn name(&self) -> &str {
        "Translate"
    }

    fn icon(&self) -> Option<&str> {
        Some("\u{2194}")
    }

    fn tooltip(&self) -> &str {
        "Translate selected node (W)"
    }

    fn on_activate(&mut self, app: &mut EditorApp) {
        app.gizmo.mode = GizmoMode::Translate;
        log::info!("[tool:translate] activated");
    }

    fn render_overlay(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        draw_hint(ui, 8.0, "Translate Mode (W)", HINT_COLOR);
    }

    fn consumes_input(&self) -> bool {
        false
    }
}

/// Tool that switches the active gizmo to Rotate mode.
pub struct RotateTool;

impl RotateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RotateTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTool for RotateTool {
    fn id(&self) -> &str {
        "tool.rotate"
    }

    fn name(&self) -> &str {
        "Rotate"
    }

    fn icon(&self) -> Option<&str> {
        Some("\u{21BB}")
    }

    fn tooltip(&self) -> &str {
        "Rotate selected node (E)"
    }

    fn on_activate(&mut self, app: &mut EditorApp) {
        app.gizmo.mode = GizmoMode::Rotate;
        log::info!("[tool:rotate] activated");
    }

    fn render_overlay(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        draw_hint(ui, 8.0, "Rotate Mode (E)", HINT_COLOR);
    }

    fn consumes_input(&self) -> bool {
        false
    }
}

/// Tool that switches the active gizmo to Scale mode.
pub struct ScaleTool;

impl ScaleTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScaleTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTool for ScaleTool {
    fn id(&self) -> &str {
        "tool.scale"
    }

    fn name(&self) -> &str {
        "Scale"
    }

    fn icon(&self) -> Option<&str> {
        Some("\u{2922}")
    }

    fn tooltip(&self) -> &str {
        "Scale selected node (R)"
    }

    fn on_activate(&mut self, app: &mut EditorApp) {
        app.gizmo.mode = GizmoMode::Scale;
        log::info!("[tool:scale] activated");
    }

    fn render_overlay(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        draw_hint(ui, 8.0, "Scale Mode (R)", HINT_COLOR);
    }

    fn consumes_input(&self) -> bool {
        false
    }
}

/// Tool that places a new `NodeType::Mesh` node on the y=0 ground plane at
/// the clicked world position. The new node is parented to the currently
/// selected node (or the root, id=0, if nothing is selected).
pub struct MeshPlacerTool {
    /// Asset path of the mesh to spawn (e.g. "cube.glb").
    pub mesh_path: String,
}

impl MeshPlacerTool {
    pub fn new() -> Self {
        Self { mesh_path: "cube.glb".to_string() }
    }
}

impl Default for MeshPlacerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTool for MeshPlacerTool {
    fn id(&self) -> &str {
        "tool.mesh_placer"
    }

    fn name(&self) -> &str {
        "Place Mesh"
    }

    fn icon(&self) -> Option<&str> {
        Some("\u{25A3}")
    }

    fn tooltip(&self) -> &str {
        "Click in the viewport to place a mesh on the ground plane"
    }

    fn on_activate(&mut self, _app: &mut EditorApp) {
        log::info!("[tool:mesh_placer] activated; mesh_path=\"{}\"", self.mesh_path);
    }

    fn apply(&mut self, ctx: &mut ToolContext, app: &mut EditorApp) {
        let pos = match ctx.ground_hit() {
            Some(p) => p,
            None => return,
        };

        let parent_id = ctx.selection.selected_id.unwrap_or(0);

        let new_id = match ctx.scene.add_child(parent_id, "Mesh") {
            Some(id) => id,
            None => {
                log::warn!(
                    "[tool:mesh_placer] failed to add child under parent_id={}",
                    parent_id
                );
                return;
            }
        };

        if let Some(node) = ctx.scene.find_node_mut(new_id) {
            node.transform.translation = pos;
            node.node_type = NodeType::Mesh { path: self.mesh_path.clone() };
        }

        app.dirty = true;
        log::info!(
            "[tool:mesh_placer] placed mesh \"{}\" at ({:.2}, {:.2}, {:.2}) under parent {} (new id={})",
            self.mesh_path, pos.x, pos.y, pos.z, parent_id, new_id
        );
    }

    fn render_overlay(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        draw_hint(ui, 8.0, "Click to place mesh", PLACE_HINT_COLOR);
        draw_hint(ui, 28.0, &format!("Mesh: {}", self.mesh_path), META_COLOR);
    }

    fn render_options(&mut self, ui: &mut egui::Ui, _app: &mut EditorApp) {
        ui.heading("Mesh Placer");
        ui.add_space(4.0);
        ui.label("Mesh asset path:");
        ui.text_edit_singleline(&mut self.mesh_path);
        ui.add_space(6.0);
        ui.label(
            "Click in the viewport to spawn a new mesh node on the ground plane (y=0). \
             The node is parented to the current selection (or the root if nothing is selected).",
        );
    }

    fn consumes_input(&self) -> bool {
        true
    }
}
