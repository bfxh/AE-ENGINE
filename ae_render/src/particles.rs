//! GPU Particle System: compute-driven 粒子模拟 + billboard 渲染
//!
//! 在 GPU 上使用 compute shader 更新粒子物理（位置/速度/生命），使用 billboard
//! 四边形技术渲染。所有粒子数据驻留在 GPU 内存中，CPU 端只上传 uniform。
//!
//! 设计：
//! - ParticleData: 64 bytes/粒子（4 × vec4），存储在 storage buffer（read_write）
//! - Compute pipeline: @workgroup_size(64)，每线程处理一个粒子
//! - Render pipeline: 4 顶点 quad × N 实例，billboard 朝向相机
//! - 混合：加法混合（SrcAlpha + One），适合火焰/火花；不写深度，深度比较 LessEqual
//!
//! 数据流：
//! ```text
//!   CPU uniform ──► uniform_buffer ──► compute_shader ──► particle_buffer ──► vertex_shader
//!                                              ▲                                   │
//!                                              └───────(下一帧)──────────────────────┘
//! ```
//!
//! WGSL 约束：
//! - 不允许 mat4x4 作为 vertex input/output（矩阵通过 uniform 传递）
//! - vec2<f32> 在结构体中需 8-byte 对齐，使用两个独立 f32 标量代替（uv_x, uv_y）

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CommandEncoder, ComputePipeline, Device, Queue, RenderPass,
    RenderPipeline,
};

use crate::camera::CameraUniform;

/// Compute shader 工作组大小（每工作组处理 64 个粒子）
const WORKGROUP_SIZE: u32 = 64;

// ============ 数据结构 ============

/// 单个粒子数据（64 bytes = 4 × vec4）
///
/// 字段布局与 WGSL `struct ParticleData` 一致：
/// - position: xyz 位置, w = size（billboard 大小）
/// - velocity: xyz 速度, w = life（剩余生命，0 = 死亡）
/// - color:    rgba 颜色
/// - spawn:    x = max_life, y = spawn_time, z = seed, w = flags
///   flags: 0=普通, 1=火焰, 2=烟雾, 3=火花, 4=灰尘
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ParticleData {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub color: [f32; 4],
    pub spawn: [f32; 4],
}

impl Default for ParticleData {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 0.1],
            velocity: [0.0, 0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            spawn: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

impl ParticleData {
    /// 创建一个死亡的粒子（life=0），给定随机种子
    pub fn dead(seed: u32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 0.1],
            velocity: [0.0, 0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            spawn: [1.0, 0.0, seed as f32, 0.0],
        }
    }
}

/// 粒子系统 Uniform（80 bytes = 4 × f32 + 4 × vec4，符合 WGSL 16-byte 对齐）
///
/// 字段布局与 WGSL `struct ParticleUniform` 一致：
/// - delta_time:  帧时间（秒）
/// - time:        累计时间（秒）
/// - gravity:     重力加速度（默认 -9.8）
/// - drag:        空气阻力系数（默认 0.1）
/// - emitter_pos: xyz 发射器位置 + w radius（发射半径）
/// - emitter_vel: xyz 发射器速度 + w spawn_rate（每秒每粒子的复活概率）
/// - wind:        xyz 风向 + w strength（风强度）
/// - _pad:        填充（保持 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ParticleUniform {
    pub delta_time: f32,
    pub time: f32,
    pub gravity: f32,
    pub drag: f32,
    pub emitter_pos: [f32; 4],
    pub emitter_vel: [f32; 4],
    pub wind: [f32; 4],
    pub _pad: [f32; 4],
}

impl ParticleUniform {
    /// 默认场景参数：标准重力 + 低阻力 + 小范围发射器
    pub fn default_scene() -> Self {
        Self {
            delta_time: 0.016,
            time: 0.0,
            gravity: -9.8,
            drag: 0.1,
            emitter_pos: [0.0, 0.0, 0.0, 0.5],
            emitter_vel: [0.0, 0.0, 0.0, 10.0],
            wind: [0.0, 0.0, 0.0, 0.0],
            _pad: [0.0; 4],
        }
    }
}

// ============ WGSL Compute Shader ============

const COMPUTE_SHADER: &str = r#"
struct ParticleData {
    position: vec4<f32>,
    velocity: vec4<f32>,
    color: vec4<f32>,
    spawn: vec4<f32>,
};

struct ParticleUniform {
    delta_time: f32,
    time: f32,
    gravity: f32,
    drag: f32,
    emitter_pos: vec4<f32>,
    emitter_vel: vec4<f32>,
    wind: vec4<f32>,
    _pad: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<ParticleData>;
@group(0) @binding(1) var<uniform> u: ParticleUniform;

// 简单哈希函数：基于 fract 乘法的 GPU 随机数
fn hash_f(seed: f32) -> f32 {
    var s = seed;
    s = fract(s * 0.1031);
    s = s * (s + 33.33);
    s = s * (s + s);
    return fract(s);
}

@workgroup_size(64)
@compute
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let count = arrayLength(&particles);
    if (idx >= count) {
        return;
    }

    var p = particles[idx];
    let dt = u.delta_time;
    let max_life = max(p.spawn.x, 0.0001);

    // 更新生命
    p.velocity.w = p.velocity.w - dt;

    if (p.velocity.w <= 0.0) {
        // 死亡粒子：根据 spawn_rate 概率复活
        let spawn_rate = u.emitter_vel.w;
        let respawn_prob = clamp(spawn_rate * dt, 0.0, 1.0);
        let rand = hash_f(p.spawn.z + u.time * 1.37);

        if (rand < respawn_prob) {
            // 复活：重置位置到 emitter_pos + 随机偏移
            let r1 = hash_f(p.spawn.z * 1.13 + u.time * 0.7);
            let r2 = hash_f(p.spawn.z * 2.27 + u.time * 1.3);
            let r3 = hash_f(p.spawn.z * 3.41 + u.time * 1.9);
            let r4 = hash_f(p.spawn.z * 4.55 + u.time * 2.5);

            let radius = u.emitter_pos.w;
            let offset = vec3<f32>(r1 - 0.5, r2 - 0.5, r3 - 0.5) * (2.0 * radius);
            p.position = vec4<f32>(u.emitter_pos.xyz + offset, p.position.w);

            // 随机方向速度（偏向上方）
            let speed = 1.0 + r4 * 2.0;
            p.velocity = vec4<f32>(
                (r1 - 0.5) * speed,
                (0.5 + r2 * 0.5) * speed,
                (r3 - 0.5) * speed,
                max_life,
            );

            // 根据 flags 设置颜色
            let flags = p.spawn.w;
            if (flags < 0.5) {
                // 普通：白色
                p.color = vec4<f32>(0.8, 0.8, 0.8, 0.8);
            } else if (flags < 1.5) {
                // 火焰：暖橙色
                p.color = vec4<f32>(1.0, 0.5, 0.1, 0.9);
            } else if (flags < 2.5) {
                // 烟雾：深灰半透明
                p.color = vec4<f32>(0.3, 0.3, 0.3, 0.4);
            } else if (flags < 3.5) {
                // 火花：亮黄色
                p.color = vec4<f32>(1.0, 0.85, 0.3, 1.0);
            } else {
                // 灰尘：棕色
                p.color = vec4<f32>(0.7, 0.6, 0.5, 0.5);
            }
        } else {
            // 保持死亡
            p.velocity.w = 0.0;
        }
    } else {
        // 存活粒子：更新物理
        // pos += vel * dt
        p.position.xyz = p.position.xyz + p.velocity.xyz * dt;
        // vel.y += gravity * dt
        p.velocity.y = p.velocity.y + u.gravity * dt;
        // vel *= (1 - drag * dt)
        let drag_factor = max(1.0 - u.drag * dt, 0.0);
        p.velocity.xyz = p.velocity.xyz * drag_factor;
        // vel += wind * strength * dt
        p.velocity.xyz = p.velocity.xyz + u.wind.xyz * (u.wind.w * dt);
    }

    particles[idx] = p;
}
"#;

// ============ WGSL Render Shader ============

const RENDER_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

struct ParticleData {
    position: vec4<f32>,
    velocity: vec4<f32>,
    color: vec4<f32>,
    spawn: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<storage, read> particles: array<ParticleData>;

// 注意：不使用 vec2 作为 vertex output（避免 8-byte 对齐问题），
// 用两个独立 f32 标量 uv_x / uv_y 代替。
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_x: f32,
    @location(2) uv_y: f32,
    @location(3) life_ratio: f32,
    @location(4) flags: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> VertexOutput {
    let p = particles[iid];

    // Quad 四个角（triangle strip）：offset 与 uv
    var offsets_x = array<f32, 4>(-1.0, 1.0, -1.0, 1.0);
    var offsets_y = array<f32, 4>(-1.0, -1.0, 1.0, 1.0);
    var uvs_x = array<f32, 4>(0.0, 1.0, 0.0, 1.0);
    var uvs_y = array<f32, 4>(0.0, 0.0, 1.0, 1.0);

    let ox = offsets_x[vid];
    let oy = offsets_y[vid];

    // Billboard：从 view 矩阵提取相机的 right 和 up 向量（世界空间）
    let right = camera.view[0].xyz;
    let up = camera.view[1].xyz;

    let size = p.position.w;
    let world_pos = p.position.xyz + (right * ox + up * oy) * size;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = p.color;
    out.uv_x = uvs_x[vid];
    out.uv_y = uvs_y[vid];

    let life = max(p.velocity.w, 0.0);
    let max_life = max(p.spawn.x, 0.0001);
    out.life_ratio = clamp(life / max_life, 0.0, 1.0);
    out.flags = p.spawn.w;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 重建 UV 坐标
    let uv = vec2<f32>(in.uv_x, in.uv_y);

    // 软粒子：圆形 alpha 衰减
    let d = length(uv - vec2<f32>(0.5, 0.5)) * 2.0;
    let soft_alpha = smoothstep(1.0, 0.0, d);

    // 生命衰减
    let life_alpha = in.life_ratio;

    var color = in.color;

    // 根据粒子类型调制颜色
    let flags = in.flags;
    if (flags < 0.5) {
        // 普通：保持原色
    } else if (flags < 1.5) {
        // 火焰：暖色提亮，年轻时更亮
        color.rgb = color.rgb * (1.5 + in.life_ratio * 0.5);
    } else if (flags < 2.5) {
        // 烟雾：随年龄变淡
        color.rgb = color.rgb * (0.4 + in.life_ratio * 0.6);
    } else if (flags < 3.5) {
        // 火花：高亮
        color.rgb = color.rgb * 2.5;
    } else {
        // 灰尘：柔和
        color.rgb = color.rgb * 0.8;
    }

    let alpha = color.a * soft_alpha * life_alpha;
    return vec4<f32>(color.rgb, alpha);
}
"#;

// ============ ParticleSystem ============

/// GPU 粒子系统
///
/// 使用方式：
/// ```ignore
/// let particles = ParticleSystem::new(&device, hdr_format, depth_format, 4096);
/// particles.update(&queue, &uniform);
/// particles.dispatch(&mut encoder);
/// // 渲染时：
/// particles.draw(&mut render_pass, &camera_bind_group);
/// ```
pub struct ParticleSystem {
    /// 粒子数据 buffer（GPU 可读写 storage buffer）
    pub particle_buffer: Buffer,
    /// 最大粒子数
    pub max_particles: u32,
    /// Compute pipeline（粒子物理更新）
    pub compute_pipeline: ComputePipeline,
    /// Render pipeline（billboard 渲染）
    pub render_pipeline: RenderPipeline,
    /// Uniform buffer（ParticleUniform）
    pub uniform_buffer: Buffer,
    /// Uniform bind group（绑定到 uniform_layout）
    pub uniform_bind_group: BindGroup,
    /// Uniform bind group layout（仅 uniform buffer）
    pub uniform_layout: BindGroupLayout,
    /// Camera bind group layout（与 mesh_renderer 一致，供外部 bind group 兼容）
    pub camera_layout: BindGroupLayout,
    /// Particle storage bind group layout（compute shader 用）
    pub particle_layout: BindGroupLayout,
    /// Compute bind group（绑定 particle_buffer 到 particle_layout）
    pub compute_bind_group: BindGroup,
    /// Compute dispatch 工作组数 = (max_particles + 63) / 64
    pub workgroup_count: u32,
}

impl ParticleSystem {
    /// 创建粒子系统
    ///
    /// # Arguments
    /// - `device`: wgpu device
    /// - `color_format`: HDR 颜色附件格式（如 Rgba16Float）
    /// - `depth_format`: 深度附件格式（如 Depth32Float）
    /// - `max_particles`: 最大粒子数（0 表示空系统，内部仍会创建 1 粒子 buffer）
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_particles: u32,
    ) -> Self {
        // 边界保护：buffer 至少容纳 1 个粒子，避免 0-size buffer
        let safe_count = max_particles.max(1);
        let buffer_size = (safe_count as u64) * std::mem::size_of::<ParticleData>() as u64;
        let workgroup_count = (max_particles + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        // ============ Bind group layouts ============

        // particle_layout: storage buffer（read_write），compute 和 render 共用
        let particle_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle storage layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // uniform_layout: uniform buffer（ParticleUniform）
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ParticleUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        // camera_layout: uniform buffer（CameraUniform），与 mesh_renderer 一致
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle camera layout"),
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

        // ============ Buffers ============

        // 初始化所有粒子为死亡状态，每粒子 seed = index（保证随机性不同）
        let initial_particles: Vec<ParticleData> = (0..safe_count)
            .map(|i| ParticleData::dead(i as u32))
            .collect();
        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle data buffer"),
            contents: bytemuck::cast_slice(&initial_particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle uniform buffer"),
            contents: bytemuck::cast_slice(&[ParticleUniform::default_scene()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ============ Bind groups ============

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle compute bind group"),
            layout: &particle_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle uniform bind group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // ============ Compute pipeline ============

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle compute shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle compute pipeline layout"),
            bind_group_layouts: &[&particle_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // ============ Render pipeline ============

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle render shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle render pipeline layout"),
            bind_group_layouts: &[&camera_layout, &particle_layout],
            push_constant_ranges: &[],
        });

        // 加法混合：src.rgb * src.a + dst.rgb * 1.0（适合火焰/火花）
        let additive_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                // 无 vertex buffer：4 顶点 quad 由 vertex_index 程序化生成
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(additive_blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // 不剔除（粒子小，剔除收益低且 billboard 朝向相机）
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                // 不写深度（粒子是半透明效果，不参与深度遮挡）
                depth_write_enabled: false,
                // LessEqual：与已写入的几何体共存
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            particle_buffer,
            max_particles,
            compute_pipeline,
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            uniform_layout,
            camera_layout,
            particle_layout,
            compute_bind_group,
            workgroup_count,
        }
    }

    /// 上传 ParticleUniform 到 GPU
    pub fn update(&self, queue: &Queue, uniform: &ParticleUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 执行 compute shader 更新粒子物理
    ///
    /// 在每帧 render pass 之前调用。dispatch 工作组数 = workgroup_count。
    pub fn dispatch(&self, encoder: &mut CommandEncoder) {
        if self.workgroup_count == 0 {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("particle compute pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.compute_bind_group, &[]);
        pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        pass.dispatch_workgroups(self.workgroup_count, 1, 1);
    }

    /// 在 render pass 中绘制所有粒子（billboard quad 实例化）
    ///
    /// # Arguments
    /// - `pass`: render pass（已 begin）
    /// - `camera_bind_group`: 外部 camera bind group，需与 `camera_layout` 兼容
    pub fn draw<'a>(
        &'a self,
        pass: &mut RenderPass<'a>,
        camera_bind_group: &'a BindGroup,
    ) {
        if self.max_particles == 0 {
            return;
        }
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        // 复用 compute_bind_group（particle_buffer 绑定到 particle_layout）
        pass.set_bind_group(1, &self.compute_bind_group, &[]);
        // 4 顶点 quad × max_particles 实例
        pass.draw(0..4, 0..self.max_particles);
    }

    /// 重置所有粒子为死亡状态（life=0）
    ///
    /// 保留每粒子的 seed（= index）以保证随机性差异。
    pub fn reset(&self, queue: &Queue) {
        if self.max_particles == 0 {
            return;
        }
        let dead: Vec<ParticleData> = (0..self.max_particles)
            .map(|i| ParticleData::dead(i as u32))
            .collect();
        queue.write_buffer(&self.particle_buffer, 0, bytemuck::cast_slice(&dead));
    }
}

// ============ 辅助函数 ============

/// 计算 dispatch 所需的工作组数：(count + 63) / 64
pub fn calc_workgroups(count: u32) -> u32 {
    (count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_data_size() {
        // 4 × vec4 = 4 × 16 = 64 bytes
        assert_eq!(std::mem::size_of::<ParticleData>(), 64);
    }

    #[test]
    fn particle_uniform_size() {
        // 4 × f32 + 4 × vec4 = 16 + 64 = 80 bytes
        assert_eq!(std::mem::size_of::<ParticleUniform>(), 80);
    }

    #[test]
    fn particle_data_default() {
        let p = ParticleData::default();
        // 默认死亡（life=0）
        assert_eq!(p.velocity[3], 0.0);
        // 默认 size=0.1
        assert!((p.position[3] - 0.1).abs() < 1e-6);
        // 默认 max_life=1.0
        assert!((p.spawn[0] - 1.0).abs() < 1e-6);
        // 默认白色不透明
        assert_eq!(p.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn particle_data_dead_with_seed() {
        let p = ParticleData::dead(42);
        assert_eq!(p.velocity[3], 0.0); // life=0
        assert!((p.spawn[2] - 42.0).abs() < 1e-6); // seed=42
    }

    #[test]
    fn particle_uniform_default_scene() {
        let u = ParticleUniform::default_scene();
        assert!((u.gravity - (-9.8)).abs() < 1e-6);
        assert!((u.drag - 0.1).abs() < 1e-6);
        assert!((u.delta_time - 0.016).abs() < 1e-6);
        assert_eq!(u.emitter_pos, [0.0, 0.0, 0.0, 0.5]);
        assert_eq!(u.emitter_vel[3], 10.0); // spawn_rate
    }

    #[test]
    fn particle_uniform_default_impl() {
        // Default trait impl 应全部为零
        let u = ParticleUniform::default();
        assert_eq!(u.delta_time, 0.0);
        assert_eq!(u.gravity, 0.0);
        assert_eq!(u.emitter_pos, [0.0; 4]);
        assert_eq!(u.wind, [0.0; 4]);
        assert_eq!(u._pad, [0.0; 4]);
    }

    #[test]
    fn workgroup_count_zero() {
        // max_particles = 0 → 0 工作组（不 dispatch）
        assert_eq!(calc_workgroups(0), 0);
    }

    #[test]
    fn workgroup_count_one() {
        // max_particles = 1 → 1 工作组
        assert_eq!(calc_workgroups(1), 1);
    }

    #[test]
    fn workgroup_count_exact_multiple() {
        // 64 粒子 = 1 工作组（恰好填满）
        assert_eq!(calc_workgroups(64), 1);
    }

    #[test]
    fn workgroup_count_just_over() {
        // 65 粒子 = 2 工作组（第二个组仅 1 个有效线程）
        assert_eq!(calc_workgroups(65), 2);
    }

    #[test]
    fn workgroup_count_large() {
        // 4096 粒子 = 64 工作组
        assert_eq!(calc_workgroups(4096), 64);
    }

    #[test]
    fn particle_data_field_offsets() {
        // 验证 #[repr(C)] 字段偏移量（与 WGSL vec4 布局一致）
        let p = ParticleData {
            position: [1.0, 2.0, 3.0, 4.0],
            velocity: [5.0, 6.0, 7.0, 8.0],
            color: [9.0, 10.0, 11.0, 12.0],
            spawn: [13.0, 14.0, 15.0, 16.0],
        };
        let base = &p as *const _ as usize;
        let pos_off = (&p.position as *const _ as usize) - base;
        let vel_off = (&p.velocity as *const _ as usize) - base;
        let col_off = (&p.color as *const _ as usize) - base;
        let spn_off = (&p.spawn as *const _ as usize) - base;
        assert_eq!(pos_off, 0);
        assert_eq!(vel_off, 16);
        assert_eq!(col_off, 32);
        assert_eq!(spn_off, 48);
    }

    #[test]
    fn particle_uniform_field_offsets() {
        // 验证字段偏移量与 WGSL uniform 布局一致
        let u = ParticleUniform::default_scene();
        let base = &u as *const _ as usize;
        let dt_off = (&u.delta_time as *const _ as usize) - base;
        let ep_off = (&u.emitter_pos as *const _ as usize) - base;
        let ev_off = (&u.emitter_vel as *const _ as usize) - base;
        let wind_off = (&u.wind as *const _ as usize) - base;
        let pad_off = (&u._pad as *const _ as usize) - base;
        // 4 × f32 = 16 bytes
        assert_eq!(dt_off, 0);
        // vec4 需要 16-byte 对齐，前 4×f32 恰好 16 bytes，无需填充
        assert_eq!(ep_off, 16);
        assert_eq!(ev_off, 32);
        assert_eq!(wind_off, 48);
        assert_eq!(pad_off, 64);
    }

    #[test]
    fn compute_shader_sanity() {
        // WGSL 关键元素检查
        assert!(COMPUTE_SHADER.contains("@compute"));
        assert!(COMPUTE_SHADER.contains("@workgroup_size(64)"));
        assert!(COMPUTE_SHADER.contains("cs_main"));
        assert!(COMPUTE_SHADER.contains("read_write")); // storage 可读写
        assert!(COMPUTE_SHADER.contains("arrayLength"));
        // 物理更新逻辑
        assert!(COMPUTE_SHADER.contains("p.velocity.w = p.velocity.w - dt"));
        assert!(COMPUTE_SHADER.contains("u.gravity"));
        assert!(COMPUTE_SHADER.contains("u.drag"));
        assert!(COMPUTE_SHADER.contains("u.wind"));
    }

    #[test]
    fn render_shader_sanity() {
        // WGSL 关键元素检查
        assert!(RENDER_SHADER.contains("@vertex"));
        assert!(RENDER_SHADER.contains("@fragment"));
        assert!(RENDER_SHADER.contains("vs_main"));
        assert!(RENDER_SHADER.contains("fs_main"));
        // billboard 技术：使用 view 矩阵的 right/up
        assert!(RENDER_SHADER.contains("camera.view[0].xyz"));
        assert!(RENDER_SHADER.contains("camera.view[1].xyz"));
        // 软粒子圆形衰减
        assert!(RENDER_SHADER.contains("smoothstep"));
        // 不使用 vec2 作为 vertex output（使用 uv_x/uv_y 标量）
        assert!(RENDER_SHADER.contains("uv_x"));
        assert!(RENDER_SHADER.contains("uv_y"));
        // 不允许 mat4x4 作为 vertex input/output - 验证 VertexOutput 无 mat4
        assert!(!RENDER_SHADER.contains("clip_position: mat4x4"));
    }

    #[test]
    fn render_shader_no_vec2_in_vertex_output() {
        // VertexOutput 不应包含 vec2（避免 8-byte 对齐问题）
        // 提取 VertexOutput 结构体部分检查
        let start = RENDER_SHADER.find("struct VertexOutput").unwrap();
        let end = RENDER_SHADER[start..].find("};").unwrap() + start + 2;
        let vertex_output = &RENDER_SHADER[start..end];
        assert!(
            !vertex_output.contains("vec2<f32>"),
            "VertexOutput 不应包含 vec2（使用两个 f32 标量代替以避免对齐问题）"
        );
    }
}
