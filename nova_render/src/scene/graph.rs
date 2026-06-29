//! Scene Graph

use super::node::{Node, NodeId};
use crate::core::Pool;

/// Scene Root
pub struct SceneRoot(pub NodeId);

/// Scene Graph
pub struct SceneGraph {
    pub nodes: Pool<Node>,
    pub root: Option<NodeId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self { nodes: Pool::new(), root: None }
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        let handle = self.nodes.spawn(node);
        NodeId(handle)
    }
}

impl Default for SceneGraph {
    fn default() -> Self { Self::new() }
}