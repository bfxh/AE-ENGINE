use super::mesh_data::{MeshBuffer, Vertex};
use glam::{Mat4, Vec3};
use ae_bvh::{frustum_from_view_proj, Aabb, Bvh};
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub selected: u32,
    pub _pad: [u32; 3],
}

pub struct SceneRenderer {
    pub render_pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub model_buffer: Buffer,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub depth_texture: Option<Texture>,
    pub cube_mesh: MeshBuffer,
    pub sphere_mesh: MeshBuffer,
    pub sample_count: u32,
}

impl SceneRenderer {
    pub fn new(device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Scene Shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/scene.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                camera_pos: [0.0, 0.0, 5.0],
                _pad: 0,
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Uniform Buffer"),
            contents: bytemuck::cast_slice(&[ModelUniform {
                model: Mat4::IDENTITY.to_cols_array_2d(),
                color: [1.0, 1.0, 1.0, 1.0],
                selected: 0,
                _pad: [0, 0, 0],
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Scene Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: model_buffer.as_entire_binding() },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Scene Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
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

        let cube_mesh = MeshBuffer::cube(device);
        let sphere_mesh = MeshBuffer::sphere(device, 16, 16);

        SceneRenderer {
            render_pipeline,
            uniform_buffer,
            model_buffer,
            bind_group_layout,
            bind_group,
            depth_texture: None,
            cube_mesh,
            sphere_mesh,
            sample_count,
        }
    }

    pub fn update_camera(&self, queue: &Queue, view_proj: Mat4, camera_pos: Vec3) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform {
                view_proj: view_proj.to_cols_array_2d(),
                camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z],
                _pad: 0,
            }]),
        );
    }

    pub fn render_scene(
        &self,
        render_pass: &mut RenderPass,
        scene: &crate::scene::Scene,
        selection: &crate::selection::Selection,
        queue: &Queue,
        view_proj: Mat4,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Build BVH from scene nodes and cull via frustum query.
        let bvh = build_scene_bvh(scene);
        let planes = frustum_from_view_proj(view_proj);
        let visible: std::collections::HashSet<u32> =
            bvh.frustum_query(&planes).into_iter().collect();

        for node in &scene.nodes {
            if node.id == 0 {
                continue;
            }
            if !visible.contains(&(node.id as u32)) {
                continue;
            }

            let model = Mat4::from_translation(node.transform.translation)
                * Mat4::from_quat(node.transform.rotation)
                * Mat4::from_scale(node.transform.scale);

            let color = match &node.node_type {
                crate::scene::NodeType::Light { color, .. } => [color.x, color.y, color.z, 1.0],
                _ => [0.7, 0.7, 0.7, 1.0],
            };

            let selected = if selection.selected_id == Some(node.id) { 1 } else { 0 };

            queue.write_buffer(
                &self.model_buffer,
                0,
                bytemuck::cast_slice(&[ModelUniform {
                    model: model.to_cols_array_2d(),
                    color,
                    selected,
                    _pad: [0, 0, 0],
                }]),
            );

            match &node.node_type {
                crate::scene::NodeType::Mesh { .. } | crate::scene::NodeType::Empty => {
                    render_pass.set_vertex_buffer(0, self.cube_mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        self.cube_mesh.index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..self.cube_mesh.num_indices, 0, 0..1);
                },
                crate::scene::NodeType::Light { .. } => {
                    render_pass.set_vertex_buffer(0, self.sphere_mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        self.sphere_mesh.index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..self.sphere_mesh.num_indices, 0, 0..1);
                },
                crate::scene::NodeType::Camera { .. } => {
                    render_pass.set_vertex_buffer(0, self.cube_mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        self.cube_mesh.index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..self.cube_mesh.num_indices, 0, 0..1);
                },
            }
        }
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.depth_texture = Some(device.create_texture(&TextureDescriptor {
            label: Some("Depth Texture"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }));
    }
}

/// Build a BVH over all non-root scene nodes for ray/frustum queries.
///
/// Each node's AABB is the unit cube [-0.5, 0.5]^3 transformed by its model matrix.
/// This is a conservative bound for the placeholder cube/sphere gizmos the editor draws;
/// replace with real mesh AABBs when asset pipelines land.
///
/// Shared between `SceneRenderer::render_scene` (frustum culling) and
/// `ViewportPanel::handle_picking` (ray picking) for API consistency.
pub fn build_scene_bvh(scene: &crate::scene::Scene) -> Bvh {
    let items: Vec<(u32, Aabb)> = scene
        .nodes
        .iter()
        .filter(|n| n.id != 0)
        .map(|n| {
            let model = Mat4::from_translation(n.transform.translation)
                * Mat4::from_quat(n.transform.rotation)
                * Mat4::from_scale(n.transform.scale);
            (n.id as u32, aabb_from_unit_cube(&model))
        })
        .collect();
    Bvh::build(&items)
}

/// Compute the world-space AABB of a unit cube [-0.5, 0.5]^3 transformed by `model`.
/// Used as a conservative bounding volume for scene nodes that have no explicit mesh AABB.
fn aabb_from_unit_cube(model: &Mat4) -> Aabb {
    const CORNERS: [Vec3; 8] = [
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
    ];
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for c in &CORNERS {
        let p = model.transform_point3(*c);
        min = min.min(p);
        max = max.max(p);
    }
    Aabb::new(min, max)
}
