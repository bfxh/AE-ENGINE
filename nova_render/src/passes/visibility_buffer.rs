//! Visibility Buffer Pass: 仅记录可见几何 ID（triangle + instance + barycentric）
//!
//! 论文：Burns et al. 2013 "Visibility Buffer: A Framework for Real-Time Rendering"
//!
//! 与 G-Buffer 不同，本 pass 不存储 albedo/normal/roughness 等材质属性，
//! 仅写入：
//!   - triangle_id  (u32) — 用于后续 pass 查询顶点位置/法线/UV
//!   - instance_id  (u32) — 用于查询 instance 变换矩阵
//!   - barycentric  (vec2<f32>) — 重心坐标用于插值（bary.z = 1 - x - y 可后续重建）
//!
//! 后续 pass 通过 triangle_id + instance_id 反查几何缓冲，带宽开销远低于 G-Buffer。
//!
//! 简化实现（wgpu 24）：硬件光栅化，fragment shader 输出 Rgba32Uint color attachment，
//! 同时输出 Depth32Float 用于深度测试。软件光栅化路径留待 P2。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, Extent3d, Queue, RenderPipeline, Texture,
    TextureView,
};

use crate::passes::forward::{MeshInstanceData, RegisteredMesh};
use crate::render_graph::{NodeContext, NodeResult, RenderGraphNode};
use crate::scene::camera::Camera;

/// Visibility Uniform：相机矩阵 + 视口
///
/// WGSL 布局（uniform address space，16-byte 对齐）：
///   - view_proj: mat4x4<f32>  (64 bytes, align 16)
///   - viewport:  vec4<f32>    (16 bytes, align 16)
///   - _pad:      vec2<f32>    ( 8 bytes, align 8)
///
/// 总大小 88 bytes，wgpu 24 接受非 16 倍数的 uniform struct（与 ShadowUniform 一致）。
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct VisibilityUniform {
    pub view_proj: [[f32; 4]; 4],
    pub viewport: [f32; 4], // x, y, w, h
    pub _pad: [f32; 2],
}

/// 从 visibility buffer 重建的可见点信息
///
/// 对应 Rgba32Uint 像素的 4 个通道：
///   R = triangle_id (u32)
///   G = instance_id (u32)
///   B = barycentric_x (f32 bitcast 为 u32)
///   A = barycentric_y (f32 bitcast 为 u32)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct VisibilityData {
    pub triangle_id: u32,
    pub instance_id: u32,
    pub barycentric_x: f32,
    pub barycentric_y: f32,
}

/// 内嵌 WGSL shader：写入 Rgba32Uint visibility buffer
///
/// vertex shader：
///   - 计算三角形 ID（vertex_index / 3）
///   - 计算 instance ID（builtin instance_index）
///   - 计算重心坐标（vertex_index % 3 → (1,0,0) / (0,1,0) / (0,0,1)）
///
/// fragment shader：
///   - 输出 vec4<u32>：bitcast f32 → u32 后写入 Rgba32Uint color attachment
const VISIBILITY_SHADER: &str = r#"
struct VisibilityUniform {
    view_proj: mat4x4<f32>,
    viewport: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: VisibilityUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_col0: vec4<f32>,
    @location(6) model_col1: vec4<f32>,
    @location(7) model_col2: vec4<f32>,
    @location(8) model_col3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) triangle_id: u32,
    @location(1) instance_id: u32,
    @location(2) barycentric: vec3<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(
        instance.model_col0,
        instance.model_col1,
        instance.model_col2,
        instance.model_col3,
    );
    let world_pos = model * vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;

    // triangle_id：每 3 个顶点构成一个三角形
    out.triangle_id = vertex_index / 3u;
    out.instance_id = instance_index;

    // 重心坐标：根据顶点在三角形内的位置 (0/1/2)
    let idx = vertex_index % 3u;
    if (idx == 0u) {
        out.barycentric = vec3<f32>(1.0, 0.0, 0.0);
    } else if (idx == 1u) {
        out.barycentric = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        out.barycentric = vec3<f32>(0.0, 0.0, 1.0);
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<u32> {
    // Rgba32Uint 输出：(triangle_id, instance_id, bary.x, bary.y)
    // bary.z 可在后续 pass 中通过 1 - x - y 重建
    return vec4<u32>(
        in.triangle_id,
        in.instance_id,
        bitcast<u32>(in.barycentric.x),
        bitcast<u32>(in.barycentric.y),
    );
}
"#;

/// Visibility Buffer Pass
///
/// 设计要点：
///   1. Rgba32Uint color attachment 存储 visibility 数据
///   2. Depth32Float depth attachment 用于深度测试（防止 z-fighting 用轻度 bias）
///   3. 单一 bind group：camera uniform
///   4. vertex buffer 布局兼容 forward.rs / shadow_map.rs（position @ location 0，
///      instance model cols @ location 5-8），方便共享 mesh 数据
///   5. 实例数据由外部 RenderGraph 调度时绑定（与 forward.rs 一致）
pub struct VisibilityBufferPass {
    pub pipeline: Option<RenderPipeline>,
    pub bind_layout: Option<BindGroupLayout>,
    pub uniform_buffer: Option<Buffer>,
    pub bind_group: Option<BindGroup>,
    pub vis_texture: Option<Texture>,
    pub vis_view: Option<TextureView>,
    pub depth_texture: Option<Texture>,
    pub depth_view: Option<TextureView>,
    pub width: u32,
    pub height: u32,
}

impl VisibilityBufferPass {
    /// Visibility buffer 像素格式：Rgba32Uint（每通道 32 位无符号整数）
    pub const VIS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Uint;
    /// 深度格式：Depth32Float
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// 创建 visibility buffer pass
    ///
    /// `width` / `height` 至少为 1（内部自动 clamp）
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let mut pass = Self {
            pipeline: None,
            bind_layout: None,
            uniform_buffer: None,
            bind_group: None,
            vis_texture: None,
            vis_view: None,
            depth_texture: None,
            depth_view: None,
            width: width.max(1),
            height: height.max(1),
        };
        pass.create_resources(device);
        pass
    }

    /// 创建所有 GPU 资源（pipeline + textures + uniform buffer）
    fn create_resources(&mut self, device: &Device) {
        // ---------- 1. BindGroupLayout ----------
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("visibility buffer layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<VisibilityUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        // ---------- 2. Uniform buffer + BindGroup ----------
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visibility uniform buffer"),
            size: std::mem::size_of::<VisibilityUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("visibility bind group"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // ---------- 3. Visibility texture (Rgba32Uint) ----------
        let vis_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visibility buffer texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::VIS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vis_view = vis_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---------- 4. Depth texture ----------
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visibility depth texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---------- 5. Pipeline layout ----------
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visibility buffer pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        // ---------- 6. Shader ----------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visibility buffer shader"),
            source: wgpu::ShaderSource::Wgsl(VISIBILITY_SHADER.into()),
        });

        // ---------- 7. Render pipeline ----------
        // Vertex buffer 布局兼容 forward.rs / shadow_map.rs：
        //   - vertex: position @ location(0)
        //   - instance: model matrix cols 0-3 @ location(5-8)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("visibility buffer pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 12, // vec3<f32>
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: 64, // 4 × vec4<f32>
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
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: Self::VIS_FORMAT,
                    blend: None, // Rgba32Uint 不支持 blend
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
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.pipeline = Some(pipeline);
        self.bind_layout = Some(bind_layout);
        self.uniform_buffer = Some(uniform_buffer);
        self.bind_group = Some(bind_group);
        self.vis_texture = Some(vis_texture);
        self.vis_view = Some(vis_view);
        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
    }

    /// 调整 visibility buffer 与 depth texture 尺寸（pipeline / uniform 可复用）
    ///
    /// 旧纹理自动 Drop 回收。`width` / `height` 至少为 1。
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);

        let vis_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visibility buffer texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::VIS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vis_view = vis_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visibility depth texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.vis_texture = Some(vis_texture);
        self.vis_view = Some(vis_view);
        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
    }

    /// 渲染场景几何到 visibility buffer
    ///
    /// 本方法负责：
    ///   1. 更新 camera uniform（view_proj + viewport）
    ///   2. 写入 instance buffer（由外部 RenderGraph 调度时绑定到 pipeline）
    ///
    /// 注意：实际的 render pass 编码 + draw call 由 RenderGraph 调度器在
    /// `RenderGraphNode::execute` 中完成，需要外部提供 vertex/index buffer。
    /// `instances` 参数当前用于将来的 RenderGraph 集成，本方法仅更新 uniform。
    pub fn render(
        &self,
        _device: &Device,
        queue: &Queue,
        camera: &Camera,
        _instances: &[MeshInstanceData],
    ) {
        let view_proj = camera.view_proj();
        let uniform = VisibilityUniform {
            view_proj: view_proj.to_cols_array_2d(),
            viewport: [0.0, 0.0, self.width as f32, self.height as f32],
            _pad: [0.0; 2],
        };

        if let Some(buf) = &self.uniform_buffer {
            queue.write_buffer(buf, 0, bytemuck::cast_slice(&[uniform]));
        }
    }
}

impl Default for VisibilityBufferPass {
    fn default() -> Self {
        unreachable!("VisibilityBufferPass requires device + dimensions");
    }
}

impl RenderGraphNode for VisibilityBufferPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "visibility_buffer"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 1. 取出所需 GPU 资源（Option → 引用，避免后续 &mut self 借用冲突）
        //    注：view_proj uniform 由外部 render() 方法在 execute 前写入
        let pipeline = self
            .pipeline
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("visibility_buffer: pipeline not built"))?;
        let vis_view = self
            .vis_view
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("visibility_buffer: vis_view not built"))?;
        let depth_view = self
            .depth_view
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("visibility_buffer: depth_view not built"))?;
        let bind_group = self
            .bind_group
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("visibility_buffer: bind_group not built"))?;

        // 2. 开 render pass：清空 vis（全 0）+ depth（1.0）
        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("visibility_buffer render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: vis_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // 3. 绑定 pipeline + camera bind group
        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, bind_group, &[]);

        // 4. 注：mesh draw 需要外部 RenderGraph 调度器在 execute 前调用
        //    set_vertex_buffer/set_index_buffer/draw_indexed（与 forward.rs 一致）。
        //    当前 pass 字段未持有 mesh buffer，待后续扩展 meshes 字段后补全 draw 调用。
        log::debug!(
            "visibility_buffer: pipeline + bind_group set ({}x{}), mesh draw pending mesh field extension",
            self.width,
            self.height
        );
        Ok(())
    }
}
