//! Skybox Pass: 程序化天空盒 + 大气散射（port 自 v1 ae_render）
//!
//! 基于 Rayleigh + Mie 散射的实时大气散射，支持太阳/月亮方向与 HDR 输出。
//! - 天空穹顶球体（半径 1.0，仅渲染内表面）
//! - 顶点位置即世界空间方向（不应用 view 矩阵的平移）
//! - 深度设为最大（clip_pos.z = clip_pos.w），depth_compare = LessEqual，不写深度
//! - 输出 HDR 值（>1.0），由后处理管线做 tone mapping

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, Device, Queue, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// Skybox Camera Uniform（256 bytes = 4 × mat4，与 v1 CameraUniform 兼容）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SkyCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

/// 太阳/月亮 uniform（32 bytes = 两个 vec4，满足 WGSL uniform 16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SunUniform {
    pub direction: [f32; 4],
    pub color: [f32; 4],
}

/// 天空穹顶顶点：仅需位置（12 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SkyVertex {
    pub position: [f32; 3],
}

/// 内嵌 WGSL shader：Rayleigh + Mie 大气散射（保留 v1 原样）
const SKYBOX_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

struct SunUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> sun: SunUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_dir: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position;
    // 去除 view 矩阵的平移部分（让天空始终在远处）
    let view_no_translate = mat3x3<f32>(
        camera.view[0].xyz,
        camera.view[1].xyz,
        camera.view[2].xyz,
    );
    let view_pos = view_no_translate * world_pos;
    out.clip_position = camera.proj * vec4<f32>(view_pos, 1.0);
    // 深度设为最大（让天空盒始终在最远处）
    out.clip_position.z = out.clip_position.w;
    out.world_dir = normalize(in.position);
    return out;
}

// Rayleigh 散射系数（蓝色主导）
const RAYLEIGH_BETA: vec3<f32> = vec3<f32>(5.8e-6, 1.35e-5, 3.31e-5);
// Mie 散射系数
const MIE_BETA: vec3<f32> = vec3<f32>(2.1e-6, 2.1e-6, 2.1e-6);
// Rayleigh 散射高度比例
const RAYLEIGH_SCALE_HEIGHT: f32 = 0.25;
// Mie 散射高度比例
const MIE_SCALE_HEIGHT: f32 = 0.1;
// Mie 相位函数常数（g）
const MIE_G: f32 = 0.758;

// Rayleigh 相位函数
fn rayleigh_phase(cos_theta: f32) -> f32 {
    return 0.05968310365 * (1.0 + cos_theta * cos_theta);
}

// Mie 相位函数（Henyey-Greenstein）
fn mie_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let num = (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return 0.11936620729 * num / (denom * sqrt(max(denom, 0.0001)));
}

// 简单哈希噪声（用于夜空星星）
fn hash13(p: vec3<f32>) -> f32 {
    var v = p;
    v = v * vec3<f32>(0.1031, 0.1030, 0.0973);
    v.x = v.x + dot(v, v.yzx + 33.33);
    return fract((v.x + v.y) * v.z);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_dir);
    let sun_dir = normalize(sun.direction.xyz);
    let sun_color = sun.color.rgb;
    let sun_intensity = sun.direction.w;

    let cos_zenith = max(dir.y, 0.0);

    // 大气光学深度（简化）
    let optical_depth = 1.0 / (cos_zenith + 0.15 * pow(1.0 - cos_zenith, 4.0));

    // 太阳与视线方向的角度
    let cos_sun = dot(dir, sun_dir);
    let sun_zenith = sun_dir.y;

    // ============ 散射相位 ============
    let r_phase = rayleigh_phase(cos_sun);
    let m_phase = mie_phase(cos_sun, MIE_G);

    let r_scatter = RAYLEIGH_BETA * RAYLEIGH_SCALE_HEIGHT * optical_depth * r_phase;
    let m_scatter = MIE_BETA * MIE_SCALE_HEIGHT * optical_depth * m_phase;

    // 太阳在水平线以下时减少散射
    let sun_factor = smoothstep(-0.1, 0.2, sun_zenith);

    // ============ 天空颜色调色板（HDR） ============
    let day_sky = vec3<f32>(0.3, 0.5, 2.0) * 2.0;
    let sunset_color = vec3<f32>(1.0, 0.4, 0.2) * 3.0;
    let night_sky = vec3<f32>(0.02, 0.02, 0.05) * 1.0;

    // 地平线渐变（近地平线偏白/橙）
    let horizon_factor = pow(1.0 - cos_zenith, 2.0);
    let sunset_blend = smoothstep(0.0, 0.3, sun_factor) * (1.0 - sun_zenith);
    let horizon_color = mix(day_sky, sunset_color, sunset_blend);

    let sky_base = mix(day_sky, horizon_color, horizon_factor);

    // 应用散射
    var color = sky_base * (1.0 + (r_scatter + m_scatter) * 1000.0 * sun_intensity * sun_factor);

    // 太阳亮斑（Mie 散射强光）
    let sun_disk = pow(max(cos_sun, 0.0), 256.0);
    let sun_glow = pow(max(cos_sun, 0.0), 8.0) * 0.5;
    let sun_light = (sun_disk * 100.0 + sun_glow * 2.0) * sun_color * sun_intensity * sun_factor;
    color = color + sun_light;

    // 夜晚过渡
    let night_factor = 1.0 - smoothstep(-0.1, 0.15, sun_zenith);
    color = mix(color, night_sky, night_factor * 0.7);

    // 夜空星星（基于哈希噪声）
    if (night_factor > 0.5) {
        let star_grid = floor(dir * 200.0);
        let star_hash = hash13(star_grid);
        if (star_hash > 0.995) {
            let star_brightness = (star_hash - 0.995) / 0.005;
            color = color + vec3<f32>(star_brightness, star_brightness, star_brightness * 1.1) * night_factor;
        }
    }

    return vec4<f32>(color, 1.0);
}
"#;

/// 生成天空穹顶球体（半径 1.0，绕序从球外看 CCW，配合 cull_mode=Front 渲染内表面）
fn generate_sky_dome(segments: u32, rings: u32) -> (Vec<SkyVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let theta = std::f32::consts::PI * (ring as f32 / rings as f32);
        let sin_t = theta.sin();
        let cos_t = theta.cos();
        for seg in 0..=segments {
            let phi = 2.0 * std::f32::consts::PI * (seg as f32 / segments as f32);
            let x = sin_t * phi.cos();
            let y = cos_t;
            let z = sin_t * phi.sin();
            vertices.push(SkyVertex { position: [x, y, z] });
        }
    }

    let cols = segments + 1;
    for ring in 0..rings {
        for seg in 0..segments {
            let a = ring * cols + seg;
            let b = a + 1;
            let c = a + cols;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c]);
            indices.extend_from_slice(&[b, d, c]);
        }
    }

    (vertices, indices)
}

/// SkyboxPass: 程序化天空盒渲染节点
pub struct SkyboxPass {
    pub pipeline: RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub sun_buffer: Buffer,
    pub sun_bind_group: BindGroup,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    /// 自管理的 depth texture（与 surface 尺寸匹配，lazy 创建/resize）
    pub depth_texture: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
    pub depth_size: (u32, u32),
    /// depth format（pipeline 创建时确定）
    pub depth_format: wgpu::TextureFormat,
}

impl SkyboxPass {
    pub fn new(
        device: &Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skybox camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SkyCameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skybox camera buffer"),
            size: std::mem::size_of::<SkyCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skybox camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let sun_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skybox sun layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SunUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let sun_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skybox sun buffer"),
            size: std::mem::size_of::<SunUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sun_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skybox sun bind group"),
            layout: &sun_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sun_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skybox pipeline layout"),
            bind_group_layouts: &[&camera_layout, &sun_layout],
            push_constant_ranges: &[],
        });

        let (vertices, indices) = generate_sky_dome(32, 16);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky dome vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky dome index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let index_count = indices.len() as u32;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skybox shader"),
            source: wgpu::ShaderSource::Wgsl(SKYBOX_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skybox pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
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
                cull_mode: Some(wgpu::Face::Front),
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
            pipeline,
            camera_buffer,
            camera_bind_group,
            sun_buffer,
            sun_bind_group,
            vertex_buffer,
            index_buffer,
            index_count,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            depth_format,
        }
    }

    pub fn update_camera(&self, queue: &Queue, uniform: &SkyCameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn update_sun(
        &self,
        queue: &Queue,
        sun_dir: [f32; 3],
        sun_color: [f32; 3],
        sun_intensity: f32,
    ) {
        let len = (sun_dir[0] * sun_dir[0] + sun_dir[1] * sun_dir[1] + sun_dir[2] * sun_dir[2]).sqrt();
        let dir = if len > 1e-10 {
            [sun_dir[0] / len, sun_dir[1] / len, sun_dir[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };
        let uniform = SunUniform {
            direction: [dir[0], dir[1], dir[2], sun_intensity],
            color: [sun_color[0], sun_color[1], sun_color[2], 0.0],
        };
        queue.write_buffer(&self.sun_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// 确保 depth texture 与目标尺寸匹配（不匹配则重建）
    ///
    /// 由 `execute` 在每帧调用；尺寸不变时直接复用。
    pub fn ensure_depth(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        let w = w.max(1);
        let h = h.max(1);
        if self.depth_size == (w, h) && self.depth_view.is_some() {
            return;
        }
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skybox depth texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.depth_texture = Some(tex);
        self.depth_view = Some(view);
        self.depth_size = (w, h);
    }

    /// 构造默认相机 uniform：绕 Y 轴缓慢旋转 + 60° 透视投影
    ///
    /// 用于 RenderGraph 端到端 MVP（无外部相机驱动时）。`time` 推动旋转。
    pub fn default_camera(time: f32, aspect: f32) -> SkyCameraUniform {
        let theta = time * 0.1;
        let (c, s) = (theta.cos(), theta.sin());
        // rotation_y(theta)
        let view = [
            [c, 0.0, s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        // perspective_vulkan(fov=60°, aspect, near=0.1, far=100) — z clip in [0,1]
        let fov = 60f32.to_radians();
        let f = 1.0 / (fov / 2.0).tan();
        let near = 0.1f32;
        let far = 100.0f32;
        let proj = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), near * far / (near - far)],
            [0.0, 0.0, -1.0, 0.0],
        ];
        let view_proj = mat4_mul(&proj, &view);
        SkyCameraUniform {
            view_proj,
            view,
            proj,
            position: [0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.sun_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

impl Default for SkyboxPass {
    fn default() -> Self {
        unreachable!("SkyboxPass requires device + format arguments");
    }
}

impl RenderGraphNode for SkyboxPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "skybox"
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 1. 确保深度资源与 surface 尺寸匹配
        let (w, h) = ctx.surface_size;
        self.ensure_depth(ctx.device, w, h);

        // 2. 写入默认相机（绕 Y 轴缓慢旋转）+ 默认太阳
        let aspect = w as f32 / h.max(1) as f32;
        let cam = Self::default_camera(ctx.time, aspect);
        self.update_camera(ctx.queue, &cam);
        self.update_sun(ctx.queue, [0.3, 0.5, 0.8], [1.0, 0.95, 0.85], 8.0);

        // 3. 拿 surface view（color）+ depth view（clone 到局部变量避免 borrow 冲突）
        let color_view = ctx
            .surface_view
            .ok_or_else(|| anyhow::anyhow!("SkyboxPass::execute missing surface_view"))?;
        let depth_view = self
            .depth_view
            .clone()
            .ok_or_else(|| anyhow::anyhow!("SkyboxPass::execute depth_view not initialized"))?;

        // 4. begin_render_pass：color 清屏为深夜空色，depth 清为 1.0
        let mut render_pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("skybox render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.03,
                        b: 0.05,
                        a: 1.0,
                    }),
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

        // 5. 录制 draw 命令
        self.draw(&mut render_pass);
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
