//! Scene Node

use glam::{Mat4, Vec3, Quat};
use crate::assets::MeshHandle;
use crate::assets::material::MaterialHandle;
use smallvec::SmallVec;

/// Node ID（包装 Handle）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub crate::core::Handle<Node>);

/// Node Transform
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }
}

/// Node 数据
pub struct NodeData {
    pub transform: Transform,
    pub world_transform: Mat4,
    pub mesh: Option<MeshHandle>,
    pub material: Option<MaterialHandle>,
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
    pub visible: bool,
}

impl Default for NodeData {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            world_transform: Mat4::IDENTITY,
            mesh: None,
            material: None,
            children: SmallVec::new(),
            parent: None,
            visible: true,
        }
    }
}

/// Node（包装 NodeData）
pub struct Node(pub NodeData);

impl Default for Node {
    fn default() -> Self { Self(NodeData::default()) }
}