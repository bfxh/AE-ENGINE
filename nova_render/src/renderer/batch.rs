//! Batch 合批

use crate::assets::MeshHandle;
use crate::renderer::phase::PhaseItem;

/// BatchedItem
#[derive(Clone)]
pub struct BatchedItem {
    pub mesh: MeshHandle,
    pub instances: Vec<glam::Mat4>,
}

/// Batch
pub struct Batch {
    pub items: Vec<BatchedItem>,
}

impl Batch {
    pub fn new() -> Self { Self { items: Vec::new() } }
    pub fn add(&mut self, item: PhaseItem) {
        // 简化：按 mesh 分组
        if let Some(b) = self.items.iter_mut().find(|b| b.mesh.index() == item.mesh.index()) {
            b.instances.push(item.transform);
        } else {
            self.items.push(BatchedItem { mesh: item.mesh, instances: vec![item.transform] });
        }
    }
}

impl Default for Batch {
    fn default() -> Self { Self::new() }
}