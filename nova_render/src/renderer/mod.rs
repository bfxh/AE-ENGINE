//! Renderer 模块（借鉴 bevy PhaseItem + RenderPhase）
//!
//! 设计：
//! - PhaseItem：单个 draw call 数据（mesh + material + transform）
//! - RenderPhase：相同渲染阶段的 PhaseItem 集合
//! - Culling：BVH + 视锥剔除
//! - Batch：合批小 mesh

pub mod phase;
pub mod culling;
pub mod batch;

pub use phase::{PhaseItem, RenderPhase, PhaseId};
pub use culling::{Culling, CullingResult, Frustum};
pub use batch::{Batch, BatchedItem};

pub mod cluster_lod;
pub use cluster_lod::{ClusterLod, Cluster, ClusterLodUniform, DrawIndexedArgs, DrawCount, ClusterLodStats};