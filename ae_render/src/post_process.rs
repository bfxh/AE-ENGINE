//! Post Process Pipeline: HDR Tone Mapping + Bloom + Gamma
//!
//! 3A 级后处理管线：
//! 1. Bloom Extract: 从 HDR 纹理提取高亮度区域
//! 2. Bloom Blur: 高斯模糊（多 pass）
//! 3. Tonemap + Combine: ACES Filmic + Bloom 合成 + Gamma 校正
//!
//! 使用全屏三角形（无需 vertex buffer），降低 draw call 开销。

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline};

/// 后处理 Uniform（32 bytes，2 × vec4 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct PostProcessUniform {
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

impl PostProcessUniform {
    pub fn new(exposure: f32, gamma: f32, bloom_threshold: f32, bloom_intensity: f32) -> Self {
        Self {
            exposure,
            gamma,
            bloom_threshold,
            bloom_intensity,
            time: 0.0,
            blur_dir: [1.0, 0.0],
            _pad: 0.0,
        }
    }

    pub fn default_quality() -> Self {
        Self::new(1.0, 2.2, 1.0, 0.4)
    }
}

/// 后处理参数（运行时传入）
#[derive(Debug, Clone, Copy)]
pub struct PostProcessParams {
    pub exposure: f32,
    pub gamma: f32,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub time: f32,
}

impl Default for PostProcessParams {
    fn default() -> Self {
        Self { exposure: 1.0, gamma: 2.2, bloom_threshold: 1.0, bloom_intensity: 0.4, time: 0.0 }
    }
}

const FULLSCREEN_VS: &str = r#"
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
"#;

const BLOOM_EXTRACT_SHADER: &str = r#"
@group(0) @binding(0)
var t_src: texture_2d<f32>;
@group(0) @binding(1)
var s_src: sampler;
@group(0) @binding(2)
var<uniform> u: PostProcessUniform;

struct PostProcessUniform {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    time: f32,
    blur_dir: vec2<f32>,
    _pad: f32,
};

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

const BLOOM_BLUR_SHADER: &str = r#"
@group(0) @binding(0)
var t_src: texture_2d<f32>;
@group(0) @binding(1)
var s_src: sampler;
@group(0) @binding(2)
var<uniform> u: PostProcessUniform;

struct PostProcessUniform {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    time: f32,
    blur_dir: vec2<f32>,
    _pad: f32,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

// 9-tap 高斯模糊
@fragment
fn fs_blur(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_src);
    let uv = pos.xy / vec2<f32>(dims);
    let texel = 1.0 / vec2<f32>(dims);
    let dir = u.blur_dir * texel;

    // 高斯权重（9-tap, sigma ≈ 2.0）
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

const TONEMAP_SHADER: &str = r#"
@group(0) @binding(0)
var t_hdr: texture_2d<f32>;
@group(0) @binding(1)
var s_hdr: sampler;
@group(0) @binding(2)
var t_bloom: texture_2d<f32>;
@group(0) @binding(3)
var s_bloom: sampler;
@group(0) @binding(4)
var<uniform> u: PostProcessUniform;

struct PostProcessUniform {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    time: f32,
    blur_dir: vec2<f32>,
    _pad: f32,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

// ACES Filmic Tone Mapping
fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_tonemap(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_hdr);
    let uv = pos.xy / vec2<f32>(dims);

    let hdr_color = textureSample(t_hdr, s_hdr, uv).rgb;
    let bloom_color = textureSample(t_bloom, s_bloom, uv).rgb;

    // 合成 Bloom
    var color = hdr_color + bloom_color * u.bloom_intensity;

    // 曝光
    color = color * u.exposure;

    // ACES Tone Mapping
    color = aces_tonemap(color);

    // Gamma 校正
    color = pow(color, vec3<f32>(1.0 / u.gamma));

    return vec4<f32>(color, 1.0);
}
"#;

/// 后处理渲染器
pub struct PostProcessRenderer {
    pub extract_pipeline: RenderPipeline,
    pub blur_pipeline: RenderPipeline,
    pub tonemap_pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub uniform_layout: BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl PostProcessRenderer {
    pub fn new(
        device: &Device,
        hdr_format: wgpu::TextureFormat,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        // Uniform layout
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post process uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                        PostProcessUniform,
                    >() as u64),
                },
                count: None,
            }],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post process uniform buffer"),
            size: std::mem::size_of::<PostProcessUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 共享 sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post process sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bloom Extract pipeline (input: hdr, output: bloom)
        let extract_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom extract shader"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_EXTRACT_SHADER.into()),
        });

        let extract_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom extract layout"),
            bind_group_layouts: &[&Self::texture_sampler_uniform_layout(device, hdr_format)],
            push_constant_ranges: &[],
        });

        let extract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom extract pipeline"),
            layout: Some(&extract_layout),
            vertex: wgpu::VertexState {
                module: &extract_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &extract_shader,
                entry_point: Some("fs_extract"),
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

        // Blur pipeline
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom blur shader"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_BLUR_SHADER.into()),
        });

        let blur_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom blur layout"),
            bind_group_layouts: &[&Self::texture_sampler_uniform_layout(device, hdr_format)],
            push_constant_ranges: &[],
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom blur pipeline"),
            layout: Some(&blur_layout),
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

        // Tonemap pipeline (input: hdr + bloom, output: LDR)
        let tonemap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tonemap shader"),
            source: wgpu::ShaderSource::Wgsl(TONEMAP_SHADER.into()),
        });

        let tonemap_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap layout"),
            bind_group_layouts: &[&Self::hdr_bloom_layout(device, hdr_format)],
            push_constant_ranges: &[],
        });

        let tonemap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemap pipeline"),
            layout: Some(&tonemap_layout),
            vertex: wgpu::VertexState {
                module: &tonemap_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &tonemap_shader,
                entry_point: Some("fs_tonemap"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
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

        Self {
            extract_pipeline,
            blur_pipeline,
            tonemap_pipeline,
            uniform_buffer,
            uniform_layout,
            sampler,
        }
    }

    /// 创建 texture + sampler + uniform 的 bind group layout（用于 extract/blur）
    fn texture_sampler_uniform_layout(
        device: &Device,
        _format: wgpu::TextureFormat,
    ) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post process texture layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            PostProcessUniform,
                        >()
                            as u64),
                    },
                    count: None,
                },
            ],
        })
    }

    /// 创建 hdr + bloom + sampler + uniform 的 bind group layout（用于 tonemap）
    fn hdr_bloom_layout(device: &Device, _format: wgpu::TextureFormat) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemap bind group layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            PostProcessUniform,
                        >()
                            as u64),
                    },
                    count: None,
                },
            ],
        })
    }

    /// 更新 uniform
    pub fn update_uniform(&self, queue: &Queue, params: &PostProcessParams, blur_dir: [f32; 2]) {
        let u = PostProcessUniform {
            exposure: params.exposure,
            gamma: params.gamma,
            bloom_threshold: params.bloom_threshold,
            bloom_intensity: params.bloom_intensity,
            time: params.time,
            blur_dir,
            _pad: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[u]));
    }

    /// 创建 extract/blur bind group（1 texture + 1 sampler + uniform）
    pub fn create_texture_bind_group(
        &self,
        device: &Device,
        texture_view: &wgpu::TextureView,
    ) -> BindGroup {
        let layout = Self::texture_sampler_uniform_layout(device, wgpu::TextureFormat::Rgba16Float);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post process texture bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 tonemap bind group（hdr + bloom + 2 samplers + uniform）
    pub fn create_tonemap_bind_group(
        &self,
        device: &Device,
        hdr_view: &wgpu::TextureView,
        bloom_view: &wgpu::TextureView,
    ) -> BindGroup {
        let layout = Self::hdr_bloom_layout(device, wgpu::TextureFormat::Rgba16Float);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建 HDR 纹理（用于中间结果）
    pub fn create_hdr_texture(
        device: &Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        // 8 floats = 32 bytes
        assert_eq!(std::mem::size_of::<PostProcessUniform>(), 32);
    }

    #[test]
    fn default_quality() {
        let u = PostProcessUniform::default_quality();
        assert!((u.exposure - 1.0).abs() < 0.001);
        assert!((u.gamma - 2.2).abs() < 0.001);
        assert!((u.bloom_threshold - 1.0).abs() < 0.001);
        assert!((u.bloom_intensity - 0.4).abs() < 0.001);
    }

    #[test]
    fn params_default() {
        let p = PostProcessParams::default();
        assert!((p.exposure - 1.0).abs() < 0.001);
        assert!((p.gamma - 2.2).abs() < 0.001);
    }
}
