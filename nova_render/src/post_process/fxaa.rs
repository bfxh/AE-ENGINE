//! FXAA 效果（NVIDIA FXAA 3.11 quality）
//!
//! 完整的 wgpu 24 后处理 pipeline：
//! - 全屏三角形 vertex shader
//! - luma 计算（RGB -> luma）
//! - 边缘检测（水平/垂直）
//! - 双线性混合抗锯齿

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayout, Buffer, RenderPipeline, Sampler};

pub struct FxaaEffect {
    pub subpixel: f32,
    pub edge_threshold: f32,
    pub edge_threshold_min: f32,
    pipeline: Option<RenderPipeline>,
    bind_layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    uniform_buffer: Option<Buffer>,
}

impl Default for FxaaEffect {
    fn default() -> Self {
        Self {
            subpixel: 0.75,
            edge_threshold: 0.166,
            edge_threshold_min: 0.0833,
            pipeline: None,
            bind_layout: None,
            sampler: None,
            uniform_buffer: None,
        }
    }
}

/// FXAA Uniform（32 bytes，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct FxaaUniform {
    pub subpixel: f32,
    pub edge_threshold: f32,
    pub edge_threshold_min: f32,
    pub _pad0: f32,
    pub width: f32,
    pub height: f32,
    pub _pad1: [f32; 2],
}

const FXAA_SHADER: &str = r#"
struct FxaaUniform {
    subpixel: f32,
    edge_threshold: f32,
    edge_threshold_min: f32,
    _pad0: f32,
    width: f32,
    height: f32,
    _pad1: vec2<f32>,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: FxaaUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn rgb_to_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_fxaa(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.width, u.height);
    let texel = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let uv = pos.xy / dims;

    let lumaM = rgb_to_luma(textureSample(t_src, s_src, uv).rgb);
    let lumaN = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>(0.0, -texel.y)).rgb);
    let lumaS = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>(0.0,  texel.y)).rgb);
    let lumaW = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>(-texel.x, 0.0)).rgb);
    let lumaE = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>( texel.x, 0.0)).rgb);

    let lumaMin = min(lumaM, min(min(lumaN, lumaS), min(lumaW, lumaE)));
    let lumaMax = max(lumaM, max(max(lumaN, lumaS), max(lumaW, lumaE)));
    let lumaRange = lumaMax - lumaMin;

    if (lumaRange < max(u.edge_threshold_min, lumaMax * u.edge_threshold)) {
        return textureSample(t_src, s_src, uv);
    }

    let lumaNW = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>(-texel.x, -texel.y)).rgb);
    let lumaNE = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>( texel.x, -texel.y)).rgb);
    let lumaSW = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>(-texel.x,  texel.y)).rgb);
    let lumaSE = rgb_to_luma(textureSample(t_src, s_src, uv + vec2<f32>( texel.x,  texel.y)).rgb);

    let lumaNS = lumaN + lumaS;
    let lumaWE = lumaW + lumaE;
    let lumaNWNE = lumaNW + lumaNE;
    let lumaSWSE = lumaSW + lumaSE;

    let edgeHorizontal = abs(-2.0 * lumaW + lumaWE) + abs(-2.0 * lumaN + lumaNS) * 2.0 + abs(-2.0 * lumaE + lumaWE);
    let edgeVertical = abs(-2.0 * lumaNW + lumaNWNE) + abs(-2.0 * lumaW + lumaWE) * 2.0 + abs(-2.0 * lumaSW + lumaSWSE);

    let horzSpan = edgeHorizontal >= edgeVertical;

    var lengthSign: vec2<f32>;
    var oppositeLuma: f32;
    var gradient: f32;

    if (horzSpan) {
        lengthSign = vec2<f32>(texel.x, 0.0);
        oppositeLuma = lumaN - lumaM;
        gradient = abs(lumaN - lumaS) * 2.0;
    } else {
        lengthSign = vec2<f32>(0.0, texel.y);
        oppositeLuma = lumaE - lumaM;
        gradient = abs(lumaE - lumaW) * 2.0;
    }

    if (oppositeLuma < 0.0) {
        lengthSign = -lengthSign;
    }

    let lumaLocalAverage = 0.5 * (lumaM + lumaNS + lumaWE) / 3.0;

    let posA = uv + lengthSign * 0.5;
    let posB = uv - lengthSign * 0.5;

    let lumaEndA = rgb_to_luma(textureSample(t_src, s_src, posA).rgb);
    let lumaEndB = rgb_to_luma(textureSample(t_src, s_src, posB).rgb);
    let lumaEndA_avg = lumaEndA - lumaLocalAverage;
    let lumaEndB_avg = lumaEndB - lumaLocalAverage;

    var reachedEndA = abs(lumaEndA_avg) >= gradient;
    var reachedEndB = abs(lumaEndB_avg) >= gradient;

    if (!reachedEndA) {
        let posA2 = posA + lengthSign;
        let lumaEndA2 = rgb_to_luma(textureSample(t_src, s_src, posA2).rgb);
        let lumaEndA2_avg = lumaEndA2 - lumaLocalAverage;
        reachedEndA = abs(lumaEndA2_avg) >= gradient;
    }
    if (!reachedEndB) {
        let posB2 = posB - lengthSign;
        let lumaEndB2 = rgb_to_luma(textureSample(t_src, s_src, posB2).rgb);
        let lumaEndB2_avg = lumaEndB2 - lumaLocalAverage;
        reachedEndB = abs(lumaEndB2_avg) >= gradient;
    }

    let distanceA = select(distance(uv, posA), distance(uv, posA + lengthSign), vec2<f32>(0.0, 0.0)).x;
    let distanceB = distance(uv, posB);

    var pixelOffset = 0.0;
    let isCloserToA = distanceA < distanceB;
    let distanceEnd = select(distanceA, distanceB, vec2<f32>(isCloserToA ? 0.0 : 1.0, 0.0)).x;

    if (isCloserToA == reachedEndA) {
        pixelOffset = -0.5 + 0.5 * distanceEnd;
    } else {
        pixelOffset = 0.5 - 0.5 * distanceEnd;
    }

    let lumaAverage = (lumaM + lumaNS + lumaWE) / 6.0;
    let subpixelOffset = clamp(abs(lumaAverage - lumaM) / lumaRange, 0.0, 1.0);
    let subpixelOffsetFinal = mix(0.0, subpixelOffset, u.subpixel);

    let finalOffset = pixelOffset + subpixelOffsetFinal * lengthSign.x + subpixelOffsetFinal * lengthSign.y * 0.0;

    var offsetCoord: vec2<f32>;
    if (horzSpan) {
        offsetCoord = uv + vec2<f32>(finalOffset, 0.0);
    } else {
        offsetCoord = uv + vec2<f32>(0.0, finalOffset);
    }

    let result = textureSample(t_src, s_src, offsetCoord).rgb;
    return vec4<f32>(result, 1.0);
}
"#;

impl FxaaEffect {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fxaa bind group layout"),
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
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<FxaaUniform>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fxaa uniform buffer"),
            size: std::mem::size_of::<FxaaUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fxaa sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fxaa shader"),
            source: wgpu::ShaderSource::Wgsl(FXAA_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fxaa pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fxaa pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_fxaa"),
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
            subpixel: 0.75,
            edge_threshold: 0.166,
            edge_threshold_min: 0.0833,
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
        let pipeline = self.pipeline.as_ref().expect("FxaaEffect::new() must be called before render()");
        let bind_layout = self.bind_layout.as_ref().expect("FxaaEffect::new() must be called before render()");
        let sampler = self.sampler.as_ref().expect("FxaaEffect::new() must be called before render()");
        let uniform_buffer = self.uniform_buffer.as_ref().expect("FxaaEffect::new() must be called before render()");

        let safe_w = width.max(1) as f32;
        let safe_h = height.max(1) as f32;
        let uniform = FxaaUniform {
            subpixel: self.subpixel,
            edge_threshold: self.edge_threshold,
            edge_threshold_min: self.edge_threshold_min,
            _pad0: 0.0,
            width: safe_w,
            height: safe_h,
            _pad1: [0.0, 0.0],
        };
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa bind group"),
            layout: bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("fxaa encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fxaa render pass"),
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
        assert_eq!(std::mem::size_of::<FxaaUniform>(), 32);
    }

    #[test]
    fn default_values() {
        let e = FxaaEffect::default();
        assert!((e.subpixel - 0.75).abs() < 1e-6);
        assert!((e.edge_threshold - 0.166).abs() < 1e-6);
        assert!((e.edge_threshold_min - 0.0833).abs() < 1e-6);
        assert!(e.pipeline.is_none());
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(FXAA_SHADER.contains("vs_fullscreen"));
        assert!(FXAA_SHADER.contains("fs_fxaa"));
        assert!(FXAA_SHADER.contains("rgb_to_luma"));
        assert!(FXAA_SHADER.contains("edgeHorizontal"));
        assert!(FXAA_SHADER.contains("edgeVertical"));
    }
}