//! RenderGraph Node trait + NodeContext

use super::handle::{ResourceDesc, ResourceHandle};
use super::resource_table::ResourceTable;
use super::graph::ResourceCache;
use anyhow::Result;

/// Node 执行上下文
///
/// 由 `RenderGraph::execute` 在每个节点执行前构造，提供：
/// - `device` / `queue`：wgpu 句柄
/// - `encoder`：命令录制器（pass 在此 begin_render_pass / begin_compute_pass）
/// - `resources`：跨 pass 资源表（handle → TextureView / Buffer）
/// - `surface_view`：最终输出目标（swapchain view，最终 pass 写入此）
/// - `time` / `frame`：时间与帧号（动画 / TAA 用）
pub struct NodeContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub resources: &'a mut ResourceTable,
    /// 最终输出目标（swapchain view），最终 pass 写入此
    pub surface_view: Option<&'a wgpu::TextureView>,
    /// 当前 surface 尺寸 (width, height)，pass 用于创建匹配的 depth/GBuffer 资源
    pub surface_size: (u32, u32),
    /// 当前时间（秒，从启动起）
    pub time: f32,
    /// 当前帧号
    pub frame: u64,
    /// 节点声明的输入 handle（由 edge 推导，可为空）
    pub inputs: Vec<ResourceHandle>,
    /// 节点声明的输出 handle
    pub outputs: Vec<ResourceHandle>,
    /// GPU 资源缓存（按描述哈希复用，跨帧复用）
    pub cache: &'a mut ResourceCache,
    /// 帧内请求的纹理（pass 通过 request_texture 请求，帧末自动释放回缓存）
    pub frame_textures: &'a mut Vec<(ResourceDesc, wgpu::Texture)>,
    /// 帧内请求的 buffer（pass 通过 request_buffer 请求，帧末自动释放回缓存）
    pub frame_buffers: &'a mut Vec<(ResourceDesc, wgpu::Buffer)>,
}

impl<'a> NodeContext<'a> {
    /// 构造完整 NodeContext（RenderGraph::execute 内部调用）
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        encoder: &'a mut wgpu::CommandEncoder,
        resources: &'a mut ResourceTable,
        surface_view: Option<&'a wgpu::TextureView>,
        surface_size: (u32, u32),
        time: f32,
        frame: u64,
        cache: &'a mut ResourceCache,
        frame_textures: &'a mut Vec<(ResourceDesc, wgpu::Texture)>,
        frame_buffers: &'a mut Vec<(ResourceDesc, wgpu::Buffer)>,
    ) -> Self {
        Self {
            device,
            queue,
            encoder,
            resources,
            surface_view,
            surface_size,
            time,
            frame,
            inputs: vec![],
            outputs: vec![],
            cache,
            frame_textures,
            frame_buffers,
        }
    }

    /// 请求自动管理的纹理（从缓存复用或新建，帧末自动释放回缓存）
    ///
    /// pass 无需手动管理纹理生命周期，适合临时 GBuffer / 中间结果。
    /// 返回的 TextureView 归 pass 所有（view 不参与缓存复用）。
    pub fn request_texture(&mut self, desc: ResourceDesc) -> wgpu::TextureView {
        let tex = self.cache.acquire_texture(self.device, &desc);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.frame_textures.push((desc, tex));
        view
    }

    /// 请求自动管理的 buffer（从缓存复用或新建，帧末自动释放回缓存）
    ///
    /// 注意：返回的 buffer 是 clone（wgpu::Buffer 内部是 Arc，clone 开销低），
    /// 原始 buffer 留在 frame_buffers 中用于帧末释放。
    pub fn request_buffer(&mut self, desc: ResourceDesc) -> wgpu::Buffer {
        let buf = self.cache.acquire_buffer(self.device, &desc);
        self.frame_buffers.push((desc, buf.clone()));
        buf
    }

    /// 便捷：查询输入 texture
    pub fn input_texture(&self, h: ResourceHandle) -> Option<&wgpu::TextureView> {
        self.resources.get_texture(h)
    }

    /// 便捷：查询输入 buffer
    pub fn input_buffer(&self, h: ResourceHandle) -> Option<&wgpu::Buffer> {
        self.resources.get_buffer(h)
    }

    /// 便捷：注册输出 texture
    pub fn output_texture(&mut self, h: ResourceHandle, view: wgpu::TextureView) {
        self.resources.insert_texture(h, view);
    }

    /// 便捷：注册输出 buffer
    pub fn output_buffer(&mut self, h: ResourceHandle, buffer: wgpu::Buffer) {
        self.resources.insert_buffer(h, buffer);
    }
}

/// Node 执行结果
pub type NodeResult = Result<()>;

/// RenderGraph Node trait
///
/// 每个实现者代表一个渲染 pass（shadow / forward / post-process / GI 等）。
/// `execute` 内部应通过 `ctx.encoder` 录制 GPU 命令。
///
/// `as_any` / `as_any_mut` 提供 downcast 能力，让外部代码在 pass 加入 RenderGraph 后
/// 仍能按具体类型访问 pass 的配置方法（如 `ForwardPass::update_camera`）。
/// 实现者可使用 [`impl_rgn_downcast`] 宏自动填充这两个方法。
pub trait RenderGraphNode: Send + Sync {
    /// 节点名称（调试 / 日志用）
    fn name(&self) -> &str;

    /// 执行节点：录制 GPU 命令到 `ctx.encoder`
    ///
    /// 约定：
    /// - 通过 `ctx.resources.get_texture(handle)` 读取上游 pass 输出
    /// - 通过 `ctx.resources.insert_texture(handle, view)` 注册本 pass 输出
    /// - 最终 pass 应写入 `ctx.surface_view`（swapchain）
    /// - 失败时返回 Err，RenderGraph 会记录但继续执行其他节点
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult;

    /// 返回 `&dyn Any` 用于 downcast（配合 [`RenderGraph::node_mut`](crate::render_graph::RenderGraph::node_mut)）
    fn as_any(&self) -> &dyn std::any::Any;

    /// 返回 `&mut dyn Any` 用于 downcast
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// 为 `RenderGraphNode` 实现者批量提供 `as_any` / `as_any_mut` 方法
///
/// 用法：在 `impl RenderGraphNode for XxxPass` 块内调用 `crate::impl_rgn_downcast!();`
/// （crate 外部使用 `nova_render::impl_rgn_downcast!();`）
#[macro_export]
macro_rules! impl_rgn_downcast {
    () => {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}
