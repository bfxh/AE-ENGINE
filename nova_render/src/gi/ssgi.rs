//! SSGI（Screen Space Global Illumination）
//!
//! 完整的 wgpu 24 后处理 pipeline：
//! - 全屏三角形 vertex shader
//! - 简化版 SSGI：采样邻域颜色（半球采样模式，复用 SSAO 模式）
//! - 累积邻域颜色作为间接光照，叠加到原始颜色
//!
//! 简化策略：无深度重构，直接在屏幕空间采样邻域颜色求和。
//! 适合 MVP；后续可加入深度 + 法线重构做精确 SSGI。

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayout, Buffer, RenderPipeline, Sampler};

pub struct Ssgi {
    pub max_steps: u32,
    pub radius: f32,
    pub intensity: f32,
    pipeline: Option<RenderPipeline>,
    bind_layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    uniform_buffer: Option<Buffer>,
}

impl Default for Ssgi {
    fn default() -> Self {
        Self {
            max_steps: 16,
            radius: 0.5,
            intensity: 0.3,
            pipeline: None,
            bind_layout: None,
            sampler: None,
            uniform_buffer: None,
        }
    }
}

/// SSGI Uniform（32 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SsgiUniform {
    pub max_steps: u32,
    pub radius: f32,
    pub intensity: f32,
    pub _pad0: f32,
    pub width: f32,
    pub height: f32,
    pub _pad1: [f32; 2],
}

const SSGI_SHADER: &str = r#"
struct SsgiUniform {
    max_steps: u32,
    radius: f32,
    intensity: f32,
    _pad0: f32,
    width: f32,
    height: f32,
    _pad1: vec2<f32>,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: SsgiUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var x = p;
    x = vec2<f32>(dot(x, vec2<f32>(127.1, 311.7)), dot(x, vec2<f32>(269.5, 183.3)));
    return fract(sin(x) * 43758.5453);
}

@fragment
fn fs_ssgi(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.width, u.height);
    let texel = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let uv = pos.xy / dims;

    let center = textureSample(t_src, s_src, uv).rgb;

    let samples = clamp(u.max_steps, 1u, 32u);
    let radius_pixels = max(u.radius * 0.01 * max(dims.x, dims.y), 1.0);

    var indirect = vec3<f32>(0.0);
    var total_weight = 0.0;

    // 黄金角螺旋采样（与 SSAO 类似的均匀分布）
    let golden_angle = 2.399963;
    for (var i = 0u; i < 32u; i = i + 1u) {
        if (i >= samples) {
            break;
        }
        let fi = f32(i) + 0.5;
        let r = sqrt(fi / f32(samples)) * radius_pixels;
        let theta = fi * golden_angle;
        let offset = vec2<f32>(cos(theta), sin(theta)) * r * texel;

        let sample_uv = uv + offset;
        let sample_color = textureSample(t_src, s_src, sample_uv).rgb;

        // 距离衰减权重（模拟近处贡献更大）
        let dist = length(offset) / (radius_pixels * texel.x);
        let weight = 1.0 - dist;
        let w = max(weight, 0.0);

        indirect = indirect + sample_color * w;
        total_weight = total_weight + w;
    }

    if (total_weight > 0.0001) {
        indirect = indirect / total_weight;
    } else {
        indirect = vec3<f32>(0.0);
    }

    // SSGI = 原色 + 间接光 * intensity
    let result = center + indirect * u.intensity;
    return vec4<f32>(result, 1.0);
}
"#;

impl Ssgi {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi bind group layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<SsgiUniform>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ssgi uniform buffer"),
            size: std::mem::size_of::<SsgiUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssgi sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi shader"),
            source: wgpu::ShaderSource::Wgsl(SSGI_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssgi pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ssgi"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
            max_steps: 16,
            radius: 0.5,
            intensity: 0.3,
            pipeline: Some(pipeline),
            bind_layout: Some(bind_layout),
            sampler: Some(sampler),
            uniform_buffer: Some(uniform_buffer),
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        let pipeline = self.pipeline.as_ref().expect("Ssgi::new() must be called before render()");
        let bind_layout = self.bind_layout.as_ref().expect("Ssgi::new() must be called before render()");
        let sampler = self.sampler.as_ref().expect("Ssgi::new() must be called before render()");
        let uniform_buffer = self.uniform_buffer.as_ref().expect("Ssgi::new() must be called before render()");

        let safe_w = width.max(1) as f32;
        let safe_h = height.max(1) as f32;
        let uniform = SsgiUniform {
            max_steps: self.max_steps,
            radius: self.radius,
            intensity: self.intensity,
            _pad0: 0.0,
            width: safe_w,
            height: safe_h,
            _pad1: [0.0, 0.0],
        };
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssgi bind group"),
            layout: bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ssgi encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssgi render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        assert_eq!(std::mem::size_of::<SsgiUniform>(), 32);
    }

    #[test]
    fn default_values() {
        let e = Ssgi::default();
        assert_eq!(e.max_steps, 16);
        assert!((e.radius - 0.5).abs() < 1e-6);
        assert!((e.intensity - 0.3).abs() < 1e-6);
        assert!(e.pipeline.is_none());
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(SSGI_SHADER.contains("vs_fullscreen"));
        assert!(SSGI_SHADER.contains("fs_ssgi"));
        assert!(SSGI_SHADER.contains("golden_angle"));
        assert!(SSGI_SHADER.contains("indirect"));
        assert!(SSGI_SHADER.contains("intensity"));
    }
}