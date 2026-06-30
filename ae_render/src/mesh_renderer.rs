//! Mesh Renderer: 自定义 mesh 实例化渲染
//!
//! 用于渲染程序化生成的建筑/NPC/Morph 等复杂几何体。
//! 与 InstancedRenderer（只能画单位立方体）互补：
//! - InstancedRenderer: 36 顶点单位立方体 × N 实例（体素/粒子）
//! - MeshRenderer: 任意顶点 mesh × M 实例（建筑/NPC/生物）
//!
//! 设计：
//! - 每个注册的 mesh 拥有独立 vertex/index buffer
//! - 实例数据：model matrix (Mat4) + tint color (Vec4) = 80 bytes
//! - 共享 camera bind group（@group(0)）
//! - 共享 light bind group（@group(1)）：方向光 + 环境光
//! - 阴影 bind group（@group(2)）：PCF 软阴影采样

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler, TextureView};

use crate::camera::CameraUniform;
use crate::mesh::{Mesh, Vertex};
use crate::shadow_map::ShadowUniform;

/// 实例数据：模型矩阵 + 颜色调制
/// 64 bytes (Mat4) + 16 bytes (Vec4) = 80 bytes
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct MeshInstanceData {
    /// 模型矩阵（列主序，4x4 = 16 floats）
    pub model: [[f32; 4]; 4],
    /// 颜色调制（rgba）
    pub tint: [f32; 4],
}

impl MeshInstanceData {
    /// 从 translation + rotation(quat) + scale 构建
    pub fn from_trs(translation: [f32; 3], rotation: [f32; 4], scale: f32, tint: [f32; 4]) -> Self {
        let t = glam::Vec3::from(translation);
        let q = glam::Quat::from_vec4(glam::Vec4::from(rotation));
        let s = scale;
        // Mat4::from_translation * Mat4::from_quat * Mat4::from_scale
        let m = glam::Mat4::from_translation(t)
            * glam::Mat4::from_quat(q)
            * glam::Mat4::from_scale(glam::Vec3::splat(s));
        Self {
            model: m.to_cols_array_2d(),
            tint,
        }
    }

    /// 仅平移（无旋转，单位缩放）
    pub fn from_position(position: [f32; 3], tint: [f32; 4]) -> Self {
        let m = glam::Mat4::from_translation(glam::Vec3::from(position));
        Self {
            model: m.to_cols_array_2d(),
            tint,
        }
    }

    /// 平移 + 缩放（无旋转）
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
    /// xyz: 方向（指向光源），w: 强度
    pub direction: [f32; 4],
    /// rgb: 光色，w: unused
    pub color: [f32; 4],
    /// rgb: 环境光颜色，w: unused
    pub ambient: [f32; 4],
}

impl LightUniform {
    /// 创建方向光
    pub fn new(direction: [f32; 3], color: [f32; 3], intensity: f32, ambient: [f32; 3]) -> Self {
        // 归一化方向
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

    /// 白天默认光照
    pub fn day_default() -> Self {
        Self::new(
            [0.5, 1.0, 0.3],
            [1.0, 0.95, 0.9],
            2.5,
            [0.18, 0.20, 0.24],
        )
    }

    /// 夜晚默认光照
    pub fn night_default() -> Self {
        Self::new(
            [-0.3, -0.5, 0.4],
            [0.4, 0.5, 0.7],
            0.3,
            [0.05, 0.06, 0.10],
        )
    }

    /// 日落默认光照
    pub fn sunset_default() -> Self {
        Self::new(
            [0.8, 0.15, 0.4],
            [1.0, 0.5, 0.2],
            3.0,
            [0.20, 0.12, 0.08],
        )
    }
}

/// 已注册的 mesh：拥有独立 vertex/index buffer + 共享实例 buffer 段
pub struct RegisteredMesh {
    pub mesh: Mesh,
    pub instance_offset: usize, // 在全局 instance buffer 中的起始位置
    pub instance_count: usize,  // 实例数
}

/// MeshRenderer: 管理多个 mesh + 共享实例 buffer 的批量渲染
pub struct MeshRenderer {
    pub pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub camera_layout: BindGroupLayout,
    pub light_buffer: Buffer,
    pub light_bind_group: BindGroup,
    pub light_layout: BindGroupLayout,
    pub instance_buffer: Buffer,
    pub max_instances: usize,
    pub meshes: Vec<RegisteredMesh>,
    /// 阴影系统（@group(2)）：PCF 软阴影采样
    pub shadow_buffer: Buffer,
    pub shadow_bind_group: BindGroup,
    pub shadow_layout: BindGroupLayout,
    /// 当前绑定的 shadow map texture view（set_shadow_resources 设置）
    pub shadow_texture_view: Option<TextureView>,
    /// 当前绑定的 PCF comparison sampler（set_shadow_resources 设置）
    pub shadow_sampler: Option<Sampler>,
    /// 内部：初始 dummy 1x1 深度纹理，保持初始 shadow_bind_group 有效
    _dummy_shadow_texture: wgpu::Texture,
}

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
    direction: vec4<f32>,  // xyz: 方向, w: 强度
    color: vec4<f32>,      // rgb: 光色, w: unused
    ambient: vec4<f32>,    // rgb: 环境光, w: unused
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
    // 旋转法线（model 矩阵的左上 3x3）
    let n = (model * vec4<f32>(in.normal, 0.0)).xyz;
    out.normal = normalize(n);
    out.color = in.color * instance.tint;
    out.view_dir = normalize(camera.position.xyz - world_pos.xyz);
    out.uv = in.uv;
    out.world_pos = world_pos.xyz;
    out.light_space_pos = shadow_uniform.light_view_proj * world_pos;
    return out;
}

// 距离雾：根据世界坐标 Y 和距离相机距离计算雾因子
fn compute_fog(world_pos: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let dist = length(world_pos - camera.position.xyz);
    // 距离雾：远距离变浅灰蓝
    let fog_start = 200.0;
    let fog_end = 1500.0;
    let fog_factor = clamp((dist - fog_start) / (fog_end - fog_start), 0.0, 1.0);
    // 高度雾：低处雾更浓
    let height_factor = clamp((50.0 - world_pos.y) / 50.0, 0.0, 0.5);
    let total_fog = clamp(fog_factor + height_factor, 0.0, 1.0);
    let fog_color = vec3<f32>(0.55, 0.60, 0.65);
    return mix(vec3<f32>(0.0), fog_color, total_fog);
}

// PCF 3x3 软阴影采样
fn pcf_shadow(light_space_pos: vec4<f32>) -> f32 {
    let proj_coords = light_space_pos.xyz / light_space_pos.w;
    let uv = proj_coords.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let current_depth = proj_coords.z;
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || current_depth > 1.0) {
        return 1.0;  // 超出 shadow map 范围，无阴影
    }
    let texel_size = 1.0 / shadow_uniform.shadow_map_size.xy;
    var shadow = 0.0;
    // 3x3 PCF
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

    // 距离雾混合
    let fog_color = compute_fog(in.world_pos, in.view_dir);
    let dist = length(in.world_pos - camera.position.xyz);
    let fog_factor = clamp((dist - 200.0) / 1300.0, 0.0, 1.0);
    let height_factor = clamp((50.0 - in.world_pos.y) / 100.0, 0.0, 0.4);
    let total_fog = clamp(fog_factor + height_factor, 0.0, 1.0);
    color = mix(color, fog_color, total_fog);

    return vec4<f32>(color, in.color.a);
}
"#;

impl MeshRenderer {
    /// 创建 MeshRenderer，预分配 instance buffer
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_instances: usize,
    ) -> Self {
        // 共享 camera bind group layout（与 InstancedRenderer 兼容）
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh renderer camera layout"),
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
            label: Some("mesh renderer camera buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh renderer camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Light bind group layout（@group(1)）
        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh renderer light layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<LightUniform>() as u64),
                },
                count: None,
            }],
        });

        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh renderer light buffer"),
            size: std::mem::size_of::<LightUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh renderer light bind group"),
            layout: &light_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        // Shadow bind group layout（@group(2)）：uniform + depth texture + comparison sampler
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh renderer shadow layout"),
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
            label: Some("mesh renderer shadow uniform buffer"),
            size: std::mem::size_of::<ShadowUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Dummy 1x1 depth texture + comparison sampler，用于初始 shadow_bind_group
        // （set_shadow_resources 被调用前保持 bind_group 有效）
        let dummy_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mesh renderer dummy shadow texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
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
        let dummy_shadow_view = dummy_shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mesh renderer dummy shadow sampler"),
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

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh renderer shadow bind group"),
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
            label: Some("mesh renderer pipeline layout"),
            bind_group_layouts: &[&camera_layout, &light_layout, &shadow_layout],
            push_constant_ranges: &[],
        });

        // Instance buffer (pre-allocated)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh renderer instance buffer"),
            size: (max_instances * std::mem::size_of::<MeshInstanceData>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh renderer shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });

        // Vertex buffer layout for Vertex (64 bytes): position + normal + tangent + uv + color
        // Instance buffer layout for MeshInstanceData (80 bytes): model mat4 + tint vec4
        // mat4 在 WGSL 中占 4 个 location (5,6,7,8)，tint 占 1 个 location (9)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh renderer pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 24, shader_location: 2 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 40, shader_location: 3 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 4 },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MeshInstanceData>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // mat4 = 4 columns × Float32x4
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 5 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 6 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 7 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 8 },
                            // tint
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 64, shader_location: 9 },
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
            meshes: Vec::new(),
            shadow_buffer,
            shadow_bind_group,
            shadow_layout,
            shadow_texture_view: Some(dummy_shadow_view),
            shadow_sampler: Some(dummy_shadow_sampler),
            _dummy_shadow_texture: dummy_shadow_texture,
        }
    }

    /// 注册一个 mesh（上传 vertex/index buffer 到 GPU），返回 mesh_id
    pub fn register_mesh(
        &mut self,
        device: &Device,
        vertices: &[Vertex],
        indices: &[u32],
        label: Option<&str>,
    ) -> usize {
        let mesh = Mesh::from_data(device, vertices, indices, label);
        let id = self.meshes.len();
        self.meshes.push(RegisteredMesh {
            mesh,
            instance_offset: 0,
            instance_count: 0,
        });
        id
    }

    /// 设置每个 mesh 的实例数据（批量上传）
    /// `instances_per_mesh[i]` = mesh i 的实例列表
    pub fn update_instances(&mut self, queue: &Queue, instances_per_mesh: &[Vec<MeshInstanceData>]) {
        let mut flat: Vec<MeshInstanceData> = Vec::new();
        let mut offset = 0usize;
        for (i, mesh) in self.meshes.iter_mut().enumerate() {
            mesh.instance_offset = offset;
            let count = if i < instances_per_mesh.len() {
                instances_per_mesh[i].len()
            } else {
                0
            };
            mesh.instance_count = count;
            if i < instances_per_mesh.len() && count > 0 {
                flat.extend_from_slice(&instances_per_mesh[i]);
                offset += count;
            }
        }
        let total = flat.len().min(self.max_instances);
        if total > 0 {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&flat[..total]));
        }
    }

    /// Upload camera uniform
    pub fn update_camera(&self, queue: &Queue, uniform: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Upload light uniform（方向光 + 环境光）
    pub fn update_light(&self, queue: &Queue, light: &LightUniform) {
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[*light]));
    }

    /// 更新阴影 uniform（光源 VP 矩阵 + shadow map 尺寸），供 fragment shader 采样
    pub fn update_shadow_uniform(&self, queue: &Queue, shadow_uniform: &ShadowUniform) {
        queue.write_buffer(
            &self.shadow_buffer,
            0,
            bytemuck::cast_slice(&[*shadow_uniform]),
        );
    }

    /// 设置阴影资源（shadow map texture view + comparison sampler）
    /// 重建 shadow_bind_group 绑定新的 texture view 和 sampler
    pub fn set_shadow_resources(
        &mut self,
        device: &Device,
        shadow_view: &TextureView,
        shadow_sampler: &Sampler,
    ) {
        self.shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh renderer shadow bind group"),
            layout: &self.shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.shadow_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(shadow_sampler),
                },
            ],
        });
        self.shadow_texture_view = Some(shadow_view.clone());
        self.shadow_sampler = Some(shadow_sampler.clone());
    }

    /// 在 render pass 中绘制所有已注册 mesh 的实例
    pub fn draw_all(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.meshes.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.light_bind_group, &[]);
        pass.set_bind_group(2, &self.shadow_bind_group, &[]);
        // 实例 buffer 在 slot 1（共享）
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        for mesh in &self.meshes {
            if mesh.instance_count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            // 实例化绘制：使用 vertex_offset (instance_offset) 偏移到对应实例段
            // 注意：wgpu 的 draw_indexed 的 instances 参数是 (start_instance, instance_count)
            pass.draw_indexed(
                0..mesh.mesh.num_indices,
                0,
                mesh.instance_offset as u32..(mesh.instance_offset + mesh.instance_count) as u32,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_data_size() {
        // 64 (mat4) + 16 (vec4) = 80 bytes
        assert_eq!(std::mem::size_of::<MeshInstanceData>(), 80);
    }

    #[test]
    fn light_uniform_size() {
        // 3 × vec4 = 48 bytes
        assert_eq!(std::mem::size_of::<LightUniform>(), 48);
    }

    #[test]
    fn from_position_identity() {
        let d = MeshInstanceData::from_position([1.0, 2.0, 3.0], [1.0, 1.0, 1.0, 1.0]);
        // 第 4 列 (index 3) 应该是 translation (1, 2, 3, 1)
        assert_eq!(d.model[3], [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(d.tint, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn from_position_scale_uniform() {
        let d = MeshInstanceData::from_position_scale([0.0, 0.0, 0.0], 2.0, [1.0; 4]);
        // 第一列应该是 (2, 0, 0, 0)
        assert_eq!(d.model[0], [2.0, 0.0, 0.0, 0.0]);
        assert_eq!(d.model[1], [0.0, 2.0, 0.0, 0.0]);
    }

    #[test]
    fn light_normalizes_direction() {
        let l = LightUniform::new([5.0, 10.0, 3.0], [1.0; 3], 2.0, [0.2; 3]);
        let len = (l.direction[0] * l.direction[0]
            + l.direction[1] * l.direction[1]
            + l.direction[2] * l.direction[2])
        .sqrt();
        assert!((len - 1.0).abs() < 0.001, "direction should be normalized");
        assert!((l.direction[3] - 2.0).abs() < 0.001, "intensity preserved in w");
    }

    #[test]
    fn light_day_default_has_sun_high() {
        let l = LightUniform::day_default();
        assert!(l.direction[1] > 0.0, "day sun should be above horizon");
        assert!(l.direction[3] > 1.0, "day intensity should be bright");
    }

    #[test]
    fn light_night_default_is_dim() {
        let l = LightUniform::night_default();
        assert!(l.direction[3] < 1.0, "night intensity should be dim");
        assert!(l.ambient[0] < 0.15, "night ambient should be dark");
    }

    #[test]
    fn light_sunset_default_has_orange_tint() {
        let l = LightUniform::sunset_default();
        assert!(l.color[0] > l.color[2], "sunset should be reddish");
        assert!(l.color[1] > l.color[2], "sunset should have orange");
    }
}
