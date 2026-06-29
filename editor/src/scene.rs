//! Scene data model for the Wasteland Editor.
//!
//! Defines the serializable scene graph that the editor operates on.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Top-level scene container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Human-readable scene name.
    pub name: String,
    /// All nodes in the scene graph (flat list; hierarchy via parent/children ids).
    pub nodes: Vec<SceneNode>,
    /// Monotonically increasing counter for assigning new node ids.
    pub next_id: u64,
}

impl Default for Scene {
    fn default() -> Self {
        Self { name: "Untitled".to_string(), nodes: Vec::new(), next_id: 1 }
    }
}

impl Scene {
    /// Create an empty scene with a default root node.
    pub fn new_empty() -> Self {
        let root = SceneNode {
            id: 0,
            name: "Root".to_string(),
            parent: None,
            children: Vec::new(),
            transform: NodeTransform::default(),
            node_type: NodeType::Empty,
        };
        Scene { name: "Untitled".to_string(), nodes: vec![root], next_id: 1 }
    }

    /// Reset the scene to default empty state.
    pub fn reset(&mut self) {
        *self = Self::new_empty();
    }

    /// Find a node by id.
    pub fn find_node(&self, id: u64) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find a node by id (mutable).
    pub fn find_node_mut(&mut self, id: u64) -> Option<&mut SceneNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Add a child node to a parent.
    pub fn add_child(&mut self, parent_id: u64, name: &str) -> Option<u64> {
        // Phase 6 fix: reject invalid parent_id (was silently creating orphan nodes)
        if self.find_node(parent_id).is_none() {
            return None;
        }
        let new_id = self.next_id;
        self.next_id += 1;

        let node = SceneNode {
            id: new_id,
            name: name.to_string(),
            parent: Some(parent_id),
            children: Vec::new(),
            transform: NodeTransform::default(),
            node_type: NodeType::Empty,
        };

        if let Some(parent) = self.find_node_mut(parent_id) {
            parent.children.push(new_id);
        }

        self.nodes.push(node);
        Some(new_id)
    }

    /// Remove a node and all its descendants.
    pub fn remove_node(&mut self, id: u64) {
        // Collect all descendant ids first (to avoid borrow issues).
        let descendants: Vec<u64> = self.collect_descendants(id);
        // Remove from parent's children list.
        if let Some(node) = self.find_node(id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.find_node_mut(parent_id) {
                    parent.children.retain(|c| *c != id);
                }
            }
        }
        // Remove all descendants and the node itself.
        self.nodes.retain(|n| n.id != id && !descendants.contains(&n.id));
    }

    /// Recursively collect all descendant ids of a node.
    fn collect_descendants(&self, id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        if let Some(node) = self.find_node(id) {
            for child_id in &node.children {
                result.push(*child_id);
                result.extend(self.collect_descendants(*child_id));
            }
        }
        result
    }

    /// Move a node (and its subtree) to a new parent.
    ///
    /// Returns `Some(old_parent_id)` on success, or `None` if the operation is
    /// rejected. Rejections:
    /// - `node_id` or `new_parent_id` does not exist
    /// - `node_id` has no parent (root cannot be reparented)
    /// - `new_parent_id == node_id` (cannot reparent to self)
    /// - `new_parent_id` is a descendant of `node_id` (would create a cycle)
    /// - `new_parent_id` is already the current parent (no-op)
    pub fn reparent_node(&mut self, node_id: u64, new_parent_id: u64) -> Option<u64> {
        // Validate node existence and that it's not the root.
        let old_parent_id = self.find_node(node_id)?.parent?;
        // Validate new parent existence.
        if self.find_node(new_parent_id).is_none() {
            return None;
        }
        // Reject self-reparenting.
        if new_parent_id == node_id {
            return None;
        }
        // Reject cycles: new_parent cannot be a descendant of node.
        let descendants = self.collect_descendants(node_id);
        if descendants.contains(&new_parent_id) {
            return None;
        }
        // No-op if already the parent.
        if new_parent_id == old_parent_id {
            return None;
        }

        // Unlink from old parent.
        if let Some(parent) = self.find_node_mut(old_parent_id) {
            parent.children.retain(|c| *c != node_id);
        }
        // Link to new parent.
        if let Some(parent) = self.find_node_mut(new_parent_id) {
            if !parent.children.contains(&node_id) {
                parent.children.push(node_id);
            }
        }
        // Update node's parent pointer.
        if let Some(node) = self.find_node_mut(node_id) {
            node.parent = Some(new_parent_id);
        }
        Some(old_parent_id)
    }

    /// Duplicate a node and all its descendants, adding the clone as a sibling
    /// of the original (under the same parent). Returns the new root node id.
    pub fn duplicate_subtree(&mut self, node_id: u64) -> Option<u64> {
        let parent_id = self.find_node(node_id)?.parent;

        // Collect subtree ids in depth-first order (root first).
        let subtree_ids: Vec<u64> = {
            let mut ids = vec![node_id];
            let mut stack: Vec<u64> = self.find_node(node_id).map(|n| n.children.iter().rev().copied().collect()).unwrap_or_default();
            while let Some(id) = stack.pop() {
                ids.push(id);
                if let Some(node) = self.find_node(id) {
                    for child in node.children.iter().rev() {
                        stack.push(*child);
                    }
                }
            }
            ids
        };

        // Build old_id -> new_id mapping.
        let mut id_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for &old_id in &subtree_ids {
            let new_id = self.next_id;
            self.next_id += 1;
            id_map.insert(old_id, new_id);
        }

        // Clone each node with new ID and fixed references.
        let mut cloned_nodes: Vec<SceneNode> = Vec::new();
        for &old_id in &subtree_ids {
            let node = match self.find_node(old_id) {
                Some(n) => n.clone(),
                None => continue,
            };
            let new_id = id_map[&old_id];
            let new_parent = if old_id == node_id {
                parent_id
            } else {
                node.parent.and_then(|p| id_map.get(&p).copied())
            };
            let new_children: Vec<u64> = node.children.iter().map(|c| id_map.get(c).copied().unwrap_or(*c)).collect();
            let mut cloned = node.clone();
            cloned.id = new_id;
            cloned.parent = new_parent;
            cloned.children = new_children;
            if old_id == node_id {
                cloned.transform.translation.x += 2.0;
                cloned.name = format!("{} (copy)", node.name);
            }
            cloned_nodes.push(cloned);
        }

        // Add the cloned subtree root to the parent's children list.
        if let Some(pid) = parent_id {
            if let Some(parent) = self.find_node_mut(pid) {
                parent.children.push(id_map[&node_id]);
            }
        }

        // Add all cloned nodes to the scene.
        self.nodes.extend(cloned_nodes);

        Some(id_map[&node_id])
    }

    /// Get the depth-first flat list of visible node ids (for hierarchy display).
    pub fn hierarchy_order(&self) -> Vec<u64> {
        let mut order = Vec::new();
        // Find root nodes (no parent).
        let roots: Vec<u64> =
            self.nodes.iter().filter(|n| n.parent.is_none()).map(|n| n.id).collect();
        for root_id in roots {
            self.collect_hierarchy(root_id, 0, &mut order);
        }
        order
    }

    fn collect_hierarchy(&self, id: u64, _depth: usize, order: &mut Vec<u64>) {
        order.push(id);
        if let Some(node) = self.find_node(id) {
            let children: Vec<u64> = node.children.clone();
            for child_id in children {
                self.collect_hierarchy(child_id, _depth + 1, order);
            }
        }
    }

    /// Collect a node and all its descendants as a flat list (DFS, root first).
    /// Used by the copy operation to snapshot a subtree to the clipboard.
    pub fn collect_subtree_nodes(&self, node_id: u64) -> Vec<SceneNode> {
        let mut result = Vec::new();
        self.collect_subtree_recursive(node_id, &mut result);
        result
    }

    fn collect_subtree_recursive(&self, node_id: u64, out: &mut Vec<SceneNode>) {
        if let Some(node) = self.find_node(node_id) {
            out.push(node.clone());
            for child in &node.children {
                self.collect_subtree_recursive(*child, out);
            }
        }
    }

    /// Paste a subtree from an external source (e.g., clipboard) under the given parent.
    ///
    /// `source_nodes` is a flat list where `source_nodes[0]` is the root of the
    /// subtree. All nodes get new IDs; parent/children references are remapped.
    /// The root is offset by x+2 and gets a "(copy)" suffix.
    /// Returns the new root node id.
    pub fn paste_subtree(&mut self, source_nodes: &[SceneNode], parent_id: u64) -> Option<u64> {
        if source_nodes.is_empty() {
            return None;
        }
        if self.find_node(parent_id).is_none() {
            return None;
        }

        // Build old_id → new_id mapping.
        let mut id_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for node in source_nodes {
            let new_id = self.next_id;
            self.next_id += 1;
            id_map.insert(node.id, new_id);
        }

        // Clone each node with new ID and fixed references.
        let mut cloned_nodes: Vec<SceneNode> = Vec::new();
        for (i, node) in source_nodes.iter().enumerate() {
            let new_id = id_map[&node.id];
            let new_parent = if i == 0 {
                Some(parent_id)
            } else {
                node.parent.and_then(|p| id_map.get(&p).copied())
            };
            let new_children: Vec<u64> = node
                .children
                .iter()
                .map(|c| id_map.get(c).copied().unwrap_or(*c))
                .collect();
            let mut cloned = node.clone();
            cloned.id = new_id;
            cloned.parent = new_parent;
            cloned.children = new_children;
            if i == 0 {
                cloned.transform.translation.x += 2.0;
                cloned.name = format!("{} (copy)", node.name);
            }
            cloned_nodes.push(cloned);
        }

        // Add to parent's children list.
        if let Some(parent) = self.find_node_mut(parent_id) {
            parent.children.push(id_map[&source_nodes[0].id]);
        }

        // Add all cloned nodes to the scene.
        self.nodes.extend(cloned_nodes);

        Some(id_map[&source_nodes[0].id])
    }
}

/// A single node in the scene graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    /// Unique node identifier.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Parent node id (None for root nodes).
    pub parent: Option<u64>,
    /// Child node ids.
    pub children: Vec<u64>,
    /// Local transform.
    pub transform: NodeTransform,
    /// What this node represents.
    pub node_type: NodeType,
}

/// Local transform of a scene node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for NodeTransform {
    fn default() -> Self {
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

impl NodeTransform {
    /// Compute the 4x4 matrix for this transform.
    pub fn to_mat4(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// The type of content a scene node holds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    /// An empty group / transform node.
    Empty,
    /// Reference to an external mesh asset.
    Mesh {
        /// Path to the mesh file (e.g., glTF, OBJ).
        path: String,
    },
    /// A light source.
    Light { light_type: LightType, color: Vec3, intensity: f32 },
    /// A camera definition.
    Camera { fov: f32, near: f32, far: f32 },
}

/// Types of light sources.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LightType {
    Point,
    Directional,
    Spot,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Empty => write!(f, "Empty"),
            NodeType::Mesh { path } => write!(f, "Mesh ({})", path),
            NodeType::Light { light_type, .. } => write!(f, "Light ({:?})", light_type),
            NodeType::Camera { fov, .. } => write!(f, "Camera (fov={})", fov),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_subtree_nodes_dfs_order() {
        let mut scene = Scene::new_empty();
        // Root(0) > Parent(1) > [ChildA(2) > Grandchild(3), ChildB(4)]
        let parent = scene.add_child(0, "Parent").unwrap();
        let child_a = scene.add_child(parent, "ChildA").unwrap();
        let _grandchild = scene.add_child(child_a, "Grandchild").unwrap();
        let _child_b = scene.add_child(parent, "ChildB").unwrap();

        let subtree = scene.collect_subtree_nodes(parent);
        // DFS order: Parent first, then its children recursively.
        assert_eq!(subtree.len(), 4);
        assert_eq!(subtree[0].name, "Parent");
        assert_eq!(subtree[1].name, "ChildA");
        assert_eq!(subtree[2].name, "Grandchild");
        assert_eq!(subtree[3].name, "ChildB");
    }

    #[test]
    fn test_paste_subtree_creates_new_ids_and_fixes_references() {
        let mut scene = Scene::new_empty();
        let parent = scene.add_child(0, "Parent").unwrap();
        let child_a = scene.add_child(parent, "ChildA").unwrap();
        let _grandchild = scene.add_child(child_a, "Grandchild").unwrap();
        let _child_b = scene.add_child(parent, "ChildB").unwrap();

        // Copy the Parent subtree.
        let clipboard = scene.collect_subtree_nodes(parent);
        assert_eq!(clipboard.len(), 4);

        let original_count = scene.nodes.len();
        let original_next_id = scene.next_id;

        // Paste under root (0).
        let new_root = scene.paste_subtree(&clipboard, 0).expect("paste should succeed");

        // Verify new nodes were added.
        assert_eq!(scene.nodes.len(), original_count + 4);
        assert!(scene.next_id > original_next_id);

        // The pasted root should be a child of root (0).
        let pasted_root = scene.find_node(new_root).unwrap();
        assert_eq!(pasted_root.parent, Some(0));
        assert!(pasted_root.name.contains("(copy)"));
        assert_eq!(pasted_root.children.len(), 2);

        // The pasted root should have an x offset.
        let original_parent = scene.find_node(parent).unwrap();
        assert!(pasted_root.transform.translation.x > original_parent.transform.translation.x);

        // Root's children should now include the pasted root.
        let root = scene.find_node(0).unwrap();
        assert!(root.children.contains(&new_root));

        // The pasted subtree's children should have correct parent references.
        for &child_id in &pasted_root.children {
            let child = scene.find_node(child_id).unwrap();
            assert_eq!(child.parent, Some(new_root));
        }
    }

    #[test]
    fn test_paste_subtree_invalid_parent_returns_none() {
        let mut scene = Scene::new_empty();
        let parent = scene.add_child(0, "Parent").unwrap();
        let clipboard = scene.collect_subtree_nodes(parent);

        // Paste under a non-existent parent.
        let result = scene.paste_subtree(&clipboard, 9999);
        assert!(result.is_none());
    }

    #[test]
    fn test_paste_subtree_empty_clipboard_returns_none() {
        let mut scene = Scene::new_empty();
        let empty_clipboard: Vec<SceneNode> = Vec::new();
        let result = scene.paste_subtree(&empty_clipboard, 0);
        assert!(result.is_none());
    }
}
