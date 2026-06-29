//! SSR Pass（port 自 v1 wasteland_render::ssr）
//!
//! 屏幕空间反射：在屏幕空间内通过 ray marching 近似镜面反射。
//!
//! 三阶段管线：
//! 1. **SSR 生成**：从深度重建 view-space 位置/法线，沿反射方向 ray march，命中后采样 HDR 颜色作为反射色
//! 2. **双边滤波模糊**：基于深度差加权的双边滤波，平滑反射噪声同时保留边缘
//! 3. **反射合成**：将反射颜色按菲涅尔混合回 HDR 颜色缓冲
//!
//! 使用全屏三角形（无需 vertex buffer），降低 draw call 开销。
//! 深度格式 Depth32Float 不支持 filtering，使用 NonFiltering sampler。
//! 反射中间纹理使用 Rgba16Float（与 HDR 一致，支持 HDR 反射色）。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline, Sampler};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// 默认最大 ray march 步数
const DEFAULT_MAX_STEPS: u32 = 32;
/// 反射纹理格式（与 HDR 一致，支持高动态范围反射色）
pub const REFLECTION_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// SSR Uniform（320 bytes = 4 × mat4x4 + 4 × vec4，符合 WGSL 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SsrUniform {
    /// 当前帧 view-projection 矩阵（列主序）
    pub view_proj: [[f32; 4]; 4],
    /// 逆 view-projection 矩阵（用于从 NDC 重建世界坐标）
    pub view_inv: [[f32; 4]; 4],
    /// view 矩阵（用于转换到 view-space）
    pub view: [[f32; 4]; 4],
    /// projection 矩阵（用于将 view-space 投影到 clip-space）
    pub proj: [[f32; 4]; 4],
    /// 相机世界坐标（xyz），w=1
    pub camera_pos: [f32; 4],
    /// x=max_steps(32), y=thickness, z=step_scale, w=reflection_intensity
    pub params: [f32; 4],
    /// x=width, y=height, z=1/width, w=1/height
    pub screen_size: [f32; 4],
    /// 填充（保持 16-byte 对齐）
    pub _pad: [f32; 4],
}

/// SSR Ray Marching Shader (WGSL)
const SSR_SHADER: &str = r#"
struct SsrUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
    _pad: vec4<f32>,
};

@group(0) @binding(0) var t_hdr: texture_2d<f32>;
@group(0) @binding(1) var s_hdr: sampler;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_depth: sampler;
@group(0) @binding(4) var<uniform> u: SsrUniform;

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

fn project_to_uv(view_pos: vec3<f32>, proj: mat4x4<f32>) -> vec2<f32> {
    let clip = proj * vec4<f32>(view_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}

@fragment
fn fs_ssr(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let uv = pos.xy / dims;

    let depth_val = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    if (depth_val >= 1.0) {
        return vec4<f32>(0.0);
    }

    let inv_proj = u.view * u.view_inv;
    let view_pos = reconstruct_view_pos(uv, depth_val, inv_proj);

    let dpdx_v = dpdx(view_pos);
    let dpdy_v = dpdy(view_pos);
    var normal = normalize(cross(dpdy_v, dpdx_v));

    let view_dir = normalize(-view_pos);

    if (dot(normal, view_dir) < 0.0) {
        normal = -normal;
    }

    if (dot(normal, view_dir) <= 0.001) {
        return vec4<f32>(0.0);
    }

    let reflect_dir = reflect(-view_dir, normal);

    if (reflect_dir.z >= 0.0) {
        return vec4<f32>(0.0);
    }

    let max_steps = u32(u.params.x);
    let thickness = u.params.y;
    let step_scale = u.params.z;

    var ray_pos = view_pos;
    var hit = false;
    var hit_uv = vec2<f32>(0.0);

    for (var i = 0u; i < 64u; i = i + 1u) {
        if (i >= max_steps) {
            break;
        }

        ray_pos = ray_pos + reflect_dir * step_scale;
        let sample_uv = project_to_uv(ray_pos, u.proj);

        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            break;
        }

        let sample_depth = textureSampleLevel(t_depth, s_depth, sample_uv, 0.0).x;

        if (sample_depth >= 1.0) {
            continue;
        }

        let sample_view_pos = reconstruct_view_pos(sample_uv, sample_depth, inv_proj);

        let depth_diff = sample_view_pos.z - ray_pos.z;
        if (depth_diff > 0.0 && depth_diff < thickness) {
            hit = true;
            hit_uv = sample_uv;
            break;
        }
    }

    if (!hit) {
        return vec4<f32>(0.0);
    }

    let reflection_color = textureSampleLevel(t_hdr, s_hdr, hit_uv, 0.0).rgb;
    let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);

    let edge_dist = min(min(hit_uv.x, 1.0 - hit_uv.x), min(hit_uv.y, 1.0 - hit_uv.y));
    let edge_atten = smoothstep(0.0, 0.1, edge_dist);

    return vec4<f32>(reflection_color * edge_atten, fresnel * edge_atten);
}
"#;

/// 双边滤波模糊 Shader (WGSL)
const SSR_BLUR_SHADER: &str = r#"
struct SsrUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
    _pad: vec4<f32>,
};

@group(0) @binding(0) var t_ssr: texture_2d<f32>;
@group(0) @binding(1) var s_ssr: sampler;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_depth: sampler;
@group(0) @binding(4) var<uniform> u: SsrUniform;

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

    let center = textureSampleLevel(t_ssr, s_ssr, uv, 0.0);
    let center_depth = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    if (center_depth >= 1.0) {
        return vec4<f32>(0.0);
    }

    let sigma_space = 2.0;
    let sigma_depth = 0.05;

    var sum = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let sample_uv = uv + offset;
            let sample_val = textureSampleLevel(t_ssr, s_ssr, sample_uv, 0.0);
            let sample_depth = textureSampleLevel(t_depth, s_depth, sample_uv, 0.0).x;

            let spatial_dist = f32(x * x + y * y);
            let spatial_weight = exp(-spatial_dist / (2.0 * sigma_space * sigma_space));

            let depth_diff = abs(sample_depth - center_depth);
            let range_weight = exp(-depth_diff * depth_diff / (2.0 * sigma_depth * sigma_depth));

            let weight = spatial_weight * range_weight;
            sum = sum + sample_val * weight;
            total_weight = total_weight + weight;
        }
    }

    return sum / max(total_weight, 0.0001);
}
"#;

/// 反射合成 Shader (WGSL)
const SSR_APPLY_SHADER: &str = r#"
struct SsrUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
    _pad: vec4<f32>,
};

@group(0) @binding(0) var t_hdr: texture_2d<f32>;
@group(0) @binding(1) var s_hdr: sampler;
@group(0) @binding(2) var t_ssr: texture_2d<f32>;
@group(0) @binding(3) var s_ssr: sampler;
@group(0) @binding(4) var<uniform> u: SsrUniform;

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
    let ssr = textureSampleLevel(t_ssr, s_ssr, uv, 0.0);

    let intensity = u.params.w;
    let result = hdr_color + ssr.rgb * ssr.a * intensity;

    return vec4<f32>(result, 1.0);
}
"#;

/// SSR 渲染 Pass（port 自 v1 wasteland_render::SsrRenderer）
///
/// 使用方式：
/// ```ignore
/// let ssr = SsrPass::new(&device, hdr_format, depth_format);
/// ssr.update_uniform(&queue, &ssr_uniform);
/// let ssr_bg = ssr.create_ssr_bind_group(&device, &hdr_view, &depth_view);
/// ssr.draw_ssr(&mut pass, &ssr_bg);
/// ```
pub struct SsrPass {
    /// SSR 生成 pipeline（ray marching）
    pub ssr_pipeline: RenderPipeline,
    /// 双边滤波模糊 pipeline
    pub blur_pipeline: RenderPipeline,
    /// 反射合成到 HDR pipeline
    pub apply_pipeline: RenderPipeline,
    /// Uniform buffer
    pub uniform_buffer: Buffer,
    /// uniform-only bind group layout（基础布局）
    pub uniform_layout: BindGroupLayout,
    /// SSR pass bind group layout
    pub ssr_layout: BindGroupLayout,
    /// blur pass bind group layout
    pub blur_layout: BindGroupLayout,
    /// apply pass bind group layout
    pub apply_layout: BindGroupLayout,
    /// HDR sampler（Filtering，Rgba16Float 支持 filtering）
    pub hdr_sampler: Sampler,
    /// 深度 sampler（NonFiltering，Depth32Float 不支持 filtering）
    pub depth_sampler: Sampler,
    /// SSR 中间输出纹理（Rgba16Float，lazy 创建，匹配 surface 尺寸）
    pub ssr_target: Option<wgpu::Texture>,
    /// SSR 中间输出 view
    pub ssr_target_view: Option<wgpu::TextureView>,
    /// 当前 ssr_target 尺寸（用于检测是否需要重建）
    pub ssr_target_size: (u32, u32),
    /// 占位 HDR 纹理（1x1 Rgba16Float，供 generate pass 采样）
    pub dummy_hdr: Option<wgpu::Texture>,
    /// 占位 HDR view
    pub dummy_hdr_view: Option<wgpu::TextureView>,
    /// 占位深度纹理（1x1 Depth32Float，供 generate pass 采样）
    pub dummy_depth: Option<wgpu::Texture>,
    /// 占位深度 view（DepthOnly aspect）
    pub dummy_depth_view: Option<wgpu::TextureView>,
    /// generate pass 每帧 bind group（缓存，尺寸不变可复用）
    pub ssr_frame_bind_group: Option<BindGroup>,
}

impl SsrPass {
    /// 创建 SSR Pass
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // ---------- Uniform layout ----------
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<SsrUniform>() as u64),
                },
                count: None,
            }],
        });

        // ---------- SSR pass bind group layout ----------
        let ssr_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr pass bind group layout"),
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
                            std::mem::size_of::<SsrUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Blur pass bind group layout ----------
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr blur bind group layout"),
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
                            std::mem::size_of::<SsrUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Apply pass bind group layout ----------
        let apply_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr apply bind group layout"),
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
                            std::mem::size_of::<SsrUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Uniform buffer ----------
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssr uniform buffer"),
            contents: bytemuck::cast_slice(&[SsrUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ---------- Samplers ----------
        let hdr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssr hdr sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssr depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---------- SSR pipeline ----------
        let ssr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssr shader"),
            source: wgpu::ShaderSource::Wgsl(SSR_SHADER.into()),
        });

        let ssr_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssr pipeline layout"),
                bind_group_layouts: &[&ssr_layout],
                push_constant_ranges: &[],
            });

        let ssr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssr pipeline"),
            layout: Some(&ssr_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ssr_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssr_shader,
                entry_point: Some("fs_ssr"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: REFLECTION_TEXTURE_FORMAT,
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
            label: Some("ssr blur shader"),
            source: wgpu::ShaderSource::Wgsl(SSR_BLUR_SHADER.into()),
        });

        let blur_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssr blur pipeline layout"),
                bind_group_layouts: &[&blur_layout],
                push_constant_ranges: &[],
            });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssr blur pipeline"),
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
                    format: REFLECTION_TEXTURE_FORMAT,
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
            label: Some("ssr apply shader"),
            source: wgpu::ShaderSource::Wgsl(SSR_APPLY_SHADER.into()),
        });

        let apply_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssr apply pipeline layout"),
                bind_group_layouts: &[&apply_layout],
                push_constant_ranges: &[],
            });

        let apply_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssr apply pipeline"),
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
            ssr_pipeline,
            blur_pipeline,
            apply_pipeline,
            uniform_buffer,
            uniform_layout,
            ssr_layout,
            blur_layout,
            apply_layout,
            hdr_sampler,
            depth_sampler,
            ssr_target: None,
            ssr_target_view: None,
            ssr_target_size: (0, 0),
            dummy_hdr: None,
            dummy_hdr_view: None,
            dummy_depth: None,
            dummy_depth_view: None,
            ssr_frame_bind_group: None,
        }
    }

    /// 更新 uniform buffer
    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &SsrUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Lazy 创建/重建 SSR 中间输出纹理 + 占位 HDR/深度纹理 + generate bind group
    pub fn ensure_frame_resources(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let need_target = self.ssr_target_view.is_none() || self.ssr_target_size != (width, height);
        if need_target {
            let target = Self::create_reflection_texture(device, width, height, "ssr frame target");
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            self.ssr_target = Some(target);
            self.ssr_target_view = Some(view);
            self.ssr_target_size = (width, height);
            self.ssr_frame_bind_group = None;
        }

        if self.dummy_hdr_view.is_none() {
            let hdr = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ssr dummy hdr"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: REFLECTION_TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = hdr.create_view(&wgpu::TextureViewDescriptor::default());
            self.dummy_hdr = Some(hdr);
            self.dummy_hdr_view = Some(view);
        }

        if self.dummy_depth_view.is_none() {
            let depth = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ssr dummy depth"),
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

        if self.ssr_frame_bind_group.is_none() {
            let hdr_view = self.dummy_hdr_view.as_ref().unwrap();
            let depth_view = self.dummy_depth_view.as_ref().unwrap();
            let bg = self.create_ssr_bind_group(device, hdr_view, depth_view);
            self.ssr_frame_bind_group = Some(bg);
        }
    }

    /// 创建 SSR pass bind group
    pub fn create_ssr_bind_group(
        &self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssr pass bind group"),
            layout: &self.ssr_layout,
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
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 blur pass bind group
    pub fn create_blur_bind_group(
        &self,
        device: &wgpu::Device,
        ssr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssr blur bind group"),
            layout: &self.blur_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
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
        ssr_blur_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssr apply bind group"),
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
                    resource: wgpu::BindingResource::TextureView(ssr_blur_view),
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

    /// 创建反射纹理（Rgba16Float）
    pub fn create_reflection_texture(
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
            format: REFLECTION_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// 渲染 SSR（ray marching 生成 pass）
    pub fn draw_ssr(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.ssr_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 渲染双边滤波模糊 pass
    pub fn draw_blur(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 应用 SSR 反射到 HDR（apply pass）
    pub fn draw_apply(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.apply_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl RenderGraphNode for SsrPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "ssr"
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        let (w, h) = ctx.surface_size;
        let (w, h) = (w.max(1), h.max(1));

        // 1. 确保 Rgba16Float 中间纹理 + 占位 HDR/深度 + bind_group 就绪
        self.ensure_frame_resources(ctx.device, w, h);

        // 2. 写入默认 uniform（identity 矩阵 + 屏幕尺寸，避免 NaN）
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut u = SsrUniform::default();
        u.view_proj = identity;
        u.view_inv = identity;
        u.view = identity;
        u.proj = identity;
        u.camera_pos = [0.0, 0.0, 5.0, 1.0];
        u.params = [DEFAULT_MAX_STEPS as f32, 0.5, 0.1, 0.5];
        u.screen_size = [w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32];
        u._pad = [0.0; 4];
        self.update_uniform(ctx.queue, &u);

        // 3. 取目标 view + bind_group
        let target_view = self.ssr_target_view.as_ref().ok_or_else(|| {
            anyhow::anyhow!("ssr: ssr_target_view 未初始化")
        })?;
        let bind_group = self.ssr_frame_bind_group.as_ref().ok_or_else(|| {
            anyhow::anyhow!("ssr: ssr_frame_bind_group 未初始化")
        })?;

        // 4. 仅执行第一阶段（generate ray marching）— 写入 Rgba16Float 反射纹理
        //    blur/apply 需要上游 HDR/depth 输入，留待 RenderGraph 接线后实现。
        log::warn!("ssr: 仅执行第一阶段（generate），blur/apply 未接入");

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssr generate render pass"),
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
        self.draw_ssr(&mut rpass, bind_group);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        assert_eq!(std::mem::size_of::<SsrUniform>(), 320);
    }

    #[test]
    fn default_impl_exists() {
        let u = SsrUniform::default();
        assert_eq!(u.view_proj, [[0.0; 4]; 4]);
        assert_eq!(u.view_inv, [[0.0; 4]; 4]);
        assert_eq!(u.view, [[0.0; 4]; 4]);
        assert_eq!(u.proj, [[0.0; 4]; 4]);
        assert_eq!(u.camera_pos, [0.0; 4]);
        assert_eq!(u.params, [0.0; 4]);
        assert_eq!(u.screen_size, [0.0; 4]);
        assert_eq!(u._pad, [0.0; 4]);
    }

    #[test]
    fn uniform_field_offsets() {
        let u = SsrUniform::default();
        let base = &u as *const _ as usize;

        assert_eq!(&u.view_proj as *const _ as usize - base, 0, "view_proj offset");
        assert_eq!(&u.view_inv as *const _ as usize - base, 64, "view_inv offset");
        assert_eq!(&u.view as *const _ as usize - base, 128, "view offset");
        assert_eq!(&u.proj as *const _ as usize - base, 192, "proj offset");
        assert_eq!(&u.camera_pos as *const _ as usize - base, 256, "camera_pos offset");
        assert_eq!(&u.params as *const _ as usize - base, 272, "params offset");
        assert_eq!(&u.screen_size as *const _ as usize - base, 288, "screen_size offset");
        assert_eq!(&u._pad as *const _ as usize - base, 304, "_pad offset");
    }

    #[test]
    fn reflection_texture_format_is_rgba16float() {
        assert_eq!(REFLECTION_TEXTURE_FORMAT, wgpu::TextureFormat::Rgba16Float);
    }

    #[test]
    fn default_max_steps_is_32() {
        assert_eq!(DEFAULT_MAX_STEPS, 32);
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(SSR_SHADER.contains("vs_fullscreen"));
        assert!(SSR_SHADER.contains("fs_ssr"));
        assert!(SSR_SHADER.contains("reconstruct_view_pos"));
        assert!(SSR_SHADER.contains("reflect_dir"));
        assert!(SSR_SHADER.contains("reflect("));
        assert!(SSR_SHADER.contains("ray_pos"));
        assert!(SSR_SHADER.contains("thickness"));
        assert!(SSR_SHADER.contains("fresnel"));
        assert!(SSR_SHADER.contains("edge_atten"));
        assert!(SSR_SHADER.contains("dpdx"));
        assert!(SSR_SHADER.contains("dpdy"));

        assert!(SSR_BLUR_SHADER.contains("fs_blur"));
        assert!(SSR_BLUR_SHADER.contains("spatial_weight"));
        assert!(SSR_BLUR_SHADER.contains("range_weight"));
        assert!(SSR_BLUR_SHADER.contains("depth_diff"));

        assert!(SSR_APPLY_SHADER.contains("fs_apply"));
        assert!(SSR_APPLY_SHADER.contains("ssr.rgb * ssr.a * intensity"));
    }

    #[test]
    fn uniform_is_pod_zeroable() {
        let bytes = [0u8; 320];
        let u: &SsrUniform = bytemuck::from_bytes(&bytes);
        assert_eq!(u.params, [0.0; 4]);
        assert_eq!(u.screen_size, [0.0; 4]);
    }
}
