//! Instanced rendering: cube instancing + point cloud billboard
//!
//! Used for voxel grids, meta-entities, near-field particles (cube instancing),
//! and mid/far-field particles (point cloud).
//!
//! Design goals:
//! - GPU pre-allocated instance buffers (no per-frame allocation)
//! - Two pipelines sharing one camera bind group
//! - Compatible with ae_render::CameraUniform (view_proj + view + proj + position)

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline};

use crate::camera::CameraUniform;

/// Per-instance data for cube instancing (voxels / meta-entities / near particles).
/// 32 bytes: position(3) + pad(1) + color(4).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct InstanceData {
    pub position: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 4],
}

impl InstanceData {
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, _pad: 0.0, color }
    }
}

/// Per-instance data for point cloud billboard (mid/far particles).
/// 32 bytes: position(3) + size(1) + color(4).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct PointInstanceData {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

impl PointInstanceData {
    pub fn new(position: [f32; 3], size: f32, color: [f32; 4]) -> Self {
        Self { position, size, color }
    }
}

/// Simple cube vertex: position + normal (24 bytes).
/// Used for instanced cube rendering (each voxel = 1 instance of a unit cube).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CubeVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Generate 36 vertices (6 faces × 2 triangles × 3 verts) for a unit cube.
pub fn cube_vertices() -> Vec<CubeVertex> {
    let s = 0.5_f32;
    let raw: [([f32; 3], [f32; 3]); 36] = [
        // +X
        ([s, -s, -s], [1.0, 0.0, 0.0]),
        ([s, s, -s], [1.0, 0.0, 0.0]),
        ([s, s, s], [1.0, 0.0, 0.0]),
        ([s, -s, -s], [1.0, 0.0, 0.0]),
        ([s, s, s], [1.0, 0.0, 0.0]),
        ([s, -s, s], [1.0, 0.0, 0.0]),
        // -X
        ([-s, -s, s], [-1.0, 0.0, 0.0]),
        ([-s, s, s], [-1.0, 0.0, 0.0]),
        ([-s, s, -s], [-1.0, 0.0, 0.0]),
        ([-s, -s, s], [-1.0, 0.0, 0.0]),
        ([-s, s, -s], [-1.0, 0.0, 0.0]),
        ([-s, -s, -s], [-1.0, 0.0, 0.0]),
        // +Y
        ([-s, s, -s], [0.0, 1.0, 0.0]),
        ([-s, s, s], [0.0, 1.0, 0.0]),
        ([s, s, s], [0.0, 1.0, 0.0]),
        ([-s, s, -s], [0.0, 1.0, 0.0]),
        ([s, s, s], [0.0, 1.0, 0.0]),
        ([s, s, -s], [0.0, 1.0, 0.0]),
        // -Y
        ([-s, -s, s], [0.0, -1.0, 0.0]),
        ([-s, -s, -s], [0.0, -1.0, 0.0]),
        ([s, -s, -s], [0.0, -1.0, 0.0]),
        ([-s, -s, s], [0.0, -1.0, 0.0]),
        ([s, -s, -s], [0.0, -1.0, 0.0]),
        ([s, -s, s], [0.0, -1.0, 0.0]),
        // +Z
        ([-s, -s, s], [0.0, 0.0, 1.0]),
        ([s, -s, s], [0.0, 0.0, 1.0]),
        ([s, s, s], [0.0, 0.0, 1.0]),
        ([-s, -s, s], [0.0, 0.0, 1.0]),
        ([s, s, s], [0.0, 0.0, 1.0]),
        ([-s, s, s], [0.0, 0.0, 1.0]),
        // -Z
        ([s, -s, -s], [0.0, 0.0, -1.0]),
        ([-s, -s, -s], [0.0, 0.0, -1.0]),
        ([-s, s, -s], [0.0, 0.0, -1.0]),
        ([s, -s, -s], [0.0, 0.0, -1.0]),
        ([-s, s, -s], [0.0, 0.0, -1.0]),
        ([s, s, -s], [0.0, 0.0, -1.0]),
    ];
    raw.iter().map(|(p, n)| CubeVertex { position: *p, normal: *n }).collect()
}

// ==================== Shaders ====================

/// Cube instancing shader: renders unit cubes at instance positions with simple lighting.
pub const INSTANCED_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(2) instance_position: vec3<f32>,
    @location(3) instance_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_dir: vec3<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position + instance.instance_position;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = instance.instance_color;
    out.normal = in.normal;
    out.view_dir = normalize(camera.position.xyz - world_pos);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Directional light + ambient
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let light_color = vec3<f32>(1.0, 0.95, 0.9) * 2.5;
    let ambient = vec3<f32>(0.15, 0.18, 0.22);

    let n_dot_l = max(dot(in.normal, light_dir), 0.0);
    let half_dir = normalize(light_dir + in.view_dir);
    let n_dot_h = max(dot(in.normal, half_dir), 0.0);
    let n_dot_v = max(dot(in.normal, in.view_dir), 0.0);

    // Simple specular (Blinn-Phong)
    let spec_power = 32.0;
    let specular = pow(n_dot_h, spec_power) * 0.3;

    let diffuse = n_dot_l * light_color;
    let color = (ambient + diffuse) * in.color.rgb + specular * light_color;
    return vec4<f32>(color, in.color.a);
}
"#;

/// Point cloud billboard shader: renders particles as screen-space circular sprites.
pub const POINT_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct PointInstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) size: f32,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

// Quad vertices: 2 triangles forming a 1x1 quad centered at origin
const QUAD_CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: PointInstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let corner = QUAD_CORNERS[vertex_index];
    let world_pos = instance.position;
    let clip = camera.view_proj * vec4<f32>(world_pos, 1.0);
    let w = clip.w;
    let offset = vec2<f32>(corner.x * instance.size / w, corner.y * instance.size / w);
    out.clip_position = vec4<f32>(clip.x + offset.x * 2.0, clip.y + offset.y * 2.0, clip.z, clip.w);
    out.color = instance.color;
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(in.uv);
    if (d > 1.0) {
        discard;
    }
    let alpha = smoothstep(1.0, 0.8, d);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

// ==================== Instanced Renderer ====================

/// Manages cube-instancing + point-cloud pipelines with pre-allocated GPU buffers.
///
/// Usage:
/// ```ignore
/// let mut ir = InstancedRenderer::new(&device, &config, &camera_layout, 10000, 150000);
/// ir.update_camera(&queue, &camera_uniform);
/// ir.update_instances(&queue, &instances);
/// ir.update_points(&queue, &points);
/// // In render pass:
/// ir.draw_cubes(&mut pass);
/// ir.draw_points(&mut pass);
/// ```
pub struct InstancedRenderer {
    pub cube_pipeline: RenderPipeline,
    pub point_pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub camera_layout: BindGroupLayout,
    pub cube_vertex_buffer: Buffer,
    pub instance_buffer: Buffer,
    pub point_instance_buffer: Buffer,
    pub max_instances: usize,
    pub max_points: usize,
}

impl InstancedRenderer {
    /// Create with pre-allocated instance buffers.
    ///
    /// # Arguments
    /// - `device`: wgpu device
    /// - `color_format`: surface texture format
    /// - `depth_format`: depth buffer format
    /// - `max_instances`: max cube instances (voxels + meta + near particles)
    /// - `max_points`: max point cloud instances (mid/far particles)
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_instances: usize,
        max_points: usize,
    ) -> Self {
        // Camera bind group layout (shared by both pipelines)
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("instanced camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<CameraUniform>() as u64),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instanced camera buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instanced camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("instanced pipeline layout"),
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[],
        });

        // Cube vertex buffer (36 vertices, static)
        let cube_verts = cube_vertices();
        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube vertex buffer"),
            contents: bytemuck::cast_slice(&cube_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Instance buffer (pre-allocated, max_instances)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size: (max_instances * std::mem::size_of::<InstanceData>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Point instance buffer (pre-allocated, max_points)
        let point_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("point instance buffer"),
            size: (max_points * std::mem::size_of::<PointInstanceData>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Cube instancing shader
        let cube_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("instanced cube shader"),
            source: wgpu::ShaderSource::Wgsl(INSTANCED_SHADER.into()),
        });

        // Point cloud shader
        let point_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point cloud shader"),
            source: wgpu::ShaderSource::Wgsl(POINT_SHADER.into()),
        });

        // Cube pipeline (instanced, depth write, back-face cull)
        let cube_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube instanced pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &cube_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<CubeVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 2 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 3 },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &cube_shader,
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

        // Point pipeline (instanced billboard, no depth write, alpha blend, no cull)
        let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("point cloud pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &point_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<PointInstanceData>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 12, shader_location: 1 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &point_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            cube_pipeline,
            point_pipeline,
            camera_buffer,
            camera_bind_group,
            camera_layout,
            cube_vertex_buffer,
            instance_buffer,
            point_instance_buffer,
            max_instances,
            max_points,
        }
    }

    /// Upload camera uniform to GPU.
    pub fn update_camera(&self, queue: &Queue, uniform: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Upload cube instance data (auto-clamped to max_instances).
    pub fn update_instances(&self, queue: &Queue, instances: &[InstanceData]) {
        let count = instances.len().min(self.max_instances);
        if count > 0 {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        }
    }

    /// Upload point cloud instance data (auto-clamped to max_points).
    pub fn update_points(&self, queue: &Queue, points: &[PointInstanceData]) {
        let count = points.len().min(self.max_points);
        if count > 0 {
            queue.write_buffer(&self.point_instance_buffer, 0, bytemuck::cast_slice(&points[..count]));
        }
    }

    /// Draw cube instances in a render pass.
    /// Draws `instance_count` cubes (36 vertices each).
    pub fn draw_cubes(&self, pass: &mut wgpu::RenderPass<'_>, instance_count: u32) {
        if instance_count == 0 {
            return;
        }
        let count = instance_count.min(self.max_instances as u32);
        pass.set_pipeline(&self.cube_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw(0..36, 0..count);
    }

    /// Draw point cloud in a render pass.
    /// Draws `point_count` billboard quads (6 vertices each).
    pub fn draw_points(&self, pass: &mut wgpu::RenderPass<'_>, point_count: u32) {
        if point_count == 0 {
            return;
        }
        let count = point_count.min(self.max_points as u32);
        pass.set_pipeline(&self.point_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.point_instance_buffer.slice(..));
        pass.draw(0..6, 0..count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_data_size() {
        // 3 + 1 + 4 = 8 floats = 32 bytes
        assert_eq!(std::mem::size_of::<InstanceData>(), 32);
    }

    #[test]
    fn point_instance_data_size() {
        // 3 + 1 + 4 = 8 floats = 32 bytes
        assert_eq!(std::mem::size_of::<PointInstanceData>(), 32);
    }

    #[test]
    fn cube_vertex_size() {
        // 3 + 3 = 6 floats = 24 bytes
        assert_eq!(std::mem::size_of::<CubeVertex>(), 24);
    }

    #[test]
    fn cube_vertices_count() {
        let verts = cube_vertices();
        assert_eq!(verts.len(), 36); // 6 faces × 2 triangles × 3 verts
    }

    #[test]
    fn shaders_nonempty() {
        assert!(!INSTANCED_SHADER.is_empty());
        assert!(!POINT_SHADER.is_empty());
    }

    #[test]
    fn point_shader_has_builtin_vertex_index() {
        // Must have @builtin(vertex_index) to pass wgpu 24 validation
        assert!(POINT_SHADER.contains("@builtin(vertex_index)"));
    }

    #[test]
    fn instanced_shader_has_vs_and_fs() {
        assert!(INSTANCED_SHADER.contains("@vertex"));
        assert!(INSTANCED_SHADER.contains("@fragment"));
        assert!(INSTANCED_SHADER.contains("vs_main"));
        assert!(INSTANCED_SHADER.contains("fs_main"));
    }

    #[test]
    fn instance_data_new() {
        let d = InstanceData::new([1.0, 2.0, 3.0], [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(d.position, [1.0, 2.0, 3.0]);
        assert_eq!(d.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn point_instance_data_new() {
        let d = PointInstanceData::new([1.0, 2.0, 3.0], 0.5, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(d.position, [1.0, 2.0, 3.0]);
        assert_eq!(d.size, 0.5);
    }
}
