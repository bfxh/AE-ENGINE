//! Shadow Pass（CSM 级联阴影调度入口）
//!
//! 设计：
//! - 级联阴影（CSM）：多级 cascade，每级独立 ShadowMapPass 实例
//! - 每级 cascade 用递增 scene_radius 切分视锥体（近处高分辨率，远处低分辨率）
//! - PCF 软阴影由 ShadowMapPass 的 comparison sampler 实现
//! - Point Light 立体阴影：待后续扩展 [补充]
//!
//! 调度策略：方案 A —— 本 pass 内为每级 cascade 调用 ShadowMapPass::execute()
//! （PCF 软阴影在主渲染 pass 的 fragment shader 中完成）
//!
//! 注意：当前 ForwardPass shader 仅采样单级 shadow map（cascade 0）。
//! 多级 cascade 采样需要扩展 ForwardPass shader [补充]。

use crate::passes::shadow_map::{ShadowInstance, ShadowMapPass, ShadowVertex};
use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// ShadowPass：CSM 级联阴影调度入口
///
/// 持有 `cascades` 个 ShadowMapPass 实例，每级独立 depth texture + VP 矩阵。
/// execute() 顺序调用每级 ShadowMapPass::execute()。
pub struct ShadowPass {
    pub resolution: u32,
    pub cascades: u32,
    /// 每级 cascade 的 ShadowMapPass 实例
    pub cascade_passes: Vec<ShadowMapPass>,
    /// 是否已初始化（with_cascades 调用后为 true）
    pub initialized: bool,
}

impl ShadowPass {
    /// 默认配置（未初始化，需调用 with_cascades 初始化 GPU 资源）
    pub fn new() -> Self {
        Self {
            resolution: 2048,
            cascades: 4,
            cascade_passes: Vec::new(),
            initialized: false,
        }
    }

    /// 创建并初始化 CSM（创建 cascades 个 ShadowMapPass 实例）
    pub fn with_cascades(device: &wgpu::Device, resolution: u32, cascades: u32) -> Self {
        let n = cascades.max(1);
        let cascade_passes = (0..n)
            .map(|_| ShadowMapPass::new(device, resolution))
            .collect();
        Self {
            resolution,
            cascades: n,
            cascade_passes,
            initialized: true,
        }
    }

    /// 注册 mesh 到所有 cascade（每级独立 vertex/index buffer）
    pub fn register_mesh(
        &mut self,
        device: &wgpu::Device,
        vertices: &[ShadowVertex],
        indices: &[u32],
    ) {
        for cascade in &mut self.cascade_passes {
            cascade.register_mesh(device, vertices, indices);
        }
    }

    /// 更新所有 cascade 的实例数据（model matrix）
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[ShadowInstance]) {
        for cascade in &mut self.cascade_passes {
            cascade.update_instances(queue, instances);
        }
    }

    /// 设置所有 cascade 的实例数
    pub fn set_instance_count(&mut self, count: u32) {
        for cascade in &mut self.cascade_passes {
            cascade.set_instance_count(count);
        }
    }

    /// 更新所有 cascade 的光源 VP 矩阵
    ///
    /// 每级 cascade 用递增 scene_radius 切分视锥体（简化版 CSM split）：
    /// - cascade 0: scene_radius * 0.15（近处，高分辨率）
    /// - cascade 1: scene_radius * 0.35
    /// - cascade 2: scene_radius * 0.65
    /// - cascade 3+: scene_radius * 1.0（远处，低分辨率）
    ///
    /// [补充] 后续可改为基于对数/线性混合的精确 cascade split
    pub fn update_cascade_matrices(
        &mut self,
        queue: &wgpu::Queue,
        light_dir: [f32; 3],
        camera_pos: [f32; 3],
        scene_radius: f32,
    ) {
        for (i, cascade) in self.cascade_passes.iter_mut().enumerate() {
            let split = match i {
                0 => scene_radius * 0.15,
                1 => scene_radius * 0.35,
                2 => scene_radius * 0.65,
                _ => scene_radius * 1.0,
            };
            cascade.update_light_matrix(queue, light_dir, camera_pos, split);
        }
    }

    /// 获取第 0 级 cascade 的 shadow bind group（供主渲染 pass 采样）
    ///
    /// [补充] 多级 cascade 采样需要扩展主渲染 shader 支持纹理数组
    pub fn shadow_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.cascade_passes.first().map(|c| c.shadow_bind_group())
    }

    /// 获取第 0 级 cascade 的 shadow layout（供主渲染 pipeline 使用）
    pub fn shadow_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.cascade_passes.first().map(|c| c.shadow_layout())
    }
}

impl Default for ShadowPass {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphNode for ShadowPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "shadow"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        if !self.initialized || self.cascade_passes.is_empty() {
            log::debug!(
                "shadow: CSM not initialized (cascades={}), skipping",
                self.cascades
            );
            return Ok(());
        }
        // 顺序执行每级 cascade 的 ShadowMapPass::execute()
        // 每级 cascade 独立 begin_render_pass（清空自己的 depth texture 为 1.0）
        for (i, cascade) in self.cascade_passes.iter_mut().enumerate() {
            log::trace!("shadow: executing cascade {}", i);
            cascade.execute(ctx)?;
        }
        log::debug!(
            "shadow: CSM rendered {} cascade(s) at resolution {}",
            self.cascade_passes.len(),
            self.resolution
        );
        Ok(())
    }
}
