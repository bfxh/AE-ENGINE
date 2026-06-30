//! SSR (Screen-Space Reflections) Module
//!
//! 屏幕空间反射：在屏幕空间内通过 ray marching 近似镜面反射。
//!
//! 三阶段管线：
//! 1. **SSR 生成**：从深度重建 view-space 位置/法线，沿反射方向 ray march，命中后采样 HDR 颜色作为反射色
//! 2. **双边滤波模糊**：基于深度差加权的双边滤波，平滑反射噪声同时保留边缘
//! 3. **反射合成**：将反射颜色按菲涅尔混合回 HDR 颜色缓冲
//!
//! 算法要点：
//! - **从深度重建法线**：使用 `dpdx`/`dpdy` 屏幕空间导数在 view-space 交叉乘积得到几何法线
//! - **Ray march**：在 view-space 沿反射方向步进，每步投影到屏幕空间采样深度，比较厚度阈值
//! - **菲涅尔**：Schlick 近似 `fresnel = pow(1.0 - max(dot(N, V), 0.0), 5.0)`
//! - **边缘衰减**：屏幕边缘的反射衰减到 0，避免边缘伪影
//!
//! 使用全屏三角形（无需 vertex buffer），降低 draw call 开销。
//! 深度格式 Depth32Float 不支持 filtering，使用 NonFiltering sampler。
//! 反射中间纹理使用 Rgba16Float（与 HDR 一致，支持 HDR 反射色）。
//!
//! WGSL 注意事项：
//! - mat4x4 不允许作为 vertex input/output，全部通过 uniform 传递
//! - 全屏三角形：3 个顶点覆盖 NDC，无 vertex buffer
//! - 在循环中使用 textureSampleLevel 避免 non-uniform control flow 限制
//! - Depth32Float 不支持 filtering，sampler 用 NonFiltering

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler};

/// 默认最大 ray march 步数
const DEFAULT_MAX_STEPS: u32 = 32;
/// 反射纹理格式（与 HDR 一致，支持高动态范围反射色）
pub const REFLECTION_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// SSR Uniform（320 bytes = 4 × mat4x4 + 4 × vec4，符合 WGSL 16-byte 对齐）
///
/// 字段布局：
/// - `view_proj`：当前帧 view-projection
/// - `view_inv`：逆 view-projection（用于从 NDC+深度重建世界坐标）
/// - `view`：view 矩阵（用于将世界坐标转换到 view-space，或构建 inverse(proj)）
/// - `proj`：projection 矩阵（用于将 view-space 投影到 clip-space）
/// - `camera_pos`：相机世界坐标（xyz），w=1
/// - `params`：x=max_steps(32), y=thickness, z=step_scale, w=reflection_intensity
/// - `screen_size`：x=width, y=height, z=1/width, w=1/height
/// - `_pad`：填充（保持 16-byte 对齐）
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
///
/// 全屏三角形 vertex + SSR fragment：
/// - 采样深度，重建 view-space 位置
/// - 从屏幕空间导数重建法线（dpdx/dpdy 交叉乘积）
/// - 计算反射方向，沿反射方向在 view-space ray march
/// - 每步投影到屏幕空间，采样深度，比较厚度阈值判断命中
/// - 命中后采样 HDR 颜色作为反射色，应用菲涅尔和边缘衰减
/// - 输出：rgb=反射色, a=反射强度（菲涅尔×边缘衰减）
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

// 将 view-space 位置投影到屏幕 UV
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

    // 远平面/背景：无反射
    if (depth_val >= 1.0) {
        return vec4<f32>(0.0);
    }

    // 计算 inverse(proj) = view * view_inv
    let inv_proj = u.view * u.view_inv;

    // 重建 view-space 位置
    let view_pos = reconstruct_view_pos(uv, depth_val, inv_proj);

    // 从屏幕空间导数重建法线（view-space geometric normal）
    let dpdx_v = dpdx(view_pos);
    let dpdy_v = dpdy(view_pos);
    var normal = normalize(cross(dpdy_v, dpdx_v));

    // 视线方向（从表面指向相机，view-space 中相机在原点）
    let view_dir = normalize(-view_pos);

    // 翻转法线使其朝向相机
    if (dot(normal, view_dir) < 0.0) {
        normal = -normal;
    }

    // 背面朝向相机：无反射
    if (dot(normal, view_dir) <= 0.001) {
        return vec4<f32>(0.0);
    }

    // 反射方向（入射方向 = -view_dir，反射离开表面）
    let reflect_dir = reflect(-view_dir, normal);

    // 反射方向朝向相机或平行：无有效反射
    // view-space 右手系，相机看向 -z，有效反射应进入 -z 方向
    if (reflect_dir.z >= 0.0) {
        return vec4<f32>(0.0);
    }

    // Ray march 参数
    let max_steps = u32(u.params.x);
    let thickness = u.params.y;
    let step_scale = u.params.z;

    var ray_pos = view_pos;
    var hit = false;
    var hit_uv = vec2<f32>(0.0);

    // 限定最大循环次数（WGSL 要求有界循环），实际步数由 max_steps 控制
    for (var i = 0u; i < 64u; i = i + 1u) {
        if (i >= max_steps) {
            break;
        }

        // 沿反射方向前进一步
        ray_pos = ray_pos + reflect_dir * step_scale;

        // 投影到屏幕 UV
        let sample_uv = project_to_uv(ray_pos, u.proj);

        // 超出屏幕边界：终止
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            break;
        }

        // 采样深度（循环内使用 textureSampleLevel 避免 non-uniform control flow 限制）
        let sample_depth = textureSampleLevel(t_depth, s_depth, sample_uv, 0.0).x;

        // 跳过背景
        if (sample_depth >= 1.0) {
            continue;
        }

        // 重建采样位置的 view-space 位置
        let sample_view_pos = reconstruct_view_pos(sample_uv, sample_depth, inv_proj);

        // 厚度测试：ray 在几何后面（更远），且差距小于 thickness
        // view-space 右手系：z 越小（更负）越远
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

    // 采样命中位置的 HDR 颜色作为反射色
    let reflection_color = textureSampleLevel(t_hdr, s_hdr, hit_uv, 0.0).rgb;

    // 菲涅尔（Schlick 近似）：视角越平，反射越强
    let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);

    // 边缘衰减：靠近屏幕边缘的反射衰减到 0，避免边缘伪影
    let edge_dist = min(min(hit_uv.x, 1.0 - hit_uv.x), min(hit_uv.y, 1.0 - hit_uv.y));
    let edge_atten = smoothstep(0.0, 0.1, edge_dist);

    // 输出：rgb=反射色×边缘衰减, a=菲涅尔×边缘衰减
    return vec4<f32>(reflection_color * edge_atten, fresnel * edge_atten);
}
"#;

/// 双边滤波模糊 Shader (WGSL)
///
/// 5x5 双边滤波：空间权重（高斯）× 值域权重（深度差异），保留边缘。
/// 输入：SSR 反射纹理 + 深度纹理（用于边缘感知）。
/// 输出：模糊后的 SSR 反射纹理。
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

    // 远平面/背景：无反射需模糊
    if (center_depth >= 1.0) {
        return vec4<f32>(0.0);
    }

    // 5x5 双边滤波
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

            // 空间权重（高斯）
            let spatial_dist = f32(x * x + y * y);
            let spatial_weight = exp(-spatial_dist / (2.0 * sigma_space * sigma_space));

            // 值域权重（双边 - 基于深度差异，保留边缘）
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
///
/// 将模糊后的 SSR 反射按菲涅尔混合回 HDR 颜色缓冲。
/// `result = hdr + reflection.rgb * reflection.a * intensity`
/// 其中 `reflection.a` 已包含菲涅尔和边缘衰减。
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

    // 菲涅尔混合：反射强度由 SSR alpha（含菲涅尔×边缘衰减）和全局强度参数控制
    let intensity = u.params.w;
    let result = hdr_color + ssr.rgb * ssr.a * intensity;

    return vec4<f32>(result, 1.0);
}
"#;

/// SSR 渲染器：管理三阶段 SSR 管线
///
/// 使用方式：
/// ```ignore
/// let ssr = SsrRenderer::new(&device, hdr_format, depth_format);
/// ssr.update_uniform(&queue, &ssr_uniform);
/// // 每帧：
/// let ssr_tex = SsrRenderer::create_reflection_texture(&device, w, h, "ssr");
/// let ssr_view = ssr_tex.create_view(&Default::default());
/// let ssr_blur_tex = SsrRenderer::create_reflection_texture(&device, w, h, "ssr_blur");
/// let ssr_blur_view = ssr_blur_tex.create_view(&Default::default());
///
/// // 1. SSR pass: 渲染到 ssr_view
/// let ssr_bg = ssr.create_ssr_bind_group(&device, &hdr_view, &depth_view);
/// ssr.draw_ssr(&mut pass, &ssr_bg);
///
/// // 2. Blur pass: 渲染到 ssr_blur_view
/// let blur_bg = ssr.create_blur_bind_group(&device, &ssr_view, &depth_view);
/// ssr.draw_blur(&mut pass, &blur_bg);
///
/// // 3. Apply pass: 渲染回 hdr_view
/// let apply_bg = ssr.create_apply_bind_group(&device, &hdr_view, &ssr_blur_view);
/// ssr.draw_apply(&mut pass, &apply_bg);
/// ```
pub struct SsrRenderer {
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
}

impl SsrRenderer {
    /// 创建 SSR 渲染器
    ///
    /// # 参数
    /// - `device`: wgpu 设备
    /// - `hdr_format`: HDR 纹理格式（如 Rgba16Float）
    /// - `depth_format`: 深度纹理格式（如 Depth32Float）
    pub fn new(
        device: &Device,
        hdr_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // ---------- Uniform layout（仅 uniform buffer，binding 0） ----------
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
        // 0: HDR texture (filterable, Rgba16Float)
        // 1: HDR sampler (filtering)
        // 2: depth texture (non-filterable, Depth32Float)
        // 3: depth sampler (non-filtering)
        // 4: uniform buffer
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
        // 0: SSR texture (filterable, Rgba16Float)
        // 1: SSR sampler (filtering)
        // 2: depth texture (non-filterable, Depth32Float)
        // 3: depth sampler (non-filtering)
        // 4: uniform buffer
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
        // 0: HDR texture (filterable, Rgba16Float)
        // 1: HDR sampler (filtering)
        // 2: SSR blur texture (filterable, Rgba16Float)
        // 3: SSR blur sampler (filtering)
        // 4: uniform buffer
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
        // HDR sampler：Filtering（Rgba16Float 支持 filtering）
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

        // 深度 sampler：NonFiltering（Depth32Float 不支持 filtering）
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

        // 抑制未使用格式参数警告（depth_format 用于未来扩展/校验）
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
        }
    }

    /// 更新 uniform buffer
    pub fn update_uniform(&self, queue: &Queue, uniform: &SsrUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 创建 SSR pass bind group
    ///
    /// SSR pass：从 HDR + 深度生成反射纹理。
    /// `depth_view` 应使用 `TextureAspect::DepthOnly` 创建，便于作为 `texture_2d<f32>` 采样。
    pub fn create_ssr_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 双边滤波使用深度差做值域权重，保留边缘。
    /// `depth_view` 应使用 `TextureAspect::DepthOnly` 创建。
    pub fn create_blur_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 将模糊后的 SSR 反射合成到 HDR 纹理。
    pub fn create_apply_bind_group(
        &self,
        device: &Device,
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
    ///
    /// 用作 SSR 生成和模糊 pass 的渲染目标。
    /// 使用 Rgba16Float 以支持 HDR 反射颜色。
    pub fn create_reflection_texture(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        // 4 × mat4x4 (64) + 4 × vec4 (16) = 256 + 64 = 320 bytes
        assert_eq!(std::mem::size_of::<SsrUniform>(), 320);
    }

    #[test]
    fn default_impl_exists() {
        // 验证 Default 实现存在并产生有效结构体
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
        // 验证反射纹理格式为 Rgba16Float
        assert_eq!(REFLECTION_TEXTURE_FORMAT, wgpu::TextureFormat::Rgba16Float);
    }

    #[test]
    fn default_max_steps_is_32() {
        // 验证默认最大步数为 32
        assert_eq!(DEFAULT_MAX_STEPS, 32);
    }

    #[test]
    fn shader_contains_key_elements() {
        // SSR shader 关键元素
        assert!(SSR_SHADER.contains("vs_fullscreen"), "SSR shader missing fullscreen VS");
        assert!(SSR_SHADER.contains("fs_ssr"), "SSR shader missing fragment entry");
        assert!(SSR_SHADER.contains("reconstruct_view_pos"), "SSR shader missing view pos reconstruction");
        assert!(SSR_SHADER.contains("reflect_dir"), "SSR shader missing reflection direction");
        assert!(SSR_SHADER.contains("reflect("), "SSR shader missing reflect() call");
        assert!(SSR_SHADER.contains("ray_pos"), "SSR shader missing ray march");
        assert!(SSR_SHADER.contains("thickness"), "SSR shader missing thickness test");
        assert!(SSR_SHADER.contains("fresnel"), "SSR shader missing fresnel");
        assert!(SSR_SHADER.contains("edge_atten"), "SSR shader missing edge attenuation");
        assert!(SSR_SHADER.contains("dpdx"), "SSR shader missing dpdx for normal reconstruction");
        assert!(SSR_SHADER.contains("dpdy"), "SSR shader missing dpdy for normal reconstruction");

        // Blur shader 关键元素
        assert!(SSR_BLUR_SHADER.contains("fs_blur"), "blur shader missing fragment entry");
        assert!(SSR_BLUR_SHADER.contains("spatial_weight"), "blur shader missing spatial weight");
        assert!(SSR_BLUR_SHADER.contains("range_weight"), "blur shader missing range weight");
        assert!(SSR_BLUR_SHADER.contains("depth_diff"), "blur shader missing depth-based bilateral");

        // Apply shader 关键元素
        assert!(SSR_APPLY_SHADER.contains("fs_apply"), "apply shader missing fragment entry");
        assert!(SSR_APPLY_SHADER.contains("ssr.rgb * ssr.a * intensity"), "apply shader missing fresnel blend");
    }

    #[test]
    fn uniform_is_pod_zeroable() {
        // 验证 Pod/Zeroable trait（编译时已保证，这里验证 all-zeros 有效）
        let bytes = [0u8; 320];
        let u: &SsrUniform = bytemuck::from_bytes(&bytes);
        assert_eq!(u.params, [0.0; 4]);
        assert_eq!(u.screen_size, [0.0; 4]);
    }
}
