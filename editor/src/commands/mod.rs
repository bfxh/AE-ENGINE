//! Command system for undo/redo support.
//!
//! Implements the Command pattern for editor operations.
//! Commands receive `&mut Scene` to apply or reverse their changes.

use crate::scene::Scene;
use glam::Vec3;
use std::collections::VecDeque;

/// A reversible editor operation that acts on the scene.
pub trait Command: std::fmt::Debug {
    /// Execute (or redo) the command, modifying the scene.
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()>;
    /// Undo the command, restoring the scene to its previous state.
    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()>;
    /// Human-readable description of the command.
    fn description(&self) -> &str;
}

/// History of executed commands for undo/redo.
#[derive(Debug, Default)]
pub struct CommandHistory {
    /// Commands that can be undone.
    undo_stack: VecDeque<Box<dyn Command>>,
    /// Commands that can be redone.
    redo_stack: VecDeque<Box<dyn Command>>,
    /// Maximum number of undo steps.
    max_undo: usize,
}

impl CommandHistory {
    /// Create a new command history with the given capacity.
    pub fn new(max_undo: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_undo),
            redo_stack: VecDeque::new(),
            max_undo,
        }
    }

    /// Execute a command, add it to the undo stack, and clear the redo stack.
    pub fn execute(&mut self, mut cmd: Box<dyn Command>, scene: &mut Scene) -> anyhow::Result<()> {
        cmd.execute(scene)?;
        self.redo_stack.clear();
        self.undo_stack.push_back(cmd);
        while self.undo_stack.len() > self.max_undo {
            self.undo_stack.pop_front();
        }
        Ok(())
    }

    /// Undo the most recent command.
    pub fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(mut cmd) = self.undo_stack.pop_back() {
            cmd.undo(scene)?;
            self.redo_stack.push_back(cmd);
        }
        Ok(())
    }

    /// Redo the most recently undone command.
    pub fn redo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(mut cmd) = self.redo_stack.pop_back() {
            cmd.execute(scene)?;
            self.undo_stack.push_back(cmd);
        }
        Ok(())
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Update the maximum undo limit, trimming excess entries if needed.
    pub fn set_max_undo(&mut self, max: usize) {
        self.max_undo = max;
        while self.undo_stack.len() > self.max_undo {
            self.undo_stack.pop_front();
        }
    }

    /// Get a description of the next undo command, if any.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|c| c.description())
    }

    /// Get a description of the next redo command, if any.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.back().map(|c| c.description())
    }
}

// ---------------------------------------------------------------------------
// Concrete command implementations
// ---------------------------------------------------------------------------

/// Command that changes the translation of a node.
#[derive(Debug, Clone)]
pub struct TransformCommand {
    pub node_id: u64,
    pub old_translation: Vec3,
    pub new_translation: Vec3,
}

impl TransformCommand {
    pub fn new(node_id: u64, old_translation: Vec3, new_translation: Vec3) -> Self {
        Self { node_id, old_translation, new_translation }
    }
}

impl Command for TransformCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.transform.translation = self.new_translation;
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.transform.translation = self.old_translation;
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Move Object"
    }
}

/// Command that sets the full transform (translation + rotation + scale) of a node.
/// Used by the Inspector panel for batched undo of drag edits.
#[derive(Debug, Clone)]
pub struct SetTransformCommand {
    pub node_id: u64,
    pub old_transform: crate::scene::NodeTransform,
    pub new_transform: crate::scene::NodeTransform,
}

impl SetTransformCommand {
    pub fn new(
        node_id: u64,
        old_transform: crate::scene::NodeTransform,
        new_transform: crate::scene::NodeTransform,
    ) -> Self {
        Self { node_id, old_transform, new_transform }
    }
}

impl Command for SetTransformCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.transform = self.new_transform.clone();
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.transform = self.old_transform.clone();
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Set Transform"
    }
}

/// Command for creating a node.
#[derive(Debug, Clone)]
pub struct CreateNodeCommand {
    pub child_id: u64,
    pub parent_id: u64,
}

impl CreateNodeCommand {
    pub fn new(parent_id: u64, child_id: u64) -> Self {
        Self { parent_id, child_id }
    }
}

impl Command for CreateNodeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // Re-create the node by re-adding it.
        // The node was already created by add_child externally; this just records undo.
        // Actually, for proper undo we'd store the full node state. Simplified: just track parent.
        // Since add_child already created the node with a given id, we just ensure parent link.
        if let Some(parent) = scene.find_node_mut(self.parent_id) {
            if !parent.children.contains(&self.child_id) {
                parent.children.push(self.child_id);
            }
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // Remove child from parent.
        if let Some(parent) = scene.find_node_mut(self.parent_id) {
            parent.children.retain(|c| *c != self.child_id);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Create Node"
    }
}

/// Command for deleting a node.
#[derive(Debug, Clone)]
pub struct DeleteNodeCommand {
    pub node_id: u64,
    pub parent_id: Option<u64>,
    pub stored_node: Option<crate::scene::SceneNode>,
}

impl DeleteNodeCommand {
    pub fn new(node_id: u64, parent_id: Option<u64>, stored_node: crate::scene::SceneNode) -> Self {
        Self { node_id, parent_id, stored_node: Some(stored_node) }
    }
}

impl Command for DeleteNodeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // Remove the node.
        scene.nodes.retain(|n| n.id != self.node_id);
        if let Some(pid) = self.parent_id {
            if let Some(parent) = scene.find_node_mut(pid) {
                parent.children.retain(|c| *c != self.node_id);
            }
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // Restore the node.
        if let Some(node) = self.stored_node.take() {
            let id = node.id;
            let pid = node.parent;
            scene.nodes.push(node);
            if let Some(pid) = pid {
                if let Some(parent) = scene.find_node_mut(pid) {
                    if !parent.children.contains(&id) {
                        parent.children.push(id);
                    }
                }
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Delete Node"
    }
}

/// Command that renames a node.
#[derive(Debug, Clone)]
pub struct RenameNodeCommand {
    pub node_id: u64,
    pub old_name: String,
    pub new_name: String,
}

impl RenameNodeCommand {
    pub fn new(node_id: u64, old_name: String, new_name: String) -> Self {
        Self { node_id, old_name, new_name }
    }
}

impl Command for RenameNodeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.name = self.new_name.clone();
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.name = self.old_name.clone();
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Rename Node"
    }
}

/// Command that duplicates a node subtree.
///
/// The actual duplication is performed externally (via `Scene::duplicate_subtree`)
/// before the command is recorded. This command stores the cloned nodes so that
/// undo removes them and redo restores them.
#[derive(Debug, Clone)]
pub struct DuplicateNodeCommand {
    pub new_root_id: u64,
    pub parent_id: Option<u64>,
    /// Stored cloned nodes for redo after undo removes them.
    pub stored_nodes: Option<Vec<crate::scene::SceneNode>>,
}

impl DuplicateNodeCommand {
    pub fn new(new_root_id: u64, parent_id: Option<u64>) -> Self {
        Self { new_root_id, parent_id, stored_nodes: None }
    }
}

impl Command for DuplicateNodeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(nodes) = self.stored_nodes.take() {
            // Redo: re-add the stored nodes and relink parent.
            scene.nodes.extend(nodes);
            if let Some(pid) = self.parent_id {
                if let Some(parent) = scene.find_node_mut(pid) {
                    if !parent.children.contains(&self.new_root_id) {
                        parent.children.push(self.new_root_id);
                    }
                }
            }
        }
        // First execution: duplication done externally, nothing to do here.
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // Collect the duplicated subtree (root + descendants).
        let mut to_remove: Vec<u64> = vec![self.new_root_id];
        let mut stack = vec![self.new_root_id];
        while let Some(id) = stack.pop() {
            if let Some(node) = scene.find_node(id) {
                for child in &node.children {
                    to_remove.push(*child);
                    stack.push(*child);
                }
            }
        }

        // Store nodes before removing (for redo).
        let stored: Vec<crate::scene::SceneNode> = scene
            .nodes
            .iter()
            .filter(|n| to_remove.contains(&n.id))
            .cloned()
            .collect();
        self.stored_nodes = Some(stored);

        // Unlink from parent.
        if let Some(pid) = self.parent_id {
            if let Some(parent) = scene.find_node_mut(pid) {
                parent.children.retain(|c| *c != self.new_root_id);
            }
        }

        // Remove the nodes.
        scene.nodes.retain(|n| !to_remove.contains(&n.id));

        Ok(())
    }

    fn description(&self) -> &str {
        "Duplicate Node"
    }
}

/// Command that sets the `NodeType` (type-specific properties) of a node.
///
/// Used by the Inspector panel for batched undo of Light color/intensity and
/// Camera fov/near/far edits. The entire edit session is committed as a single
/// command to avoid flooding the undo stack with per-frame entries.
#[derive(Debug, Clone)]
pub struct SetNodeTypeCommand {
    pub node_id: u64,
    pub old_node_type: crate::scene::NodeType,
    pub new_node_type: crate::scene::NodeType,
}

impl SetNodeTypeCommand {
    pub fn new(
        node_id: u64,
        old_node_type: crate::scene::NodeType,
        new_node_type: crate::scene::NodeType,
    ) -> Self {
        Self { node_id, old_node_type, new_node_type }
    }
}

impl Command for SetNodeTypeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.node_type = self.new_node_type.clone();
        }
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        if let Some(node) = scene.find_node_mut(self.node_id) {
            node.node_type = self.old_node_type.clone();
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Set Node Properties"
    }
}

/// Command that moves a node to a new parent (reparenting).
///
/// Stores the node id, old parent, and new parent so the operation can be
/// reversed. Cycle prevention and validation are performed by
/// `Scene::reparent_node` before the command is constructed.
#[derive(Debug, Clone)]
pub struct ReparentNodeCommand {
    pub node_id: u64,
    pub old_parent_id: u64,
    pub new_parent_id: u64,
}

impl ReparentNodeCommand {
    pub fn new(node_id: u64, old_parent_id: u64, new_parent_id: u64) -> Self {
        Self { node_id, old_parent_id, new_parent_id }
    }
}

impl Command for ReparentNodeCommand {
    fn execute(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        // reparent_node returns None for no-ops (same parent), but since the
        // command was constructed only after a successful reparent, this should
        // always succeed. We call it for safety; if it fails the scene is
        // unchanged.
        let _ = scene.reparent_node(self.node_id, self.new_parent_id);
        Ok(())
    }

    fn undo(&mut self, scene: &mut Scene) -> anyhow::Result<()> {
        let _ = scene.reparent_node(self.node_id, self.old_parent_id);
        Ok(())
    }

    fn description(&self) -> &str {
        "Reparent Node"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_max_undo_trims_excess() {
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let child_id = scene.add_child(0, "Child").unwrap();

        for i in 0..5 {
            let cmd = TransformCommand::new(child_id, Vec3::new(i as f32, 0.0, 0.0), Vec3::new((i + 1) as f32, 0.0, 0.0));
            let _ = history.execute(Box::new(cmd), &mut scene);
        }
        assert!(history.can_undo());
        assert_eq!(history.undo_description(), Some("Move Object"));

        history.set_max_undo(3);
        assert!(history.can_undo());

        for _ in 0..3 {
            let _ = history.undo(&mut scene);
        }
        assert!(!history.can_undo());
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let child_id = scene.add_child(0, "Child").unwrap();

        let cmd = TransformCommand::new(child_id, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let _ = history.execute(Box::new(cmd), &mut scene);

        assert!(history.can_undo());
        let _ = history.undo(&mut scene);
        assert!(!history.can_undo());
        assert!(history.can_redo());
        let _ = history.redo(&mut scene);
        assert!(history.can_undo());
    }

    #[test]
    fn test_rename_node_command_undo_redo() {
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let child_id = scene.add_child(0, "Child").unwrap();

        let cmd = RenameNodeCommand::new(child_id, "Child".to_string(), "Renamed".to_string());
        let _ = history.execute(Box::new(cmd), &mut scene);

        assert_eq!(scene.find_node(child_id).unwrap().name, "Renamed");

        let _ = history.undo(&mut scene);
        assert_eq!(scene.find_node(child_id).unwrap().name, "Child");

        let _ = history.redo(&mut scene);
        assert_eq!(scene.find_node(child_id).unwrap().name, "Renamed");
    }

    #[test]
    fn test_set_transform_command_undo_redo() {
        use crate::scene::NodeTransform;
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let child_id = scene.add_child(0, "Child").unwrap();

        let old_t = NodeTransform::default();
        let new_t = NodeTransform {
            translation: Vec3::new(5.0, 10.0, -3.0),
            rotation: glam::Quat::from_euler(glam::EulerRot::YXZ, 0.5, 0.3, 0.1),
            scale: glam::Vec3::new(2.0, 0.5, 3.0),
        };

        let cmd = SetTransformCommand::new(child_id, old_t.clone(), new_t.clone());
        let _ = history.execute(Box::new(cmd), &mut scene);

        let n = scene.find_node(child_id).unwrap();
        assert_eq!(n.transform.translation, new_t.translation);
        assert_eq!(n.transform.scale, new_t.scale);

        let _ = history.undo(&mut scene);
        let n = scene.find_node(child_id).unwrap();
        assert_eq!(n.transform.translation, old_t.translation);
        assert_eq!(n.transform.scale, old_t.scale);

        let _ = history.redo(&mut scene);
        let n = scene.find_node(child_id).unwrap();
        assert_eq!(n.transform.translation, new_t.translation);
    }

    #[test]
    fn test_duplicate_subtree_and_undo_redo() {
        let mut scene = Scene::new_empty();
        let parent_id = scene.add_child(0, "Parent").unwrap();
        let child_a = scene.add_child(parent_id, "ChildA").unwrap();
        let _grandchild = scene.add_child(child_a, "Grandchild").unwrap();
        let _child_b = scene.add_child(parent_id, "ChildB").unwrap();

        // Scene: Root(0) > Parent(1) > [ChildA(2) > Grandchild(3), ChildB(4)]
        // Duplicated subtree (Parent + 3 descendants) = 4 nodes.
        let original_node_count = scene.nodes.len(); // 5
        let subtree_size = 4;

        // Duplicate the Parent subtree.
        let new_root = scene.duplicate_subtree(parent_id).expect("dup should succeed");
        assert_ne!(new_root, parent_id);
        assert_eq!(scene.nodes.len(), original_node_count + subtree_size);

        // The duplicate should be a sibling of the original (under Root, id=0).
        let new_node = scene.find_node(new_root).unwrap();
        assert_eq!(new_node.parent, Some(0));
        assert!(new_node.name.contains("(copy)"));
        // The duplicate should have the same number of children as the original.
        assert_eq!(new_node.children.len(), 2);

        // Now wrap it in a command and test undo/redo.
        let mut history = CommandHistory::new(10);
        let cmd = DuplicateNodeCommand::new(new_root, Some(0));
        let _ = history.execute(Box::new(cmd), &mut scene);

        // Undo: removes the duplicated subtree.
        let _ = history.undo(&mut scene);
        assert_eq!(scene.nodes.len(), original_node_count);
        assert!(scene.find_node(new_root).is_none());

        // Redo: restores the duplicated subtree.
        let _ = history.redo(&mut scene);
        assert_eq!(scene.nodes.len(), original_node_count + subtree_size);
        assert!(scene.find_node(new_root).is_some());
        let restored = scene.find_node(new_root).unwrap();
        assert_eq!(restored.children.len(), 2);
    }

    #[test]
    fn test_set_node_type_command_undo_redo_light() {
        use crate::scene::{LightType, NodeType};
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let light_id = scene.add_child(0, "Light").unwrap();
        // Manually set the node type to Light.
        if let Some(node) = scene.find_node_mut(light_id) {
            node.node_type = NodeType::Light {
                light_type: LightType::Point,
                color: Vec3::new(1.0, 1.0, 1.0),
                intensity: 10.0,
            };
        }

        let old_type = scene.find_node(light_id).unwrap().node_type.clone();
        let new_type = NodeType::Light {
            light_type: LightType::Point,
            color: Vec3::new(0.5, 0.2, 0.8),
            intensity: 50.0,
        };

        let cmd = SetNodeTypeCommand::new(light_id, old_type, new_type.clone());
        let _ = history.execute(Box::new(cmd), &mut scene);

        // Verify execution.
        let n = scene.find_node(light_id).unwrap();
        assert_eq!(n.node_type, new_type);

        // Undo: should restore old color and intensity.
        let _ = history.undo(&mut scene);
        let n = scene.find_node(light_id).unwrap();
        if let NodeType::Light { color, intensity, .. } = &n.node_type {
            assert_eq!(*color, Vec3::new(1.0, 1.0, 1.0));
            assert_eq!(*intensity, 10.0);
        } else {
            panic!("expected Light type after undo");
        }

        // Redo: should reapply new color and intensity.
        let _ = history.redo(&mut scene);
        let n = scene.find_node(light_id).unwrap();
        if let NodeType::Light { color, intensity, .. } = &n.node_type {
            assert_eq!(*color, Vec3::new(0.5, 0.2, 0.8));
            assert_eq!(*intensity, 50.0);
        } else {
            panic!("expected Light type after redo");
        }
    }

    #[test]
    fn test_set_node_type_command_undo_redo_camera() {
        use crate::scene::NodeType;
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let cam_id = scene.add_child(0, "Camera").unwrap();
        if let Some(node) = scene.find_node_mut(cam_id) {
            node.node_type = NodeType::Camera { fov: 60.0, near: 0.1, far: 1000.0 };
        }

        let old_type = scene.find_node(cam_id).unwrap().node_type.clone();
        let new_type = NodeType::Camera { fov: 90.0, near: 0.5, far: 5000.0 };

        let cmd = SetNodeTypeCommand::new(cam_id, old_type, new_type.clone());
        let _ = history.execute(Box::new(cmd), &mut scene);

        let n = scene.find_node(cam_id).unwrap();
        assert_eq!(n.node_type, new_type);

        let _ = history.undo(&mut scene);
        let n = scene.find_node(cam_id).unwrap();
        if let NodeType::Camera { fov, near, far } = &n.node_type {
            assert_eq!(*fov, 60.0);
            assert_eq!(*near, 0.1);
            assert_eq!(*far, 1000.0);
        } else {
            panic!("expected Camera type after undo");
        }

        let _ = history.redo(&mut scene);
        let n = scene.find_node(cam_id).unwrap();
        if let NodeType::Camera { fov, near, far } = &n.node_type {
            assert_eq!(*fov, 90.0);
            assert_eq!(*near, 0.5);
            assert_eq!(*far, 5000.0);
        } else {
            panic!("expected Camera type after redo");
        }
    }

    #[test]
    fn test_reparent_node_command_undo_redo() {
        // Build: Root(0) > A(1) > [B(2), C(3)], Root(0) > D(4)
        let mut history = CommandHistory::new(10);
        let mut scene = Scene::new_empty();
        let a = scene.add_child(0, "A").unwrap();
        let b = scene.add_child(a, "B").unwrap();
        let _c = scene.add_child(a, "C").unwrap();
        let d = scene.add_child(0, "D").unwrap();

        // Move B(2) from A(1) to D(4).
        let old_parent = scene.find_node(b).unwrap().parent.unwrap();
        assert_eq!(old_parent, a);
        let cmd = ReparentNodeCommand::new(b, old_parent, d);
        let _ = history.execute(Box::new(cmd), &mut scene);

        // Verify execution: B should now be under D.
        assert_eq!(scene.find_node(b).unwrap().parent, Some(d));
        assert!(scene.find_node(d).unwrap().children.contains(&b));
        assert!(!scene.find_node(a).unwrap().children.contains(&b));

        // Undo: B should return to A.
        let _ = history.undo(&mut scene);
        assert_eq!(scene.find_node(b).unwrap().parent, Some(a));
        assert!(scene.find_node(a).unwrap().children.contains(&b));
        assert!(!scene.find_node(d).unwrap().children.contains(&b));

        // Redo: B should be back under D.
        let _ = history.redo(&mut scene);
        assert_eq!(scene.find_node(b).unwrap().parent, Some(d));
        assert!(scene.find_node(d).unwrap().children.contains(&b));
        assert!(!scene.find_node(a).unwrap().children.contains(&b));
    }

    #[test]
    fn test_reparent_node_command_cycle_prevention() {
        // Build: Root(0) > A(1) > B(2) > C(3)
        let mut scene = Scene::new_empty();
        let a = scene.add_child(0, "A").unwrap();
        let b = scene.add_child(a, "B").unwrap();
        let c = scene.add_child(b, "C").unwrap();

        // Try to reparent A(1) under C(3) — should be rejected (cycle).
        let result = scene.reparent_node(a, c);
        assert!(result.is_none());
        // Verify hierarchy unchanged.
        assert_eq!(scene.find_node(a).unwrap().parent, Some(0));

        // Try to reparent B(2) under itself — should be rejected.
        let result = scene.reparent_node(b, b);
        assert!(result.is_none());

        // Try to reparent root (0) — should be rejected.
        let result = scene.reparent_node(0, b);
        assert!(result.is_none());
    }

    #[test]
    fn test_reparent_node_command_noop_same_parent() {
        let mut scene = Scene::new_empty();
        let a = scene.add_child(0, "A").unwrap();
        let b = scene.add_child(a, "B").unwrap();

        // Reparenting B to its current parent A should be a no-op.
        let result = scene.reparent_node(b, a);
        assert!(result.is_none());
    }
}
