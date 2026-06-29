use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::destruction::VoxelFlags;
use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::material::MaterialProperties;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OctreeNode {
    pub depth: u8,
    pub bounds: OctreeBounds,
    pub node_type: NodeType,
    pub children: Option<[Box<OctreeNode>; 8]>,
    pub material_id: Option<u64>,
    pub avg_temperature: FixedPoint,
    pub avg_radiation: FixedPoint,
    pub active_voxel_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OctreeBounds {
    pub min: IVec3,
    pub max: IVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Leaf,
    Branch,
    Homogeneous,
    Empty,
}

impl OctreeNode {
    pub fn new(depth: u8, bounds: OctreeBounds) -> Self {
        Self {
            depth,
            bounds,
            node_type: NodeType::Empty,
            children: None,
            material_id: None,
            avg_temperature: FixedPoint::from_f32(293.0),
            avg_radiation: FixedPoint::ZERO,
            active_voxel_count: 0,
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf | NodeType::Homogeneous)
    }

    pub fn center(&self) -> IVec3 {
        (self.bounds.min + self.bounds.max) / 2
    }

    pub fn size(&self) -> IVec3 {
        self.bounds.max - self.bounds.min
    }

    pub fn contains(&self, pos: IVec3) -> bool {
        pos.x >= self.bounds.min.x
            && pos.x < self.bounds.max.x
            && pos.y >= self.bounds.min.y
            && pos.y < self.bounds.max.y
            && pos.z >= self.bounds.min.z
            && pos.z < self.bounds.max.z
    }

    fn split_point(&self) -> IVec3 {
        (self.bounds.min + self.bounds.max) / 2
    }
}

pub type OctreeIndex = (u8, i32, i32, i32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseOctree {
    pub root: OctreeNode,
    pub max_depth: u8,
    pub min_cell_size: FixedPoint,
    pub world_origin: FixedVec3,
    pub total_nodes: usize,
    pub total_leaves: usize,
    pub total_active_voxels: usize,
    pub material: MaterialProperties,
}

impl SparseOctree {
    pub fn new(
        world_size: FixedPoint,
        max_depth: u8,
        origin: FixedVec3,
        material: MaterialProperties,
    ) -> Self {
        let size = 1i32 << max_depth;
        let bounds = OctreeBounds { min: IVec3::ZERO, max: IVec3::splat(size) };
        let root = OctreeNode::new(0, bounds);
        let min_cell_size = world_size / FixedPoint::from_i32(size);

        Self {
            root,
            max_depth,
            min_cell_size,
            world_origin: origin,
            total_nodes: 1,
            total_leaves: 0,
            total_active_voxels: 0,
            material,
        }
    }

    pub fn world_to_tree(&self, world_pos: FixedVec3) -> Option<IVec3> {
        let local = world_pos - self.world_origin;
        let size = 1i32 << self.max_depth;
        let world_size = FixedPoint::from_i32(size) * self.min_cell_size;
        if local.x < FixedPoint::ZERO
            || local.y < FixedPoint::ZERO
            || local.z < FixedPoint::ZERO
            || local.x >= world_size
            || local.y >= world_size
            || local.z >= world_size
        {
            return None;
        }
        Some(IVec3::new(
            (local.x / self.min_cell_size).to_f32() as i32,
            (local.y / self.min_cell_size).to_f32() as i32,
            (local.z / self.min_cell_size).to_f32() as i32,
        ))
    }

    pub fn tree_to_world(&self, pos: IVec3) -> FixedVec3 {
        let half = self.min_cell_size * FixedPoint::from_f32(0.5);
        FixedVec3::new(
            FixedPoint::from_i32(pos.x) * self.min_cell_size + half,
            FixedPoint::from_i32(pos.y) * self.min_cell_size + half,
            FixedPoint::from_i32(pos.z) * self.min_cell_size + half,
        ) + self.world_origin
    }

    pub fn activate_voxel(&mut self, world_pos: FixedVec3) -> bool {
        let pos = match self.world_to_tree(world_pos) {
            Some(p) => p,
            None => return false,
        };
        Self::activate_recursive_static(
            &mut self.root,
            pos,
            self.max_depth,
            &mut self.total_nodes,
            &mut self.total_leaves,
            &mut self.total_active_voxels,
        )
    }

    fn activate_recursive_static(
        node: &mut OctreeNode,
        pos: IVec3,
        max_depth: u8,
        total_nodes: &mut usize,
        total_leaves: &mut usize,
        total_active_voxels: &mut usize,
    ) -> bool {
        if !node.contains(pos) {
            return false;
        }

        let depth = node.depth;
        if depth >= max_depth {
            match node.node_type {
                NodeType::Empty | NodeType::Leaf => {
                    let was_empty = node.node_type == NodeType::Empty;
                    node.node_type = NodeType::Leaf;
                    node.active_voxel_count += 1;
                    *total_active_voxels += 1;
                    if was_empty {
                        *total_leaves += 1;
                    }
                    return true;
                },
                NodeType::Branch => {
                    node.active_voxel_count += 1;
                    *total_active_voxels += 1;
                    return true;
                },
                _ => return false,
            }
        }

        if matches!(node.node_type, NodeType::Empty) && node.children.is_none() {
            node.node_type = NodeType::Branch;
            *total_nodes += 7;
            let sp = node.split_point();
            let child_depth = node.depth + 1;
            let children: [Box<OctreeNode>; 8] = [
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds { min: node.bounds.min, max: sp },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(sp.x, node.bounds.min.y, node.bounds.min.z),
                        max: IVec3::new(node.bounds.max.x, sp.y, sp.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(node.bounds.min.x, sp.y, node.bounds.min.z),
                        max: IVec3::new(sp.x, node.bounds.max.y, sp.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(sp.x, sp.y, node.bounds.min.z),
                        max: IVec3::new(node.bounds.max.x, node.bounds.max.y, sp.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(node.bounds.min.x, node.bounds.min.y, sp.z),
                        max: IVec3::new(sp.x, sp.y, node.bounds.max.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(sp.x, node.bounds.min.y, sp.z),
                        max: IVec3::new(node.bounds.max.x, sp.y, node.bounds.max.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds {
                        min: IVec3::new(node.bounds.min.x, sp.y, sp.z),
                        max: IVec3::new(sp.x, node.bounds.max.y, node.bounds.max.z),
                    },
                )),
                Box::new(OctreeNode::new(
                    child_depth,
                    OctreeBounds { min: sp, max: node.bounds.max },
                )),
            ];
            node.children = Some(children);
        }

        if node.node_type == NodeType::Homogeneous {
            return false;
        }

        let sp = node.split_point();
        if let Some(ref mut children) = node.children {
            let idx = Self::child_index(pos, sp);
            let child = &mut children[idx];
            let activated = Self::activate_recursive_static(
                child,
                pos,
                max_depth,
                total_nodes,
                total_leaves,
                total_active_voxels,
            );
            if activated {
                node.active_voxel_count += 1;
                node.node_type = NodeType::Branch;
                return true;
            }
        }
        false
    }

    pub fn deactivate_voxel(&mut self, world_pos: FixedVec3) -> bool {
        let pos = match self.world_to_tree(world_pos) {
            Some(p) => p,
            None => return false,
        };
        Self::deactivate_recursive_static(
            &mut self.root,
            pos,
            self.max_depth,
            &mut self.total_nodes,
            &mut self.total_leaves,
            &mut self.total_active_voxels,
        )
    }

    fn deactivate_recursive_static(
        node: &mut OctreeNode,
        pos: IVec3,
        max_depth: u8,
        total_nodes: &mut usize,
        total_leaves: &mut usize,
        total_active_voxels: &mut usize,
    ) -> bool {
        if !node.contains(pos) || node.active_voxel_count == 0 {
            return false;
        }

        let depth = node.depth;
        if depth >= max_depth {
            if node.active_voxel_count > 0 {
                node.active_voxel_count -= 1;
                *total_active_voxels -= 1;
                if node.active_voxel_count == 0 {
                    node.node_type = NodeType::Empty;
                    *total_leaves -= 1;
                }
                return true;
            }
            return false;
        }

        let sp = node.split_point();
        if let Some(ref mut children) = node.children {
            let idx = Self::child_index(pos, sp);
            let child = &mut children[idx];
            if Self::deactivate_recursive_static(
                child,
                pos,
                max_depth,
                total_nodes,
                total_leaves,
                total_active_voxels,
            ) {
                node.active_voxel_count -= 1;
                if node.active_voxel_count == 0 {
                    node.children = None;
                    node.node_type = NodeType::Empty;
                    *total_nodes -= 7;
                }
                return true;
            }
        }
        false
    }

    pub fn is_active(&self, world_pos: FixedVec3) -> bool {
        let pos = match self.world_to_tree(world_pos) {
            Some(p) => p,
            None => return false,
        };
        self.is_active_recursive(&self.root, pos, 0)
    }

    fn is_active_recursive(&self, node: &OctreeNode, pos: IVec3, _depth: u8) -> bool {
        if !node.contains(pos) {
            return false;
        }
        match node.node_type {
            NodeType::Empty => false,
            NodeType::Leaf if node.depth >= self.max_depth => true,
            NodeType::Homogeneous => true,
            NodeType::Branch => {
                if let Some(ref children) = node.children {
                    let sp = node.split_point();
                    let idx = Self::child_index(pos, sp);
                    self.is_active_recursive(&children[idx], pos, _depth + 1)
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn collect_active_positions(&self) -> Vec<FixedVec3> {
        let mut result = Vec::new();
        self.collect_active_recursive(&self.root, &mut result);
        result
    }

    fn collect_active_recursive(&self, node: &OctreeNode, result: &mut Vec<FixedVec3>) {
        match node.node_type {
            NodeType::Empty => (),
            NodeType::Leaf | NodeType::Homogeneous => {
                for z in node.bounds.min.z..node.bounds.max.z {
                    for y in node.bounds.min.y..node.bounds.max.y {
                        for x in node.bounds.min.x..node.bounds.max.x {
                            result.push(self.tree_to_world(IVec3::new(x, y, z)));
                        }
                    }
                }
            },
            NodeType::Branch => {
                if let Some(ref children) = node.children {
                    for child in children.iter() {
                        self.collect_active_recursive(child, result);
                    }
                }
            },
        }
    }

    pub fn active_in_sphere(&self, center: FixedVec3, radius: FixedPoint) -> Vec<FixedVec3> {
        let all = self.collect_active_positions();
        all.into_iter().filter(|p| (center - *p).length() <= radius).collect()
    }

    pub fn compression_ratio(&self) -> FixedPoint {
        let total_cells = FixedPoint::from_f32((1u64 << (self.max_depth * 3)) as f32);
        if total_cells == FixedPoint::ZERO {
            return FixedPoint::ONE;
        }
        FixedPoint::from_f32(self.total_active_voxels as f32) / total_cells
    }

    pub fn to_voxel_grid(&self, resolution: IVec3) -> Option<crate::destruction::VoxelGrid> {
        let voxel_size = self.min_cell_size * FixedPoint::from_i32(1i32 << self.max_depth)
            / FixedPoint::from_i32(resolution.x);
        let mut grid = crate::destruction::VoxelGrid::new(
            resolution,
            voxel_size,
            self.world_origin,
            self.material,
        );

        for world_pos in self.collect_active_positions() {
            if let Some(voxel_pos) = grid.world_to_voxel(world_pos) {
                if let Some(voxel) = grid.get_voxel_mut(voxel_pos) {
                    voxel.flags.insert(VoxelFlags::ACTIVE);
                }
            }
        }
        Some(grid)
    }

    fn child_index(pos: IVec3, split: IVec3) -> usize {
        let x = if pos.x >= split.x { 1 } else { 0 };
        let y = if pos.y >= split.y { 1 } else { 0 };
        let z = if pos.z >= split.z { 1 } else { 0 };
        (z << 2) | (y << 1) | x
    }
}

impl Default for Box<OctreeNode> {
    fn default() -> Self {
        Box::new(OctreeNode::new(0, OctreeBounds { min: IVec3::ZERO, max: IVec3::ONE }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octree_activation() {
        let material = MaterialProperties::concrete();
        let mut octree =
            SparseOctree::new(FixedPoint::from_f32(64.0), 6, FixedVec3::ZERO, material);

        octree.activate_voxel(FixedVec3::from_f32(10.0, 5.0, 10.0));
        octree.activate_voxel(FixedVec3::from_f32(11.0, 5.0, 10.0));

        assert!(octree.is_active(FixedVec3::from_f32(10.0, 5.0, 10.0)));
        assert!(octree.is_active(FixedVec3::from_f32(11.0, 5.0, 10.0)));
        assert!(!octree.is_active(FixedVec3::from_f32(50.0, 50.0, 50.0)));
        assert_eq!(octree.total_active_voxels, 2);
    }

    #[test]
    fn test_octree_deactivation() {
        let material = MaterialProperties::concrete();
        let mut octree =
            SparseOctree::new(FixedPoint::from_f32(64.0), 6, FixedVec3::ZERO, material);

        octree.activate_voxel(FixedVec3::from_f32(10.0, 5.0, 10.0));
        assert_eq!(octree.total_active_voxels, 1);

        octree.deactivate_voxel(FixedVec3::from_f32(10.0, 5.0, 10.0));
        assert_eq!(octree.total_active_voxels, 0);
        assert!(!octree.is_active(FixedVec3::from_f32(10.0, 5.0, 10.0)));
    }

    #[test]
    fn test_compression_ratio() {
        let material = MaterialProperties::concrete();
        let mut octree =
            SparseOctree::new(FixedPoint::from_f32(64.0), 6, FixedVec3::ZERO, material);

        for i in 0..10 {
            octree.activate_voxel(FixedVec3::from_f32(i as f32, 0.0, 0.0));
        }

        let ratio = octree.compression_ratio();
        assert!(ratio < FixedPoint::ONE);
        assert!(ratio > FixedPoint::ZERO);
    }
}
