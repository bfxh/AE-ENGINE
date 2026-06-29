//! SSAO Pass（port 自 v1 wasteland_render::ssao）
//!
//! 屏幕空间环境光遮蔽：基于深度缓冲在屏幕空间近似全局光照中的环境遮蔽。
//!
//! 三阶段管线：
//! 1. **SSAO 生成**：从深度重建 view-space 位置/法线，对半球采样核做遮蔽测试
//! 2. **双边滤波模糊**：保留边缘的 5x5 双边滤波，平滑噪声
//! 3. **SSAO 应用**：将 AO 因子乘到 HDR 颜色上
//!
//! 使用全屏三角形（无需 vertex buffer），降低 draw call 开销。
//! 深度格式 Depth32Float 不支持 filtering，使用 NonFiltering sampler。
//! SSAO 中间纹理使用 R8Unorm（单通道 AO 系数）。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// 默认采样核数量
const DEFAULT_KERNEL_SIZE: u32 = 64;
/// 噪声纹理尺寸（4x4）
const NOISE_TEXTURE_SIZE: u32 = 4;

/// SSAO Uniform（320 bytes = 4 × mat4x4 + 4 × vec4，符合 WGSL 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SsaoUniform {
    /// 相机 view-projection 矩阵（列主序）
    pub view_proj: [[f32; 4]; 4],
    /// 相机逆 view-projection 矩阵（用于从 NDC 重建世界坐标）
    pub view_inv: [[f32; 4]; 4],
    /// 相机 view 矩阵（用于转换到 view-space）
    pub view: [[f32; 4]; 4],
    /// 相机 projection 矩阵（用于将 view-space 投影到 clip-space）
    pub proj: [[f32; 4]; 4],
    /// 相机世界坐标（xyz），w=1
    pub camera_pos: [f32; 4],
    /// x=采样数(16/32/64), y=半径, z=bias, w=intensity
    pub kernel_size: [f32; 4],
    /// x=width, y=height, z=1/width, w=1/height
    pub screen_size: [f32; 4],
    /// xy=noise_scale (noise_size/screen_size), zw=pad
    pub noise_scale: [f32; 4],
}

/// SSAO 采样核样本（16 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SsaoKernelSample {
    /// xyz=半球采样方向, w=未用
    pub sample: [f32; 4],
}

impl Default for SsaoKernelSample {
    fn default() -> Self {
        Self { sample: [0.0, 0.0, 0.0, 0.0] }
    }
}

/// SSAO 噪声样本（16 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SsaoNoiseSample {
    /// xy=旋转向量, zw=0
    pub noise: [f32; 4],
}

impl Default for SsaoNoiseSample {
    fn default() -> Self {
        Self { noise: [0.0, 0.0, 0.0, 0.0] }
    }
}

/// SSAO 生成 shader（WGSL）
const SSAO_SHADER: &str = r#"
struct SsaoUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    kernel_size: vec4<f32>,
    screen_size: vec4<f32>,
    noise_scale: vec4<f32>,
};

struct SsaoKernelSample {
    sample: vec4<f32>,
};

@group(0) @binding(0) var t_depth: texture_2d<f32>;
@group(0) @binding(1) var s_depth: sampler;
@group(0) @binding(2) var t_noise: texture_2d<f32>;
@group(0) @binding(3) var s_noise: sampler;
@group(0) @binding(4) var<uniform> u: SsaoUniform;
@group(0) @binding(5) var<storage, read> kernel: array<SsaoKernelSample>;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn reconstruct_view_pos(uv: vec2<f32>, depth: f32, inv_proj: mat4x4<f32>) -> vec3<f32> {
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let view_pos_h = inv_proj * ndc;
    return view_pos_h.xyz / view_pos_h.w;
}

@fragment
fn fs_ssao(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let uv = pos.xy / dims;

    let depth_val = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    if (depth_val >= 1.0) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let inv_proj = u.view * u.view_inv;
    let view_pos = reconstruct_view_pos(uv, depth_val, inv_proj);

    let dpdx_v = dpdx(view_pos);
    let dpdy_v = dpdy(view_pos);
    let normal = normalize(cross(dpdx_v, dpdy_v));

    let noise_uv = uv / u.noise_scale.xy;
    let noise_vec = textureSampleLevel(t_noise, s_noise, noise_uv, 0.0).xy;

    let rand_vec = vec3<f32>(noise_vec, 0.0);
    let tangent = normalize(rand_vec - normal * dot(rand_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    let kernel_count = u32(u.kernel_size.x);
    let radius = u.kernel_size.y;
    let bias = u.kernel_size.z;

    var occlusion = 0.0;
    for (var i = 0u; i < 64u; i = i + 1u) {
        if (i >= kernel_count) {
            break;
        }

        let sample_dir = kernel[i].sample.xyz;
        let sample_pos = view_pos + tbn * sample_dir * radius;

        let sample_clip = u.proj * vec4<f32>(sample_pos, 1.0);
        let sample_ndc = sample_clip.xyz / sample_clip.w;
        let sample_uv = vec2<f32>(
            sample_ndc.x * 0.5 + 0.5,
            0.5 - sample_ndc.y * 0.5,
        );

        let sample_depth = textureSampleLevel(t_depth, s_depth, sample_uv, 0.0).x;
        let sample_view_pos = reconstruct_view_pos(sample_uv, sample_depth, inv_proj);

        let range_check = smoothstep(0.0, 1.0, radius / abs(view_pos.z - sample_view_pos.z));

        if (sample_view_pos.z >= sample_pos.z + bias) {
            occlusion = occlusion + range_check;
        }
    }

    occlusion = occlusion / f32(kernel_count);
    let ao = 1.0 - clamp(occlusion, 0.0, 1.0);
    return vec4<f32>(vec3<f32>(ao), 1.0);
}
"#;

/// 双边滤波模糊 shader（WGSL）
const SSAO_BLUR_SHADER: &str = r#"
struct SsaoUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    kernel_size: vec4<f32>,
    screen_size: vec4<f32>,
    noise_scale: vec4<f32>,
};

@group(0) @binding(0) var t_ssao: texture_2d<f32>;
@group(0) @binding(1) var s_ssao: sampler;
@group(0) @binding(2) var<uniform> u: SsaoUniform;

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
    let texel = vec2<f32>(u.screen_size.z, u.screen_size.w);
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let uv = pos.xy / dims;

    let center = textureSampleLevel(t_ssao, s_ssao, uv, 0.0).x;

    let sigma_space = 2.0;
    let sigma_range = 0.1;

    var sum = 0.0;
    var total_weight = 0.0;

    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let sample_uv = uv + offset;
            let sample_val = textureSampleLevel(t_ssao, s_ssao, sample_uv, 0.0).x;

            let spatial_dist = f32(x * x + y * y);
            let spatial_weight = exp(-spatial_dist / (2.0 * sigma_space * sigma_space));

            let range_diff = abs(sample_val - center);
            let range_weight = exp(-range_diff * range_diff / (2.0 * sigma_range * sigma_range));

            let weight = spatial_weight * range_weight;
            sum = sum + sample_val * weight;
            total_weight = total_weight + weight;
        }
    }

    let result = sum / max(total_weight, 0.0001);
    return vec4<f32>(vec3<f32>(result), 1.0);
}
"#;

/// SSAO 应用 shader（WGSL）
const SSAO_APPLY_SHADER: &str = r#"
struct SsaoUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    kernel_size: vec4<f32>,
    screen_size: vec4<f32>,
    noise_scale: vec4<f32>,
};

@group(0) @binding(0) var t_hdr: texture_2d<f32>;
@group(0) @binding(1) var s_hdr: sampler;
@group(0) @binding(2) var t_ssao: texture_2d<f32>;
@group(0) @binding(3) var s_ssao: sampler;
@group(0) @binding(4) var<uniform> u: SsaoUniform;

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
fn fs_apply(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let uv = pos.xy / dims;

    let hdr_color = textureSampleLevel(t_hdr, s_hdr, uv, 0.0).rgb;
    let ao = textureSampleLevel(t_ssao, s_ssao, uv, 0.0).x;

    let intensity = u.kernel_size.w;
    let result = hdr_color * mix(1.0, ao, intensity);

    return vec4<f32>(result, 1.0);
}
"#;

/// SSAO 渲染 Pass（port 自 v1 wasteland_render::SsaoRenderer）
///
/// 使用方式：
/// ```ignore
/// let ssao = SsaoPass::new(&device, &queue, hdr_format, depth_format);
/// ssao.update_uniform(&queue, &ssao_uniform);
/// let ssao_bg = ssao.create_ssao_bind_group(&device, &hdr_view, &depth_view);
/// ssao.draw_ssao(&mut pass, &ssao_bg);
/// ```
pub struct SsaoPass {
    /// SSAO 生成 pipeline
    pub ssao_pipeline: RenderPipeline,
    /// 双边滤波模糊 pipeline
    pub blur_pipeline: RenderPipeline,
    /// 应用到 HDR pipeline
    pub apply_pipeline: RenderPipeline,
    /// Uniform buffer
    pub uniform_buffer: Buffer,
    /// 采样核 buffer（64 个样本，storage buffer）
    pub kernel_buffer: Buffer,
    /// 4x4 旋转向量噪声纹理
    pub noise_texture: wgpu::Texture,
    /// 噪声纹理 view
    pub noise_view: wgpu::TextureView,
    /// 噪声 sampler（NonFiltering + Repeat）
    pub noise_sampler: wgpu::Sampler,
    /// 深度 sampler（NonFiltering，Depth32Float 不支持 filtering）
    pub depth_sampler: wgpu::Sampler,
    /// HDR sampler（Filtering，Rgba16Float 支持 filtering）
    pub hdr_sampler: wgpu::Sampler,
    /// SSAO pass bind group layout
    pub ssao_layout: BindGroupLayout,
    /// blur pass bind group layout
    pub blur_layout: BindGroupLayout,
    /// apply pass bind group layout
    pub apply_layout: BindGroupLayout,
    /// uniform-only bind group layout（基础布局）
    pub uniform_layout: BindGroupLayout,
    /// 采样核数量（默认 64）
    pub kernel_size: u32,
    /// SSAO 中间输出纹理（R8Unorm，lazy 创建，匹配 surface 尺寸）
    pub ssao_target: Option<wgpu::Texture>,
    /// SSAO 中间输出 view
    pub ssao_target_view: Option<wgpu::TextureView>,
    /// 当前 ssao_target 尺寸（用于检测是否需要重建）
    pub ssao_target_size: (u32, u32),
    /// 占位深度纹理（1x1 Depth32Float，供 generate pass 采样）
    pub dummy_depth: Option<wgpu::Texture>,
    /// 占位深度 view（DepthOnly aspect）
    pub dummy_depth_view: Option<wgpu::TextureView>,
    /// generate pass 每帧 bind group（缓存，尺寸不变可复用）
    pub ssao_frame_bind_group: Option<BindGroup>,
}

impl SsaoPass {
    /// 创建 SSAO Pass
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // ---------- Uniform layout（仅 uniform buffer，binding 0） ----------
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SsaoUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        // ---------- SSAO pass bind group layout ----------
        let ssao_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao pass bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SsaoUniform>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            (DEFAULT_KERNEL_SIZE as u64)
                                * std::mem::size_of::<SsaoKernelSample>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Blur pass bind group layout ----------
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao blur bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SsaoUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Apply pass bind group layout ----------
        let apply_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao apply bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SsaoUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Uniform buffer ----------
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao uniform buffer"),
            contents: bytemuck::cast_slice(&[SsaoUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ---------- Kernel buffer（storage, read-only） ----------
        let kernel_samples = generate_kernel_samples();
        let kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao kernel buffer"),
            contents: bytemuck::cast_slice(&kernel_samples),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // ---------- Noise texture (4x4 Rgba32Float) ----------
        let noise_data = generate_noise_samples();
        let noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ssao noise texture"),
            size: wgpu::Extent3d {
                width: NOISE_TEXTURE_SIZE,
                height: NOISE_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let noise_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let noise_bytes_per_row =
            NOISE_TEXTURE_SIZE * std::mem::size_of::<SsaoNoiseSample>() as u32;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &noise_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&noise_data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(noise_bytes_per_row),
                rows_per_image: Some(NOISE_TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: NOISE_TEXTURE_SIZE,
                height: NOISE_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );

        // ---------- Samplers ----------
        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssao noise sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssao depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let hdr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssao hdr sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---------- SSAO pipeline ----------
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao shader"),
            source: wgpu::ShaderSource::Wgsl(SSAO_SHADER.into()),
        });

        let ssao_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssao pipeline layout"),
                bind_group_layouts: &[&ssao_layout],
                push_constant_ranges: &[],
            });

        let ssao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao pipeline"),
            layout: Some(&ssao_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ssao_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssao_shader,
                entry_point: Some("fs_ssao"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---------- Blur pipeline ----------
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao blur shader"),
            source: wgpu::ShaderSource::Wgsl(SSAO_BLUR_SHADER.into()),
        });

        let blur_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssao blur pipeline layout"),
                bind_group_layouts: &[&blur_layout],
                push_constant_ranges: &[],
            });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao blur pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_blur"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---------- Apply pipeline ----------
        let apply_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao apply shader"),
            source: wgpu::ShaderSource::Wgsl(SSAO_APPLY_SHADER.into()),
        });

        let apply_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssao apply pipeline layout"),
                bind_group_layouts: &[&apply_layout],
                push_constant_ranges: &[],
            });

        let apply_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao apply pipeline"),
            layout: Some(&apply_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &apply_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &apply_shader,
                entry_point: Some("fs_apply"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let _ = depth_format;

        Self {
            ssao_pipeline,
            blur_pipeline,
            apply_pipeline,
            uniform_buffer,
            kernel_buffer,
            noise_texture,
            noise_view,
            noise_sampler,
            depth_sampler,
            hdr_sampler,
            ssao_layout,
            blur_layout,
            apply_layout,
            uniform_layout,
            kernel_size: DEFAULT_KERNEL_SIZE,
            ssao_target: None,
            ssao_target_view: None,
            ssao_target_size: (0, 0),
            dummy_depth: None,
            dummy_depth_view: None,
            ssao_frame_bind_group: None,
        }
    }

    /// 更新 uniform buffer
    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &SsaoUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Lazy 创建/重建 SSAO 中间输出纹理 + 占位深度纹理 + generate bind group
    ///
    /// 当 surface 尺寸变化时重建 R8Unorm 目标纹理；占位深度纹理仅在首次创建。
    /// bind_group 在尺寸变化或首次时重建。
    pub fn ensure_frame_resources(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let need_target = self.ssao_target_view.is_none() || self.ssao_target_size != (width, height);
        if need_target {
            let target = Self::create_ssao_texture(device, width, height, "ssao frame target");
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            self.ssao_target = Some(target);
            self.ssao_target_view = Some(view);
            self.ssao_target_size = (width, height);
            // 尺寸变化后 bind_group 中的 view 引用失效，强制重建
            self.ssao_frame_bind_group = None;
        }

        if self.dummy_depth_view.is_none() {
            let depth = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ssao dummy depth"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = depth.create_view(&wgpu::TextureViewDescriptor {
                aspect: wgpu::TextureAspect::DepthOnly,
                ..Default::default()
            });
            self.dummy_depth = Some(depth);
            self.dummy_depth_view = Some(view);
        }

        if self.ssao_frame_bind_group.is_none() {
            let depth_view = self.dummy_depth_view.as_ref().unwrap();
            let bg = self.create_ssao_bind_group(device, depth_view, depth_view);
            self.ssao_frame_bind_group = Some(bg);
        }
    }

    /// 创建 SSAO pass bind group
    pub fn create_ssao_bind_group(
        &self,
        device: &wgpu::Device,
        _hdr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao pass bind group"),
            layout: &self.ssao_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.noise_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.kernel_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 blur pass bind group
    pub fn create_blur_bind_group(
        &self,
        device: &wgpu::Device,
        ssao_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao blur bind group"),
            layout: &self.blur_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 apply pass bind group
    pub fn create_apply_bind_group(
        &self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
        ssao_blur_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao apply bind group"),
            layout: &self.apply_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(ssao_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 SSAO 中间纹理（R8Unorm）
    pub fn create_ssao_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// 渲染 SSAO（生成 pass）
    pub fn draw_ssao(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.ssao_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 渲染模糊 pass
    pub fn draw_blur(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 应用 SSAO 到 HDR（apply pass）
    pub fn draw_apply(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.apply_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl RenderGraphNode for SsaoPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "ssao"
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        let (w, h) = ctx.surface_size;
        let (w, h) = (w.max(1), h.max(1));

        // 1. 确保 R8Unorm 中间纹理 + 占位深度 + bind_group 就绪
        self.ensure_frame_resources(ctx.device, w, h);

        // 2. 写入默认 uniform（identity 矩阵 + 屏幕尺寸，避免 NaN）
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut u = SsaoUniform::default();
        u.view_proj = identity;
        u.view_inv = identity;
        u.view = identity;
        u.proj = identity;
        u.camera_pos = [0.0, 0.0, 5.0, 1.0];
        u.kernel_size = [self.kernel_size as f32, 0.5, 0.025, 1.0];
        u.screen_size = [w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32];
        u.noise_scale = [4.0 / w as f32, 4.0 / h as f32, 0.0, 0.0];
        self.update_uniform(ctx.queue, &u);

        // 3. 取目标 view + bind_group（此时必定 Some）
        let target_view = self.ssao_target_view.as_ref().ok_or_else(|| {
            anyhow::anyhow!("ssao: ssao_target_view 未初始化")
        })?;
        let bind_group = self.ssao_frame_bind_group.as_ref().ok_or_else(|| {
            anyhow::anyhow!("ssao: ssao_frame_bind_group 未初始化")
        })?;

        // 4. 仅执行第一阶段（generate）— 写入 R8Unorm AO 纹理
        //    blur/apply 需要上游 HDR/depth 输入，留待 RenderGraph 接线后实现。
        log::warn!("ssao: 仅执行第一阶段（generate），blur/apply 未接入");

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssao generate render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.draw_ssao(&mut rpass, bind_group);
        Ok(())
    }
}

/// LCG 随机数生成器（线性同余）
fn lcg(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed as f32 / u32::MAX as f32
}

/// 生成 64 个半球采样核
fn generate_kernel_samples() -> [SsaoKernelSample; DEFAULT_KERNEL_SIZE as usize] {
    let mut samples = [SsaoKernelSample::default(); DEFAULT_KERNEL_SIZE as usize];
    let mut seed: u32 = 12345;

    for sample in samples.iter_mut() {
        let rx = lcg(&mut seed) * 2.0 - 1.0;
        let ry = lcg(&mut seed) * 2.0 - 1.0;
        let rz = lcg(&mut seed) * 2.0 - 1.0;

        let len = (rx * rx + ry * ry + rz * rz).sqrt();
        let (nx, ny, nz) = if len > 1e-6 {
            (rx / len, ry / len, rz / len)
        } else {
            (0.0, 0.0, 1.0)
        };

        let nz = nz.abs();
        let length = lcg(&mut seed);
        let scale = length * length;

        sample.sample = [nx * scale, ny * scale, nz * scale, 0.0];
    }

    samples
}

/// 生成 16 个 2D 旋转向量噪声（4x4 纹理）
fn generate_noise_samples() -> [SsaoNoiseSample; (NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE) as usize]
{
    let mut samples =
        [SsaoNoiseSample::default(); (NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE) as usize];
    let mut seed: u32 = 98765;

    for sample in samples.iter_mut() {
        let rx = lcg(&mut seed) * 2.0 - 1.0;
        let ry = lcg(&mut seed) * 2.0 - 1.0;

        let len = (rx * rx + ry * ry).sqrt();
        let (nx, ny) = if len > 1e-6 {
            (rx / len, ry / len)
        } else {
            (1.0, 0.0)
        };

        sample.noise = [nx, ny, 0.0, 0.0];
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        assert_eq!(std::mem::size_of::<SsaoUniform>(), 320);
    }

    #[test]
    fn kernel_sample_size() {
        assert_eq!(std::mem::size_of::<SsaoKernelSample>(), 16);
    }

    #[test]
    fn noise_sample_size() {
        assert_eq!(std::mem::size_of::<SsaoNoiseSample>(), 16);
    }

    #[test]
    fn default_is_zero() {
        let u = SsaoUniform::default();
        assert_eq!(u.view_proj, [[0.0; 4]; 4]);
        assert_eq!(u.view_inv, [[0.0; 4]; 4]);
        assert_eq!(u.view, [[0.0; 4]; 4]);
        assert_eq!(u.proj, [[0.0; 4]; 4]);
        assert_eq!(u.camera_pos, [0.0; 4]);
        assert_eq!(u.kernel_size, [0.0; 4]);
        assert_eq!(u.screen_size, [0.0; 4]);
        assert_eq!(u.noise_scale, [0.0; 4]);
    }

    #[test]
    fn uniform_field_offsets() {
        let u = SsaoUniform::default();
        let base = &u as *const _ as usize;

        assert_eq!(&u.view_proj as *const _ as usize - base, 0, "view_proj offset");
        assert_eq!(&u.view_inv as *const _ as usize - base, 64, "view_inv offset");
        assert_eq!(&u.view as *const _ as usize - base, 128, "view offset");
        assert_eq!(&u.proj as *const _ as usize - base, 192, "proj offset");
        assert_eq!(&u.camera_pos as *const _ as usize - base, 256, "camera_pos offset");
        assert_eq!(&u.kernel_size as *const _ as usize - base, 272, "kernel_size offset");
        assert_eq!(&u.screen_size as *const _ as usize - base, 288, "screen_size offset");
        assert_eq!(&u.noise_scale as *const _ as usize - base, 304, "noise_scale offset");
    }

    #[test]
    fn kernel_size_default_is_64() {
        assert_eq!(DEFAULT_KERNEL_SIZE, 64);
    }

    #[test]
    fn noise_texture_size_is_4() {
        assert_eq!(NOISE_TEXTURE_SIZE, 4);
    }

    #[test]
    fn generate_kernel_produces_hemisphere() {
        let samples = generate_kernel_samples();
        assert_eq!(samples.len(), DEFAULT_KERNEL_SIZE as usize);

        for s in samples.iter() {
            assert!(s.sample[2] >= 0.0, "sample z should be >= 0 for hemisphere");
        }

        for s in samples.iter() {
            assert_eq!(s.sample[3], 0.0, "sample w should be 0");
        }
    }

    #[test]
    fn generate_noise_produces_unit_vectors() {
        let samples = generate_noise_samples();
        assert_eq!(samples.len(), (NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE) as usize);

        for s in samples.iter() {
            let len = (s.noise[0] * s.noise[0] + s.noise[1] * s.noise[1]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "noise vector should be unit length, got {}",
                len
            );
            assert_eq!(s.noise[2], 0.0, "noise z should be 0");
            assert_eq!(s.noise[3], 0.0, "noise w should be 0");
        }
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(SSAO_SHADER.contains("vs_fullscreen"));
        assert!(SSAO_SHADER.contains("fs_ssao"));
        assert!(SSAO_SHADER.contains("reconstruct_view_pos"));
        assert!(SSAO_SHADER.contains("tbn"));
        assert!(SSAO_SHADER.contains("occlusion"));
        assert!(SSAO_SHADER.contains("kernel"));
        assert!(SSAO_SHADER.contains("noise"));

        assert!(SSAO_BLUR_SHADER.contains("fs_blur"));
        assert!(SSAO_BLUR_SHADER.contains("spatial_weight"));
        assert!(SSAO_BLUR_SHADER.contains("range_weight"));

        assert!(SSAO_APPLY_SHADER.contains("fs_apply"));
        assert!(SSAO_APPLY_SHADER.contains("mix(1.0, ao, intensity)"));
    }

    #[test]
    fn lcg_produces_values_in_unit_range() {
        let mut seed: u32 = 42;
        for _ in 0..100 {
            let v = lcg(&mut seed);
            assert!(v >= 0.0 && v <= 1.0, "LCG value out of [0,1]: {}", v);
        }
    }
}
