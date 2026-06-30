//! Bloom Pass（port 自 v1 ae_render::post_process）
//!
//! 3A 级 Bloom 管线：
//! 1. **Bloom Extract**：从 HDR 纹理提取高亮度区域（soft knee 软阈值）
//! 2. **Bloom Blur**：9-tap 高斯模糊（可分离，水平/垂直多 pass）
//!
//! Tonemap 合成在 nova 的 tonemap.rs 中单独处理。
//! 使用全屏三角形（无需 vertex buffer）。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// Bloom Uniform（32 bytes = 8 × f32，符合 WGSL 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct BloomUniform {
    /// 曝光
    pub exposure: f32,
    /// Gamma
    pub gamma: f32,
    /// Bloom 亮度阈值
    pub bloom_threshold: f32,
    /// Bloom 强度
    pub bloom_intensity: f32,
    /// 时间
    pub time: f32,
    /// UV 方向（水平/垂直模糊控制）：xy 方向
    pub blur_dir: [f32; 2],
    /// padding
    pub _pad: f32,
}

impl BloomUniform {
    pub fn new(exposure: f32, gamma: f32, bloom_threshold: f32, bloom_intensity: f32) -> Self {
        Self { exposure, gamma, bloom_threshold, bloom_intensity, time: 0.0, blur_dir: [1.0, 0.0], _pad: 0.0 }
    }

    pub fn default_quality() -> Self {
        Self::new(1.0, 2.2, 1.0, 0.4)
    }
}

/// Bloom 效果配置（EffectStack 兼容）
pub struct BloomEffect {
    pub threshold: f32,
    pub intensity: f32,
    pub mip_levels: u32,
}

impl Default for BloomEffect {
    fn default() -> Self {
        Self { threshold: 1.0, intensity: 0.8, mip_levels: 5 }
    }
}

/// Bloom Extract Shader（WGSL）
const BLOOM_EXTRACT_SHADER: &str = r#"
struct PostProcessUniform {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    time: f32,
    blur_dir_x: f32,
    blur_dir_y: f32,
    _pad: f32,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: PostProcessUniform;

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
fn fs_extract(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2<f32>(textureDimensions(t_src));
    let color = textureSample(t_src, s_src, uv).rgb;
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let threshold = u.bloom_threshold;
    let soft_knee = 0.5;
    let knee = threshold * soft_knee + 1e-5;
    let soft = luminance - threshold + knee;
    let soft_factor = clamp(soft / (2.0 * knee), 0.0, 1.0);
    let contribution = soft_factor * soft_factor;
    let bright = max(color - vec3<f32>(threshold), vec3<f32>(0.0)) * contribution;
    return vec4<f32>(bright, 1.0);
}
"#;

/// Bloom Blur Shader（WGSL，9-tap 高斯模糊，可分离）
const BLOOM_BLUR_SHADER: &str = r#"
struct PostProcessUniform {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    time: f32,
    blur_dir_x: f32,
    blur_dir_y: f32,
    _pad: f32,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: PostProcessUniform;

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
fn fs_blur(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_src);
    let uv = pos.xy / vec2<f32>(dims);
    let texel = 1.0 / vec2<f32>(dims);
    let dir = vec2<f32>(u.blur_dir_x, u.blur_dir_y) * texel;

    let weights = array<f32, 9>(
        0.0625, 0.09375, 0.125, 0.15625, 0.1875,
        0.15625, 0.125, 0.09375, 0.0625,
    );

    var sum = vec3<f32>(0.0);
    for (var i = -4; i <= 4; i = i + 1) {
        let offset = dir * f32(i);
        let w = weights[i + 4];
        sum = sum + textureSample(t_src, s_src, uv + offset).rgb * w;
    }
    return vec4<f32>(sum, 1.0);
}
"#;

/// Bloom Pass（port 自 v1 ae_render::PostProcessRenderer 的 Bloom 部分）
///
/// 使用方式：
/// ```ignore
/// let bloom = BloomPass::new(&device, hdr_format);
/// bloom.update_uniform(&queue, &bloom_uniform);
/// let bg = bloom.create_bind_group(&device, &hdr_view);
/// bloom.draw_extract(&mut pass, &bg);
/// ```
pub struct BloomPass {
    pub extract_pipeline: RenderPipeline,
    pub blur_pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub uniform_layout: BindGroupLayout,
    pub sampler: wgpu::Sampler,
    /// extract pipeline 的 color target 格式（构造时确定，用于创建匹配的中间纹理）
    pub hdr_format: wgpu::TextureFormat,
    /// Bloom extract 输出纹理（lazy 创建，匹配 surface 尺寸）
    pub bloom_target: Option<wgpu::Texture>,
    /// Bloom extract 输出 view
    pub bloom_target_view: Option<wgpu::TextureView>,
    /// 当前 bloom_target 尺寸（用于检测是否需要重建）
    pub bloom_target_size: (u32, u32),
}

impl BloomPass {
    pub fn new(device: &wgpu::Device, hdr_format: wgpu::TextureFormat) -> Self {
        // texture + sampler + uniform 的 bind group layout
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom uniform layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<BloomUniform>() as u64) }, count: None },
            ],
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bloom uniform buffer"),
            contents: bytemuck::cast_slice(&[BloomUniform::default_quality()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bloom Extract pipeline
        let extract_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("bloom extract shader"), source: wgpu::ShaderSource::Wgsl(BLOOM_EXTRACT_SHADER.into()) });
        let extract_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("bloom extract layout"), bind_group_layouts: &[&uniform_layout], push_constant_ranges: &[] });
        let extract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom extract pipeline"), layout: Some(&extract_layout),
            vertex: wgpu::VertexState { module: &extract_shader, entry_point: Some("vs_fullscreen"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &extract_shader, entry_point: Some("fs_extract"), compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format: hdr_format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });

        // Blur pipeline
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("bloom blur shader"), source: wgpu::ShaderSource::Wgsl(BLOOM_BLUR_SHADER.into()) });
        let blur_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("bloom blur layout"), bind_group_layouts: &[&uniform_layout], push_constant_ranges: &[] });
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom blur pipeline"), layout: Some(&blur_layout),
            vertex: wgpu::VertexState { module: &blur_shader, entry_point: Some("vs_fullscreen"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &blur_shader, entry_point: Some("fs_blur"), compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format: hdr_format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });

        Self { extract_pipeline, blur_pipeline, uniform_buffer, uniform_layout, sampler, hdr_format, bloom_target: None, bloom_target_view: None, bloom_target_size: (0, 0) }
    }

    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &BloomUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Lazy 创建/重建 bloom extract 输出纹理（匹配 surface 尺寸 + hdr_format）
    pub fn ensure_target(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.bloom_target_view.is_some() && self.bloom_target_size == (width, height) {
            return;
        }
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom extract target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.bloom_target = Some(target);
        self.bloom_target_view = Some(view);
        self.bloom_target_size = (width, height);
    }

    /// 创建 extract/blur bind group（1 texture + 1 sampler + uniform）
    pub fn create_bind_group(&self, device: &wgpu::Device, texture_view: &wgpu::TextureView) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom bind group"), layout: &self.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(texture_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: self.uniform_buffer.as_entire_binding() },
            ],
        })
    }

    /// 创建 HDR 纹理（用于中间结果）
    pub fn create_hdr_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// 执行 Bloom Extract（从 HDR 提取高亮）
    pub fn draw_extract(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.extract_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 执行 Bloom Blur（9-tap 高斯模糊）
    pub fn draw_blur(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl RenderGraphNode for BloomPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str { "bloom" }
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // extract 需要采样输入纹理（此处用 surface_view 作为 HDR 源）
        let src_view = match ctx.surface_view {
            Some(v) => v,
            None => {
                log::warn!("bloom: surface_view 缺失，跳过");
                return Ok(());
            }
        };

        let (w, h) = ctx.surface_size;
        let (w, h) = (w.max(1), h.max(1));

        // 1. 确保 extract 输出纹理就绪（匹配 hdr_format + surface 尺寸）
        self.ensure_target(ctx.device, w, h);

        // 2. 更新 uniform（注入时间）
        let mut u = BloomUniform::default_quality();
        u.time = ctx.time;
        self.update_uniform(ctx.queue, &u);

        // 3. 创建每帧 bind_group（输入 = surface_view）
        let bind_group = self.create_bind_group(ctx.device, src_view);

        // 4. 取目标 view
        let target_view = self.bloom_target_view.as_ref().ok_or_else(|| {
            anyhow::anyhow!("bloom: bloom_target_view 未初始化")
        })?;

        // 5. 仅执行第一阶段（extract）— 从 surface 提取高亮到 bloom_target
        //    多 pass blur + composite 留待 RenderGraph 接线后实现。
        log::warn!("bloom: 仅执行第一阶段（extract），blur/composite 未接入");

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom extract render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
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
        self.draw_extract(&mut rpass, &bind_group);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        assert_eq!(std::mem::size_of::<BloomUniform>(), 32);
    }

    #[test]
    fn default_quality() {
        let u = BloomUniform::default_quality();
        assert!((u.exposure - 1.0).abs() < 0.001);
        assert!((u.gamma - 2.2).abs() < 0.001);
        assert!((u.bloom_threshold - 1.0).abs() < 0.001);
        assert!((u.bloom_intensity - 0.4).abs() < 0.001);
    }

    #[test]
    fn default_is_zero() {
        let u = BloomUniform::default();
        assert_eq!(u.exposure, 0.0);
        assert_eq!(u.blur_dir, [0.0, 0.0]);
    }

    #[test]
    fn bloom_effect_default() {
        let e = BloomEffect::default();
        assert_eq!(e.threshold, 1.0);
        assert_eq!(e.intensity, 0.8);
        assert_eq!(e.mip_levels, 5);
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(BLOOM_EXTRACT_SHADER.contains("vs_fullscreen"));
        assert!(BLOOM_EXTRACT_SHADER.contains("fs_extract"));
        assert!(BLOOM_EXTRACT_SHADER.contains("bloom_threshold"));
        assert!(BLOOM_EXTRACT_SHADER.contains("soft_knee"));
        assert!(BLOOM_BLUR_SHADER.contains("fs_blur"));
        assert!(BLOOM_BLUR_SHADER.contains("blur_dir_x"));
        assert!(BLOOM_BLUR_SHADER.contains("weights"));
    }
}
