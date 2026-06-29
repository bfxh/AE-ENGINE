//! PCSS 软阴影 Pass（Fernando 2005）
//!
//! Percentage-Closer Soft Shadows 三阶段算法：
//! 1. **Blocker Search** — 在 shadow map 局部区域采样，求平均 blocker 深度
//! 2. **Penumbra Estimation** — penumbra = (z_recv - z_block) * light_size / z_block
//! 3. **Variable-radius PCF** — 按 penumbra 宽度做 Poisson disk PCF
//!
//! 输入:
//! - shadow map (Depth32Float) — 来自 `ShadowMapPass`
//! - scene depth (Depth32Float) — 来自 depth pre-pass 或 forward pass
//!
//! 输出:
//! - visibility texture (R8Unorm) — 0=全影, 1=全亮，供 forward pass 乘到直接光
//!
//! 参考:
//! - Fernando 2005, "Percentage-Closer Soft Shadows" (NVIDIA SDK)
//! - Ubaldo 2018, "Practical PCSS" (GPU Zen)
//! -learnopengl "Shadow Mapping" PCF / PCSS 章节

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, RenderPipeline, Texture, TextureView};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// PCSS Uniform（与 WGSL `PcssUniform` 对齐）
///
/// 布局（176 bytes, 16-byte aligned）:
/// - `light_view_proj` (64)  — 世界 → shadow clip
/// - `inv_view_proj` (64)    — 屏幕 NDC → 世界
/// - `shadow_map_size` (16)  — xy=shadow map 分辨率, z=light_size, w=bias
/// - `params` (16)           — x=blocker_search_radius, y=near, z=far, w=max_penumbra
/// - `_pad` (16)             — 保留
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct PcssUniform {
    pub light_view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    /// xy = shadow map 分辨率, z = light_size (世界空间), w = shadow bias
    pub shadow_map_size: [f32; 4],
    /// x = blocker_search_radius (UV 空间), y = near, z = far, w = max_penumbra (UV 空间)
    pub params: [f32; 4],
    pub _pad: [f32; 4],
}

/// PCSS 配置参数
#[derive(Debug, Clone, Copy)]
pub struct PcssConfig {
    /// 光源面积大小（世界空间，控制半影宽度）
    pub light_size: f32,
    /// Shadow bias（避免 acne）
    pub bias: f32,
    /// Blocker search 采样半径（shadow map UV 空间，如 0.002）
    pub blocker_search_radius: f32,
    /// 最大半影半径（UV 空间，避免过度模糊）
    pub max_penumbra: f32,
}

impl Default for PcssConfig {
    fn default() -> Self {
        Self {
            light_size: 1.0,
            bias: 0.001,
            blocker_search_radius: 0.002,
            max_penumbra: 0.01,
        }
    }
}

/// 内嵌 WGSL shader：PCSS 软阴影
///
/// 工作流：
/// 1. full-screen triangle → 对每个屏幕像素执行
/// 2. 读取 scene depth → 重建世界坐标
/// 3. 世界坐标 → shadow clip space → shadow UV
/// 4. Blocker search (16 点 Poisson disk)
/// 5. Penumbra estimation
/// 6. Variable-radius PCF (16 点 Poisson disk)
const PCSS_SHADER: &str = r#"
struct PcssUniform {
    light_view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    shadow_map_size: vec4<f32>,  // xy=size, z=light_size, w=bias
    params: vec4<f32>,           // x=blocker_search_radius, y=near, z=far, w=max_penumbra
    _pad: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u_params: PcssUniform;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(0) @binding(3) var scene_depth: texture_depth_2d;

// 16 点 Poisson disk（来自学习 OpenGL / SIGGPU 慢性采样集）
const POISSON_DISK: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216),
    vec2<f32>( 0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870),
    vec2<f32>( 0.34495938,  0.29387760),
    vec2<f32>(-0.91588581,  0.45771432),
    vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>( 0.38277543,  0.84867917),
    vec2<f32>( 0.26585243, -0.59861303),
    vec2<f32>( 0.52356868,  0.78073987),
    vec2<f32>(-0.28019485, -0.74345869),
    vec2<f32>( 0.32695429, -0.13966937),
    vec2<f32>(-0.37615781,  0.21433923),
    vec2<f32>( 0.14977800,  0.00695270),
    vec2<f32>( 0.64071835, -0.22891067),
    vec2<f32>(-0.49757599, -0.67801487),
    vec2<f32>( 0.20755944,  0.31324859),
);

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Full-screen triangle（无 vertex buffer）
@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> VsOut {
    var out: VsOut;
    out.uv = vec2<f32>(f32((vid << 1u) & 2u), f32(vid & 2u));
    out.position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// 从屏幕 UV + NDC z 重建世界坐标
fn reconstruct_world_pos(uv: vec2<f32>, ndc_z: f32) -> vec3<f32> {
    // wgpu texture (0,0) = 左上角; NDC (-1,-1) = 左下角
    // uv.x → ndc.x:  0→-1, 1→1
    // uv.y → ndc.y:  0→1, 1→-1
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, ndc_z, 1.0);
    let world = u_params.inv_view_proj * ndc;
    return world.xyz / world.w;
}

// Blocker search: 在 shadow map 局部区域采样，返回平均 blocker 深度
// 返回 -1.0 表示无遮挡者
fn find_blocker(shadow_uv: vec2<f32>, z_receiver: f32) -> f32 {
    var sum = 0.0;
    var count = 0.0;
    let radius = u_params.params.x;  // blocker_search_radius (UV space)
    let shadow_size = u_params.shadow_map_size.xy;
    let bias = u_params.shadow_map_size.w;

    for (var i = 0; i < 16; i++) {
        let offset = POISSON_DISK[i] * radius;
        let sample_uv = shadow_uv + offset;
        // 越界跳过
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 ||
            sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            continue;
        }
        let shadow_z = textureLoad(shadow_map, vec2<i32>(sample_uv * shadow_size), 0);
        if (shadow_z < z_receiver - bias) {
            sum += shadow_z;
            count += 1.0;
        }
    }

    if (count < 1.0) { return -1.0; }
    return sum / count;
}

// Variable-radius PCF（硬件 comparison sampler 自动 2×2 PCF）
fn pcf(shadow_uv: vec2<f32>, z_receiver: f32, filter_radius: f32) -> f32 {
    var sum = 0.0;
    let bias = u_params.shadow_map_size.w;

    for (var i = 0; i < 16; i++) {
        let offset = POISSON_DISK[i] * filter_radius;
        let sample_uv = shadow_uv + offset;
        // textureSampleCompare 自动处理越界（ClampToEdge）
        sum += textureSampleCompare(
            shadow_map, shadow_sampler,
            sample_uv, z_receiver - bias,
        );
    }

    return sum / 16.0;
}

@fragment
fn fs_pcss(in: VsOut) -> @location(0) f32 {
    let uv = in.uv;
    let dims = vec2<f32>(textureDimensions(scene_depth, 0));
    let pixel = vec2<i32>(clamp(uv * dims, vec2<f32>(0.0), dims - 1.0));
    let depth = textureLoad(scene_depth, pixel, 0);

    // 远平面（背景）：无阴影
    if (depth >= 1.0) { return 1.0; }

    // 1. 重建世界坐标
    let world_pos = reconstruct_world_pos(uv, depth);

    // 2. 世界 → shadow clip space
    let shadow_clip = u_params.light_view_proj * vec4<f32>(world_pos, 1.0);
    // 透视除法（防止 w ≤ 0）
    if (shadow_clip.w <= 0.0) { return 1.0; }
    let shadow_ndc = shadow_clip.xyz / shadow_clip.w;
    let shadow_uv = shadow_ndc.xy * 0.5 + 0.5;

    // shadow map 范围外：全亮
    if (shadow_uv.x < 0.0 || shadow_uv.x > 1.0 ||
        shadow_uv.y < 0.0 || shadow_uv.y > 1.0) {
        return 1.0;
    }

    let z_receiver = shadow_ndc.z;

    // 3. Blocker search
    let z_blocker = find_blocker(shadow_uv, z_receiver);
    if (z_blocker < 0.0) {
        return 1.0;  // 无 blocker
    }

    // 4. Penumbra estimation
    //    penumbra ∝ (z_receiver - z_blocker) * light_size / z_blocker
    let depth_diff = max(z_receiver - z_blocker, 0.0);
    let penumbra = depth_diff * u_params.shadow_map_size.z / max(z_blocker, 0.0001);
    let filter_radius = clamp(penumbra, 0.0, u_params.params.w);

    // 5. Variable-radius PCF
    return pcf(shadow_uv, z_receiver, filter_radius);
}
"#;

/// PCSS 软阴影 Pass
///
/// 用法：
/// ```ignore
/// let pcss = PcssPass::new(device, shadow_size, (width, height), &shadow_layout);
/// pcss.update_uniforms(queue, light_view_proj, inv_view_proj, &config);
/// // execute 时传入 shadow bind group + scene depth view
/// pcss.execute(encoder, &shadow_bind_group, &scene_depth_view);
/// ```
pub struct PcssPass {
    pub pipeline: RenderPipeline,
    pub uniform_layout: BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub output_texture: Texture,
    pub output_view: TextureView,
    pub bind_group: Option<BindGroup>,
    pub shadow_size: u32,
    pub output_size: [u32; 2],
    pub uniform: PcssUniform,
}

impl PcssPass {
    /// 创建 PCSS Pass
    ///
    /// - `shadow_size`: shadow map 分辨率（与 ShadowMapPass 一致）
    /// - `output_size`: (width, height) 屏幕分辨率
    /// - `shadow_layout`: ShadowMapPass 的 shadow_bind_group_layout（binding 0 uniform, 1 texture, 2 sampler）
    pub fn new(
        device: &wgpu::Device,
        shadow_size: u32,
        output_size: (u32, u32),
        shadow_layout: &BindGroupLayout,
    ) -> Self {
        let safe_shadow = shadow_size.max(1);
        let (w, h) = (output_size.0.max(1), output_size.1.max(1));

        // ---- Output visibility texture (R8Unorm) ----
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pcss visibility texture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---- Uniform buffer ----
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pcss uniform buffer"),
            size: std::mem::size_of::<PcssUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- Bind group layout ----
        // 0: uniform | 1: shadow_map | 2: shadow_sampler | 3: scene_depth
        // 注意：binding 1/2 复用 ShadowMapPass 的 shadow_layout（但 wgpu 要求合并到一个 layout）
        // 这里采用独立 layout，由调用方在 execute 时把 shadow 资源填进来
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pcss bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<PcssUniform>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // ---- Pipeline ----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pcss pipeline layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pcss shader"),
            source: wgpu::ShaderSource::Wgsl(PCSS_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pcss pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_pcss"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 初始 uniform
        let uniform = PcssUniform {
            light_view_proj: [[0.0; 4]; 4],
            inv_view_proj: [[0.0; 4]; 4],
            shadow_map_size: [safe_shadow as f32, safe_shadow as f32, 1.0, 0.001],
            params: [0.002, 0.1, 100.0, 0.01],
            _pad: [0.0; 4],
        };

        // 引用 shadow_layout 避免未使用警告（实际由调用方在构建 bind group 时使用）
        let _ = shadow_layout;

        Self {
            pipeline,
            uniform_layout,
            uniform_buffer,
            output_texture,
            output_view,
            bind_group: None,
            shadow_size: safe_shadow,
            output_size: [w, h],
            uniform,
        }
    }

    /// 更新 PCSS uniform
    ///
    /// - `light_view_proj`: 光源 view-projection（世界 → shadow clip）
    /// - `inv_view_proj`: 主相机逆 view-projection（屏幕 NDC → 世界）
    /// - `config`: PCSS 参数
    pub fn update_uniforms(
        &mut self,
        queue: &wgpu::Queue,
        light_view_proj: [[f32; 4]; 4],
        inv_view_proj: [[f32; 4]; 4],
        config: &PcssConfig,
    ) {
        self.uniform = PcssUniform {
            light_view_proj,
            inv_view_proj,
            shadow_map_size: [
                self.shadow_size as f32,
                self.shadow_size as f32,
                config.light_size,
                config.bias,
            ],
            params: [
                config.blocker_search_radius,
                0.1,  // near
                100.0, // far
                config.max_penumbra,
            ],
            _pad: [0.0; 4],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    /// 构建 bind group（每帧调用，因为 shadow_map / scene_depth 可能变化）
    pub fn build_bind_group(
        &mut self,
        device: &wgpu::Device,
        shadow_map_view: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
        scene_depth_view: &wgpu::TextureView,
    ) {
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pcss bind group"),
            layout: &self.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(shadow_map_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(shadow_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(scene_depth_view) },
            ],
        });
        self.bind_group = Some(bg);
    }

    /// 执行 PCSS pass（full-screen render pass，输出到 visibility texture）
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shadow_map_view: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
        scene_depth_view: &wgpu::TextureView,
    ) {
        let bg = self.bind_group.as_ref().unwrap_or_else(|| {
            panic!("PcssPass::execute: bind_group 未构建，先调用 build_bind_group()");
        });

        // 清成全亮（1.0），避免未覆盖区域产生黑边
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pcss render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, bg, &[]);
        rpass.draw(0..3, 0..1);

        // 引用参数避免未使用警告（实际通过 bind_group 使用）
        let _ = (shadow_map_view, shadow_sampler, scene_depth_view);
    }

    /// 获取 visibility texture view（供 forward pass 采样）
    pub fn visibility_view(&self) -> &TextureView { &self.output_view }
    /// 获取 visibility texture（供 RenderGraph 注册）
    pub fn visibility_texture(&self) -> &Texture { &self.output_texture }

    /// 输出尺寸
    pub fn output_size(&self) -> [u32; 2] { self.output_size }

    /// 调整输出尺寸（窗口 resize 时）
    pub fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        let (w, h) = (new_size.0.max(1), new_size.1.max(1));
        if self.output_size == [w, h] { return; }

        self.output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pcss visibility texture (resized)"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.output_view = self.output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.output_size = [w, h];
        // bind_group 失效（output 没在 bind_group 里，但保险起见清掉）
        // 注意：bind_group 引用的是 shadow_map/scene_depth，不是 output，所以其实不需要清
    }
}

impl RenderGraphNode for PcssPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str { "pcss" }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // bind_group 由调用方通过 build_bind_group() 预先构建（内含 shadow_map +
        // scene_depth 绑定）。未构建时 warn 并跳过，避免 RenderGraph 标记失败。
        let bg = match self.bind_group.as_ref() {
            Some(bg) => bg,
            None => {
                log::warn!("pcss: bind_group 未构建（先调用 build_bind_group），跳过");
                return Ok(());
            }
        };

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pcss render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // 清成全亮（1.0），未覆盖区域不产生黑边
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, bg, &[]);
        rpass.draw(0..3, 0..1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcss_uniform_size_aligned() {
        // 64 + 64 + 16 + 16 + 16 = 176 bytes
        assert_eq!(std::mem::size_of::<PcssUniform>(), 176);
    }

    #[test]
    fn pcss_uniform_pod() {
        fn assert_pod<T: Pod>() {}
        assert_pod::<PcssUniform>();
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(PCSS_SHADER.contains("vs_fullscreen"));
        assert!(PCSS_SHADER.contains("fs_pcss"));
        assert!(PCSS_SHADER.contains("find_blocker"));
        assert!(PCSS_SHADER.contains("pcf"));
        assert!(PCSS_SHADER.contains("POISSON_DISK"));
        assert!(PCSS_SHADER.contains("light_view_proj"));
        assert!(PCSS_SHADER.contains("inv_view_proj"));
        assert!(PCSS_SHADER.contains("reconstruct_world_pos"));
        // Penumbra 公式核心项
        assert!(PCSS_SHADER.contains("z_receiver - z_blocker"));
        assert!(PCSS_SHADER.contains("light_size"));
    }

    #[test]
    fn config_default_sane() {
        let c = PcssConfig::default();
        assert!(c.light_size > 0.0);
        assert!(c.bias > 0.0);
        assert!(c.blocker_search_radius > 0.0);
        assert!(c.max_penumbra > c.blocker_search_radius);
    }

    #[test]
    fn poisson_disk_16_points() {
        // 数 16 个 vec2 条目
        let count = PCSS_SHADER.matches("vec2<f32>(").count();
        // POISSON_DISK 16 + VsOut 等其他地方可能有，但至少 16
        assert!(count >= 16, "expected >=16 vec2 entries, got {}", count);
    }
}
