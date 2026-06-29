//! ShadowMap Pass（port 自 v1 wasteland_render::shadow_map）
//!
//! 方向光阴影系统，PCF 软阴影：
//! - 从光源视角渲染深度（depth-only pass）
//! - Shadow camera：正交投影，跟随光源方向，覆盖场景范围
//! - 比较采样器（SamplerBindingType::Comparison）+ Linear filter 实现 PCF
//!
//! 注意：v1 版本依赖 MeshRenderer 的 Vertex/MeshInstanceData。nova 版本保留 WGSL
//! shader 和 pipeline 创建，vertex buffer 布局与 v1 兼容（@location(0) position,
//! @location(5-8) model matrix），具体 mesh draw 由 RenderGraph 调度。

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// 阴影相机 Uniform（与 v1 CameraUniform 兼容，64+16+16+16=112 bytes）
///
/// shadow shader 只读 view_proj 字段，其余字段保留以保持内存布局兼容。
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ShadowCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

/// 阴影 Uniform：光源 view-projection + shadow map 尺寸
/// 64 bytes (mat4) + 16 bytes (vec4) = 80 bytes
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ShadowUniform {
    pub light_view_proj: [[f32; 4]; 4],
    /// xy: shadow map 分辨率，zw: unused
    pub shadow_map_size: [f32; 4],
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

/// 顶点位置（仅 position，对应 @location(0)）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShadowVertex {
    pub position: [f32; 3],
}

/// 实例模型矩阵（4 × vec4，对应 @location(5-8)）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ShadowInstance {
    pub model_col0: [f32; 4],
    pub model_col1: [f32; 4],
    pub model_col2: [f32; 4],
    pub model_col3: [f32; 4],
}

/// 已注册的 mesh：拥有独立 vertex/index buffer（用于阴影 depth-only 渲染）
pub struct RegisteredShadowMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub index_format: wgpu::IndexFormat,
}

/// ShadowMap Pass（port 自 v1 wasteland_render::ShadowMapRenderer）
pub struct ShadowMapPass {
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
    /// 已注册的 mesh 列表（每个 mesh 有独立 vertex/index buffer）
    pub meshes: Vec<RegisteredShadowMesh>,
    /// 实例 buffer（存储 model matrix，对应 @location(5-8)）
    pub instance_buffer: Buffer,
    pub max_instances: usize,
    pub instance_count: u32,
}

impl ShadowMapPass {
    /// 创建阴影 Pass
    /// `shadow_size`: shadow map 分辨率（如 2048）
    pub fn new(device: &wgpu::Device, shadow_size: u32) -> Self {
        let safe_size = shadow_size.max(1);

        // ---- Shadow texture (Depth32Float) ----
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map texture"),
            size: wgpu::Extent3d { width: safe_size, height: safe_size, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---- PCF comparison sampler ----
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow comparison sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0, lod_max_clamp: 0.0,
            ..Default::default()
        });

        // ---- Camera bind group layout ----
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<ShadowCameraUniform>() as u64) },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow camera buffer"),
            size: std::mem::size_of::<ShadowCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow camera bind group"), layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
        });

        // ---- Shadow uniform bind group layout（供主渲染 pass 采样）----
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<ShadowUniform>() as u64) }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), count: None },
            ],
        });

        let shadow_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow uniform buffer"),
            size: std::mem::size_of::<ShadowUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow bind group"), layout: &shadow_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: shadow_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
            ],
        });

        // ---- Shadow pipeline (depth-only, no fragment) ----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"), bind_group_layouts: &[&camera_layout], push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("shadow shader"), source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()) });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_shadow"), compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<ShadowVertex>() as wgpu::BufferAddress, step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }] },
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<ShadowInstance>() as wgpu::BufferAddress, step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 5 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 6 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 7 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 8 },
                        ] },
                ] },
            fragment: None,
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, strip_index_format: None, front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), unclipped_depth: false, polygon_mode: wgpu::PolygonMode::Fill, conservative: false },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });

        let safe_instances = 64usize;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow instance buffer"),
            size: (safe_instances * std::mem::size_of::<ShadowInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            shadow_pipeline, shadow_texture, shadow_view, shadow_sampler,
            shadow_uniform_buffer, shadow_bind_group, shadow_layout,
            camera_layout, camera_buffer, camera_bind_group,
            shadow_size: safe_size, shadow_uniform: ShadowUniform::default(),
            meshes: Vec::new(),
            instance_buffer,
            max_instances: safe_instances,
            instance_count: 0,
        }
    }

    /// 更新光源 view-projection 矩阵
    pub fn update_light_matrix(&mut self, queue: &wgpu::Queue, light_dir: [f32; 3], camera_pos: [f32; 3], scene_radius: f32) {
        let light_view_proj = compute_light_view_proj(light_dir, camera_pos, scene_radius);

        let shadow_uniform = ShadowUniform {
            light_view_proj,
            shadow_map_size: [self.shadow_size as f32, self.shadow_size as f32, 0.0, 0.0],
        };
        self.shadow_uniform = shadow_uniform;
        queue.write_buffer(&self.shadow_uniform_buffer, 0, bytemuck::cast_slice(&[shadow_uniform]));

        let camera_uniform = ShadowCameraUniform {
            view_proj: light_view_proj, view: light_view_proj, proj: light_view_proj,
            position: [camera_pos[0], camera_pos[1], camera_pos[2], 1.0],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// 获取 shadow bind group（供主渲染 pass 使用）
    pub fn shadow_bind_group(&self) -> &BindGroup { &self.shadow_bind_group }
    /// 获取 shadow layout（供主渲染 pipeline 使用）
    pub fn shadow_layout(&self) -> &BindGroupLayout { &self.shadow_layout }

    /// 注册 mesh（vertex + index buffer）
    pub fn register_mesh(
        &mut self,
        device: &wgpu::Device,
        vertices: &[ShadowVertex],
        indices: &[u32],
    ) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.push(RegisteredShadowMesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            index_format: wgpu::IndexFormat::Uint32,
        });
    }

    /// 更新实例数据（model matrix）
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[ShadowInstance]) {
        let count = instances.len().min(self.max_instances);
        if count == 0 {
            self.instance_count = 0;
            return;
        }
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        self.instance_count = count as u32;
    }

    /// 设置实例数（用于 mesh draw）
    pub fn set_instance_count(&mut self, count: u32) {
        self.instance_count = count.min(self.max_instances as u32);
    }
}

impl RenderGraphNode for ShadowMapPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str { "shadow_map" }
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // ShadowMapPass 是 depth-only pass（fragment: None），无 color attachment。
        // shadow_texture/shadow_view 在 new() 时按 shadow_size 创建，固定尺寸，无需 ensure。
        //
        // 当前未实现 mesh/instance 注册接口，仅录制空 pass 清空 shadow texture 为 1.0
        // （最远深度），避免 ForwardPass 采样到未初始化数据导致阴影测试异常。
        // TODO: 后续添加 mesh 注册接口后，在此处 set_vertex_buffer + draw_indexed 渲染阴影几何体。

        // clone 到局部变量，避免 render_pass 借用 self.shadow_view 后无法借用 self.shadow_pipeline
        let shadow_view = self.shadow_view.clone();

        let mut render_pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow_map render pass"),
            color_attachments: &[], // depth-only pass，无 color 输出
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.shadow_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        // 渲染所有注册的 mesh（depth-only，无 fragment shader）
        if self.instance_count > 0 && !self.meshes.is_empty() {
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            for mesh in &self.meshes {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..self.instance_count);
            }
            log::debug!(
                "shadow_map: rendered {} mesh(es) x {} instances (shadow_size={})",
                self.meshes.len(),
                self.instance_count,
                self.shadow_size
            );
        } else {
            log::debug!(
                "shadow_map: no mesh/instance, only cleared shadow texture (shadow_size={})",
                self.shadow_size
            );
        }

        Ok(())
    }
}

/// 计算光源 view-projection 矩阵
fn compute_light_view_proj(light_dir: [f32; 3], camera_pos: [f32; 3], scene_radius: f32) -> [[f32; 4]; 4] {
    let dir = Vec3::from(light_dir);
    let len = dir.length();
    let dir = if len > 1e-6 { dir / len } else { Vec3::new(0.0, 1.0, 0.0) };

    let cam = Vec3::from(camera_pos);
    let light_pos = cam - dir * scene_radius;

    let up = if dir.y.abs() > 0.99 { Vec3::new(0.0, 0.0, 1.0) } else { Vec3::Y };
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
        assert_eq!(std::mem::size_of::<ShadowUniform>(), 80);
    }

    #[test]
    fn shadow_camera_uniform_size() {
        // view_proj(64) + view(64) + proj(64) + position(16) = 208 bytes
        assert_eq!(std::mem::size_of::<ShadowCameraUniform>(), 208);
    }

    #[test]
    fn compute_light_view_proj_basic() {
        let m = compute_light_view_proj([0.0, -1.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        let has_nonzero = m.iter().flat_map(|r| r.iter()).any(|&v| v.abs() > 1e-6);
        assert!(has_nonzero);
    }

    #[test]
    fn compute_light_view_proj_normalizes_direction() {
        let m1 = compute_light_view_proj([0.0, -1.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        let m2 = compute_light_view_proj([0.0, -10.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        for i in 0..4 {
            for j in 0..4 {
                assert!((m1[i][j] - m2[i][j]).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn compute_light_view_proj_zero_direction_uses_default() {
        let m = compute_light_view_proj([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 50.0);
        let has_nonzero = m.iter().flat_map(|r| r.iter()).any(|&v| v.abs() > 1e-6);
        assert!(has_nonzero);
    }

    #[test]
    fn shader_contains_key_elements() {
        assert!(SHADOW_SHADER.contains("vs_shadow"));
        assert!(SHADOW_SHADER.contains("CameraUniform"));
        assert!(SHADOW_SHADER.contains("model_col0"));
        assert!(SHADOW_SHADER.contains("view_proj"));
    }
}
