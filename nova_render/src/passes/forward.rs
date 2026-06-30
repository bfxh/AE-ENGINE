//! Forward Pass: 自定义 mesh 实例化渲染（port 自 v1 ae_render mesh_renderer）
//!
//! 用于渲染程序化生成的建筑/NPC/Morph 等复杂几何体：
//! - 每个注册的 mesh 拥有独立 vertex/index buffer（由 RenderGraph 调度绑定）
//! - 实例数据：model matrix (Mat4) + tint color (Vec4) = 80 bytes
//! - 共享 camera bind group（@group(0)）
//! - 共享 light bind group（@group(1)）：方向光 + 环境光
//! - 阴影 bind group（@group(2)）：PCF 软阴影采样

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline};

use crate::passes::shadow_map::ShadowUniform;
use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// Forward Camera Uniform（256 bytes = 4 × mat4，与 v1 CameraUniform 兼容）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ForwardCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

/// 实例数据：模型矩阵 + 颜色调制
/// 64 bytes (Mat4) + 16 bytes (Vec4) = 80 bytes
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct MeshInstanceData {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
}

impl MeshInstanceData {
    pub fn from_trs(translation: [f32; 3], rotation: [f32; 4], scale: f32, tint: [f32; 4]) -> Self {
        let t = glam::Vec3::from(translation);
        let q = glam::Quat::from_vec4(glam::Vec4::from(rotation));
        let s = scale;
        let m = glam::Mat4::from_translation(t)
            * glam::Mat4::from_quat(q)
            * glam::Mat4::from_scale(glam::Vec3::splat(s));
        Self {
            model: m.to_cols_array_2d(),
            tint,
        }
    }

    pub fn from_position(position: [f32; 3], tint: [f32; 4]) -> Self {
        let m = glam::Mat4::from_translation(glam::Vec3::from(position));
        Self {
            model: m.to_cols_array_2d(),
            tint,
        }
    }

    pub fn from_position_scale(position: [f32; 3], scale: f32, tint: [f32; 4]) -> Self {
        let m = glam::Mat4::from_translation(glam::Vec3::from(position))
            * glam::Mat4::from_scale(glam::Vec3::splat(scale));
        Self {
            model: m.to_cols_array_2d(),
            tint,
        }
    }
}

/// 光源 Uniform：方向光 + 环境光
/// 48 bytes（3 × vec4），符合 WGSL 16-byte 对齐
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct LightUniform {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub ambient: [f32; 4],
}

impl LightUniform {
    pub fn new(direction: [f32; 3], color: [f32; 3], intensity: f32, ambient: [f32; 3]) -> Self {
        let len = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
        .sqrt();
        let dir = if len > 0.0001 {
            [
                direction[0] / len,
                direction[1] / len,
                direction[2] / len,
            ]
        } else {
            [0.0, 1.0, 0.0]
        };
        Self {
            direction: [dir[0], dir[1], dir[2], intensity],
            color: [color[0], color[1], color[2], 0.0],
            ambient: [ambient[0], ambient[1], ambient[2], 0.0],
        }
    }

    pub fn default_day() -> Self {
        Self::new(
            [0.5, -0.8, 0.3],
            [1.0, 0.95, 0.85],
            1.2,
            [0.2, 0.22, 0.25],
        )
    }
}

/// Forward mesh 顶点（与 v1 Vertex 兼容，80 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ForwardVertex {
    pub position: [f32; 3],
    pub _pad1: f32,
    pub normal: [f32; 3],
    pub _pad2: f32,
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub _pad3: [f32; 2],
    pub color: [f32; 4],
}

impl Default for ForwardVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            _pad1: 0.0,
            normal: [0.0, 1.0, 0.0],
            _pad2: 0.0,
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            _pad3: [0.0; 2],
            color: [1.0; 4],
        }
    }
}

/// 内嵌 WGSL shader：Forward 渲染 + PCF 软阴影 + 距离雾（保留 v1 原样）
const MESH_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LightUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> light: LightUniform;

struct ShadowUniform {
    light_view_proj: mat4x4<f32>,
    shadow_map_size: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> shadow_uniform: ShadowUniform;
@group(2) @binding(1)
var t_shadow: texture_depth_2d;
@group(2) @binding(2)
var s_shadow: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct InstanceInput {
    @location(5) model_col0: vec4<f32>,
    @location(6) model_col1: vec4<f32>,
    @location(7) model_col2: vec4<f32>,
    @location(8) model_col3: vec4<f32>,
    @location(9) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_dir: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) world_pos: vec3<f32>,
    @location(5) light_space_pos: vec4<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(instance.model_col0, instance.model_col1, instance.model_col2, instance.model_col3);
    let world_pos = model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    let n = (model * vec4<f32>(in.normal, 0.0)).xyz;
    out.normal = normalize(n);
    out.color = in.color * instance.tint;
    out.view_dir = normalize(camera.position.xyz - world_pos.xyz);
    out.uv = in.uv;
    out.world_pos = world_pos.xyz;
    out.light_space_pos = shadow_uniform.light_view_proj * world_pos;
    return out;
}

fn compute_fog(world_pos: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let dist = length(world_pos - camera.position.xyz);
    let fog_start = 200.0;
    let fog_end = 1500.0;
    let fog_factor = clamp((dist - fog_start) / (fog_end - fog_start), 0.0, 1.0);
    let height_factor = clamp((50.0 - world_pos.y) / 50.0, 0.0, 0.5);
    let total_fog = clamp(fog_factor + height_factor, 0.0, 1.0);
    let fog_color = vec3<f32>(0.55, 0.60, 0.65);
    return mix(vec3<f32>(0.0), fog_color, total_fog);
}

fn pcf_shadow(light_space_pos: vec4<f32>) -> f32 {
    let proj_coords = light_space_pos.xyz / light_space_pos.w;
    let uv = proj_coords.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let current_depth = proj_coords.z;
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || current_depth > 1.0) {
        return 1.0;
    }
    let texel_size = 1.0 / shadow_uniform.shadow_map_size.xy;
    var shadow = 0.0;
    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow = shadow + textureSampleCompare(t_shadow, s_shadow, uv + offset, current_depth);
        }
    }
    return shadow / 9.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(light.direction.xyz);
    let light_color = light.color.rgb * light.direction.w;
    let ambient = light.ambient.rgb;

    let n_dot_l = max(dot(in.normal, light_dir), 0.0);
    let half_dir = normalize(light_dir + in.view_dir);
    let n_dot_h = max(dot(in.normal, half_dir), 0.0);

    let spec_power = 32.0;
    let specular = pow(n_dot_h, spec_power) * 0.25;

    let shadow_factor = pcf_shadow(in.light_space_pos);
    let diffuse = n_dot_l * light_color * shadow_factor;
    var color = (ambient + diffuse) * in.color.rgb + specular * light_color * shadow_factor;

    let fog_color = compute_fog(in.world_pos, in.view_dir);
    let dist = length(in.world_pos - camera.position.xyz);
    let fog_factor = clamp((dist - 200.0) / 1300.0, 0.0, 1.0);
    let height_factor = clamp((50.0 - in.world_pos.y) / 100.0, 0.0, 0.4);
    let total_fog = clamp(fog_factor + height_factor, 0.0, 1.0);
    color = mix(color, fog_color, total_fog);

    return vec4<f32>(color, in.color.a);
}
"#;

/// ForwardPass: 自定义 mesh 实例化渲染节点
pub struct ForwardPass {
    pub pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub camera_layout: BindGroupLayout,
    pub light_buffer: Buffer,
    pub light_bind_group: BindGroup,
    pub light_layout: BindGroupLayout,
    pub instance_buffer: Buffer,
    pub max_instances: usize,
    pub shadow_buffer: Buffer,
    pub shadow_bind_group: BindGroup,
    pub shadow_layout: BindGroupLayout,
    pub shadow_texture_view: Option<wgpu::TextureView>,
    pub shadow_sampler: Option<wgpu::Sampler>,
    _dummy_shadow_texture: wgpu::Texture,
    /// 已注册的 mesh 列表（每个 mesh 有独立 vertex/index buffer）
    pub meshes: Vec<RegisteredMesh>,
    /// 当前实例数（由 update_instances 跟踪）
    pub instance_count: u32,
    /// 深度纹理（与 surface 尺寸匹配）
    pub depth_texture: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
    pub depth_size: (u32, u32),
    pub depth_format: wgpu::TextureFormat,
    /// 自动动画相机：execute() 根据 ctx.time 绕 Y 轴旋转相机（用于 demo）
    pub auto_animate_camera: bool,
}

/// 已注册的 mesh：拥有独立 vertex/index buffer
pub struct RegisteredMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub index_format: wgpu::IndexFormat,
}

impl ForwardPass {
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_instances: usize,
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forward camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ForwardCameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forward camera buffer"),
            size: std::mem::size_of::<ForwardCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forward light layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<LightUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forward light buffer"),
            contents: bytemuck::cast_slice(&[LightUniform::default_day()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward light bind group"),
            layout: &light_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let safe_instances = max_instances.max(1);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forward instance buffer"),
            size: (safe_instances * std::mem::size_of::<MeshInstanceData>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forward shadow layout"),
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

        let shadow_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forward shadow uniform buffer"),
            size: std::mem::size_of::<ShadowUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let dummy_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("forward dummy shadow texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_shadow_view = dummy_shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let dummy_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("forward dummy shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward shadow bind group"),
            layout: &shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dummy_shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&dummy_shadow_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("forward pipeline layout"),
            bind_group_layouts: &[&camera_layout, &light_layout, &shadow_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("forward mesh shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("forward pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ForwardVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 16,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 48,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 64,
                                shader_location: 4,
                            },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MeshInstanceData>() as wgpu::BufferAddress,
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
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 64,
                                shader_location: 9,
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
            camera_layout,
            light_buffer,
            light_bind_group,
            light_layout,
            instance_buffer,
            max_instances,
            shadow_buffer,
            shadow_bind_group,
            shadow_layout,
            shadow_texture_view: None,
            shadow_sampler: None,
            _dummy_shadow_texture: dummy_shadow_texture,
            meshes: Vec::new(),
            instance_count: 0,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            depth_format,
            auto_animate_camera: false,
        }
    }

    pub fn update_camera(&self, queue: &Queue, uniform: &ForwardCameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn update_light(&self, queue: &Queue, uniform: &LightUniform) {
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn update_shadow(&self, queue: &Queue, uniform: &ShadowUniform) {
        queue.write_buffer(&self.shadow_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn update_instances(&self, queue: &Queue, instances: &[MeshInstanceData]) {
        let count = instances.len().min(self.max_instances);
        if count > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instances[..count]),
            );
        }
    }

    pub fn set_shadow_resources(
        &mut self,
        device: &Device,
        texture_view: wgpu::TextureView,
        sampler: wgpu::Sampler,
    ) {
        self.shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward shadow bind group (live)"),
            layout: &self.shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.shadow_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        self.shadow_texture_view = Some(texture_view);
        self.shadow_sampler = Some(sampler);
    }

    /// 确保深度纹理与 surface 尺寸匹配（不匹配则重建）
    ///
    /// 参考 SkyboxPass::ensure_depth 模式：每帧检查尺寸，窗口 resize 时自动重建。
    pub fn ensure_depth(&mut self, device: &Device, width: u32, height: u32) {
        if self.depth_size == (width, height) && self.depth_view.is_some() {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("forward depth texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
    }

    /// 注册一个 mesh：上传 vertex/index buffer 到 GPU，返回 mesh 索引
    ///
    /// 每个 mesh 拥有独立的 vertex/index buffer，execute() 时遍历绘制。
    pub fn register_mesh(
        &mut self,
        device: &Device,
        vertices: &[ForwardVertex],
        indices: &[u32],
    ) -> usize {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forward mesh vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forward mesh index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mesh = RegisteredMesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            index_format: wgpu::IndexFormat::Uint32,
        };
        self.meshes.push(mesh);
        self.meshes.len() - 1
    }

    /// 清空已注册的 mesh 列表（每帧开始前调用，避免内存累积）
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
    }

    /// 设置当前实例数（execute() 时按此值绘制实例）
    pub fn set_instance_count(&mut self, count: u32) {
        self.instance_count = count;
    }
}

impl Default for ForwardPass {
    fn default() -> Self {
        unreachable!("ForwardPass requires device + format + instance count arguments");
    }
}

impl RenderGraphNode for ForwardPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "forward"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 无 mesh 或无实例时直接跳过（避免空 pass 浪费 GPU 周期）
        if self.meshes.is_empty() || self.instance_count == 0 {
            return Ok(());
        }

        let (width, height) = ctx.surface_size;
        self.ensure_depth(ctx.device, width, height);

        // 自动动画相机：绕 Y 轴旋转，0.5 rad/s（demo 模式，无需外部 node_mut 更新）
        if self.auto_animate_camera {
            let aspect = width as f32 / height.max(1) as f32;
            let angle = ctx.time * 0.5;
            let eye = glam::Vec3::new(angle.sin() * 5.0, 2.0, angle.cos() * 5.0);
            let view = glam::Mat4::look_at_rh(eye, glam::Vec3::ZERO, glam::Vec3::Y);
            let proj = glam::Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 1000.0);
            let camera_uniform = ForwardCameraUniform {
                view_proj: (proj * view).to_cols_array_2d(),
                view: view.to_cols_array_2d(),
                proj: proj.to_cols_array_2d(),
                position: [eye.x, eye.y, eye.z, 0.0],
            };
            ctx.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
        }

        let color_view = ctx.surface_view.ok_or_else(|| {
            anyhow::anyhow!("forward: surface_view is None (no swapchain target)")
        })?;
        let depth_view = self.depth_view.clone().ok_or_else(|| {
            anyhow::anyhow!("forward: depth_view is None (ensure_depth failed)")
        })?;

        let mut render_pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("forward render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // LoadOp::Load: 保留 SkyboxPass 绘制的天空背景
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.light_bind_group, &[]);
        render_pass.set_bind_group(2, &self.shadow_bind_group, &[]);

        // instance buffer 绑定到 slot 1（对所有 mesh 共享，与 vertex buffer layout 第二个 entry 对应）
        let instance_byte_len =
            (self.instance_count as usize) * std::mem::size_of::<MeshInstanceData>();
        render_pass.set_vertex_buffer(
            1,
            self.instance_buffer.slice(0..instance_byte_len as u64),
        );

        // 遍历所有已注册的 mesh，每个 mesh 设置自己的 vertex/index buffer，共享 instance buffer
        for mesh in &self.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
            render_pass.draw_indexed(0..mesh.num_indices, 0, 0..self.instance_count);
        }

        Ok(())
    }
}
