use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GizmoVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GizmoUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

pub struct GizmoRenderer3D {
    pub render_pipeline: RenderPipeline,
    pub translate_mesh: GizmoMesh,
    pub rotate_mesh: GizmoMesh,
    pub scale_mesh: GizmoMesh,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
}

pub struct GizmoMesh {
    pub vertex_buffer: Buffer,
    pub num_vertices: u32,
}

impl GizmoRenderer3D {
    pub fn new(device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: ShaderSource::Wgsl(GIZMO_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GizmoUniform {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                model: Mat4::IDENTITY.to_cols_array_2d(),
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Gizmo Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Gizmo Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Gizmo Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GizmoVertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let translate_mesh = Self::create_translate_mesh(device);
        let rotate_mesh = Self::create_rotate_mesh(device);
        let scale_mesh = Self::create_scale_mesh(device);

        GizmoRenderer3D {
            render_pipeline,
            translate_mesh,
            rotate_mesh,
            scale_mesh,
            uniform_buffer,
            bind_group,
        }
    }

    fn create_translate_mesh(device: &Device) -> GizmoMesh {
        let mut verts = Vec::new();
        let axis_len = 1.0f32;
        let arrow_size = 0.1f32;

        let axes = [
            ([1.0f32, 0.0, 0.0], [0.8, 0.2, 0.2, 1.0]),
            ([0.0, 1.0, 0.0], [0.2, 0.8, 0.2, 1.0]),
            ([0.0, 0.0, 1.0], [0.2, 0.2, 0.8, 1.0]),
        ];

        for (dir, color) in axes.iter() {
            let end = [dir[0] * axis_len, dir[1] * axis_len, dir[2] * axis_len];
            verts.push(GizmoVertex { position: [0.0, 0.0, 0.0], color: *color });
            verts.push(GizmoVertex { position: end, color: *color });

            let perp1 = if dir[0] != 0.0 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
            let perp2 = [
                dir[1] * perp1[2] - dir[2] * perp1[1],
                dir[2] * perp1[0] - dir[0] * perp1[2],
                dir[0] * perp1[1] - dir[1] * perp1[0],
            ];

            for i in 0..4 {
                let angle = i as f32 * std::f32::consts::PI / 2.0;
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let p = [
                    end[0] + (perp1[0] * cos_a + perp2[0] * sin_a) * arrow_size,
                    end[1] + (perp1[1] * cos_a + perp2[1] * sin_a) * arrow_size,
                    end[2] + (perp1[2] * cos_a + perp2[2] * sin_a) * arrow_size,
                ];
                verts.push(GizmoVertex { position: p, color: *color });
                verts.push(GizmoVertex {
                    position: [
                        end[0] - dir[0] * arrow_size,
                        end[1] - dir[1] * arrow_size,
                        end[2] - dir[2] * arrow_size,
                    ],
                    color: *color,
                });
            }
        }

        let num_vertices = verts.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Translate Vertex Buffer"),
            contents: bytemuck::cast_slice(&verts),
            usage: BufferUsages::VERTEX,
        });
        GizmoMesh { vertex_buffer, num_vertices }
    }

    fn create_rotate_mesh(device: &Device) -> GizmoMesh {
        let mut verts = Vec::new();
        let radius = 1.0f32;
        let segments = 32;

        let axes = [
            ([1.0f32, 0.0, 0.0], [0.8, 0.2, 0.2, 1.0]),
            ([0.0, 1.0, 0.0], [0.2, 0.8, 0.2, 1.0]),
            ([0.0, 0.0, 1.0], [0.2, 0.2, 0.8, 1.0]),
        ];

        for (axis, color) in axes.iter() {
            let perp1 = if axis[0] != 0.0 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
            let perp2 = [
                axis[1] * perp1[2] - axis[2] * perp1[1],
                axis[2] * perp1[0] - axis[0] * perp1[2],
                axis[0] * perp1[1] - axis[1] * perp1[0],
            ];

            for i in 0..segments {
                let a0 = i as f32 * 2.0 * std::f32::consts::PI / segments as f32;
                let a1 = (i + 1) as f32 * 2.0 * std::f32::consts::PI / segments as f32;
                let p0 = [
                    (perp1[0] * a0.cos() + perp2[0] * a0.sin()) * radius,
                    (perp1[1] * a0.cos() + perp2[1] * a0.sin()) * radius,
                    (perp1[2] * a0.cos() + perp2[2] * a0.sin()) * radius,
                ];
                let p1 = [
                    (perp1[0] * a1.cos() + perp2[0] * a1.sin()) * radius,
                    (perp1[1] * a1.cos() + perp2[1] * a1.sin()) * radius,
                    (perp1[2] * a1.cos() + perp2[2] * a1.sin()) * radius,
                ];
                verts.push(GizmoVertex { position: p0, color: *color });
                verts.push(GizmoVertex { position: p1, color: *color });
            }
        }

        let num_vertices = verts.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Rotate Vertex Buffer"),
            contents: bytemuck::cast_slice(&verts),
            usage: BufferUsages::VERTEX,
        });
        GizmoMesh { vertex_buffer, num_vertices }
    }

    fn create_scale_mesh(device: &Device) -> GizmoMesh {
        let mut verts = Vec::new();
        let axis_len = 1.0f32;
        let cube_size = 0.08f32;

        let axes = [
            ([1.0f32, 0.0, 0.0], [0.8, 0.2, 0.2, 1.0]),
            ([0.0, 1.0, 0.0], [0.2, 0.8, 0.2, 1.0]),
            ([0.0, 0.0, 1.0], [0.2, 0.2, 0.8, 1.0]),
        ];

        for (dir, color) in axes.iter() {
            let end = [dir[0] * axis_len, dir[1] * axis_len, dir[2] * axis_len];
            verts.push(GizmoVertex { position: [0.0, 0.0, 0.0], color: *color });
            verts.push(GizmoVertex { position: end, color: *color });

            let perp1 = if dir[0] != 0.0 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
            let perp2 = [
                dir[1] * perp1[2] - dir[2] * perp1[1],
                dir[2] * perp1[0] - dir[0] * perp1[2],
                dir[0] * perp1[1] - dir[1] * perp1[0],
            ];

            for i in 0..4 {
                let angle = i as f32 * std::f32::consts::PI / 2.0;
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let p = [
                    end[0] + (perp1[0] * cos_a + perp2[0] * sin_a) * cube_size,
                    end[1] + (perp1[1] * cos_a + perp2[1] * sin_a) * cube_size,
                    end[2] + (perp1[2] * cos_a + perp2[2] * sin_a) * cube_size,
                ];
                verts.push(GizmoVertex { position: p, color: *color });
                let p2 = [
                    end[0]
                        + (perp1[0] * (angle + std::f32::consts::PI / 2.0).cos()
                            + perp2[0] * (angle + std::f32::consts::PI / 2.0).sin())
                            * cube_size,
                    end[1]
                        + (perp1[1] * (angle + std::f32::consts::PI / 2.0).cos()
                            + perp2[1] * (angle + std::f32::consts::PI / 2.0).sin())
                            * cube_size,
                    end[2]
                        + (perp1[2] * (angle + std::f32::consts::PI / 2.0).cos()
                            + perp2[2] * (angle + std::f32::consts::PI / 2.0).sin())
                            * cube_size,
                ];
                verts.push(GizmoVertex { position: p2, color: *color });
            }
        }

        let num_vertices = verts.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Scale Vertex Buffer"),
            contents: bytemuck::cast_slice(&verts),
            usage: BufferUsages::VERTEX,
        });
        GizmoMesh { vertex_buffer, num_vertices }
    }

    pub fn update(&self, queue: &Queue, view_proj: Mat4, model: Mat4) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[GizmoUniform {
                view_proj: view_proj.to_cols_array_2d(),
                model: model.to_cols_array_2d(),
            }]),
        );
    }

    pub fn render(&self, render_pass: &mut RenderPass, mode: GizmoMode) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        let mesh = match mode {
            GizmoMode::Translate => &self.translate_mesh,
            GizmoMode::Rotate => &self.rotate_mesh,
            GizmoMode::Scale => &self.scale_mesh,
        };
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.draw(0..mesh.num_vertices, 0..1);
    }
}

const GIZMO_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
