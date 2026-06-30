//! Water Pass: Gerstner-wave displaced grid + Fresnel reflection（port 自 v1 ae_render）
//!
//! - 4 stacked Gerstner waves with analytic normals (height-field approximation)
//! - Fresnel reflectance (Schlick approximation, F0 = 0.02 for water)
//! - Sky color reflection approximated by normal-vs-up gradient
//! - Blinn-Phong sun specular weighted by Fresnel
//! - Simple refraction approximation (deep/shallow water color mix by view angle)
//!
//! Mesh: 128x128 vertex grid covering 400x400 units on the XZ plane (Y=0).

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, Device, Queue, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// Water Camera Uniform（256 bytes = 4 × mat4，与 v1 CameraUniform 兼容）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct WaterCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

/// Water uniform: time + wave amplitude + wind direction.
///
/// Rust size = 20 bytes; WGSL uniform layout rounds the struct up to 32 bytes
/// (struct alignment 16, trailing padding). The GPU buffer is allocated at 32 bytes
/// and only the first 20 bytes are written each update.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
pub struct WaterUniform {
    pub time: f32,
    pub wave_amplitude: f32,
    pub wind_dir: [f32; 2],
    pub _pad: f32,
}

/// Water vertex: position on the XZ plane (Y=0). Displacement is computed in shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
}

/// WaterPass: Gerstner-wave displaced grid with Fresnel reflection.
pub struct WaterPass {
    pub pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub time_buffer: Buffer,
    pub time_bind_group: BindGroup,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    /// 深度纹理（与 surface 尺寸匹配，独立于 ForwardPass 的 depth）
    pub depth_texture: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
    pub depth_size: (u32, u32),
    pub depth_format: wgpu::TextureFormat,
}

impl WaterPass {
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<WaterCameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water camera buffer"),
            size: std::mem::size_of::<WaterCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let time_buffer_size: u64 = 32;
        let time_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water time layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(time_buffer_size),
                },
                count: None,
            }],
        });

        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water time buffer"),
            size: time_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let time_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water time bind group"),
            layout: &time_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water pipeline layout"),
            bind_group_layouts: &[&camera_layout, &time_layout],
            push_constant_ranges: &[],
        });

        let (vertices, indices) = generate_water_mesh(128, 400.0);
        let index_count = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water shader"),
            source: wgpu::ShaderSource::Wgsl(WATER_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<WaterVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            time_buffer,
            time_bind_group,
            vertex_buffer,
            index_buffer,
            index_count,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            depth_format,
        }
    }

    /// 确保深度纹理与 surface 尺寸匹配（类似 ForwardPass::ensure_depth）
    pub fn ensure_depth(&mut self, device: &Device, width: u32, height: u32) {
        if self.depth_size == (width, height) && self.depth_view.is_some() {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water depth texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
    }

    pub fn update_camera(&self, queue: &Queue, uniform: &WaterCameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn update_time(&self, queue: &Queue, time: f32, wave_amp: f32, wind_dir: [f32; 2]) {
        let uniform = WaterUniform {
            time,
            wave_amplitude: wave_amp,
            wind_dir,
            _pad: 0.0,
        };
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.time_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

impl Default for WaterPass {
    fn default() -> Self {
        unreachable!("WaterPass requires device + format arguments");
    }
}

/// Generate a grid mesh on the XZ plane (Y=0), centered at origin.
fn generate_water_mesh(resolution: u32, size: f32) -> (Vec<WaterVertex>, Vec<u16>) {
    let mut vertices = Vec::with_capacity((resolution * resolution) as usize);
    let half = size * 0.5;
    let step = size / (resolution - 1) as f32;

    for z in 0..resolution {
        for x in 0..resolution {
            let px = -half + x as f32 * step;
            let pz = -half + z as f32 * step;
            vertices.push(WaterVertex { position: [px, 0.0, pz] });
        }
    }

    let mut indices: Vec<u16> =
        Vec::with_capacity(((resolution - 1) * (resolution - 1) * 6) as usize);
    for z in 0..(resolution - 1) {
        for x in 0..(resolution - 1) {
            let i00 = (z * resolution + x) as u16;
            let i10 = (z * resolution + x + 1) as u16;
            let i01 = ((z + 1) * resolution + x) as u16;
            let i11 = ((z + 1) * resolution + x + 1) as u16;
            indices.extend_from_slice(&[i00, i01, i10, i10, i01, i11]);
        }
    }

    (vertices, indices)
}

/// 内嵌 WGSL shader：Gerstner 波 + Fresnel（保留 v1 原样）
const WATER_SHADER: &str = r#"struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

struct WaterUniform {
    time: f32,
    wave_amplitude: f32,
    wind_dir: vec2<f32>,
    _pad: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> water: WaterUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_dir: vec3<f32>,
};

fn gerstner_wave(pos: vec2<f32>, dir: vec2<f32>, wavelength: f32, steepness: f32, time: f32) -> vec3<f32> {
    let k = 6.28318 / wavelength;
    let c = sqrt(9.8 / k);
    let f = k * (dot(dir, pos) - c * time);
    let a = steepness / k;
    return vec3<f32>(
        dir.x * a * cos(f),
        a * sin(f),
        dir.y * a * cos(f),
    );
}

fn gerstner_derivative(pos: vec2<f32>, dir: vec2<f32>, wavelength: f32, steepness: f32, time: f32) -> vec2<f32> {
    let k = 6.28318 / wavelength;
    let c = sqrt(9.8 / k);
    let f = k * (dot(dir, pos) - c * time);
    let a = steepness / k;
    return vec2<f32>(a * k * dir.x * cos(f), a * k * dir.y * cos(f));
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let pos2d = in.position.xz;
    let amp = water.wave_amplitude;
    let wind_len = max(length(water.wind_dir), 0.0001);
    let wind = water.wind_dir / wind_len;
    let t = water.time;

    let dir1 = wind;
    let dir2 = normalize(vec2<f32>(wind.y, -wind.x));
    let dir3 = normalize(vec2<f32>(wind.x + 0.3, wind.y - 0.2));
    let dir4 = normalize(vec2<f32>(-wind.y, wind.x + 0.1));

    var displacement = vec3<f32>(0.0, 0.0, 0.0);
    var dH = vec2<f32>(0.0, 0.0);

    let d1 = gerstner_wave(pos2d, dir1, 60.0, 0.35 * amp, t);
    let n1 = gerstner_derivative(pos2d, dir1, 60.0, 0.35 * amp, t);
    displacement = displacement + d1;
    dH = dH + n1;

    let d2 = gerstner_wave(pos2d, dir2, 31.0, 0.25 * amp, t);
    let n2 = gerstner_derivative(pos2d, dir2, 31.0, 0.25 * amp, t);
    displacement = displacement + d2;
    dH = dH + n2;

    let d3 = gerstner_wave(pos2d, dir3, 18.0, 0.20 * amp, t);
    let n3 = gerstner_derivative(pos2d, dir3, 18.0, 0.20 * amp, t);
    displacement = displacement + d3;
    dH = dH + n3;

    let d4 = gerstner_wave(pos2d, dir4, 11.0, 0.15 * amp, t);
    let n4 = gerstner_derivative(pos2d, dir4, 11.0, 0.15 * amp, t);
    displacement = displacement + d4;
    dH = dH + n4;

    let normal = normalize(vec3<f32>(-dH.x, 1.0, -dH.y));

    let world_pos = in.position + displacement;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_position = world_pos;
    out.normal = normal;
    out.view_dir = normalize(camera.position.xyz - world_pos);

    return out;
}

fn fresnel(cos_theta: f32) -> f32 {
    let f0 = 0.02;
    return f0 + (1.0 - f0) * pow(1.0 - max(cos_theta, 0.0), 5.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let v = normalize(in.view_dir);
    let n_dot_v = max(dot(n, v), 0.0);

    let fres = fresnel(n_dot_v);

    let deep_color = vec3<f32>(0.02, 0.1, 0.2);
    let shallow_color = vec3<f32>(0.1, 0.4, 0.5);
    let water_color = mix(deep_color, shallow_color, n_dot_v);

    let sky_horizon = vec3<f32>(0.7, 0.8, 0.95);
    let sky_zenith = vec3<f32>(0.3, 0.5, 0.9);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let sky_t = max(dot(n, up), 0.0);
    let sky_color = mix(sky_horizon, sky_zenith, sky_t);

    let sun_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let sun_color = vec3<f32>(1.0, 0.95, 0.85) * 3.0;
    let half_dir = normalize(sun_dir + v);
    let n_dot_h = max(dot(n, half_dir), 0.0);
    let specular = pow(n_dot_h, 200.0) * 1.5;

    let refracted = water_color * (1.0 - fres);
    let reflected = sky_color * fres;
    let color = refracted + reflected + specular * sun_color * fres;

    return vec4<f32>(color, 1.0);
}"#;

impl RenderGraphNode for WaterPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "water"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 1. 确保 depth 资源与 surface 尺寸匹配
        let (width, height) = ctx.surface_size;
        self.ensure_depth(ctx.device, width, height);

        // 2. 更新 water uniform（让水面随时间动起来；camera 由外部 update_camera 设置）
        //    使用默认波浪参数：amplitude=1.0, wind_dir=[1.0, 0.0]
        self.update_time(ctx.queue, ctx.time, 1.0, [1.0, 0.0]);

        // 3. 获取 color_view（surface）+ depth_view
        let color_view = ctx.surface_view.ok_or_else(|| {
            anyhow::anyhow!("water: surface_view is None (no swapchain target)")
        })?;
        let depth_view = self.depth_view.clone().ok_or_else(|| {
            anyhow::anyhow!("water: depth_view is None (ensure_depth failed)")
        })?;

        // 4. begin_render_pass
        //    - color: LoadOp::Load（保留 SkyboxPass/ForwardPass 绘制的场景）
        //    - depth: LoadOp::Clear(1.0)（WaterPass 独立 depth buffer，重新清空）
        let mut render_pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("water render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // 5. 录制 draw 命令（复用已有 draw 方法）
        self.draw(&mut render_pass);

        Ok(())
    }
}
