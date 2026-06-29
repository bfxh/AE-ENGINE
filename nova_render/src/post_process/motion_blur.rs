//! Motion Blur 效果
//!
//! 完整的 wgpu 24 后处理 pipeline：
//! - 全屏三角形 vertex shader
//! - 基于速度向量（velocity）的邻域采样运动模糊
//! - 无速度图时使用 camera 位移（uniform velocity）

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayout, Buffer, RenderPipeline, Sampler};

pub struct MotionBlurEffect {
    pub strength: f32,
    pub max_samples: u32,
    pipeline: Option<RenderPipeline>,
    bind_layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    uniform_buffer: Option<Buffer>,
}

impl Default for MotionBlurEffect {
    fn default() -> Self {
        Self {
            strength: 0.5,
            max_samples: 8,
            pipeline: None,
            bind_layout: None,
            sampler: None,
            uniform_buffer: None,
        }
    }
}

/// Motion Blur Uniform（32 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct MotionBlurUniform {
    pub strength: f32,
    pub max_samples: u32,
    pub width: f32,
    pub height: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub _pad: [f32; 2],
}

const MOTION_BLUR_SHADER: &str = r#"
struct MotionBlurUniform {
    strength: f32,
    max_samples: u32,
    width: f32,
    height: f32,
    velocity_x: f32,
    velocity_y: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: MotionBlurUniform;

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
fn fs_motion_blur(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.width, u.height);
    let texel = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let uv = pos.xy / dims;

    let center_color = textureSample(t_src, s_src, uv);
    let velocity = vec2<f32>(u.velocity_x, u.velocity_y) * u.strength;

    let speed = length(velocity);
    if (speed < 0.0001) {
        return center_color;
    }

    let samples = clamp(u.max_samples, 1u, 32u);
    let inv_samples = 1.0 / f32(samples);

    var sum = vec3<f32>(0.0);
    var total_weight = 0.0;

    for (var i = 0u; i < 32u; i = i + 1u) {
        if (i >= samples) {
            break;
        }
        let t = (f32(i) + 0.5) * inv_samples - 0.5;
        let offset = velocity * t * texel;
        let sample_uv = uv + offset;
        let w = 1.0 - abs(t) * 2.0;
        sum = sum + textureSample(t_src, s_src, sample_uv).rgb * max(w, 0.0);
        total_weight = total_weight + max(w, 0.0);
    }

    if (total_weight > 0.0001) {
        sum = sum / total_weight;
    } else {
        sum = center_color.rgb;
    }

    return vec4<f32>(sum, center_color.a);
}
"#;

impl MotionBlurEffect {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion_blur bind group layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<MotionBlurUniform>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion_blur uniform buffer"),
            size: std::mem::size_of::<MotionBlurUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("motion_blur sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("motion_blur shader"),
            source: wgpu::ShaderSource::Wgsl(MOTION_BLUR_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion_blur pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("motion_blur pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_motion_blur"),
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
            strength: 0.5,
            max_samples: 8,
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
        velocity: [f32; 2],
    ) {
        let pipeline = self.pipeline.as_ref().expect("MotionBlurEffect::new() must be called before render()");
        let bind_layout = self.bind_layout.as_ref().expect("MotionBlurEffect::new() must be called before render()");
        let sampler = self.sampler.as_ref().expect("MotionBlurEffect::new() must be called before render()");
        let uniform_buffer = self.uniform_buffer.as_ref().expect("MotionBlurEffect::new() must be called before render()");

        let safe_w = width.max(1) as f32;
        let safe_h = height.max(1) as f32;
        let uniform = MotionBlurUniform {
            strength: self.strength,
            max_samples: self.max_samples,
            width: safe_w,
            height: safe_h,
            velocity_x: velocity[0],
            velocity_y: velocity[1],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("motion_blur bind group"),
            layout: bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("motion_blur encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("motion_blur render pass"),
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
        assert_eq!(std::mem::size_of::<MotionBlurUniform>(), 32);
    }

    #[test]
    fn default_values() {
        let e = MotionBlurEffect::default();
        assert!((e.strength - 0.5).abs() < 1e-6);
        assert_eq!(e.max_samples, 8);
        assert!(e.pipeline.is_none());
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(MOTION_BLUR_SHADER.contains("vs_fullscreen"));
        assert!(MOTION_BLUR_SHADER.contains("fs_motion_blur"));
        assert!(MOTION_BLUR_SHADER.contains("velocity"));
        assert!(MOTION_BLUR_SHADER.contains("max_samples"));
    }
}