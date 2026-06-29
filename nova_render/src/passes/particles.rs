//! Particles Pass: GPU Particle System（port 自 v1 wasteland_render）
//!
//! compute-driven 粒子模拟 + billboard 渲染：
//! - ParticleData: 64 bytes/粒子（4 × vec4），存储在 storage buffer（read_write）
//! - Compute pipeline: @workgroup_size(64)，每线程处理一个粒子
//! - Render pipeline: 4 顶点 quad × N 实例，billboard 朝向相机
//! - 混合：加法混合（SrcAlpha + One），适合火焰/火花；不写深度，深度比较 LessEqual

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CommandEncoder, ComputePipeline, Device, Queue,
    RenderPass, RenderPipeline,
};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// Compute shader 工作组大小（每工作组处理 64 个粒子）
const WORKGROUP_SIZE: u32 = 64;

/// Particles Camera Uniform（256 bytes = 4 × mat4，与 v1 CameraUniform 兼容）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ParticleCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

/// 单个粒子数据（64 bytes = 4 × vec4）
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

/// 内嵌 WGSL Compute Shader（保留 v1 原样）
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

    p.velocity.w = p.velocity.w - dt;

    if (p.velocity.w <= 0.0) {
        let spawn_rate = u.emitter_vel.w;
        let respawn_prob = clamp(spawn_rate * dt, 0.0, 1.0);
        let rand = hash_f(p.spawn.z + u.time * 1.37);

        if (rand < respawn_prob) {
            let r1 = hash_f(p.spawn.z * 1.13 + u.time * 0.7);
            let r2 = hash_f(p.spawn.z * 2.27 + u.time * 1.3);
            let r3 = hash_f(p.spawn.z * 3.41 + u.time * 1.9);
            let r4 = hash_f(p.spawn.z * 4.55 + u.time * 2.5);

            let radius = u.emitter_pos.w;
            let offset = vec3<f32>(r1 - 0.5, r2 - 0.5, r3 - 0.5) * (2.0 * radius);
            p.position = vec4<f32>(u.emitter_pos.xyz + offset, p.position.w);

            let speed = 1.0 + r4 * 2.0;
            p.velocity = vec4<f32>(
                (r1 - 0.5) * speed,
                (0.5 + r2 * 0.5) * speed,
                (r3 - 0.5) * speed,
                max_life,
            );

            let flags = p.spawn.w;
            if (flags < 0.5) {
                p.color = vec4<f32>(0.8, 0.8, 0.8, 0.8);
            } else if (flags < 1.5) {
                p.color = vec4<f32>(1.0, 0.5, 0.1, 0.9);
            } else if (flags < 2.5) {
                p.color = vec4<f32>(0.3, 0.3, 0.3, 0.4);
            } else if (flags < 3.5) {
                p.color = vec4<f32>(1.0, 0.85, 0.3, 1.0);
            } else {
                p.color = vec4<f32>(0.7, 0.6, 0.5, 0.5);
            }
        } else {
            p.velocity.w = 0.0;
        }
    } else {
        p.position.xyz = p.position.xyz + p.velocity.xyz * dt;
        p.velocity.y = p.velocity.y + u.gravity * dt;
        let drag_factor = max(1.0 - u.drag * dt, 0.0);
        p.velocity.xyz = p.velocity.xyz * drag_factor;
        p.velocity.xyz = p.velocity.xyz + u.wind.xyz * (u.wind.w * dt);
    }

    particles[idx] = p;
}
"#;

/// 内嵌 WGSL Render Shader（保留 v1 原样）
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

    var offsets_x = array<f32, 4>(-1.0, 1.0, -1.0, 1.0);
    var offsets_y = array<f32, 4>(-1.0, -1.0, 1.0, 1.0);
    var uvs_x = array<f32, 4>(0.0, 1.0, 0.0, 1.0);
    var uvs_y = array<f32, 4>(0.0, 0.0, 1.0, 1.0);

    let ox = offsets_x[vid];
    let oy = offsets_y[vid];

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
    let uv = vec2<f32>(in.uv_x, in.uv_y);

    let d = length(uv - vec2<f32>(0.5, 0.5)) * 2.0;
    let soft_alpha = smoothstep(1.0, 0.0, d);

    let life_alpha = in.life_ratio;

    var color = in.color;

    let flags = in.flags;
    if (flags < 0.5) {
    } else if (flags < 1.5) {
        color.rgb = color.rgb * (1.5 + in.life_ratio * 0.5);
    } else if (flags < 2.5) {
        color.rgb = color.rgb * (0.4 + in.life_ratio * 0.6);
    } else if (flags < 3.5) {
        color.rgb = color.rgb * 2.5;
    } else {
        color.rgb = color.rgb * 0.8;
    }

    let alpha = color.a * soft_alpha * life_alpha;
    return vec4<f32>(color.rgb, alpha);
}
"#;

/// ParticlePass: GPU 粒子系统节点
pub struct ParticlePass {
    pub particle_buffer: Buffer,
    pub max_particles: u32,
    pub compute_pipeline: ComputePipeline,
    pub render_pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub uniform_bind_group: BindGroup,
    pub uniform_layout: BindGroupLayout,
    pub camera_layout: BindGroupLayout,
    pub particle_layout: BindGroupLayout,
    pub compute_bind_group: BindGroup,
    pub workgroup_count: u32,
    /// 摄像机 uniform buffer（lazy 初始化，首次 execute 时创建）
    pub camera_buffer: Option<Buffer>,
    /// 摄像机 bind group（lazy 初始化，首次 execute 时创建）
    pub camera_bind_group: Option<BindGroup>,
}

impl ParticlePass {
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_particles: u32,
    ) -> Self {
        let safe_count = max_particles.max(1);
        let workgroup_count = (max_particles + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

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

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ParticleCameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

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

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle render shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle render pipeline layout"),
            bind_group_layouts: &[&camera_layout, &particle_layout],
            push_constant_ranges: &[],
        });

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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false,
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
            camera_buffer: None,
            camera_bind_group: None,
        }
    }

    pub fn update(&self, queue: &Queue, uniform: &ParticleUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

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

    pub fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        if self.max_particles == 0 {
            return;
        }
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_bind_group(1, &self.compute_bind_group, &[]);
        pass.draw(0..4, 0..self.max_particles);
    }

    pub fn reset(&self, queue: &Queue) {
        if self.max_particles == 0 {
            return;
        }
        let dead: Vec<ParticleData> = (0..self.max_particles)
            .map(|i| ParticleData::dead(i as u32))
            .collect();
        queue.write_buffer(&self.particle_buffer, 0, bytemuck::cast_slice(&dead));
    }

    /// Lazy 创建摄像机 uniform buffer + bind group（首次 execute 时调用）
    pub fn ensure_camera(&mut self, device: &Device) {
        if self.camera_buffer.is_some() && self.camera_bind_group.is_some() {
            return;
        }
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle camera buffer"),
            size: std::mem::size_of::<ParticleCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle camera bind group"),
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        self.camera_buffer = Some(camera_buffer);
        self.camera_bind_group = Some(camera_bind_group);
    }

    /// 写入摄像机 uniform（默认场景：原点后方观察，朝 -Z 看）
    pub fn update_camera(&self, queue: &Queue, camera: &ParticleCameraUniform) {
        if let Some(buf) = &self.camera_buffer {
            queue.write_buffer(buf, 0, bytemuck::cast_slice(&[*camera]));
        }
    }

    /// 默认摄像机：绕 Y 轴缓慢旋转，位置 (0, 5, 10) 朝向原点
    pub fn default_camera(time: f32, aspect: f32) -> ParticleCameraUniform {
        let angle = time * 0.3;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let eye = [cos_a * 10.0, 5.0, sin_a * 10.0];
        let target = [0.0, 0.0, 0.0];
        let up = [0.0, 1.0, 0.0];

        // view = look_at(eye, target, up)
        let f = [
            target[0] - eye[0],
            target[1] - eye[1],
            target[2] - eye[2],
        ];
        let fl = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-6);
        let f = [f[0] / fl, f[1] / fl, f[2] / fl];
        let s = [
            up[1] * f[2] - up[2] * f[1],
            up[2] * f[0] - up[0] * f[2],
            up[0] * f[1] - up[1] * f[0],
        ];
        let sl = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1e-6);
        let s = [s[0] / sl, s[1] / sl, s[2] / sl];
        let u = [
            f[1] * s[2] - f[2] * s[1],
            f[2] * s[0] - f[0] * s[2],
            f[0] * s[1] - f[1] * s[0],
        ];

        let view = [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [
                -(s[0] * eye[0] + s[1] * eye[1] + s[2] * eye[2]),
                -(u[0] * eye[0] + u[1] * eye[1] + u[2] * eye[2]),
                f[0] * eye[0] + f[1] * eye[1] + f[2] * eye[2],
                1.0,
            ],
        ];

        // proj = perspective(fov=60°, aspect, near=0.1, far=100)
        let fov = 60.0f32 * std::f32::consts::PI / 180.0;
        let f = 1.0 / (fov / 2.0).tan();
        let near = 0.1f32;
        let far = 100.0f32;
        let proj = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) / (near - far), -1.0],
            [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
        ];

        // view_proj = proj * view
        let view_proj = mat4_mul(&proj, &view);

        ParticleCameraUniform {
            view_proj,
            view,
            proj,
            position: [eye[0], eye[1], eye[2], 1.0],
        }
    }
}

impl Default for ParticlePass {
    fn default() -> Self {
        unreachable!("ParticlePass requires device + format + count arguments");
    }
}

/// 计算 dispatch 所需的工作组数：(count + 63) / 64
pub fn calc_workgroups(count: u32) -> u32 {
    (count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE
}

impl RenderGraphNode for ParticlePass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "particles"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        if self.max_particles == 0 || self.workgroup_count == 0 {
            return Ok(());
        }

        // 1. 更新粒子 uniform（注入时间 + 默认场景参数）
        let mut u = ParticleUniform::default_scene();
        u.time = ctx.time;
        u.delta_time = 0.016;
        self.update(ctx.queue, &u);

        // 2. Compute pass：更新粒子物理状态
        self.dispatch(ctx.encoder);

        // 3. Lazy 创建摄像机资源 + 写入默认摄像机（绕 Y 轴缓慢旋转）
        self.ensure_camera(ctx.device);
        let (w, h) = ctx.surface_size;
        let aspect = w as f32 / h.max(1) as f32;
        let cam = Self::default_camera(ctx.time, aspect);
        self.update_camera(ctx.queue, &cam);

        // 4. Render pass：billboard 绘制粒子（Load 保留背景）
        let color_view = match ctx.surface_view {
            Some(v) => v,
            None => {
                log::warn!("particles: surface_view 缺失，跳过 render pass");
                return Ok(());
            }
        };
        let camera_bg = match &self.camera_bind_group {
            Some(bg) => bg,
            None => {
                log::warn!("particles: camera_bind_group 初始化失败，跳过 render pass");
                return Ok(());
            }
        };

        // 创建临时 depth texture —— render pipeline 创建时带 depth_stencil（LessEqual），
        // render pass 必须提供匹配的 depth attachment，否则 wgpu 校验失败。
        // [假设] depth_format = Depth32Float（nova_render 约定，与 forward / skybox 一致）。
        // [性能提示] 应复用上游 pass 的 depth view，而非每帧重建。
        let depth_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("particle temp depth texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("particles render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
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
        self.draw(&mut rpass, camera_bg);
        Ok(())
    }
}

/// 4x4 矩阵乘法（手写，避免引入 nalgebra 依赖）
fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[i][k] * b[k][j];
            }
            r[i][j] = sum;
        }
    }
    r
}
