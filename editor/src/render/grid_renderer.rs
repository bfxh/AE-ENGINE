use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct GridRenderer {
    pub render_pipeline: RenderPipeline,
    pub line_buffer: Buffer,
    pub num_vertices: u32,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
}

impl GridRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: ShaderSource::Wgsl(GRID_SHADER.into()),
        });

        let mut vertices = Vec::new();
        let half = 10.0f32;
        let step = 1.0f32;
        let major_color: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
        let minor_color: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

        let mut i = -half;
        while i <= half {
            let c = if (i as i32) % 5 == 0 { major_color } else { minor_color };
            vertices.push(GridVertex { position: [i, 0.0, -half], color: c });
            vertices.push(GridVertex { position: [i, 0.0, half], color: c });
            vertices.push(GridVertex { position: [-half, 0.0, i], color: c });
            vertices.push(GridVertex { position: [half, 0.0, i], color: c });
            i += step;
        }

        vertices.push(GridVertex { position: [0.0, 0.0, -half], color: [0.8, 0.2, 0.2, 1.0] });
        vertices.push(GridVertex { position: [0.0, 0.0, half], color: [0.8, 0.2, 0.2, 1.0] });
        vertices.push(GridVertex { position: [-half, 0.0, 0.0], color: [0.2, 0.8, 0.2, 1.0] });
        vertices.push(GridVertex { position: [half, 0.0, 0.0], color: [0.2, 0.8, 0.2, 1.0] });

        let num_vertices = vertices.len() as u32;
        let line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GridUniform {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
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
            label: Some("Grid Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Grid Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GridVertex>() as BufferAddress,
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

        GridRenderer { render_pipeline, line_buffer, num_vertices, uniform_buffer, bind_group }
    }

    pub fn update_camera(&self, queue: &Queue, view_proj: Mat4) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[GridUniform { view_proj: view_proj.to_cols_array_2d() }]),
        );
    }

    /// Rebuild the grid geometry with a new half-size and step.
    pub fn rebuild(&mut self, device: &Device, half_size: f32, step: f32) {
        let step = step.max(0.01);
        let mut vertices = Vec::new();
        let major_color: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
        let minor_color: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

        let mut i = -half_size;
        while i <= half_size {
            let c = if (i as i32) % 5 == 0 { major_color } else { minor_color };
            vertices.push(GridVertex { position: [i, 0.0, -half_size], color: c });
            vertices.push(GridVertex { position: [i, 0.0, half_size], color: c });
            vertices.push(GridVertex { position: [-half_size, 0.0, i], color: c });
            vertices.push(GridVertex { position: [half_size, 0.0, i], color: c });
            i += step;
        }

        vertices.push(GridVertex { position: [0.0, 0.0, -half_size], color: [0.8, 0.2, 0.2, 1.0] });
        vertices.push(GridVertex { position: [0.0, 0.0, half_size], color: [0.8, 0.2, 0.2, 1.0] });
        vertices.push(GridVertex { position: [-half_size, 0.0, 0.0], color: [0.2, 0.8, 0.2, 1.0] });
        vertices.push(GridVertex { position: [half_size, 0.0, 0.0], color: [0.2, 0.8, 0.2, 1.0] });

        self.num_vertices = vertices.len() as u32;
        self.line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });
    }

    pub fn render(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.line_buffer.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);
    }
}

const GRID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
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
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
