//! Shadow Map Renderer: 方向光阴影系统
//!
//! 实现 PCF 软阴影：
//! - 从光源视角渲染深度（depth-only pass）
//! - Shadow camera：正交投影，跟随光源方向，覆盖场景范围
//! - 比较采样器（SamplerBindingType::Comparison）+ Linear filter 实现 PCF
//!
//! 设计：
//! - 独立的 shadow pipeline（depth-only shader，无 fragment 输出）
//! - 共享 MeshRenderer 的 vertex buffer（@location(0) position）
//!   和 instance buffer（@location(5-8) model matrix）
//! - shadow_bind_group 提供给主渲染 pass 采样阴影：
//!   binding 0: ShadowUniform（light_view_proj + shadow_map_size）
//!   binding 1: shadow_texture_view（Depth32Float）
//!   binding 2: shadow_sampler（Comparison, Linear）

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue, RenderPipeline};

use crate::camera::CameraUniform;
use crate::mesh::Vertex;
use crate::mesh_renderer::{MeshInstanceData, MeshRenderer};

/// 阴影 Uniform：光源 view-projection + shadow map 尺寸
/// 64 bytes (mat4) + 16 bytes (vec4) = 80 bytes，符合 WGSL 16-byte 对齐
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ShadowUniform {
    /// 光源视角 view-projection 矩阵（列主序）
    pub light_view_proj: [[f32; 4]; 4],
    /// xy: shadow map 分辨率，zw: unused
    pub shadow_map_size: [f32; 4],
}

/// ShadowMapRenderer: 管理方向光阴影渲染
pub struct ShadowMapRenderer {
    pub shadow_pipeline: RenderPipeline,
    pub shadow_texture: wgpu::Texture,
    pub shadow_view: wgpu::TextureView,
    pub shadow_sampler: wgpu::Sampler,
    pub shadow_uniform_buffer: Buffer,
    pub shadow_bind_group: BindGroup,
    pub shadow_layout: BindGroupLayout,
    pub camera_layout: BindGroupLayout,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub shadow_size: u32,
    pub shadow_uniform: ShadowUniform,
}

/// 内嵌 WGSL shader：从光源视角渲染深度
const SHADOW_SHADER: &str = r#"
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
};

struct InstanceInput {
    @location(5) model_col0: vec4<f32>,
    @location(6) model_col1: vec4<f32>,
    @location(7) model_col2: vec4<f32>,
    @location(8) model_col3: vec4<f32>,
};

@vertex
fn vs_shadow(
    in: VertexInput,
    instance: InstanceInput,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(
        instance.model_col0,
        instance.model_col1,
        instance.model_col2,
        instance.model_col3,
    );
    let world_pos = model * vec4<f32>(in.position, 1.0);
    return camera.view_proj * world_pos;
}
"#;

impl ShadowMapRenderer {
    /// 创建阴影渲染器
    /// `shadow_size`: shadow map 分辨率（如 2048）
    pub fn new(device: &Device, shadow_size: u32) -> Self {
        let safe_size = shadow_size.max(1);

        // ---- Shadow texture (Depth32Float, RENDER_ATTACHMENT + TEXTURE_BINDING) ----
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map texture"),
            size: wgpu::Extent3d {
                width: safe_size,
                height: safe_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---- PCF comparison sampler (Linear filter) ----
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow comparison sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            ..Default::default()
        });

        // ---- Camera bind group layout（与 MeshRenderer 兼容）----
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
            label: Some("shadow camera buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // ---- Shadow uniform bind group layout（供主渲染 pass 采样）----
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<ShadowUniform>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let shadow_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow uniform buffer"),
            size: std::mem::size_of::<ShadowUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow bind group"),
            layout: &shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        // ---- Shadow pipeline (depth-only, no fragment) ----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });

        // Vertex buffer layout:
        // - slot 0: MeshRenderer 的 Vertex（64 bytes），只用 position @location(0)
        // - slot 1: MeshRenderer 的 MeshInstanceData（80 bytes），只用 model @location(5-8)
        //   不绑定 tint @location(9)，但 stride 必须匹配 MeshInstanceData
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MeshInstanceData>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 5,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 6,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                                shader_location: 7,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 48,
                                shader_location: 8,
                            },
                        ],
                    },
                ],
            },
            fragment: None,
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
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            shadow_pipeline,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            shadow_uniform_buffer,
            shadow_bind_group,
            shadow_layout,
            camera_layout,
            camera_buffer,
            camera_bind_group,
            shadow_size: safe_size,
            shadow_uniform: ShadowUniform::default(),
        }
    }

    /// 更新光源 view-projection 矩阵
    /// 同时更新 shadow_uniform_buffer（供主 pass 采样）和 camera_buffer（供 shadow pass 渲染）
    pub fn update_light_matrix(
        &self,
        queue: &Queue,
        light_dir: [f32; 3],
        camera_pos: [f32; 3],
        scene_radius: f32,
    ) {
        let light_view_proj = compute_light_view_proj(light_dir, camera_pos, scene_radius);

        // 1. 更新 ShadowUniform（供主渲染 pass 采样阴影）
        let shadow_uniform = ShadowUniform {
            light_view_proj,
            shadow_map_size: [self.shadow_size as f32, self.shadow_size as f32, 0.0, 0.0],
        };
        queue.write_buffer(
            &self.shadow_uniform_buffer,
            0,
            bytemuck::cast_slice(&[shadow_uniform]),
        );

        // 2. 更新 camera_buffer（shadow pass 的 shader 通过 camera.view_proj 渲染深度）
        let camera_uniform = CameraUniform {
            view_proj: light_view_proj,
            view: light_view_proj,
            proj: light_view_proj,
            position: [camera_pos[0], camera_pos[1], camera_pos[2], 1.0],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// 更新 camera（用于实例渲染）
    pub fn update_camera(&self, queue: &Queue, uniform: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 渲染深度到 shadow map（在单独的 render pass 中调用）
    /// 复用 MeshRenderer 的 vertex/index/instance buffer，但使用 shadow pipeline
    pub fn draw_shadow_pass(
        &self,
        encoder: &mut CommandEncoder,
        mesh_renderer: &MeshRenderer,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow render pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if mesh_renderer.meshes.is_empty() {
            return;
        }

        pass.set_pipeline(&self.shadow_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        // slot 1 = instance buffer（与 MeshRenderer 共享）
        pass.set_vertex_buffer(1, mesh_renderer.instance_buffer.slice(..));

        for mesh in &mesh_renderer.meshes {
            if mesh.instance_count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(
                mesh.mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(
                0..mesh.mesh.num_indices,
                0,
                mesh.instance_offset as u32..(mesh.instance_offset + mesh.instance_count) as u32,
            );
        }
    }

    /// 获取 shadow bind group（供主渲染 pass 使用）
    pub fn shadow_bind_group(&self) -> &BindGroup {
        &self.shadow_bind_group
    }

    /// 获取 shadow layout（供主渲染 pipeline 使用）
    pub fn shadow_layout(&self) -> &BindGroupLayout {
        &self.shadow_layout
    }
}

/// 计算光源 view-projection 矩阵
///
/// 算法：
/// 1. 光源方向归一化
/// 2. 光源位置 = camera_pos - dir * scene_radius（从相机后方沿光源方向）
/// 3. View matrix: look_at_rh(light_pos, camera_pos, up)
/// 4. 正交投影：覆盖 scene_radius 范围
/// 5. view_proj = proj * view
fn compute_light_view_proj(
    light_dir: [f32; 3],
    camera_pos: [f32; 3],
    scene_radius: f32,
) -> [[f32; 4]; 4] {
    let dir = Vec3::from(light_dir);
    let len = dir.length();
    let dir = if len > 1e-6 {
        dir / len
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };

    let cam = Vec3::from(camera_pos);
    let light_pos = cam - dir * scene_radius;

    let up = if dir.y.abs() > 0.99 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::Y
    };
    let view = Mat4::look_at_rh(light_pos, cam, up);

    let r = scene_radius.max(1.0);
    let proj = Mat4::orthographic_rh_gl(-r, r, -r, r, 0.1, r * 4.0);

    (proj * view).to_cols_array_2d()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_uniform_size_aligned() {
        // mat4 (64) + vec4 (16) = 80 bytes
        assert_eq!(std::mem::size_of::<ShadowUniform>(), 80);
    }

    #[test]
    fn compute_light_view_proj_basic() {
        // 光源从正上方照下
        let m = compute_light_view_proj([0.0, -1.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        // 矩阵不应全为零
        let has_nonzero = m.iter().flat_map(|r| r.iter()).any(|&v| v.abs() > 1e-6);
        assert!(has_nonzero, "matrix should have non-zero elements");
    }

    #[test]
    fn compute_light_view_proj_normalizes_direction() {
        // 未归一化的方向应与归一化版本产生相同结果
        let m1 = compute_light_view_proj([0.0, -1.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        let m2 = compute_light_view_proj([0.0, -10.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (m1[i][j] - m2[i][j]).abs() < 1e-4,
                    "matrices should match after normalization"
                );
            }
        }
    }

    #[test]
    fn compute_light_view_proj_zero_direction_uses_default() {
        // 零向量方向应回退到默认 (0,1,0)，不 panic
        let m = compute_light_view_proj([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        let has_nonzero = m.iter().flat_map(|r| r.iter()).any(|&v| v.abs() > 1e-6);
        assert!(has_nonzero);
    }
}
