//! Volumetric Fog Pass（port 自 v1 ae_render）
//!
//! 高度雾 + 距离雾 + god rays 近似
//! - 采样深度重建世界坐标
//! - 太阳方向调制 god rays
//! - 受化学浓度场影响雾颜色（对接物化生）
//!
//! 体积雾后处理，在 tonemap 前合成到 HDR texture：
//! 1. **高度雾**：基于世界坐标 Y 的指数衰减
//! 2. **距离雾**：基于相机距离的线性衰减
//! 3. **体积光照**：太阳光束效果（god rays 近似）
//! 4. **雾颜色**：根据太阳方向/颜色调制
//!
//! 使用全屏三角形（无需 vertex buffer），blend 模式为 REPLACE。
//! 通过采样深度纹理重建世界坐标，因此 depth view 应使用 `DepthOnly` aspect。
//! WGSL 不允许 mat4x4 作为 vertex input/output，所有矩阵通过 uniform 传递。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPipeline};

use crate::render_graph::passes::{NodeContext, NodeResult, RenderGraphNode};

/// 雾 Uniform（240 bytes = 2 × mat4x4 + 7 × vec4，符合 WGSL 16-byte 对齐）
///
/// 注意：WGSL shader 仅声明前 7 个字段（208 bytes），后两个字段（screen_size、_pad）
/// 为 nova_render 扩展，用于未来 RenderGraph 对接和物化生参数注入。WGSL 不会读取
/// 超出 uniform block 声明范围的字节，因此多出的 32 bytes 是安全的。
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct FogUniform {
    /// 相机 view-projection 矩阵（列主序）
    pub view_proj: [[f32; 4]; 4],
    /// 相机逆 view-projection 矩阵（用于从 NDC 重建世界坐标）
    pub view_inv: [[f32; 4]; 4],
    /// 相机世界坐标（xyz），w=1
    pub camera_pos: [f32; 4],
    /// 太阳方向（xyz，归一化）+ 强度（w）
    pub sun_dir: [f32; 4],
    /// 太阳颜色（rgb）+ 强度（w）
    pub sun_color: [f32; 4],
    /// 雾基础颜色（rgb）+ 不透明度（w）
    /// TODO: 由化学浓度场驱动
    pub fog_color: [f32; 4],
    /// 雾参数：x=density, y=height_falloff, z=near, w=far
    /// TODO: density 由能见度驱动，height_falloff 由温度驱动
    pub fog_params: [f32; 4],
    /// 屏幕尺寸（xy=width/height，zw=1/width/1/height），nova_render 扩展
    pub screen_size: [f32; 4],
    /// 填充字段，保持 16-byte 对齐
    pub _pad: [f32; 4],
}

impl FogUniform {
    /// 默认场景参数（低密度大气雾 + 暖色太阳）
    pub fn default_scene() -> Self {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Self {
            view_proj: identity,
            view_inv: identity,
            camera_pos: [0.0, 5.0, 0.0, 1.0],
            sun_dir: [0.5, -1.0, 0.3, 1.0],
            sun_color: [1.0, 0.95, 0.85, 1.0],
            fog_color: [0.6, 0.7, 0.85, 1.0],
            fog_params: [0.5, 0.05, 5.0, 100.0],
            screen_size: [1920.0, 1080.0, 1.0 / 1920.0, 1.0 / 1080.0],
            _pad: [0.0; 4],
        }
    }
}

const FOG_SHADER: &str = r#"
struct FogUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
};

@group(0) @binding(0) var t_hdr: texture_2d<f32>;
@group(0) @binding(1) var s_hdr: sampler;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_depth: sampler;
@group(0) @binding(4) var<uniform> u: FogUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    // 全屏三角形：3 个顶点覆盖 NDC 全屏，无需 vertex buffer
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

@fragment
fn fs_fog(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_hdr);
    let uv = pos.xy / vec2<f32>(dims);
    let scene_color = textureSample(t_hdr, s_hdr, uv).rgb;
    let depth_val = textureSample(t_depth, s_depth, uv).x;

    // 重建世界坐标：NDC -> world
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth_val * 2.0 - 1.0, 1.0);
    let world_pos_h = u.view_inv * ndc;
    let world_pos = world_pos_h.xyz / world_pos_h.w;

    // 高度雾：基于世界坐标 Y 的指数衰减
    let height_factor = exp(-max(world_pos.y, 0.0) * u.fog_params.y);

    // 距离雾：基于相机距离的线性衰减
    let dist = length(world_pos - u.camera_pos.xyz);
    let dist_factor = clamp((dist - u.fog_params.z) / (u.fog_params.w - u.fog_params.z), 0.0, 1.0);

    // 雾密度 = 基础密度 × 高度因子 × 距离因子
    let fog_density = u.fog_params.x * height_factor * dist_factor;

    // 体积光照（god rays 近似）：基于视线与太阳方向的夹角
    let view_dir = normalize(world_pos - u.camera_pos.xyz);
    let sun_dot_view = dot(view_dir, u.sun_dir.xyz);
    let god_rays = pow(max(sun_dot_view, 0.0), 8.0) * u.sun_dir.w * height_factor;

    // 雾颜色（融合太阳颜色贡献）
    let fog_color = u.fog_color.rgb + u.sun_color.rgb * god_rays * 0.5;

    // 混合场景颜色与雾颜色
    let result = mix(scene_color, fog_color, clamp(fog_density, 0.0, 1.0));

    return vec4<f32>(result, 1.0);
}
"#;

/// 体积雾渲染 Pass（port 自 v1 ae_render::VolumetricFogRenderer）
///
/// 使用方式：
/// ```ignore
/// let fog = VolumetricFogPass::new(&device, hdr_format, depth_format);
/// fog.update_uniform(&queue, &fog_uniform);
/// // 每帧：
/// let bg = fog.create_bind_group(&device, &hdr_view, &depth_view);
/// fog.draw(&mut pass, &bg);
/// ```
pub struct VolumetricFogPass {
    pub pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    /// 占位 bind group（使用 1x1 dummy 纹理）。每帧应使用 `create_bind_group` 创建实际 bind group。
    pub uniform_bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
    /// HDR 纹理 sampler（filtering）
    pub sampler: wgpu::Sampler,
    /// 深度纹理 sampler（non-filtering，Depth32Float 不支持 filtering）
    pub depth_sampler: wgpu::Sampler,
    // 占位纹理，保持 uniform_bind_group 中的引用有效
    _dummy_hdr: wgpu::Texture,
    _dummy_hdr_view: wgpu::TextureView,
    _dummy_depth: wgpu::Texture,
    _dummy_depth_view: wgpu::TextureView,
}

impl VolumetricFogPass {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // ---------- Bind group layout ----------
        // 0: HDR texture (filterable)
        // 1: HDR sampler (filtering)
        // 2: Depth texture (non-filterable; Depth32Float 不支持 filtering)
        // 3: Depth sampler (non-filtering)
        // 4: Uniform buffer
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("volumetric fog bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<FogUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Uniform buffer（使用 DeviceExt::create_buffer_init 初始化） ----------
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("volumetric fog uniform buffer"),
            contents: bytemuck::cast_slice(&[FogUniform::default_scene()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ---------- Samplers ----------
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("volumetric fog hdr sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("volumetric fog depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---------- Pipeline ----------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("volumetric fog shader"),
            source: wgpu::ShaderSource::Wgsl(FOG_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("volumetric fog pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volumetric fog pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_fog"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---------- Dummy textures for placeholder bind group ----------
        // 用于初始化 uniform_bind_group，保持引用有效
        let dummy_hdr = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("volumetric fog dummy hdr"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let dummy_hdr_view = dummy_hdr.create_view(&wgpu::TextureViewDescriptor::default());

        let dummy_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("volumetric fog dummy depth"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let dummy_depth_view = dummy_depth.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });

        // ---------- Placeholder bind group ----------
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("volumetric fog placeholder bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dummy_hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&dummy_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            bind_group_layout,
            sampler,
            depth_sampler,
            _dummy_hdr: dummy_hdr,
            _dummy_hdr_view: dummy_hdr_view,
            _dummy_depth: dummy_depth,
            _dummy_depth_view: dummy_depth_view,
        }
    }

    /// 更新雾参数 uniform
    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &FogUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 创建每帧 bind group（因为 hdr/depth texture 可能每帧变化）
    ///
    /// 注意：`depth_view` 应使用 `TextureAspect::DepthOnly` 创建，
    /// 以便作为 `texture_2d<f32>` 采样（Depth32Float 不支持 filtering）。
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("volumetric fog frame bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 渲染体积雾（在 tonemap 前，blend 到 HDR texture）
    ///
    /// 使用 `create_bind_group` 返回的 bind group 调用此方法。
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl RenderGraphNode for VolumetricFogPass {
    crate::impl_rgn_downcast!();

    fn name(&self) -> &str {
        "volumetric_fog"
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> NodeResult {
        // 需要 surface_view 作为 color attachment（合成到当前 HDR/swapchain）
        let color_view = match ctx.surface_view {
            Some(v) => v,
            None => {
                log::warn!("volumetric_fog: surface_view 缺失，跳过");
                return Ok(());
            }
        };

        // 注：当前使用占位 bind_group（内含 1x1 dummy 纹理），采样结果不正确。
        // 真实 RenderGraph 接入时应通过 ctx.inputs 取上游 HDR/depth view，
        // 再用 create_bind_group 创建每帧 bind_group。
        log::warn!(
            "volumetric_fog: 使用占位 bind_group（dummy HDR/depth），效果不正确"
        );

        // 写入默认场景 uniform（可被外部 update_uniform 覆盖）
        let (w, h) = ctx.surface_size;
        let mut u = FogUniform::default_scene();
        u.screen_size = [w as f32, h as f32, 1.0 / w.max(1) as f32, 1.0 / h.max(1) as f32];
        self.update_uniform(ctx.queue, &u);

        let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("volumetric fog render pass"),
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
        self.draw(&mut rpass, &self.uniform_bind_group);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        // 2 × mat4x4 (64) + 7 × vec4 (16) = 128 + 112 = 240 bytes
        // 注：WGSL shader 仅声明前 7 个字段（208 bytes），screen_size 和 _pad 为 nova_render 扩展
        assert_eq!(std::mem::size_of::<FogUniform>(), 240);
    }

    #[test]
    fn default_scene_params() {
        let u = FogUniform::default_scene();
        // fog_params: x=density, y=height_falloff, z=start, w=end
        assert!((u.fog_params[0] - 0.5).abs() < 1e-4, "density mismatch");
        assert!((u.fog_params[1] - 0.05).abs() < 1e-4, "height_falloff mismatch");
        assert!((u.fog_params[2] - 5.0).abs() < 1e-4, "start mismatch");
        assert!((u.fog_params[3] - 100.0).abs() < 1e-4, "end mismatch");
    }

    #[test]
    fn default_is_zero() {
        let u = FogUniform::default();
        assert_eq!(u.fog_params, [0.0; 4]);
        assert_eq!(u.camera_pos, [0.0; 4]);
        assert_eq!(u.sun_dir, [0.0; 4]);
        assert_eq!(u.view_proj, [[0.0; 4]; 4]);
        assert_eq!(u.screen_size, [0.0; 4]);
        assert_eq!(u._pad, [0.0; 4]);
    }

    #[test]
    fn uniform_layout_matches_wgsl() {
        // 验证字段偏移量与 WGSL 16-byte 对齐一致
        let u = FogUniform::default_scene();
        let base = &u as *const _ as usize;
        let view_proj = &u.view_proj as *const _ as usize - base;
        let view_inv = &u.view_inv as *const _ as usize - base;
        let camera_pos = &u.camera_pos as *const _ as usize - base;
        let sun_dir = &u.sun_dir as *const _ as usize - base;
        let sun_color = &u.sun_color as *const _ as usize - base;
        let fog_color = &u.fog_color as *const _ as usize - base;
        let fog_params = &u.fog_params as *const _ as usize - base;
        let screen_size = &u.screen_size as *const _ as usize - base;

        assert_eq!(view_proj, 0, "view_proj offset");
        assert_eq!(view_inv, 64, "view_inv offset");
        assert_eq!(camera_pos, 128, "camera_pos offset");
        assert_eq!(sun_dir, 144, "sun_dir offset");
        assert_eq!(sun_color, 160, "sun_color offset");
        assert_eq!(fog_color, 176, "fog_color offset");
        assert_eq!(fog_params, 192, "fog_params offset");
        assert_eq!(screen_size, 208, "screen_size offset");
    }
}