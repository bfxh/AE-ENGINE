//! Scene Graph 模块（借鉴 Fyrox Pool + 强类型 Handle）
//!
//! 设计：
//! - Scene Graph：树状节点结构
//! - Node：Transform + Mesh + Material
//! - Camera：视图矩阵 + 投影矩阵 + jitter
//! - Light：方向光/点光/聚光 + 阴影

pub mod graph;
pub mod node;
pub mod camera;
pub mod light;

pub use graph::{SceneGraph, SceneRoot};
pub use node::{Node, NodeId, NodeData};
pub use camera::{Camera, CameraProjection, CameraUniform};
pub use light::{Light, LightKind, LightUniform};