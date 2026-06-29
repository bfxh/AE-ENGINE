//! Volumetric Fire Pass - 火焰体积渲染
//!
//! 基于 ray marching + 黑体色温映射（Tanner Helland 算法）
//! 输入：
//! - 火焰密度 3D 纹理（来自 wasteland_compute::StamFluidSolver3D::density）
//! - 温度 3D 纹理（来自 StamFluidSolver3D::temperature）
//! 输出：
//! - 合成到 HDR 缓冲的火焰颜色（含发光）
//!
//! 物理：
//! - Beer-Lambert 透明度衰减：T = exp(-σ·d)
//! - 黑体辐射色温：T_kelvin → RGB
//! - 累积发光：L = Σ(density · blackbody(T) · emission · T_transmittance)

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// 火焰 Uniform（96 bytes = 2×mat4x4 + 4×vec4）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct FireUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view_inv: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub grid_origin: [f32; 4],
    pub grid_scale: [f32; 4],
    pub fire_params: [f32; 4],
}

impl FireUniform {
    pub fn default_scene() -> Self {
        let identity = [[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]];
        Self {
            view_proj: identity, view_inv: identity,
            camera_pos: [0.0, 1.0, 3.0, 1.0],
            grid_origin: [-1.0, 0.0, -1.0, 0.0],
            grid_scale: [0.1, 0.1, 0.1, 32.0],
            fire_params: [0.05, 64.0, 1.5, 1.0],
        }
    }
}
const FIRE_SHADER: &str = r#"
struct FireUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    camera_pos: vec4<f32>,
    grid_origin: vec4<f32>,
    grid_scale: vec4<f32>,
    fire_params: vec4<f32>,
};

@group(0) @binding(0) var t_hdr: texture_2d<f32>;
@group(0) @binding(1) var s_hdr: sampler;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_depth: sampler;
@group(0) @binding(4) var t_density: texture_3d<f32>;
@group(0) @binding(5) var s_density: sampler;
@group(0) @binding(6) var t_temp: texture_3d<f32>;
@group(0) @binding(7) var s_temp: sampler;
@group(0) @binding(8) var<uniform> u: FireUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

fn blackbody_rgb(t: f32) -> vec3<f32> {
    let tc = clamp(t / 100.0, 10.0, 400.0);
    var r = 255.0;
    if (tc > 66.0) {
        r = 329.698727446 * pow(tc - 60.0, vec3<f32>(-0.1332047592, 0.0, 0.0).x);
    }
    var g = 0.0;
    if (tc <= 66.0) {
        g = 99.4708025861 * log(tc) - 161.1195681661;
    } else {
        g = 288.1221695283 * pow(tc - 60.0, vec3<f32>(-0.0755148492, 0.0, 0.0).x);
    }
    var b = 255.0;
    if (tc < 66.0) {
        if (tc <= 19.0) {
            b = 0.0;
        } else {
            b = 138.5177312231 * log(tc - 10.0) - 305.0447927307;
        }
    }
    return vec3<f32>(clamp(r, 0.0, 255.0) / 255.0, clamp(g, 0.0, 255.0) / 255.0, clamp(b, 0.0, 255.0) / 255.0);
}

@fragment
fn fs_fire(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_hdr);
    let uv = pos.xy / vec2<f32>(dims);
    let scene_color = textureSample(t_hdr, s_hdr, uv).rgb;
    let depth_val = textureSample(t_depth, s_depth, uv).x;

    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth_val * 2.0 - 1.0, 1.0);
    let world_pos_h = u.view_inv * ndc;
    let world_pos = world_pos_h.xyz / world_pos_h.w;

    let ray_origin = u.camera_pos.xyz;
    let ray_dir = normalize(world_pos - ray_origin);

    var accumulated_color = vec3<f32>(0.0);
    var transmittance = 1.0;
    let step_size = u.fire_params.x;
    let max_steps = u32(u.fire_params.y);
    let emission = u.fire_params.z;
    let density_scale = u.fire_params.w;

    for (var i = 0u; i < max_steps; i = i + 1u) {
        let t = f32(i) * step_size;
        let step_pos = ray_origin + ray_dir * t;
        let grid_pos = (step_pos - u.grid_origin.xyz) / u.grid_scale.xyz;

        if (grid_pos.x < 0.0 || grid_pos.x > 1.0 ||
            grid_pos.y < 0.0 || grid_pos.y > 1.0 ||
            grid_pos.z < 0.0 || grid_pos.z > 1.0) {
            continue;
        }

        let density = textureSample(t_density, s_density, grid_pos).x * density_scale;
        if (density < 0.01) {
            continue;
        }

        let temperature = textureSample(t_temp, s_temp, grid_pos).x;
        let fire_color = blackbody_rgb(temperature);

        let absorption = density * step_size;
        transmittance = transmittance * exp(-absorption);
        accumulated_color = accumulated_color + fire_color * density * emission * transmittance * step_size;

        if (transmittance < 0.01) {
            break;
        }
    }

    let result = scene_color * transmittance + accumulated_color;
    return vec4<f32>(result, 1.0);
}
"#;
pub struct VolumetricFirePass {
    pub pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub uniform_bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl VolumetricFirePass {
    pub fn new(device: &wgpu::Device, hdr_format: wgpu::TextureFormat) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VolumetricFire uniform"),
            contents: bytemuck::cast_slice(&[FireUniform::default_scene()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("VolumetricFire sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VolumetricFire layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 7, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 8, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: std::num::NonZeroU64::new(96) }, count: None },
            ],
        });
        let dummy_2d = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy 2d"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: hdr_format, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let dummy_2d_view = dummy_2d.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_3d = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy 3d"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R32Float, usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_3d_view = dummy_3d.create_view(&wgpu::TextureViewDescriptor::default());

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VolumetricFire uniform bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&dummy_2d_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&dummy_2d_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&dummy_3d_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&dummy_3d_view) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()) },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VolumetricFire shader"),
            source: wgpu::ShaderSource::Wgsl(FIRE_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VolumetricFire pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VolumetricFire pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_fullscreen"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_fire"),
                targets: &[Some(wgpu::ColorTargetState { format: hdr_format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline, uniform_buffer, uniform_bind_group, bind_group_layout, sampler }
    }
    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &FireUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn create_bind_group(&self, device: &wgpu::Device, hdr_view: &wgpu::TextureView, depth_view: &wgpu::TextureView, density_3d: &wgpu::TextureView, temp_3d: &wgpu::TextureView) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VolumetricFire frame bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(hdr_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(depth_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(density_3d) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(temp_3d) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::Buffer(self.uniform_buffer.as_entire_buffer_binding()) },
            ],
        })
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass, bind_group: &BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl RenderGraphNode for VolumetricFirePass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str { "volumetric_fire" }

    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 输出目标：写入 swapchain（fire 合成到当前场景之上，LoadOp::Load 保留背景）
        let color_view = match ctx.surface_view {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fire: 缺少 surface_view，跳过");
                return Ok(());
            }
        };

        // 输入纹理约定：
        //   inputs[0]=hdr/color（场景颜色，shader 中作 t_hdr 采样），
        //   inputs[1]=depth（场景深度），
        //   inputs[2]=density_3d（火焰密度 3D 纹理），
        //   inputs[3]=temp_3d（温度 3D 纹理）。
        // 不使用 self.uniform_bind_group（dummy，会采样 1x1 占位纹理导致画面变黑），
        // 而是每帧用真实输入构建 frame bind_group。
        if ctx.inputs.len() < 4 {
            log::warn!(
                "volumetric_fire: inputs 不足（{}，需要 4: color/depth/density_3d/temp_3d），跳过",
                ctx.inputs.len()
            );
            return Ok(());
        }
        let hdr_view = match ctx.resources.get_texture(ctx.inputs[0]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fire: hdr/color texture 缺失，跳过");
                return Ok(());
            }
        };
        let depth_view = match ctx.resources.get_texture(ctx.inputs[1]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fire: depth texture 缺失，跳过");
                return Ok(());
            }
        };
        let density_3d = match ctx.resources.get_texture(ctx.inputs[2]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fire: density_3d texture 缺失，跳过");
                return Ok(());
            }
        };
        let temp_3d = match ctx.resources.get_texture(ctx.inputs[3]).cloned() {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fire: temp_3d texture 缺失，跳过");
                return Ok(());
            }
        };

        let bg = self.create_bind_group(ctx.device, &hdr_view, &depth_view, &density_3d, &temp_3d);

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("volumetric_fire render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.draw(&mut rpass, &bg);
        Ok(())
    }
}