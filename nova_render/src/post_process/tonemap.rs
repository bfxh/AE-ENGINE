//! Tonemap 效果（ACES / Reinhard / Uncharted2）
//!
//! 完整的 wgpu 24 后处理 pipeline：
//! - 全屏三角形 vertex shader
//! - 三种 tonemap operator（uniform 切换）
//! - exposure 在 shader 中应用
//! - gamma 校正输出

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayout, Buffer, RenderPipeline, Sampler};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TonemapOperator {
    Aces,
    Reinhard,
    Uncharted2,
    None,
}

impl TonemapOperator {
    fn as_u32(self) -> u32 {
        match self {
            TonemapOperator::Aces => 0,
            TonemapOperator::Reinhard => 1,
            TonemapOperator::Uncharted2 => 2,
            TonemapOperator::None => 3,
        }
    }
}

/// Tonemap Uniform（32 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct TonemapUniform {
    pub exposure: f32,
    pub gamma: f32,
    pub operator: u32,
    pub _pad0: f32,
    pub width: f32,
    pub height: f32,
    pub _pad1: [f32; 2],
}

pub struct TonemapEffect {
    pub operator: TonemapOperator,
    pub exposure: f32,
    pub gamma: f32,
    pipeline: Option<RenderPipeline>,
    bind_layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    uniform_buffer: Option<Buffer>,
}

impl Default for TonemapEffect {
    fn default() -> Self {
        Self {
            operator: TonemapOperator::Aces,
            exposure: 1.0,
            gamma: 2.2,
            pipeline: None,
            bind_layout: None,
            sampler: None,
            uniform_buffer: None,
        }
    }
}

const TONEMAP_SHADER: &str = r#"
struct TonemapUniform {
    exposure: f32,
    gamma: f32,
    operator: u32,
    _pad0: f32,
    width: f32,
    height: f32,
    _pad1: vec2<f32>,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: TonemapUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (1.0 + x);
}

fn uncharted2_partial(x: vec3<f32>) -> vec3<f32> {
    let A = 0.15;
    let B = 0.50;
    let C = 0.10;
    let D = 0.20;
    let E = 0.02;
    let F = 0.30;
    return ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F;
}

fn uncharted2(x: vec3<f32>) -> vec3<f32> {
    let exposure_scale = 2.0;
    let curr = uncharted2_partial(x * exposure_scale);
    let white_scale = 1.0 / uncharted2_partial(vec3<f32>(11.2));
    return curr * white_scale;
}

@fragment
fn fs_tonemap(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.width, u.height);
    let uv = pos.xy / dims;
    var color = textureSample(t_src, s_src, uv).rgb;

    color = color * u.exposure;

    if (u.operator == 0u) {
        color = aces_filmic(color);
    } else if (u.operator == 1u) {
        color = reinhard(color);
    } else if (u.operator == 2u) {
        color = uncharted2(color);
    }

    color = pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / u.gamma));
    return vec4<f32>(color, 1.0);
}
"#;

impl TonemapEffect {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<TonemapUniform>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tonemap uniform buffer"),
            size: std::mem::size_of::<TonemapUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tonemap sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tonemap shader"),
            source: wgpu::ShaderSource::Wgsl(TONEMAP_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemap pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_tonemap"),
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
            operator: TonemapOperator::Aces,
            exposure: 1.0,
            gamma: 2.2,
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
        let pipeline = self.pipeline.as_ref().expect("TonemapEffect::new() must be called before render()");
        let bind_layout = self.bind_layout.as_ref().expect("TonemapEffect::new() must be called before render()");
        let sampler = self.sampler.as_ref().expect("TonemapEffect::new() must be called before render()");
        let uniform_buffer = self.uniform_buffer.as_ref().expect("TonemapEffect::new() must be called before render()");

        let safe_w = width.max(1) as f32;
        let safe_h = height.max(1) as f32;
        let uniform = TonemapUniform {
            exposure: self.exposure,
            gamma: self.gamma,
            operator: self.operator.as_u32(),
            _pad0: 0.0,
            width: safe_w,
            height: safe_h,
            _pad1: [0.0, 0.0],
        };
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap bind group"),
            layout: bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("tonemap encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tonemap render pass"),
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
        assert_eq!(std::mem::size_of::<TonemapUniform>(), 32);
    }

    #[test]
    fn operator_as_u32() {
        assert_eq!(TonemapOperator::Aces.as_u32(), 0);
        assert_eq!(TonemapOperator::Reinhard.as_u32(), 1);
        assert_eq!(TonemapOperator::Uncharted2.as_u32(), 2);
        assert_eq!(TonemapOperator::None.as_u32(), 3);
    }

    #[test]
    fn default_values() {
        let e = TonemapEffect::default();
        assert_eq!(e.operator, TonemapOperator::Aces);
        assert!((e.exposure - 1.0).abs() < 1e-6);
        assert!((e.gamma - 2.2).abs() < 1e-6);
        assert!(e.pipeline.is_none());
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(TONEMAP_SHADER.contains("vs_fullscreen"));
        assert!(TONEMAP_SHADER.contains("fs_tonemap"));
        assert!(TONEMAP_SHADER.contains("aces_filmic"));
        assert!(TONEMAP_SHADER.contains("reinhard"));
        assert!(TONEMAP_SHADER.contains("uncharted2"));
    }
}