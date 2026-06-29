//! PhaseItem + RenderPhase

use crate::assets::{MeshHandle, MaterialHandle};
use crate::scene::node::NodeId;
use glam::Mat4;

/// Phase ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseId {
    Shadow,
    Opaque,
    Transparent,
    Sky,
    Water,
    Particles,
    PostProcess,
}

/// PhaseItem（一个 draw call）
#[derive(Clone)]
pub struct PhaseItem {
    pub mesh: MeshHandle,
    pub material: Option<MaterialHandle>,
    pub transform: Mat4,
    pub node: Option<NodeId>,
    pub sort_key: u32,
}

/// RenderPhase
pub struct RenderPhase {
    pub id: PhaseId,
    pub items: Vec<PhaseItem>,
}

impl RenderPhase {
    pub fn new(id: PhaseId) -> Self {
        Self { id, items: Vec::new() }
    }
    pub fn add(&mut self, item: PhaseItem) {
        self.items.push(item);
    }
    pub fn sort(&mut self) {
        self.items.sort_by_key(|i| i.sort_key);
    }
}