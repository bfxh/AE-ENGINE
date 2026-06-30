//! SSAO (Screen Space Ambient Occlusion) Module
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
//!
//! WGSL 注意事项：
//! - mat4x4 不允许作为 vertex input/output，全部通过 uniform 传递
//! - 全屏三角形：3 个顶点覆盖 NDC，无 vertex buffer
//! - 在循环中使用 textureSampleLevel 避免 uniform control flow 限制

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline};

/// 默认采样核数量
const DEFAULT_KERNEL_SIZE: u32 = 64;
/// 噪声纹理尺寸（4x4）
const NOISE_TEXTURE_SIZE: u32 = 4;

/// SSAO Uniform（320 bytes = 4 × mat4x4 + 4 × vec4，符合 WGSL 16-byte 对齐）
///
/// 注意：spec 注释 "256 bytes" 算术有误，4×64 + 4×16 = 320。
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
    // 全屏三角形：3 个顶点覆盖 NDC 全屏，无需 vertex buffer
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

// 从深度重建 view-space 位置
// inv_proj = view * view_inv = inverse(proj)
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32, inv_proj: mat4x4<f32>) -> vec3<f32> {
    // WebGPU NDC: x ∈ [-1, 1], y ∈ [-1, 1] (Y up), z ∈ [0, 1]
    // 纹理 UV: u ∈ [0, 1] (left→right), v ∈ [0, 1] (top→bottom)
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let view_pos_h = inv_proj * ndc;
    return view_pos_h.xyz / view_pos_h.w;
}

@fragment
fn fs_ssao(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let uv = pos.xy / dims;

    let depth_val = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    // 远平面/背景：无遮蔽
    if (depth_val >= 1.0) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    // 计算 inverse(proj) = view * view_inv
    let inv_proj = u.view * u.view_inv;

    // 重建 view-space 位置
    let view_pos = reconstruct_view_pos(uv, depth_val, inv_proj);

    // 从屏幕空间导数重建法线（geometric normal）
    let dpdx_v = dpdx(view_pos);
    let dpdy_v = dpdy(view_pos);
    let normal = normalize(cross(dpdx_v, dpdy_v));

    // 从噪声纹理获取旋转向量
    let noise_uv = uv / u.noise_scale.xy;
    let noise_vec = textureSampleLevel(t_noise, s_noise, noise_uv, 0.0).xy;

    // 构建 TBN 矩阵（Gram-Schmidt 正交化）
    let rand_vec = vec3<f32>(noise_vec, 0.0);
    let tangent = normalize(rand_vec - normal * dot(rand_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    // SSAO 参数
    let kernel_count = u32(u.kernel_size.x);
    let radius = u.kernel_size.y;
    let bias = u.kernel_size.z;

    var occlusion = 0.0;
    for (var i = 0u; i < 64u; i = i + 1u) {
        if (i >= kernel_count) {
            break;
        }

        // 切线空间采样方向
        let sample_dir = kernel[i].sample.xyz;

        // 转换到 view-space
        let sample_pos = view_pos + tbn * sample_dir * radius;

        // 投影到 clip-space 并转换为 UV
        let sample_clip = u.proj * vec4<f32>(sample_pos, 1.0);
        let sample_ndc = sample_clip.xyz / sample_clip.w;
        let sample_uv = vec2<f32>(
            sample_ndc.x * 0.5 + 0.5,
            0.5 - sample_ndc.y * 0.5,
        );

        // 采样深度并重建 view-space 位置
        let sample_depth = textureSampleLevel(t_depth, s_depth, sample_uv, 0.0).x;
        let sample_view_pos = reconstruct_view_pos(sample_uv, sample_depth, inv_proj);

        // 范围检查：避免远处几何造成虚假遮蔽
        let range_check = smoothstep(0.0, 1.0, radius / abs(view_pos.z - sample_view_pos.z));

        // 遮蔽测试：view-space 中 z 更靠近相机（更大值）的几何遮挡样本
        // 右手系 view-space：相机原点看向 -z，z 越大越近
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
///
/// 5x5 双边滤波：空间权重（高斯）× 值域权重（AO 差异），保留边缘。
/// 由于 blur bind group 不含 depth，使用 AO 值本身做值域权重（edge-preserving blur）。
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

    // 5x5 双边滤波
    let sigma_space = 2.0;
    let sigma_range = 0.1;

    var sum = 0.0;
    var total_weight = 0.0;

    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let sample_uv = uv + offset;
            let sample_val = textureSampleLevel(t_ssao, s_ssao, sample_uv, 0.0).x;

            // 空间权重（高斯）
            let spatial_dist = f32(x * x + y * y);
            let spatial_weight = exp(-spatial_dist / (2.0 * sigma_space * sigma_space));

            // 值域权重（双边 - 基于 AO 值差异）
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
///
/// 将模糊后的 AO 因子乘到 HDR 颜色：result = hdr_color * mix(1.0, ao, intensity)
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

    // 应用 AO：只影响环境光，简化为整体乘
    let intensity = u.kernel_size.w;
    let result = hdr_color * mix(1.0, ao, intensity);

    return vec4<f32>(result, 1.0);
}
"#;

/// SSAO 渲染器：管理三阶段 SSAO 管线
///
/// 使用方式：
/// ```ignore
/// let ssao = SsaoRenderer::new(&device, hdr_format, depth_format);
/// ssao.update_uniform(&queue, &ssao_uniform);
/// // 每帧：
/// let ssao_tex = SsaoRenderer::create_ssao_texture(&device, w, h, "ssao");
/// let ssao_view = ssao_tex.create_view(&Default::default());
/// let ssao_bg = ssao.create_ssao_bind_group(&device, &hdr_view, &depth_view);
/// // 1. SSAO pass: 渲染到 ssao_view
/// ssao.draw_ssao(&mut pass, &ssao_bg);
/// // 2. Blur pass: 渲染到 ssao_blur_view
/// let blur_bg = ssao.create_blur_bind_group(&device, &ssao_view);
/// ssao.draw_blur(&mut pass, &blur_bg);
/// // 3. Apply pass: 渲染到 hdr_view
/// let apply_bg = ssao.create_apply_bind_group(&device, &hdr_view, &ssao_blur_view);
/// ssao.draw_apply(&mut pass, &apply_bg);
/// ```
pub struct SsaoRenderer {
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
}

impl SsaoRenderer {
    /// 创建 SSAO 渲染器
    ///
    /// # 参数
    /// - `device`: wgpu 设备
    /// - `queue`: wgpu 队列（用于上传噪声纹理数据）
    /// - `hdr_format`: HDR 纹理格式（如 Rgba16Float）
    /// - `depth_format`: 深度纹理格式（如 Depth32Float）
    pub fn new(
        device: &Device,
        queue: &Queue,
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
        // 0: depth texture (non-filterable, Depth32Float)
        // 1: depth sampler (non-filtering)
        // 2: noise texture (non-filterable, Rgba32Float)
        // 3: noise sampler (non-filtering + repeat)
        // 4: uniform buffer
        // 5: kernel storage buffer (read-only)
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
        // 0: SSAO texture (R8Unorm, filterable)
        // 1: SSAO sampler (filtering)
        // 2: uniform buffer
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
        // 0: HDR texture (filterable)
        // 1: HDR sampler (filtering)
        // 2: SSAO texture (filterable)
        // 3: SSAO sampler (filtering)
        // 4: uniform buffer
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

        // 写入噪声数据
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
        // 噪声 sampler：NonFiltering + Repeat（用于平铺）
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

        // 深度 sampler：NonFiltering（Depth32Float 不支持 filtering）
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

        // HDR sampler：Filtering（Rgba16Float 支持 filtering）
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

        // 抑制未使用格式参数警告（depth_format 用于未来扩展/校验）
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
        }
    }

    /// 更新 uniform buffer
    pub fn update_uniform(&self, queue: &Queue, uniform: &SsaoUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 创建 SSAO pass bind group
    ///
    /// 注意：`_hdr_view` 参数为 API 兼容保留，SSAO 生成阶段不使用 HDR 纹理。
    /// `depth_view` 应使用 `TextureAspect::DepthOnly` 创建，便于作为 `texture_2d<f32>` 采样。
    pub fn create_ssao_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 双边滤波使用 AO 值本身做值域权重（无 depth 依赖）。
    pub fn create_blur_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 将模糊后的 SSAO 应用到 HDR 纹理。
    pub fn create_apply_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 用作 SSAO 生成和模糊 pass 的渲染目标。
    pub fn create_ssao_texture(
        device: &Device,
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

/// LCG 随机数生成器（线性同余）
fn lcg(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed as f32 / u32::MAX as f32 // [0, 1]
}

/// 生成 64 个半球采样核
///
/// 算法：
/// 1. 在单位球内随机生成方向
/// 2. 归一化到单位长度
/// 3. 取 z 绝对值确保半球（z > 0）
/// 4. 缩放长度为 random_length^2（更靠近原点的样本更多）
fn generate_kernel_samples() -> [SsaoKernelSample; DEFAULT_KERNEL_SIZE as usize] {
    let mut samples = [SsaoKernelSample::default(); DEFAULT_KERNEL_SIZE as usize];
    let mut seed: u32 = 12345;

    for sample in samples.iter_mut() {
        // 随机方向 [-1, 1]^3
        let rx = lcg(&mut seed) * 2.0 - 1.0;
        let ry = lcg(&mut seed) * 2.0 - 1.0;
        let rz = lcg(&mut seed) * 2.0 - 1.0;

        // 归一化
        let len = (rx * rx + ry * ry + rz * rz).sqrt();
        let (nx, ny, nz) = if len > 1e-6 {
            (rx / len, ry / len, rz / len)
        } else {
            (0.0, 0.0, 1.0)
        };

        // 确保 hemisphere (z > 0)
        let nz = nz.abs();

        // 随机长度 [0, 1]，平方使分布偏向原点
        let length = lcg(&mut seed);
        let scale = length * length;

        sample.sample = [nx * scale, ny * scale, nz * scale, 0.0];
    }

    samples
}

/// 生成 16 个 2D 旋转向量噪声（4x4 纹理）
///
/// 算法：
/// 1. 随机生成 [-1, 1]^2 向量
/// 2. 归一化到单位长度
/// 3. zw 分量置 0
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
        // 4 × mat4x4 (64) + 4 × vec4 (16) = 256 + 64 = 320 bytes
        // 注意：spec 注释 "256 bytes" 算术有误，实际为 320 bytes
        assert_eq!(std::mem::size_of::<SsaoUniform>(), 320);
    }

    #[test]
    fn kernel_sample_size() {
        // 1 × vec4 = 16 bytes
        assert_eq!(std::mem::size_of::<SsaoKernelSample>(), 16);
    }

    #[test]
    fn noise_sample_size() {
        // 1 × vec4 = 16 bytes
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
    fn kernel_sample_default_is_zero() {
        let s = SsaoKernelSample::default();
        assert_eq!(s.sample, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn noise_sample_default_is_zero() {
        let s = SsaoNoiseSample::default();
        assert_eq!(s.noise, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn generate_kernel_produces_hemisphere() {
        let samples = generate_kernel_samples();
        assert_eq!(samples.len(), DEFAULT_KERNEL_SIZE as usize);

        // 所有样本的 z 分量应 >= 0（hemisphere）
        for s in samples.iter() {
            assert!(s.sample[2] >= 0.0, "sample z should be >= 0 for hemisphere");
        }

        // 所有样本的 w 分量应为 0
        for s in samples.iter() {
            assert_eq!(s.sample[3], 0.0, "sample w should be 0");
        }
    }

    #[test]
    fn generate_noise_produces_unit_vectors() {
        let samples = generate_noise_samples();
        assert_eq!(samples.len(), (NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE) as usize);

        // 所有噪声向量应为单位长度
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
        // SSAO shader 关键元素
        assert!(SSAO_SHADER.contains("vs_fullscreen"), "SSAO shader missing fullscreen VS");
        assert!(SSAO_SHADER.contains("fs_ssao"), "SSAO shader missing fragment entry");
        assert!(SSAO_SHADER.contains("reconstruct_view_pos"), "SSAO shader missing reconstruction");
        assert!(SSAO_SHADER.contains("tbn"), "SSAO shader missing TBN matrix");
        assert!(SSAO_SHADER.contains("occlusion"), "SSAO shader missing occlusion");
        assert!(SSAO_SHADER.contains("kernel"), "SSAO shader missing kernel binding");
        assert!(SSAO_SHADER.contains("noise"), "SSAO shader missing noise");

        // Blur shader 关键元素
        assert!(SSAO_BLUR_SHADER.contains("fs_blur"), "blur shader missing fragment entry");
        assert!(SSAO_BLUR_SHADER.contains("spatial_weight"), "blur shader missing spatial weight");
        assert!(SSAO_BLUR_SHADER.contains("range_weight"), "blur shader missing range weight");

        // Apply shader 关键元素
        assert!(SSAO_APPLY_SHADER.contains("fs_apply"), "apply shader missing fragment entry");
        assert!(SSAO_APPLY_SHADER.contains("mix(1.0, ao, intensity)"), "apply shader missing AO mix");
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
