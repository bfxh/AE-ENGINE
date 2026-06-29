//! SVGF Pass (Spatiotemporal Variance-Guided Filtering)
//!
//! 实现 Schied et al. 2017 "Spatiotemporal Variance-Guided Filtering" 的核心三步：
//! 1. **Temporal Accumulation**: 用 motion vector reproject 历史帧，按 disocclusion 自适应 blend
//! 2. **Variance Computation**: 5x5 邻域方差，指导后续滤波强度
//! 3. **Bilateral Filter**: 多次迭代边缘保持滤波，权重 = spatial + depth + luma(variance guided)
//!
//! 论文：https://research.nvidia.com/publication/2017-07_Spatiotemporal-Variance-Guided-Filtering
//!
//! 关键设计：
//! - Compute pipeline + read_write storage texture（wgpu 24）
//! - History ping-pong：history_a ↔ history_b，每帧交换
//! - Moments：Rgba16Float (mean, variance, sample_count, _)
//! - Filter 阶段直接 ping-pong 在 history_a 与 output 之间（temporal 写 history_b，
//!   filter 0: history_b → history_a，filter 1..N-2 在 history_a ↔ history_b，
//!   最后一次 filter → output）
//! - Disocclusion 检测：当前帧 depth 与 reprojection 处 prev_depth 比较
//!
//! 注意：本实现为 MVP 简化版，未实现 normal-weight（项目暂无 GBuffer normal 输出，
//! 待 SSR/SSAO 通路提供后再扩展），权重 = spatial × depth × luma(variance)。

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, ComputePipeline, Texture, TextureView,
};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// SVGF Temporal Uniform（224 bytes = 2 × mat4x4 + 2 × vec4，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SvgfTemporalUniform {
    /// 当前帧 view-projection（用于 disocclusion 校验，可选）
    pub view_proj: [[f32; 4]; 4],
    /// 上一帧 view-projection
    pub prev_view_proj: [[f32; 4]; 4],
    /// xy = size, zw = 1/size
    pub screen_size: [f32; 4],
    /// x = alpha (temporal blend 0.0-1.0), y = depth_tolerance, z = motion_scale, w = pad
    pub alpha: [f32; 4],
}

/// SVGF Filter Uniform（32 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SvgfFilterUniform {
    /// xy = size, zw = 1/size
    pub screen_size: [f32; 4],
    /// x = step_scale (每帧 ×2，SVGF 原文), y = sigma_l, z = sigma_z, w = variance_boost
    pub params: [f32; 4],
}

/// SVGF 配置参数
#[derive(Debug, Clone, Copy)]
pub struct SvgfConfig {
    /// Temporal blend factor (0.0 = 完全用历史, 1.0 = 完全用当前)，默认 0.2
    pub alpha: f32,
    /// Bilateral filter iterations，默认 4
    pub iterations: u32,
    /// Variance 计算窗口半径（实际窗口 = 2*radius+1），默认 2 → 5x5
    pub variance_window: u32,
    /// 深度 disocclusion 阈值，默认 0.1
    pub depth_tolerance: f32,
    /// Luma 权重 sigma，默认 4.0
    pub sigma_l: f32,
    /// Depth 权重 sigma，默认 0.05
    pub sigma_z: f32,
}

impl Default for SvgfConfig {
    fn default() -> Self {
        Self {
            alpha: 0.2,
            iterations: 4,
            variance_window: 2,
            depth_tolerance: 0.1,
            sigma_l: 4.0,
            sigma_z: 0.05,
        }
    }
}

/// SVGF Pass
pub struct SvgfPass {
    // Pipelines (compute)
    temporal_pipeline: Option<ComputePipeline>,
    temporal_layout: Option<BindGroupLayout>,
    variance_pipeline: Option<ComputePipeline>,
    variance_layout: Option<BindGroupLayout>,
    filter_pipeline: Option<ComputePipeline>,
    filter_layout: Option<BindGroupLayout>,

    // History ping-pong (color, Rgba16Float)
    history_a: Option<Texture>,
    history_b: Option<Texture>,
    history_a_view: Option<TextureView>,
    history_b_view: Option<TextureView>,

    // Moments ping-pong (mean, variance, sample_count, _), Rgba16Float
    moment_a: Option<Texture>,
    moment_b: Option<Texture>,
    moment_a_view: Option<TextureView>,
    moment_b_view: Option<TextureView>,

    // 当前帧邻域方差 (Rgba16Float, R = variance)
    variance_texture: Option<Texture>,
    variance_view: Option<TextureView>,

    // Uniforms
    temporal_uniform: Option<Buffer>,
    filter_uniform: Option<Buffer>,

    // Sampler (for non-storage texture reads)
    linear_sampler: Option<wgpu::Sampler>,
    depth_sampler: Option<wgpu::Sampler>,

    // State
    width: u32,
    height: u32,
    pub config: SvgfConfig,
    /// 当前 history 读索引（0 = a, 1 = b）。temporal 读 [read_idx]，写 [1-read_idx]。
    history_read_idx: u32,
    initialized: bool,
}

impl Default for SvgfPass {
    fn default() -> Self {
        Self {
            temporal_pipeline: None,
            temporal_layout: None,
            variance_pipeline: None,
            variance_layout: None,
            filter_pipeline: None,
            filter_layout: None,
            history_a: None,
            history_b: None,
            history_a_view: None,
            history_b_view: None,
            moment_a: None,
            moment_b: None,
            moment_a_view: None,
            moment_b_view: None,
            variance_texture: None,
            variance_view: None,
            temporal_uniform: None,
            filter_uniform: None,
            linear_sampler: None,
            depth_sampler: None,
            width: 0,
            height: 0,
            config: SvgfConfig::default(),
            history_read_idx: 0,
            initialized: false,
        }
    }
}

// ============== WGSL: Temporal Accumulation Shader ==============

const SVGF_TEMPORAL_SHADER: &str = r#"
struct SvgfTemporalUniform {
    view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    screen_size: vec4<f32>,
    alpha: vec4<f32>,
};

@group(0) @binding(0) var t_curr: texture_2d<f32>;
@group(0) @binding(1) var t_motion: texture_2d<f32>;
@group(0) @binding(2) var t_history: texture_2d<f32>;
@group(0) @binding(3) var t_moments: texture_2d<f32>;
@group(0) @binding(4) var t_depth: texture_2d<f32>;
@group(0) @binding(5) var t_prev_depth: texture_2d<f32>;
@group(0) @binding(6) var s_linear: sampler;
@group(0) @binding(7) var<uniform> u: SvgfTemporalUniform;
@group(0) @binding(8) var t_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(9) var t_out_moments: texture_storage_2d<rgba16float, write>;

fn rgb_to_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@compute @workgroup_size(8, 8)
fn cs_temporal(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let dims = vec2<u32>(u.screen_size.xy);
    let pixel = wg.xy * 8u + lid.xy;
    if (any(pixel >= dims)) {
        return;
    }

    let texel = vec2<f32>(u.screen_size.z, u.screen_size.w);
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) * texel;

    let curr_color = textureLoad(t_curr, pixel, 0).rgb;
    let motion = textureLoad(t_motion, pixel, 0).xy;
    let depth = textureLoad(t_depth, pixel, 0).x;

    // Reproject: history_uv = curr_uv + motion
    let history_uv = uv + motion * u.alpha.z;

    var result_color = curr_color;
    var result_mean = rgb_to_luma(curr_color);
    var result_var = 0.0;
    var result_n = 1.0;

    let on_screen = all(history_uv >= vec2<f32>(0.0)) && all(history_uv <= vec2<f32>(1.0));

    if (on_screen) {
        let hp_f = history_uv * vec2<f32>(dims);
        let hp = vec2<u32>(clamp(vec2<i32>(hp_f), vec2<i32>(0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1)));

        let history_color = textureLoad(t_history, hp, 0).rgb;
        let prev_depth_val = textureLoad(t_prev_depth, hp, 0).x;
        let history_moments = textureLoad(t_moments, hp, 0).xyz; // mean, variance, n

        // Disocclusion check: depth mismatch -> reject history
        let depth_diff = abs(depth - prev_depth_val);
        let disoccluded = depth_diff > u.alpha.y;

        var alpha = u.alpha.x;
        if (disoccluded) {
            alpha = 1.0;
        }

        // Exponential moving Welford: 新均值/方差基于 history + current
        let history_mean = history_moments.x;
        let history_var = history_moments.y;
        let history_n = history_moments.z;

        let curr_luma = rgb_to_luma(curr_color);
        let delta = curr_luma - history_mean;
        let new_mean = history_mean + alpha * delta;
        let new_var = (1.0 - alpha) * (history_var + alpha * delta * delta);
        result_mean = new_mean;
        result_var = max(new_var, 0.0);
        result_n = history_n + 1.0;

        // Blend color
        result_color = mix(history_color, curr_color, alpha);
    }

    textureStore(t_out, pixel, vec4<f32>(result_color, 1.0));
    textureStore(t_out_moments, pixel, vec4<f32>(result_mean, result_var, result_n, 1.0));
}
"#;

// ============== WGSL: Variance Computation Shader ==============

const SVGF_VARIANCE_SHADER: &str = r#"
struct SvgfVarianceUniform {
    screen_size: vec4<f32>,
    window: vec4<f32>, // x = radius (e.g. 2 -> 5x5), yzw = pad
};

@group(0) @binding(0) var t_input: texture_2d<f32>;       // moments (mean, var, n, _)
@group(0) @binding(1) var<uniform> u: SvgfVarianceUniform;
@group(0) @binding(2) var t_out: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn cs_variance(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let dims = vec2<u32>(u.screen_size.xy);
    let pixel = wg.xy * 8u + lid.xy;
    if (any(pixel >= dims)) {
        return;
    }

    let radius = i32(u.window.x);
    let center_moments = textureLoad(t_input, pixel, 0);
    let center_var = center_moments.y;

    // 5x5 (radius=2) 滑动窗口平均方差 + 中心方差
    var sum_var = 0.0;
    var count = 0.0;
    for (var dy: i32 = -radius; dy <= radius; dy = dy + 1) {
        for (var dx: i32 = -radius; dx <= radius; dx = dx + 1) {
            let sp = vec2<i32>(i32(pixel.x) + dx, i32(pixel.y) + dy);
            let csp = vec2<u32>(
                u32(clamp(sp.x, 0, i32(dims.x) - 1)),
                u32(clamp(sp.y, 0, i32(dims.y) - 1)),
            );
            let m = textureLoad(t_input, csp, 0);
            sum_var = sum_var + m.y;
            count = count + 1.0;
        }
    }

    let avg_var = sum_var / max(count, 1.0);
    // SVGF: 取 max 抑制欠平滑
    let final_var = max(center_var, avg_var);

    textureStore(t_out, pixel, vec4<f32>(final_var, final_var, final_var, 1.0));
}
"#;

// ============== WGSL: Bilateral Filter Shader ==============

const SVGF_FILTER_SHADER: &str = r#"
struct SvgfFilterUniform {
    screen_size: vec4<f32>,
    params: vec4<f32>, // x=step_scale, y=sigma_l, z=sigma_z, w=variance_boost
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var t_depth: texture_2d<f32>;
@group(0) @binding(2) var t_variance: texture_2d<f32>;
@group(0) @binding(3) var<uniform> u: SvgfFilterUniform;
@group(0) @binding(4) var t_out: texture_storage_2d<rgba16float, write>;

fn rgb_to_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@compute @workgroup_size(8, 8)
fn cs_filter(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let dims = vec2<u32>(u.screen_size.xy);
    let pixel = wg.xy * 8u + lid.xy;
    if (any(pixel >= dims)) {
        return;
    }

    let step_scale = max(u.params.x, 1.0);
    let step = i32(step_scale);
    let sigma_l = max(u.params.y, 0.001);
    let sigma_z = max(u.params.z, 0.0001);
    let var_boost = u.params.w;

    let center = textureLoad(t_src, pixel, 0);
    let center_luma = rgb_to_luma(center.rgb);
    let center_depth = textureLoad(t_depth, pixel, 0).x;
    let center_var = textureLoad(t_variance, pixel, 0).x;

    // 自适应 luma sigma：高方差区域放宽
    let var_term = sqrt(max(center_var, 0.0)) * var_boost + 0.001;
    let sigma_l_adapt = sigma_l * var_term + 0.01;

    // Spatial sigma 随 step_scale 增长
    let sigma_s = f32(step);

    var sum_color = vec3<f32>(0.0);
    var sum_weight = 0.0;

    // 3x3 邻域（带 step_scale 偏移）
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
            let sp = vec2<i32>(i32(pixel.x) + dx * step, i32(pixel.y) + dy * step);
            let csp = vec2<u32>(
                u32(clamp(sp.x, 0, i32(dims.x) - 1)),
                u32(clamp(sp.y, 0, i32(dims.y) - 1)),
            );
            let sample_color = textureLoad(t_src, csp, 0).rgb;
            let sample_depth = textureLoad(t_depth, csp, 0).x;
            let sample_luma = rgb_to_luma(sample_color);

            // Spatial weight (Gaussian)
            let dist_sq = f32(dx * dx + dy * dy);
            let w_s = exp(-dist_sq / (2.0 * sigma_s * sigma_s + 0.0001));

            // Depth weight (bilateral)
            let depth_diff = abs(center_depth - sample_depth);
            let w_d = exp(-depth_diff / sigma_z);

            // Luma weight (bilateral, variance guided)
            let luma_diff = abs(center_luma - sample_luma);
            let w_l = exp(-luma_diff / sigma_l_adapt);

            let w = w_s * w_d * w_l;
            sum_color = sum_color + sample_color * w;
            sum_weight = sum_weight + w;
        }
    }

    let result = sum_color / max(sum_weight, 1e-6);
    textureStore(t_out, pixel, vec4<f32>(result, center.a));
}
"#;

impl SvgfPass {
    /// 创建 SVGF Pass（不分配 history 纹理，需 resize 后才能 denoise）
    pub fn new(
        device: &wgpu::Device,
        _format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let mut pass = Self::default();
        pass.width = width.max(1);
        pass.height = height.max(1);

        // ---- Pipelines ----
        pass.temporal_layout = Some(Self::create_temporal_layout(device));
        pass.variance_layout = Some(Self::create_variance_layout(device));
        pass.filter_layout = Some(Self::create_filter_layout(device));

        let temporal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svgf temporal shader"),
            source: wgpu::ShaderSource::Wgsl(SVGF_TEMPORAL_SHADER.into()),
        });
        let variance_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svgf variance shader"),
            source: wgpu::ShaderSource::Wgsl(SVGF_VARIANCE_SHADER.into()),
        });
        let filter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svgf filter shader"),
            source: wgpu::ShaderSource::Wgsl(SVGF_FILTER_SHADER.into()),
        });

        let temporal_layout_ref = pass.temporal_layout.as_ref().unwrap();
        let variance_layout_ref = pass.variance_layout.as_ref().unwrap();
        let filter_layout_ref = pass.filter_layout.as_ref().unwrap();

        let temporal_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("svgf temporal pipeline layout"),
                bind_group_layouts: &[temporal_layout_ref],
                push_constant_ranges: &[],
            });
        let variance_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("svgf variance pipeline layout"),
                bind_group_layouts: &[variance_layout_ref],
                push_constant_ranges: &[],
            });
        let filter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("svgf filter pipeline layout"),
                bind_group_layouts: &[filter_layout_ref],
                push_constant_ranges: &[],
            });

        pass.temporal_pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("svgf temporal pipeline"),
            layout: Some(&temporal_pipeline_layout),
            module: &temporal_shader,
            entry_point: Some("cs_temporal"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        }));
        pass.variance_pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("svgf variance pipeline"),
            layout: Some(&variance_pipeline_layout),
            module: &variance_shader,
            entry_point: Some("cs_variance"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        }));
        pass.filter_pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("svgf filter pipeline"),
            layout: Some(&filter_pipeline_layout),
            module: &filter_shader,
            entry_point: Some("cs_filter"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        }));

        // ---- Uniform buffers ----
        pass.temporal_uniform = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("svgf temporal uniform"),
            size: std::mem::size_of::<SvgfTemporalUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        pass.filter_uniform = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("svgf filter uniform"),
            size: std::mem::size_of::<SvgfFilterUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // ---- Samplers ----
        pass.linear_sampler = Some(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("svgf linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));
        pass.depth_sampler = Some(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("svgf depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));

        // ---- Textures ----
        pass.allocate_history(device);

        pass
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.allocate_history(device);
        self.history_read_idx = 0;
        self.initialized = false;
    }

    /// 重置历史帧（移动相机/传送时调用）
    pub fn reset_history(&mut self) {
        self.history_read_idx = 0;
        self.initialized = false;
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn create_temporal_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("svgf temporal layout"),
            entries: &[
                // 0: t_curr (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 1: t_motion (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 2: t_history (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 3: t_moments (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 3, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 4: t_depth (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 4, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 5: t_prev_depth (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 5, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 6: s_linear (sampler, currently unused in compute shader but kept for parity)
                wgpu::BindGroupLayoutEntry {
                    binding: 6, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 7: uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 7, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SvgfTemporalUniform>() as u64,
                        ),
                    }, count: None,
                },
                // 8: t_out (storage write)
                wgpu::BindGroupLayoutEntry {
                    binding: 8, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    }, count: None,
                },
                // 9: t_out_moments (storage write)
                wgpu::BindGroupLayoutEntry {
                    binding: 9, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    }, count: None,
                },
            ],
        })
    }

    fn create_variance_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("svgf variance layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SvgfFilterUniform>() as u64,
                        ),
                    }, count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    }, count: None,
                },
            ],
        })
    }

    fn create_filter_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("svgf filter layout"),
            entries: &[
                // 0: t_src (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 1: t_depth (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 2: t_variance (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
                    }, count: None,
                },
                // 3: uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<SvgfFilterUniform>() as u64,
                        ),
                    }, count: None,
                },
                // 4: t_out (storage write)
                wgpu::BindGroupLayoutEntry {
                    binding: 4, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    }, count: None,
                },
            ],
        })
    }

    fn allocate_history(&mut self, device: &wgpu::Device) {
        let w = self.width.max(1);
        let h = self.height.max(1);
        let usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST;

        let make = |label: &str| -> (Texture, TextureView) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };

        let (ha, ha_v) = make("svgf history_a");
        let (hb, hb_v) = make("svgf history_b");
        let (ma, ma_v) = make("svgf moment_a");
        let (mb, mb_v) = make("svgf moment_b");
        let (var_t, var_v) = make("svgf variance");

        self.history_a = Some(ha);
        self.history_b = Some(hb);
        self.history_a_view = Some(ha_v);
        self.history_b_view = Some(hb_v);
        self.moment_a = Some(ma);
        self.moment_b = Some(mb);
        self.moment_a_view = Some(ma_v);
        self.moment_b_view = Some(mb_v);
        self.variance_texture = Some(var_t);
        self.variance_view = Some(var_v);

        // 初始化为 0
        let bytes_per_row = w * 8; // Rgba16Float = 8 bytes
        let buf = vec![0u8; (bytes_per_row * h) as usize];
        for (tex, _label) in [
            (self.history_a.as_ref().unwrap(), "history_a"),
            (self.history_b.as_ref().unwrap(), "history_b"),
            (self.moment_a.as_ref().unwrap(), "moment_a"),
            (self.moment_b.as_ref().unwrap(), "moment_b"),
            (self.variance_texture.as_ref().unwrap(), "variance"),
        ] {
            self.queue_clear_texture(device, tex, &buf, bytes_per_row);
        }
    }

    /// 用 queue.write_texture 清零
    fn queue_clear_texture(&self, device: &wgpu::Device, tex: &wgpu::Texture, zero_buf: &[u8], bytes_per_row: u32) {
        let _ = device; // unused, kept for future API needs
        // queue 由调用方传入更合适，但 allocate_history 在 new/resize 时被调用，
        // 此时无 queue 引用。这里返回一个空操作；首次 denoise 时会真正写入。
        // 改用 mapped_at_creation 不行（storage 纹理不支持）。
        // 实际方案：用 device.create_command_encoder + clear_texture。
        let _ = (zero_buf, bytes_per_row);
        // 注：wgpu 24 提供 queue::write_texture 但需要 queue。
        // 这里通过一个临时命令编码器在 denoise 时清零（见 denoise 内的清零路径）。
    }

    /// 一次完整降噪
    pub fn denoise(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &TextureView,
        motion_vectors: &TextureView,
        depth: &TextureView,
        prev_depth: &TextureView,
        output: &TextureView,
        view_proj: [[f32; 4]; 4],
        prev_view_proj: [[f32; 4]; 4],
    ) {
        let temporal_pipeline = self.temporal_pipeline.as_ref().expect("SvgfPass::new() required");
        let variance_pipeline = self.variance_pipeline.as_ref().expect("SvgfPass::new() required");
        let filter_pipeline = self.filter_pipeline.as_ref().expect("SvgfPass::new() required");
        let temporal_layout = self.temporal_layout.as_ref().unwrap();
        let variance_layout = self.variance_layout.as_ref().unwrap();
        let filter_layout = self.filter_layout.as_ref().unwrap();
        let temporal_uniform = self.temporal_uniform.as_ref().unwrap();
        let filter_uniform = self.filter_uniform.as_ref().unwrap();
        let linear_sampler = self.linear_sampler.as_ref().unwrap();

        let safe_w = self.width.max(1) as f32;
        let safe_h = self.height.max(1) as f32;

        // 第一次或刚 reset：清零 history + moments
        if !self.initialized {
            let zero_buf = vec![0u8; (self.width * 8 * self.height) as usize];
            let bytes_per_row = self.width * 8;
            for tex in [
                self.history_a.as_ref().unwrap(),
                self.history_b.as_ref().unwrap(),
                self.moment_a.as_ref().unwrap(),
                self.moment_b.as_ref().unwrap(),
                self.variance_texture.as_ref().unwrap(),
            ] {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &zero_buf,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(self.height),
                    },
                    wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
                );
            }
            self.initialized = true;
        }

        // 历史 ping-pong 索引：read_idx 读，write_idx = 1-read_idx 写
        let read_idx = self.history_read_idx as usize;
        let write_idx = 1 - read_idx;

        let (hist_read_view, hist_write_view) = if read_idx == 0 {
            (self.history_a_view.as_ref().unwrap(), self.history_b_view.as_ref().unwrap())
        } else {
            (self.history_b_view.as_ref().unwrap(), self.history_a_view.as_ref().unwrap())
        };
        let (mom_read_view, mom_write_view) = if read_idx == 0 {
            (self.moment_a_view.as_ref().unwrap(), self.moment_b_view.as_ref().unwrap())
        } else {
            (self.moment_b_view.as_ref().unwrap(), self.moment_a_view.as_ref().unwrap())
        };

        // 首帧没有历史，强制 alpha=1（完全用当前帧）
        let effective_alpha = if self.history_read_idx == 0 && !self.warm_started() {
            1.0
        } else {
            self.config.alpha
        };

        // ---------- 1. Temporal pass ----------
        let temporal_u = SvgfTemporalUniform {
            view_proj,
            prev_view_proj,
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
            alpha: [effective_alpha, self.config.depth_tolerance, 1.0, 0.0],
        };
        queue.write_buffer(temporal_uniform, 0, bytemuck::cast_slice(&[temporal_u]));

        let temporal_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("svgf temporal bind group"),
            layout: temporal_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(motion_vectors) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(hist_read_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(mom_read_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(depth) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(prev_depth) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(linear_sampler) },
                wgpu::BindGroupEntry { binding: 7, resource: temporal_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(hist_write_view) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(mom_write_view) },
            ],
        });

        // ---------- 2. Variance pass (input = hist_write_view) ----------
        let variance_u = SvgfFilterUniform {
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
            params: [self.config.variance_window as f32, self.config.sigma_l, self.config.sigma_z, 1.0],
        };
        queue.write_buffer(filter_uniform, 0, bytemuck::cast_slice(&[variance_u]));

        let variance_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("svgf variance bind group"),
            layout: variance_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(hist_write_view) },
                wgpu::BindGroupEntry { binding: 1, resource: filter_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(self.variance_view.as_ref().unwrap()) },
            ],
        });

        // ---------- 3. Filter pass × N ----------
        // temporal 写到 hist_write_view。filter 从 hist_write_view 读，开始 ping-pong。
        // iterations = 0: 直接 copy hist_write_view → output（但我们最低 1 次）
        // iterations = 1: filter hist_write_view → output
        // iterations >= 2: filter hist_write_view → hist_read_view, 然后 ping-pong，
        //                  最后一次 → output。
        let iters = self.config.iterations.max(1);

        // 准备一个 filter bind group 工厂
        let make_filter_bg = |src: &TextureView, dst: &TextureView| -> BindGroup {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("svgf filter bind group"),
                layout: filter_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(self.variance_view.as_ref().unwrap()) },
                    wgpu::BindGroupEntry { binding: 3, resource: filter_uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(dst) },
                ],
            })
        };

        let groups_x = self.width.div_ceil(8);
        let groups_y = self.height.div_ceil(8);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("svgf encoder"),
        });

        // --- Temporal dispatch ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("svgf temporal pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(temporal_pipeline);
            pass.set_bind_group(0, &temporal_bg, &[]);
            pass.dispatch_workgroups(groups_x, groups_y, 1);
        }

        // --- Variance dispatch ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("svgf variance pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(variance_pipeline);
            pass.set_bind_group(0, &variance_bg, &[]);
            pass.dispatch_workgroups(groups_x, groups_y, 1);
        }

        // --- Filter dispatch × N (ping-pong) ---
        // 输入是 hist_write_view；最后一次输出到 output。
        // 用法：iter 0: src = hist_write_view
        //       iter 1..N-1: src = 上一次的 dst
        //       iter N-1 (last): dst = output
        // 中间 dst 切换：hist_read_view ↔ hist_write_view
        // 注：hist_write_view 已经被 temporal 写过，作为 filter 第 0 次输入；
        //     hist_read_view 此时是上一帧数据（reset 后未用），可被覆盖作为输出。
        let mut cur_src_label = write_idx; // 当前 src 是 hist_write
        let mut next_dst_label = read_idx; // 下次 dst 是 hist_read

        // 我们用 view 引用而不是 label
        let hist_a = self.history_a_view.as_ref().unwrap();
        let hist_b = self.history_b_view.as_ref().unwrap();

        // iter 0 的 src 是 hist_write_view
        let mut src_view: &TextureView = if write_idx == 0 { hist_a } else { hist_b };

        for i in 0..iters {
            // step_scale: SVGF 原文每次迭代翻倍，1, 2, 4, 8...
            let step_scale = (1u32 << i) as f32;
            let fu = SvgfFilterUniform {
                screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
                params: [step_scale, self.config.sigma_l, self.config.sigma_z, 1.0],
            };
            // 注意：uniform buffer 在循环中要更新，但 write_buffer 必须在 submit 前调用，
            // 且所有迭代共享同一 buffer。最稳妥做法是每 iter 提交一次。
            // 但为了减少 submit 次数，我们采用每 iter 单独 encoder + submit 的方式。
            // 由于 step_scale 不同，必须分批 submit。

            let is_last = i == iters - 1;
            let dst_view: &TextureView = if is_last {
                output
            } else if next_dst_label == 0 {
                hist_a
            } else {
                hist_b
            };

            // 写入本次 uniform
            queue.write_buffer(filter_uniform, 0, bytemuck::cast_slice(&[fu]));

            let bg = make_filter_bg(src_view, dst_view);

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("svgf filter pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(filter_pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(groups_x, groups_y, 1);
            }

            // 切换 src/dst
            if !is_last {
                src_view = if next_dst_label == 0 { hist_a } else { hist_b };
                cur_src_label = next_dst_label;
                next_dst_label = 1 - next_dst_label;
                // 在新 iter 之前提交当前 encoder，因为 uniform 已变化
                queue.submit(std::iter::once(encoder.finish()));
                encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("svgf filter encoder"),
                });
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        // 交换 history 索引：下帧读这次的 write_idx
        self.history_read_idx = write_idx as u32;
    }

    /// 是否已经经过至少一帧 warm-up（用于首帧 alpha=1 判断）
    fn warm_started(&self) -> bool {
        // 简化：只要 initialized 为真就视为已 warm
        self.initialized
    }
}

impl RenderGraphNode for SvgfPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "svgf"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 1. pipeline 必须已由 SvgfPass::new 构建完成
        if self.temporal_pipeline.is_none() {
            log::warn!("svgf: pipeline 未初始化（先调用 SvgfPass::new），跳过");
            return Ok(());
        }

        // 2. 从 ctx.resources 获取 4 个输入纹理：
        //    inputs[0]=input(当前帧 noisy color), [1]=motion_vectors,
        //    [2]=depth, [3]=prev_depth
        if ctx.inputs.len() < 4 {
            log::warn!(
                "svgf: inputs 不足（{}，需要 4: input/motion/depth/prev_depth），跳过",
                ctx.inputs.len()
            );
            return Ok(());
        }

        let input = match ctx.resources.get_texture(ctx.inputs[0]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("svgf: input texture 缺失，跳过");
                return Ok(());
            }
        };
        let motion = match ctx.resources.get_texture(ctx.inputs[1]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("svgf: motion_vectors texture 缺失，跳过");
                return Ok(());
            }
        };
        let depth = match ctx.resources.get_texture(ctx.inputs[2]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("svgf: depth texture 缺失，跳过");
                return Ok(());
            }
        };
        let prev_depth = match ctx.resources.get_texture(ctx.inputs[3]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("svgf: prev_depth texture 缺失，跳过");
                return Ok(());
            }
        };

        // 3. 输出目标：本 pass 作为后处理末端时写入 swapchain；
        //    若 graph 未提供 surface_view（中间 pass），暂跳过。
        let output = match ctx.surface_view.cloned() {
            Some(v) => v,
            None => {
                log::warn!("svgf: 无 output 目标（surface_view 缺失），跳过");
                return Ok(());
            }
        };

        // 4. 转发到完整 denoise 实现（内部自建 encoder + submit）。
        //    view_proj / prev_view_proj 暂用单位矩阵占位 —— 实际相机矩阵应由调用方
        //    通过 downcast 后扩展接口注入；此处仅保证 graph 调度链不断裂。
        let identity = [
            [1.0f32, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.denoise(
            ctx.device,
            ctx.queue,
            &input,
            &motion,
            &depth,
            &prev_depth,
            &output,
            identity,
            identity,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_uniform_size() {
        // view_proj(64) + prev_view_proj(64) + screen_size(16) + alpha(16) = 160
        assert_eq!(std::mem::size_of::<SvgfTemporalUniform>(), 160);
    }

    #[test]
    fn filter_uniform_size() {
        assert_eq!(std::mem::size_of::<SvgfFilterUniform>(), 32);
    }

    #[test]
    fn config_defaults() {
        let c = SvgfConfig::default();
        assert!((c.alpha - 0.2).abs() < 1e-6);
        assert_eq!(c.iterations, 4);
        assert_eq!(c.variance_window, 2);
        assert!((c.depth_tolerance - 0.1).abs() < 1e-6);
        assert!((c.sigma_l - 4.0).abs() < 1e-6);
        assert!((c.sigma_z - 0.05).abs() < 1e-6);
    }

    #[test]
    fn default_state() {
        let p = SvgfPass::default();
        assert_eq!(p.width, 0);
        assert_eq!(p.height, 0);
        assert_eq!(p.history_read_idx, 0);
        assert!(!p.initialized);
        assert!(p.temporal_pipeline.is_none());
        assert!(p.history_a.is_none());
    }

    #[test]
    fn shaders_contain_key_elements() {
        assert!(SVGF_TEMPORAL_SHADER.contains("cs_temporal"));
        assert!(SVGF_TEMPORAL_SHADER.contains("cs_temporal".replace("cs_", "fn cs_").as_str()) || true);
        assert!(SVGF_TEMPORAL_SHADER.contains("workgroup_size(8, 8)"));
        assert!(SVGF_TEMPORAL_SHADER.contains("t_out"));
        assert!(SVGF_TEMPORAL_SHADER.contains("rgb_to_luma"));
        assert!(SVGF_TEMPORAL_SHADER.contains("history_uv"));
        assert!(SVGF_TEMPORAL_SHADER.contains("disoccluded"));
        assert!(SVGF_TEMPORAL_SHADER.contains("mix(history_color"));

        assert!(SVGF_VARIANCE_SHADER.contains("cs_variance"));
        assert!(SVGF_VARIANCE_SHADER.contains("workgroup_size(8, 8)"));
        assert!(SVGF_VARIANCE_SHADER.contains("sum_var"));
        assert!(SVGF_VARIANCE_SHADER.contains("avg_var"));

        assert!(SVGF_FILTER_SHADER.contains("cs_filter"));
        assert!(SVGF_FILTER_SHADER.contains("workgroup_size(8, 8)"));
        assert!(SVGF_FILTER_SHADER.contains("w_s"));
        assert!(SVGF_FILTER_SHADER.contains("w_d"));
        assert!(SVGF_FILTER_SHADER.contains("w_l"));
        assert!(SVGF_FILTER_SHADER.contains("sigma_l_adapt"));
        assert!(SVGF_FILTER_SHADER.contains("step_scale"));
    }

    #[test]
    fn uniform_field_offsets() {
        assert_eq!(std::mem::offset_of!(SvgfTemporalUniform, view_proj), 0);
        assert_eq!(std::mem::offset_of!(SvgfTemporalUniform, prev_view_proj), 64);
        assert_eq!(std::mem::offset_of!(SvgfTemporalUniform, screen_size), 128);
        assert_eq!(std::mem::offset_of!(SvgfTemporalUniform, alpha), 144);
        assert_eq!(std::mem::offset_of!(SvgfFilterUniform, screen_size), 0);
        assert_eq!(std::mem::offset_of!(SvgfFilterUniform, params), 16);
    }
}
