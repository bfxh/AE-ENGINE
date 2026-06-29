//! RenderGraph DAG（升级版）
//!
//! 设计借鉴：
//! - kajiya-rg：声明式 DAG + temporal 资源
//! - bevy RenderGraph：节点 + slot 系统
//! - Granite：资源生命周期 + barrier 自动管理
//!
//! 核心能力：
//! - 拓扑排序（Kahn 算法）保证执行顺序正确
//! - 资源池：自动分配/复用 GPU 资源
//! - 资源生命周期：根据读写边自动插入 barrier
//! - 跨帧资源（temporal）：TAA 历史帧等

use super::handle::{ResourceHandle, ResourceDesc};
use super::passes::{NodeContext, RenderGraphNode};
use super::resource_table::ResourceTable;
use hashbrown::HashMap;
use std::collections::VecDeque;

/// Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// 资源使用模式（决定 barrier 类型）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUsage {
    /// 只读（Texture read / Buffer read）
    Read,
    /// 读写（RenderTarget / Storage write）
    Write,
    /// 读写（同时读写）
    ReadWrite,
}

/// 边：节点间资源依赖
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub resource: ResourceHandle,
    pub usage: ResourceUsage,
}

/// RenderGraph 调度器
pub struct RenderGraph {
    nodes: HashMap<NodeId, Box<dyn RenderGraphNode>>,
    edges: Vec<Edge>,
    next_id: u64,
    /// 资源描述表（资源 handle → 描述）
    resource_descs: HashMap<ResourceHandle, ResourceDesc>,
    /// 已分配的 GPU 资源（缓存复用）
    resource_cache: ResourceCache,
    /// 拓扑顺序缓存
    topo_order: Vec<NodeId>,
    dirty: bool,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 0,
            resource_descs: HashMap::new(),
            resource_cache: ResourceCache::new(),
            topo_order: Vec::new(),
            dirty: true,
        }
    }

    pub fn add_node(&mut self, node: Box<dyn RenderGraphNode>) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, node);
        self.dirty = true;
        id
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
        self.dirty = true;
    }

    /// 声明资源（在添加边之前调用）
    pub fn declare_resource(&mut self, handle: ResourceHandle, desc: ResourceDesc) {
        self.resource_descs.insert(handle, desc);
    }

    /// 拓扑排序（Kahn 算法）
    ///
    /// 如果检测到环，返回 Err。
    pub fn topological_sort(&mut self) -> Result<Vec<NodeId>, String> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for &id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }
        // 邻接表：from → [to]
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for edge in &self.edges {
            adj.entry(edge.from).or_default().push(edge.to);
            *in_degree.entry(edge.to).or_insert(0) += 1;
        }
        // 入度 0 的节点入队
        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&n, _)| n)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(n) = queue.pop_front() {
            order.push(n);
            if let Some(nexts) = adj.get(&n) {
                for &next in nexts {
                    let d = in_degree.get_mut(&next).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            let cycle_nodes: Vec<_> = self
                .nodes
                .keys()
                .filter(|n| !order.contains(n))
                .copied()
                .collect();
            return Err(format!("RenderGraph contains cycle, nodes: {:?}", cycle_nodes));
        }
        // 按 NodeId 稳定排序（同入度时优先 id 小的，便于调试）
        // 注意：topological_sort 已保证依赖顺序，这里不再二次排序
        Ok(order)
    }

    /// 执行整个图
    ///
    /// 流程：
    /// 1. 拓扑排序（若 dirty）
    /// 2. 按顺序执行每个节点，节点通过 `ctx.encoder` 录制 GPU 命令
    /// 3. 节点失败时记录但继续（避免一个 pass 失败导致整图崩溃）
    ///
    /// 参数：
    /// - `encoder`：命令录制器（调用方负责 `encoder.finish()` 并 submit）
    /// - `resources`：跨 pass 资源表（每帧调用前应 `clear()`）
    /// - `surface_view`：swapchain view，最终 pass 写入此
    /// - `surface_size`：swapchain 尺寸 (w, h)，pass 用于创建匹配的 depth/GBuffer
    /// - `time` / `frame`：时间与帧号
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut ResourceTable,
        surface_view: Option<&wgpu::TextureView>,
        surface_size: (u32, u32),
        time: f32,
        frame: u64,
    ) -> ExecuteReport {
        if self.dirty {
            match self.topological_sort() {
                Ok(o) => {
                    self.topo_order = o;
                    self.dirty = false;
                }
                Err(e) => {
                    log::error!("RenderGraph topological sort failed: {}", e);
                    return ExecuteReport {
                        nodes_executed: 0,
                        nodes_failed: 0,
                        errors: vec![e],
                    };
                }
            }
        }
        let order = self.topo_order.clone();
        let mut report = ExecuteReport::default();
        // 帧内请求的资源（pass 通过 ctx.request_texture 请求，帧末自动释放回缓存）
        let mut frame_textures: Vec<(ResourceDesc, wgpu::Texture)> = Vec::new();
        let mut frame_buffers: Vec<(ResourceDesc, wgpu::Buffer)> = Vec::new();
        for id in order {
            if let Some(node) = self.nodes.get_mut(&id) {
                let mut ctx = NodeContext::new(
                    device, queue, encoder, resources, surface_view, surface_size, time, frame,
                    &mut self.resource_cache, &mut frame_textures, &mut frame_buffers,
                );
                match node.execute(&mut ctx) {
                    Ok(()) => report.nodes_executed += 1,
                    Err(e) => {
                        report.nodes_failed += 1;
                        report.errors.push(format!("{}: {}", node.name(), e));
                        log::warn!("RenderGraph node '{}' failed: {}", node.name(), e);
                    }
                }
            }
        }
        // 帧末释放所有请求的资源回缓存（跨帧复用，减少 GPU allocate 开销）
        for (desc, tex) in frame_textures {
            self.resource_cache.release_texture(&desc, tex);
        }
        for (desc, buf) in frame_buffers {
            self.resource_cache.release_buffer(&desc, buf);
        }
        report
    }

    /// 节点数
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 按具体类型获取节点的可变引用（downcast）
    ///
    /// 用途：pass 加入 RenderGraph 后，外部仍能调用 pass 特有的配置方法。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let id = graph.add_node(Box::new(ForwardPass::new(...)));
    /// if let Some(forward) = graph.node_mut::<ForwardPass>(id) {
    ///     forward.update_camera(queue, &camera_uniform);
    ///     forward.set_instance_count(1);
    /// }
    /// ```
    pub fn node_mut<T: RenderGraphNode + 'static>(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes
            .get_mut(&id)
            .and_then(|n| n.as_any_mut().downcast_mut::<T>())
    }

    /// 按具体类型获取节点的不可变引用（downcast）
    pub fn node_ref<T: RenderGraphNode + 'static>(&self, id: NodeId) -> Option<&T> {
        self.nodes
            .get(&id)
            .and_then(|n| n.as_any().downcast_ref::<T>())
    }

    /// 清空图（保留资源缓存）
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.topo_order.clear();
        self.dirty = true;
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行报告
#[derive(Debug, Default, Clone)]
pub struct ExecuteReport {
    pub nodes_executed: usize,
    pub nodes_failed: usize,
    pub errors: Vec<String>,
}

impl ExecuteReport {
    pub fn is_ok(&self) -> bool {
        self.nodes_failed == 0
    }
}

/// GPU 资源缓存（按描述哈希复用）
///
/// 当多个 pass 请求相同描述的资源时，可以复用已分配的 GPU 资源，
/// 减少 allocate/free 开销。
pub struct ResourceCache {
    textures: HashMap<u64, Vec<wgpu::Texture>>,
    buffers: HashMap<u64, Vec<wgpu::Buffer>>,
}

impl ResourceCache {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    /// 请求一个纹理（命中缓存则复用，否则新建）
    pub fn acquire_texture(
        &mut self,
        device: &wgpu::Device,
        desc: &ResourceDesc,
    ) -> wgpu::Texture {
        let key = resource_hash(desc);
        if let Some(pool) = self.textures.get_mut(&key) {
            if let Some(tex) = pool.pop() {
                return tex;
            }
        }
        device.create_texture(&wgpu::TextureDescriptor {
            label: desc.label.as_deref(),
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format.unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb),
            usage: desc.usage,
            view_formats: &[],
        })
    }

    /// 归还纹理（生命周期结束）
    pub fn release_texture(&mut self, desc: &ResourceDesc, tex: wgpu::Texture) {
        let key = resource_hash(desc);
        self.textures.entry(key).or_default().push(tex);
    }

    /// 请求一个 buffer（命中缓存则复用，否则新建）
    pub fn acquire_buffer(
        &mut self,
        device: &wgpu::Device,
        desc: &ResourceDesc,
    ) -> wgpu::Buffer {
        let key = resource_hash(desc);
        if let Some(pool) = self.buffers.get_mut(&key) {
            if let Some(buf) = pool.pop() {
                return buf;
            }
        }
        device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label.as_deref(),
            size: desc.buffer_size,
            usage: desc.buffer_usage,
            mapped_at_creation: false,
        })
    }

    /// 归还 buffer（生命周期结束）
    pub fn release_buffer(&mut self, desc: &ResourceDesc, buf: wgpu::Buffer) {
        let key = resource_hash(desc);
        self.buffers.entry(key).or_default().push(buf);
    }

    /// 清空缓存（释放 GPU 内存）
    pub fn clear(&mut self) {
        self.textures.clear();
        self.buffers.clear();
    }
}

fn resource_hash(desc: &ResourceDesc) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    desc.ty.hash(&mut h);
    desc.width.hash(&mut h);
    desc.height.hash(&mut h);
    desc.format.hash(&mut h);
    desc.usage.hash(&mut h);
    desc.buffer_size.hash(&mut h);
    desc.buffer_usage.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::passes::NodeResult;

    #[test]
    fn test_topological_sort_linear() {
        // 0 → 1 → 2
        let mut g = RenderGraph::new();
        struct N(&'static str);
        impl RenderGraphNode for N {
            crate::impl_rgn_downcast!();
            fn name(&self) -> &str { self.0 }
            fn execute(&mut self, _: &mut NodeContext) -> NodeResult { Ok(()) }
        }
        let n0 = g.add_node(Box::new(N("n0")));
        let n1 = g.add_node(Box::new(N("n1")));
        let n2 = g.add_node(Box::new(N("n2")));
        g.add_edge(Edge { from: n0, to: n1, resource: ResourceHandle::new(0), usage: ResourceUsage::Read });
        g.add_edge(Edge { from: n1, to: n2, resource: ResourceHandle::new(1), usage: ResourceUsage::Read });
        let order = g.topological_sort().unwrap();
        assert_eq!(order, vec![n0, n1, n2]);
    }

    #[test]
    fn test_topological_sort_detects_cycle() {
        let mut g = RenderGraph::new();
        struct N(&'static str);
        impl RenderGraphNode for N {
            crate::impl_rgn_downcast!();
            fn name(&self) -> &str { self.0 }
            fn execute(&mut self, _: &mut NodeContext) -> NodeResult { Ok(()) }
        }
        let n0 = g.add_node(Box::new(N("n0")));
        let n1 = g.add_node(Box::new(N("n1")));
        g.add_edge(Edge { from: n0, to: n1, resource: ResourceHandle::new(0), usage: ResourceUsage::Read });
        g.add_edge(Edge { from: n1, to: n0, resource: ResourceHandle::new(1), usage: ResourceUsage::Read });
        let r = g.topological_sort();
        assert!(r.is_err());
    }
}
