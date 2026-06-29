//! FidelityFX CAS（Contrast Adaptive Sharpening）
//!
//! 完整的 wgpu 24 后处理 pipeline：
//! - 全屏三角形 vertex shader
//! - AMD FidelityFX CAS 算法（参考 GPUOpen 公开实现）
//! - 对比度自适应锐化

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayout, Buffer, RenderPipeline, Sampler};

pub struct CasEffect {
    pub sharpness: f32,
    pipeline: Option<RenderPipeline>,
    bind_layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    uniform_buffer: Option<Buffer>,
}

impl Default for CasEffect {
    fn default() -> Self {
        Self {
            sharpness: 0.4,
            pipeline: None,
            bind_layout: None,
            sampler: None,
            uniform_buffer: None,
        }
    }
}

/// CAS Uniform（16 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct CasUniform {
    pub sharpness: f32,
    pub width: f32,
    pub height: f32,
    pub _pad0: f32,
}

const CAS_SHADER: &str = r#"
struct CasUniform {
    sharpness: f32,
    width: f32,
    height: f32,
    _pad0: f32,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: CasUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn max3(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn min3(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

@fragment
fn fs_cas(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.width, u.height);
    let texel = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let uv = pos.xy / dims;

    let b = textureSample(t_src, s_src, uv + vec2<f32>(0.0, -texel.y)).rgb;
    let d = textureSample(t_src, s_src, uv + vec2<f32>(-texel.x, 0.0)).rgb;
    let e = textureSample(t_src, s_src, uv).rgb;
    let f = textureSample(t_src, s_src, uv + vec2<f32>( texel.x, 0.0)).rgb;
    let h = textureSample(t_src, s_src, uv + vec2<f32>(0.0,  texel.y)).rgb;

    let minG = min3(min(b, d) + min(f, h));
    let maxG = max3(max(b, d) + max(f, h));

    let sharpenAmount = u.sharpness;

    var aoc = clamp(minG / maxG, 0.0, 1.0);
    aoc = mix(1.0, sqrt(aoc), sharpenAmount);

    let v = clamp(mix(f + b + d + h, 4.0 * e, aoc), vec3<f32>(0.0), vec3<f32>(65504.0));

    let result = mix(e, v * 0.25, sharpenAmount);
    return vec4<f32>(result, 1.0);
}
"#;

impl CasEffect {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cas bind group layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<CasUniform>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cas uniform buffer"),
            size: std::mem::size_of::<CasUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cas shader"),
            source: wgpu::ShaderSource::Wgsl(CAS_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cas pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cas pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_cas"),
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
            sharpness: 0.4,
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
        let pipeline = self.pipeline.as_ref().expect("CasEffect::new() must be called before render()");
        let bind_layout = self.bind_layout.as_ref().expect("CasEffect::new() must be called before render()");
        let sampler = self.sampler.as_ref().expect("CasEffect::new() must be called before render()");
        let uniform_buffer = self.uniform_buffer.as_ref().expect("CasEffect::new() must be called before render()");

        let safe_w = width.max(1) as f32;
        let safe_h = height.max(1) as f32;
        let uniform = CasUniform {
            sharpness: self.sharpness.clamp(0.0, 1.0),
            width: safe_w,
            height: safe_h,
            _pad0: 0.0,
        };
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cas bind group"),
            layout: bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cas encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cas render pass"),
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
        assert_eq!(std::mem::size_of::<CasUniform>(), 16);
    }

    #[test]
    fn default_values() {
        let e = CasEffect::default();
        assert!((e.sharpness - 0.4).abs() < 1e-6);
        assert!(e.pipeline.is_none());
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(CAS_SHADER.contains("vs_fullscreen"));
        assert!(CAS_SHADER.contains("fs_cas"));
        assert!(CAS_SHADER.contains("sharpenAmount"));
        assert!(CAS_SHADER.contains("max3"));
        assert!(CAS_SHADER.contains("min3"));
    }
}