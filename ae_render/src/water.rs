//! Water rendering: Gerstner-wave displaced grid + Fresnel reflection + sky color + sun specular.
//!
//! Design goals:
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

use crate::camera::CameraUniform;

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

/// Water renderer: Gerstner-wave displaced grid with Fresnel reflection.
pub struct WaterRenderer {
    pub pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub time_buffer: Buffer,
    pub time_bind_group: BindGroup,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl WaterRenderer {
    /// Create the water renderer with a 128x128 grid covering 400x400 units.
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
                        std::mem::size_of::<CameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water camera buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
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
        }
    }

    /// Upload camera uniform to GPU.
    pub fn update_camera(&self, queue: &Queue, uniform: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Upload time + wave parameters to GPU.
    pub fn update_time(&self, queue: &Queue, time: f32, wave_amp: f32, wind_dir: [f32; 2]) {
        let uniform = WaterUniform { time, wave_amplitude: wave_amp, wind_dir, _pad: 0.0 };
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Draw the water surface in a render pass.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.time_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
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

const WATER_SHADER: &str = r#"
struct CameraUniform {
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
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_dir: vec3<f32>,
};

// Gerstner wave: displaces position and computes analytic normal
// Returns (displacement_xyz, normal)
fn gerstner_wave(
    pos: vec2<f32>,
    dir: vec2<f32>,
    amplitude: f32,
    wavelength: f32,
    speed: f32,
    time: f32,
    steepness: f32,
) -> vec4<f32> {
    let w = 6.28318 / wavelength;
    let phi = speed * w;
    let phase = dot(dir, pos) * w + time * phi;
    let c = cos(phase);
    let s = sin(phase);

    // Displacement: dx, dy, dz packed into xyz
    let disp = vec3<f32>(
        steepness * dir.x * amplitude * c,
        amplitude * s,
        steepness * dir.y * amplitude * c,
    );

    // Normal packed into w component (just the ny for now, full normal computed in caller)
    return vec4<f32>(disp, 1.0 - steepness * w * amplitude * s);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let time = water.time;
    let amp = water.wave_amplitude;
    let wind = normalize(water.wind_dir);

    // 4 stacked Gerstner waves
    let dirs = array<vec2<f32>, 4>(
        wind,
        vec2<f32>(wind.y, -wind.x),
        normalize(wind + vec2<f32>(0.3, 0.2)),
        normalize(wind + vec2<f32>(-0.2, 0.4)),
    );
    let wavelengths = array<f32, 4>(12.0, 6.0, 3.0, 1.5);
    let amplitudes = array<f32, 4>(amp, amp * 0.5, amp * 0.25, amp * 0.125);
    let speeds = array<f32, 4>(1.0, 1.2, 1.5, 2.0);
    let steepness_val = 0.6;

    var displacement = vec3<f32>(0.0, 0.0, 0.0);
    var normal_acc = vec3<f32>(0.0, 1.0, 0.0);

    for (var i = 0; i < 4; i = i + 1) {
        let result = gerstner_wave(
            in.position.xz,
            dirs[i],
            amplitudes[i],
            wavelengths[i],
            speeds[i],
            time,
            steepness_val,
        );
        displacement = displacement + result.xyz;

        let w = 6.28318 / wavelengths[i];
        let dir = dirs[i];
        let phase = dot(dir, in.position.xz) * w + time * speeds[i] * w;
        let c = cos(phase);
        let nx = -dir.x * w * amplitudes[i] * c;
        let nz = -dir.y * w * amplitudes[i] * c;
        let ny = result.w;
        normal_acc = normal_acc + vec3<f32>(nx, ny, nz);
    }

    let normal = normalize(normal_acc);
    let world_pos = in.position + displacement;

    out.world_pos = world_pos;
    out.normal = normal;
    out.view_dir = camera.position.xyz - world_pos;
    out.clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(in.view_dir);
    let normal = normalize(in.normal);

    // Fresnel reflectance (Schlick approximation, F0 = 0.02 for water)
    let fresnel = 0.02 + 0.98 * pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);

    // Sky color approximation based on normal vs up gradient
    let sky_color = mix(
        vec3<f32>(0.4, 0.6, 0.9),
        vec3<f32>(0.7, 0.85, 1.0),
        clamp(normal.y, 0.0, 1.0),
    );

    // Deep/shallow water color mix by view angle
    let water_deep = vec3<f32>(0.02, 0.1, 0.2);
    let water_shallow = vec3<f32>(0.1, 0.3, 0.4);
    let water_color = mix(water_deep, water_shallow, clamp(normal.y, 0.0, 1.0));

    // Blinn-Phong sun specular weighted by Fresnel
    let sun_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let half_dir = normalize(sun_dir + view_dir);
    let spec = pow(max(dot(normal, half_dir), 0.0), 64.0);

    var color = mix(water_color, sky_color, fresnel);
    color = color + vec3<f32>(1.0, 0.95, 0.85) * spec * 0.5;

    return vec4<f32>(color, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_uniform_size() {
        assert_eq!(std::mem::size_of::<WaterUniform>(), 20);
    }

    #[test]
    fn water_vertex_size() {
        assert_eq!(std::mem::size_of::<WaterVertex>(), 12);
    }

    #[test]
    fn water_uniform_layout_matches_wgsl() {
        assert_eq!(std::mem::offset_of!(WaterUniform, time), 0);
        assert_eq!(std::mem::offset_of!(WaterUniform, wave_amplitude), 4);
        assert_eq!(std::mem::offset_of!(WaterUniform, wind_dir), 8);
        assert_eq!(std::mem::offset_of!(WaterUniform, _pad), 16);
    }

    #[test]
    fn mesh_vertex_count() {
        let (verts, _) = generate_water_mesh(128, 400.0);
        assert_eq!(verts.len(), 128 * 128);
    }

    #[test]
    fn mesh_index_count() {
        let (_, indices) = generate_water_mesh(128, 400.0);
        assert_eq!(indices.len(), 127 * 127 * 6);
    }

    #[test]
    fn mesh_index_fits_u16() {
        let (_, indices) = generate_water_mesh(128, 400.0);
        let max_index = indices.iter().copied().max().unwrap();
        assert!(max_index < u16::MAX as u16);
        assert_eq!(max_index, 128 * 128 - 1);
    }

    #[test]
    fn mesh_centered_at_origin() {
        let (verts, _) = generate_water_mesh(4, 100.0);
        let xs: Vec<f32> = verts.iter().map(|v| v.position[0]).collect();
        let zs: Vec<f32> = verts.iter().map(|v| v.position[2]).collect();
        let x_min = xs.iter().cloned().fold(f32::INFINITY, f32::min);
        let x_max = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let z_min = zs.iter().cloned().fold(f32::INFINITY, f32::min);
        let z_max = zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((x_min - (-50.0)).abs() < 1e-3);
        assert!((x_max - 50.0).abs() < 1e-3);
        assert!((z_min - (-50.0)).abs() < 1e-3);
        assert!((z_max - 50.0).abs() < 1e-3);
    }

    #[test]
    fn mesh_y_is_zero() {
        let (verts, _) = generate_water_mesh(16, 100.0);
        for v in &verts {
            assert_eq!(v.position[1], 0.0);
        }
    }

    #[test]
    fn mesh_winding_ccw_from_top() {
        let (_, indices) = generate_water_mesh(4, 100.0);
        let i00 = indices[0] as usize;
        let i01 = indices[1] as usize;
        let i10 = indices[2] as usize;
        let (verts, _) = generate_water_mesh(4, 100.0);
        let p00 = verts[i00].position;
        let p01 = verts[i01].position;
        let p10 = verts[i10].position;
        let e1 = [p01[0] - p00[0], p01[1] - p00[1], p01[2] - p00[2]];
        let e2 = [p10[0] - p00[0], p10[1] - p00[1], p10[2] - p00[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        assert!(n[1] > 0.0, "normal Y should be positive (CCW from +Y)");
    }

    #[test]
    fn shader_has_gerstner_function() {
        assert!(WATER_SHADER.contains("gerstner_wave"));
        assert!(WATER_SHADER.contains("fn gerstner_wave"));
    }

    #[test]
    fn shader_has_fresnel() {
        assert!(WATER_SHADER.contains("fresnel"));
        assert!(WATER_SHADER.contains("0.02"));
    }

    #[test]
    fn shader_has_vertex_and_fragment() {
        assert!(WATER_SHADER.contains("@vertex"));
        assert!(WATER_SHADER.contains("@fragment"));
        assert!(WATER_SHADER.contains("vs_main"));
        assert!(WATER_SHADER.contains("fs_main"));
    }
}
