//! TAA Pass（port 自 v1 ae_render::taa）
//!
//! 时域抗锯齿：利用历史帧信息在时域上累积样本，配合亚像素抖动实现全屏抗锯齿。
//!
//! 流程：
//! 1. **Jitter**：每帧用 Halton 序列生成亚像素偏移，注入投影矩阵
//! 2. **Reproject**：从当前帧深度重建世界坐标，用上一帧 view-projection 重新投影
//! 3. **Neighborhood clamp**：3x3 邻域 min/max 包围盒，将历史帧 clamp 进去（抑制鬼影）
//! 4. **Blend**：`result = mix(history_clamped, current, blend_factor)`
//!
//! 历史帧采用 ping-pong 双缓冲。

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline, Sampler, Texture, TextureView};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// TAA Uniform（304 bytes = 4 × mat4x4 + 3 × vec4，符合 WGSL 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct TaaUniform {
    /// 当前帧 view-projection（含 jitter）
    pub view_proj: [[f32; 4]; 4],
    /// 当前帧 view-projection（含 jitter，同上）
    pub view_proj_jitter: [[f32; 4]; 4],
    /// 上一帧 view-projection（含 jitter）
    pub prev_view_proj: [[f32; 4]; 4],
    /// 逆 view-projection（用于从 NDC+深度重建世界坐标）
    pub view_inv: [[f32; 4]; 4],
    /// xy=当前帧 jitter, zw=上一帧 jitter
    pub jitter: [f32; 4],
    /// x=blend_factor(0.1), y=vel_scale, z=enable(1.0), w=unused
    pub params: [f32; 4],
    /// x=width, y=height, z=1/width, w=1/height
    pub screen_size: [f32; 4],
}

impl TaaUniform {
    /// 创建默认参数 TAA uniform
    pub fn new(
        view_proj: [[f32; 4]; 4],
        prev_view_proj: [[f32; 4]; 4],
        view_inv: [[f32; 4]; 4],
        jitter: [f32; 2],
        prev_jitter: [f32; 2],
        width: f32,
        height: f32,
    ) -> Self {
        let safe_w = width.max(1.0);
        let safe_h = height.max(1.0);
        Self {
            view_proj,
            view_proj_jitter: view_proj,
            prev_view_proj,
            view_inv,
            jitter: [jitter[0], jitter[1], prev_jitter[0], prev_jitter[1]],
            params: [0.1, 100.0, 1.0, 0.0],
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
        }
    }
}

const TAA_SHADER: &str = r#"
struct TaaUniform {
    view_proj: mat4x4<f32>,
    view_proj_jitter: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    jitter: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
};

@group(0) @binding(0) var t_current: texture_2d<f32>;
@group(0) @binding(1) var t_history: texture_2d<f32>;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_linear: sampler;
@group(0) @binding(4) var s_depth: sampler;
@group(0) @binding(5) var<uniform> u: TaaUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

@fragment
fn fs_taa(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let texel = vec2<f32>(u.screen_size.z, u.screen_size.w);
    let uv = pos.xy / dims;

    let current_color = textureSampleLevel(t_current, s_linear, uv, 0.0).rgb;

    if (u.params.z < 0.5) {
        return vec4<f32>(current_color, 1.0);
    }

    let depth = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    if (depth >= 1.0) {
        return vec4<f32>(current_color, 1.0);
    }

    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world_h = u.view_inv * ndc;
    let world_pos = world_h.xyz / world_h.w;

    let prev_clip = u.prev_view_proj * vec4<f32>(world_pos, 1.0);
    let prev_ndc = prev_clip.xyz / prev_clip.w;
    let prev_uv = vec2<f32>(prev_ndc.x * 0.5 + 0.5, 0.5 - prev_ndc.y * 0.5);

    let on_screen = all(prev_uv >= vec2<f32>(0.0, 0.0)) && all(prev_uv <= vec2<f32>(1.0, 1.0));

    var history_color = textureSampleLevel(t_history, s_linear, prev_uv, 0.0).rgb;
    if (!on_screen) {
        history_color = current_color;
    }

    var neighbor_min = current_color;
    var neighbor_max = current_color;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let c = textureSampleLevel(t_current, s_linear, uv + offset, 0.0).rgb;
            neighbor_min = min(neighbor_min, c);
            neighbor_max = max(neighbor_max, c);
        }
    }

    let history_clamped = clamp(history_color, neighbor_min, neighbor_max);

    let velocity = (prev_uv - uv) * u.params.y;
    let speed = length(velocity);
    var blend_factor = clamp(u.params.x + speed * 2.0, 0.0, 1.0);
    if (!on_screen) {
        blend_factor = 1.0;
    }

    let result = mix(history_clamped, current_color, blend_factor);
    return vec4<f32>(result, 1.0);
}
"#;

/// TAA Pass（port 自 v1 ae_render::TaaRenderer）
///
/// 管理 TAA resolve 管线与 ping-pong 历史帧。
pub struct TaaPass {
    pub taa_pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub uniform_layout: BindGroupLayout,
    pub sampler: Sampler,
    pub depth_sampler: Sampler,
    /// Ping-pong 历史帧纹理。`[history_index]` 本帧读，`[1-history_index]` 本帧写
    pub history_textures: [Option<Texture>; 2],
    pub history_views: [Option<TextureView>; 2],
    pub history_index: u32,
}

impl TaaPass {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        _depth_format: wgpu::TextureFormat,
    ) -> Self {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<TaaUniform>() as u64) }, count: None },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("taa uniform buffer"),
            size: std::mem::size_of::<TaaUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("taa shader"), source: wgpu::ShaderSource::Wgsl(TAA_SHADER.into()) });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("taa pipeline layout"), bind_group_layouts: &[&uniform_layout], push_constant_ranges: &[] });
        let taa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("taa pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_fullscreen"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_taa"), compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format: hdr_format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });

        Self { taa_pipeline, uniform_buffer, uniform_layout, sampler, depth_sampler, history_textures: [None, None], history_views: [None, None], history_index: 0 }
    }

    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &TaaUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn create_bind_group(&self, device: &wgpu::Device, current_view: &TextureView, history_view: &TextureView, depth_view: &TextureView) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("taa bind group"), layout: &self.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(current_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(history_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(depth_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.depth_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: self.uniform_buffer.as_entire_binding() },
            ],
        })
    }

    pub fn create_history_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> Texture {
        let safe_w = width.max(1);
        let safe_h = height.max(1);
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: safe_w, height: safe_h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    pub fn init_history(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let tex0 = Self::create_history_texture(device, width, height, "taa history 0");
        let tex1 = Self::create_history_texture(device, width, height, "taa history 1");
        let view0 = tex0.create_view(&wgpu::TextureViewDescriptor::default());
        let view1 = tex1.create_view(&wgpu::TextureViewDescriptor::default());
        self.history_textures = [Some(tex0), Some(tex1)];
        self.history_views = [Some(view0), Some(view1)];
        self.history_index = 0;
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.init_history(device, width, height);
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.taa_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn swap_history(&mut self) {
        self.history_index = 1 - self.history_index;
    }

    pub fn history_view(&self) -> Option<&TextureView> {
        self.history_views.get(self.history_index as usize)?.as_ref()
    }

    pub fn current_output_view(&self) -> Option<&TextureView> {
        let write_idx = (1 - self.history_index) as usize;
        self.history_views.get(write_idx)?.as_ref()
    }
}

impl RenderGraphNode for TaaPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str { "taa" }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 1. history ping-pong 必须已 init（init_history / resize）
        if self.history_views[0].is_none() || self.history_views[1].is_none() {
            log::warn!("taa: history 未初始化（先调用 init_history），跳过");
            return Ok(());
        }

        // 2. 输入：inputs[0]=当前帧颜色（含 jitter），inputs[1]=深度
        if ctx.inputs.len() < 2 {
            log::warn!(
                "taa: inputs 不足（{}，需要 2: current/depth），跳过",
                ctx.inputs.len()
            );
            return Ok(());
        }
        let current_view = match ctx.resources.get_texture(ctx.inputs[0]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("taa: current color texture 缺失，跳过");
                return Ok(());
            }
        };
        let depth_view = match ctx.resources.get_texture(ctx.inputs[1]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("taa: depth texture 缺失，跳过");
                return Ok(());
            }
        };

        // 3. history 读视图（本帧读）+ bind_group
        let history_view = self.history_views[self.history_index as usize]
            .clone()
            .unwrap();
        let bg = self.create_bind_group(ctx.device, &current_view, &history_view, &depth_view);

        // 4. 输出写入 history 写视图（write_idx），下帧读；全屏三角形覆盖所有像素，
        //    LoadOp::Clear 仅作占位（每个像素都被 fs_taa 覆写）。
        let write_idx = (1 - self.history_index) as usize;
        let output_view = self.history_views[write_idx].clone().unwrap();
        {
            let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("taa render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.draw(&mut rpass, &bg);
        }

        // 5. 交换 ping-pong 索引：下帧读本帧写
        self.swap_history();
        Ok(())
    }
}

/// 生成 Halton 序列抖动值（返回 [0, 1)）
pub fn halton_sequence(index: u32, base: u32) -> f32 {
    if base == 0 { return 0.0; }
    let mut f = 1.0_f32;
    let mut r = 0.0_f32;
    let mut i = index;
    let b = base as f32;
    while i > 0 {
        f /= b;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

/// 获取帧的 jitter 偏移（返回 [-0.5, 0.5] 范围的 xy）
pub fn frame_jitter(frame_index: u32) -> [f32; 2] {
    let x = halton_sequence(frame_index + 1, 2) - 0.5;
    let y = halton_sequence(frame_index + 1, 3) - 0.5;
    [x, y]
}

/// 应用 jitter 到投影矩阵（返回带 jitter 的投影矩阵）
pub fn apply_jitter_to_projection(proj: Mat4, jitter: [f32; 2], width: f32, height: f32) -> Mat4 {
    if jitter[0] == 0.0 && jitter[1] == 0.0 {
        return proj;
    }
    let safe_w = width.max(1.0);
    let safe_h = height.max(1.0);
    let ndc_x = jitter[0] / safe_w;
    let ndc_y = jitter[1] / safe_h;
    let jitter_mat = Mat4::from_translation(Vec3::new(ndc_x, ndc_y, 0.0));
    jitter_mat * proj
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uniform_size() { assert_eq!(std::mem::size_of::<TaaUniform>(), 304); }
    #[test]
    fn uniform_default() {
        let u = TaaUniform::default();
        assert_eq!(u.jitter, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(u.params, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(u.screen_size, [0.0, 0.0, 0.0, 0.0]);
    }
    #[test]
    fn uniform_field_offsets() {
        assert_eq!(std::mem::offset_of!(TaaUniform, view_proj), 0);
        assert_eq!(std::mem::offset_of!(TaaUniform, view_proj_jitter), 64);
        assert_eq!(std::mem::offset_of!(TaaUniform, prev_view_proj), 128);
        assert_eq!(std::mem::offset_of!(TaaUniform, view_inv), 192);
        assert_eq!(std::mem::offset_of!(TaaUniform, jitter), 256);
        assert_eq!(std::mem::offset_of!(TaaUniform, params), 272);
        assert_eq!(std::mem::offset_of!(TaaUniform, screen_size), 288);
    }
    #[test]
    fn halton_known_values() {
        assert!((halton_sequence(1, 2) - 0.5).abs() < 1e-6);
        assert!((halton_sequence(2, 2) - 0.25).abs() < 1e-6);
        assert!((halton_sequence(3, 2) - 0.75).abs() < 1e-6);
        assert!((halton_sequence(1, 3) - 1.0 / 3.0).abs() < 1e-6);
        assert!((halton_sequence(2, 3) - 2.0 / 3.0).abs() < 1e-6);
        assert!((halton_sequence(0, 2) - 0.0).abs() < 1e-6);
        assert!((halton_sequence(1, 0) - 0.0).abs() < 1e-6);
    }
    #[test]
    fn frame_jitter_range() {
        for i in 0..32 {
            let j = frame_jitter(i);
            assert!(j[0] >= -0.5 && j[0] <= 0.5);
            assert!(j[1] >= -0.5 && j[1] <= 0.5);
        }
    }
    #[test]
    fn apply_jitter_zero_returns_original() {
        let proj = Mat4::IDENTITY;
        let result = apply_jitter_to_projection(proj, [0.0, 0.0], 1920.0, 1080.0);
        assert_eq!(result.to_cols_array(), proj.to_cols_array());
    }
    #[test]
    fn apply_jitter_nonzero_modifies_matrix() {
        let proj = Mat4::IDENTITY;
        let result = apply_jitter_to_projection(proj, [0.5, -0.5], 1920.0, 1080.0);
        assert_ne!(result.to_cols_array(), proj.to_cols_array());
    }
    #[test]
    fn taa_uniform_new_builder() {
        let vp = [[1.0; 4]; 4];
        let pvp = [[2.0; 4]; 4];
        let vi = [[3.0; 4]; 4];
        let u = TaaUniform::new(vp, pvp, vi, [0.25, -0.25], [0.1, 0.1], 1920.0, 1080.0);
        assert_eq!(u.view_proj, vp);
        assert_eq!(u.view_proj_jitter, vp);
        assert_eq!(u.prev_view_proj, pvp);
        assert_eq!(u.view_inv, vi);
        assert_eq!(u.jitter, [0.25, -0.25, 0.1, 0.1]);
        assert_eq!(u.params, [0.1, 100.0, 1.0, 0.0]);
        assert_eq!(u.screen_size, [1920.0, 1080.0, 1.0 / 1920.0, 1.0 / 1080.0]);
    }
}
