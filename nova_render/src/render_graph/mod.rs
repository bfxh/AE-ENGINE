//! RenderGraph（借鉴 kajiya-rg + bevy RenderGraph）
//!
//! 设计：
//! - 声明式：先描述 pass 依赖关系，再执行
//! - 资源 handle：节点输出/输入通过 ResourceHandle 连接
//! - temporal：跨帧资源（如 TAA 历史帧）
//! - imageops：资源描述（尺寸/格式/用法），系统自动分配
//! - 拓扑排序：Kahn 算法保证执行顺序
//! - 资源缓存：复用 GPU 资源

pub mod graph;
pub mod handle;
pub mod passes;
pub mod resource_table;

pub use graph::{RenderGraph, NodeId, Edge, ResourceUsage, ExecuteReport, ResourceCache};
pub use handle::{ResourceHandle, ResourceDesc, ResourceType};
pub use passes::{NodeContext, NodeResult, RenderGraphNode};
pub use resource_table::ResourceTable;