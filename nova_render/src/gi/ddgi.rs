//! DDGI (Dynamic Diffuse Global Illumination)
//!
//! 基于 Majercik et al. 2019 ("Fast Gradient-Domain Multiplication for
//! Integral Illumination") 与 Zsolnai-Kalos 2019 DDGI 方案。
//!
//! 简化实现：
//! - 默认 4x4x4 = 64 个 probe（可通过 `with_grid` 自定义）
//! - 每帧 round-robin 更新 `PROBES_PER_FRAME` 个 probe
//! - 更新（compute）：从 probe 出发沿球面方向采样场景颜色，写入 irradiance 3D 纹理
//! - 渲染（fragment）：根据像素 world position 三线性插值 8 个邻居 probe 的 irradiance
//!
//! 资源布局：
//! - irradiance_texture: 3D Rgba8Unorm (8x8xprobe_count) — storage write + sample
//! - depth_texture: 2D array Rgba8Unorm (14x14xprobe_count) — 初始化一次，fragment 采样
//! - probe_buffer: storage buffer (probe_count x ProbeData)
//! - uniform_buffer: DdgiUniform (224 bytes)
//!
//! 简化点（相对完整 DDGI）：
//! 1. 不做完整 ray-march，仅在 probe 周围球面采样一次场景颜色
//! 2. depth_texture 仅初始化为 1.0（最远），不参与遮挡测试
//! 3. 渲染时不使用场景深度，用 NDC z=0.5 估算 world position
//! 4. 每个 probe 的 irradiance 仅 8x8 = 64 个方向

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, ComputePipeline, RenderPipeline, Sampler, Texture,
    TextureView,
};

use crate::scene::Camera;

// ============ 常量 ============

/// 默认 probe 网格维度
const DEFAULT_GRID: [u32; 3] = [4, 4, 4];
/// Irradiance 纹理边长（每 probe 8x8 texel）
const IRRADIANCE_TEX_SIZE: u32 = 8;
/// Depth 纹理边长（每 probe 14x14 texel）
const DEPTH_TEX_SIZE: u32 = 14;
/// 每帧更新的 probe 数量
const PROBES_PER_FRAME: u32 = 4;
/// 默认 probe 影响半径
const DEFAULT_PROBE_RADIUS: f32 = 2.0;
/// 默认 irradiance 强度
const DEFAULT_INTENSITY: f32 = 1.0;

// ============ Uniform ============

/// DDGI Uniform（224 bytes = 2 x mat4x4 + 6 x vec4，16-byte 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct DdgiUniform {
    /// 当前帧 view-projection
    pub view_proj: [[f32; 4]; 4],
    /// 逆 view-projection（用于 NDC -> world 重建）
    pub view_inv: [[f32; 4]; 4],
    /// 网格起点 xyz，w=unused
    pub grid_origin: [f32; 4],
    /// 网格间距 xyz，w=unused
    pub grid_spacing: [f32; 4],
    /// 网格维度 xyz，w=probe_count
    pub grid_count: [f32; 4],
    /// x=probe_radius, y=intensity, z=depth_sharpness, w=irradiance_res
    pub params: [f32; 4],
    /// x=width, y=height, z=1/width, w=1/height
    pub screen_size: [f32; 4],
    /// x=frame_index, y=probe_update_offset, zw=pad
    pub frame_index: [f32; 4],
}

// ============ Probe 数据 ============

/// 单个 probe 数据（16 bytes，std140 对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ProbeData {
    /// probe 世界坐标 xyz，w=1.0
    pub position: [f32; 4],
}

// ============ Scene trait ============

/// DDGI 更新所需的场景数据
///
/// 调用方实现此 trait 提供 GPU 资源；默认实现返回 None，
/// 此时 DDGI 将退化为只推进 probe 更新偏移（不采样场景）。
pub trait DdgiScene {
    /// 场景深度纹理 view（用于 ray-march 时的遮挡测试）
    fn depth_view(&self) -> Option<&TextureView> {
        None
    }
    /// 场景颜色纹理 view（HDR，用于采样 irradiance）
    fn color_view(&self) -> Option<&TextureView> {
        None
    }
    /// 当前帧 view-projection 矩阵
    fn view_proj(&self) -> [[f32; 4]; 4] {
        [[0.0; 4]; 4]
    }
    /// 当前帧逆 view-projection 矩阵
    fn view_inv(&self) -> [[f32; 4]; 4] {
        [[0.0; 4]; 4]
    }
    /// 屏幕尺寸 [width, height]
    fn screen_size(&self) -> [f32; 2] {
        [1.0, 1.0]
    }
}

// ============ Ddgi 主体 ============

/// DDGI (Dynamic Diffuse Global Illumination)
///
/// 基于 3D 网格 light probe 的动态全局光照。
/// 每个 probe 存储 8x8 irradiance (RGB) + 14x14 depth。
pub struct Ddgi {
    // 公共配置（保留原 API）
    pub volume_count: [u32; 3],
    pub probe_radius: f32,
    pub irradiance_resolution: u32,
    pub depth_resolution: u32,

    // GPU 资源（Option 以保留 Default）
    update_pipeline: Option<ComputePipeline>,
    render_pipeline: Option<RenderPipeline>,
    update_layout: Option<BindGroupLayout>,
    render_layout: Option<BindGroupLayout>,
    uniform_buffer: Option<Buffer>,
    probe_buffer: Option<Buffer>,
    irradiance_texture: Option<Texture>,
    irradiance_view: Option<TextureView>,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    linear_sampler: Option<Sampler>,

    // 运行时状态
    frame_index: u32,
    probe_update_offset: u32,
    grid_origin: [f32; 3],
    grid_spacing: [f32; 3],
    probe_count: u32,
    intensity: f32,
    depth_initialized: bool,
}

impl Default for Ddgi {
    fn default() -> Self {
        Self {
            volume_count: DEFAULT_GRID,
            probe_radius: DEFAULT_PROBE_RADIUS,
            irradiance_resolution: IRRADIANCE_TEX_SIZE,
            depth_resolution: DEPTH_TEX_SIZE,
            update_pipeline: None,
            render_pipeline: None,
            update_layout: None,
            render_layout: None,
            uniform_buffer: None,
            probe_buffer: None,
            irradiance_texture: None,
            irradiance_view: None,
            depth_texture: None,
            depth_view: None,
            linear_sampler: None,
            frame_index: 0,
            probe_update_offset: 0,
            grid_origin: [0.0; 3],
            grid_spacing: [4.0; 3],
            probe_count: DEFAULT_GRID[0] * DEFAULT_GRID[1] * DEFAULT_GRID[2],
            intensity: DEFAULT_INTENSITY,
            depth_initialized: false,
        }
    }
}
impl Ddgi {
    /// 创建 DDGI 系统（默认 4x4x4 网格）
    pub fn new(device: &wgpu::Device) -> Self {
        Self::with_grid(device, DEFAULT_GRID, [0.0; 3], [4.0; 3])
    }

    /// 按指定网格创建 DDGI
    pub fn with_grid(
        device: &wgpu::Device,
        grid: [u32; 3],
        grid_origin: [f32; 3],
        grid_spacing: [f32; 3],
    ) -> Self {
        let probe_count = grid[0].max(1) * grid[1].max(1) * grid[2].max(1);

        // ---------- Uniform buffer ----------
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ddgi uniform buffer"),
            size: std::mem::size_of::<DdgiUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---------- Probe buffer ----------
        let probes = Self::generate_probe_positions(grid, grid_origin, grid_spacing);
        let probe_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ddgi probe buffer"),
            contents: bytemuck::cast_slice(&probes),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // ---------- Irradiance 3D texture (Rgba8Unorm) ----------
        let irradiance_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ddgi irradiance texture"),
            size: wgpu::Extent3d {
                width: IRRADIANCE_TEX_SIZE,
                height: IRRADIANCE_TEX_SIZE,
                depth_or_array_layers: probe_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let irradiance_view = irradiance_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ddgi irradiance view"),
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        // ---------- Depth 2D array texture (Rgba8Unorm) ----------
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ddgi probe depth texture"),
            size: wgpu::Extent3d {
                width: DEPTH_TEX_SIZE,
                height: DEPTH_TEX_SIZE,
                depth_or_array_layers: probe_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ddgi probe depth view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // ---------- Sampler ----------
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ddgi linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---------- Update bind group layout ----------
        let update_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ddgi update layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<DdgiUniform>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            (probe_count as u64) * std::mem::size_of::<ProbeData>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
            ],
        });

        // ---------- Render bind group layout ----------
        let render_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ddgi render layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<DdgiUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Shader modules ----------
        let update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ddgi update shader"),
            source: wgpu::ShaderSource::Wgsl(DDGI_UPDATE_SHADER.into()),
        });
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ddgi render shader"),
            source: wgpu::ShaderSource::Wgsl(DDGI_RENDER_SHADER.into()),
        });

        // ---------- Compute pipeline ----------
        let update_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ddgi update pipeline layout"),
                bind_group_layouts: &[&update_layout],
                push_constant_ranges: &[],
            });
        let update_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ddgi update pipeline"),
            layout: Some(&update_pipeline_layout),
            module: &update_shader,
            entry_point: Some("cs_update_probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // ---------- Render pipeline ----------
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ddgi render pipeline layout"),
                bind_group_layouts: &[&render_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ddgi render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_ddgi"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
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

        Self {
            volume_count: grid,
            probe_radius: DEFAULT_PROBE_RADIUS,
            irradiance_resolution: IRRADIANCE_TEX_SIZE,
            depth_resolution: DEPTH_TEX_SIZE,
            update_pipeline: Some(update_pipeline),
            render_pipeline: Some(render_pipeline),
            update_layout: Some(update_layout),
            render_layout: Some(render_layout),
            uniform_buffer: Some(uniform_buffer),
            probe_buffer: Some(probe_buffer),
            irradiance_texture: Some(irradiance_texture),
            irradiance_view: Some(irradiance_view),
            depth_texture: Some(depth_texture),
            depth_view: Some(depth_view),
            linear_sampler: Some(linear_sampler),
            frame_index: 0,
            probe_update_offset: 0,
            grid_origin,
            grid_spacing,
            probe_count,
            intensity: DEFAULT_INTENSITY,
            depth_initialized: false,
        }
    }
    /// 生成所有 probe 的世界坐标
    fn generate_probe_positions(
        grid: [u32; 3],
        origin: [f32; 3],
        spacing: [f32; 3],
    ) -> Vec<ProbeData> {
        let mut probes = Vec::with_capacity((grid[0] * grid[1] * grid[2]) as usize);
        for z in 0..grid[2] {
            for y in 0..grid[1] {
                for x in 0..grid[0] {
                    probes.push(ProbeData {
                        position: [
                            origin[0] + x as f32 * spacing[0],
                            origin[1] + y as f32 * spacing[1],
                            origin[2] + z as f32 * spacing[2],
                            1.0,
                        ],
                    });
                }
            }
        }
        probes
    }

    /// 构建 DdgiUniform
    fn build_uniform(
        &self,
        view_proj: [[f32; 4]; 4],
        view_inv: [[f32; 4]; 4],
        screen_w: f32,
        screen_h: f32,
    ) -> DdgiUniform {
        let safe_w = screen_w.max(1.0);
        let safe_h = screen_h.max(1.0);
        DdgiUniform {
            view_proj,
            view_inv,
            grid_origin: [self.grid_origin[0], self.grid_origin[1], self.grid_origin[2], 0.0],
            grid_spacing: [
                self.grid_spacing[0],
                self.grid_spacing[1],
                self.grid_spacing[2],
                0.0,
            ],
            grid_count: [
                self.volume_count[0] as f32,
                self.volume_count[1] as f32,
                self.volume_count[2] as f32,
                self.probe_count as f32,
            ],
            params: [
                self.probe_radius,
                self.intensity,
                50.0,
                IRRADIANCE_TEX_SIZE as f32,
            ],
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
            frame_index: [
                self.frame_index as f32,
                self.probe_update_offset as f32,
                0.0,
                0.0,
            ],
        }
    }

    /// 初始化 irradiance texture 为 0（无间接光）
    fn init_irradiance_texture(&self, queue: &wgpu::Queue) {
        let irradiance_texture = self
            .irradiance_texture
            .as_ref()
            .expect("Ddgi::new() must be called before init_irradiance_texture()");
        let pixel_count = (IRRADIANCE_TEX_SIZE as usize)
            * (IRRADIANCE_TEX_SIZE as usize)
            * (self.probe_count as usize);
        let bytes = vec![0u8; pixel_count * 4];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: irradiance_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(IRRADIANCE_TEX_SIZE * 4),
                rows_per_image: Some(IRRADIANCE_TEX_SIZE),
            },
            wgpu::Extent3d {
                width: IRRADIANCE_TEX_SIZE,
                height: IRRADIANCE_TEX_SIZE,
                depth_or_array_layers: self.probe_count,
            },
        );
    }

    /// 初始化 depth texture 为 1.0（最远）
    fn init_depth_texture(&self, queue: &wgpu::Queue) {
        let depth_texture = self
            .depth_texture
            .as_ref()
            .expect("Ddgi::new() must be called before init_depth_texture()");
        let pixel_count = (DEPTH_TEX_SIZE as usize)
            * (DEPTH_TEX_SIZE as usize)
            * (self.probe_count as usize);
        let bytes = vec![255u8; pixel_count * 4];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: depth_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(DEPTH_TEX_SIZE * 4),
                rows_per_image: Some(DEPTH_TEX_SIZE),
            },
            wgpu::Extent3d {
                width: DEPTH_TEX_SIZE,
                height: DEPTH_TEX_SIZE,
                depth_or_array_layers: self.probe_count,
            },
        );
    }

    /// 更新 probe（每帧调用）
    ///
    /// 如果 scene 提供 depth + color view，则 dispatch compute shader
    /// 采样场景颜色并写入 irradiance 纹理；否则只推进 probe 更新偏移。
    pub fn update_probes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &dyn DdgiScene,
    ) {
        // 首次调用：初始化 depth + irradiance texture
        if !self.depth_initialized {
            self.init_irradiance_texture(queue);
            self.init_depth_texture(queue);
            self.depth_initialized = true;
        }

        let uniform_buffer = self
            .uniform_buffer
            .as_ref()
            .expect("Ddgi::new() must be called before update_probes()");

        let [sw, sh] = scene.screen_size();
        let uniform = self.build_uniform(scene.view_proj(), scene.view_inv(), sw, sh);
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // Round-robin: 推进 probe_update_offset
        let next_offset = if self.probe_count > 0 {
            (self.probe_update_offset + PROBES_PER_FRAME) % self.probe_count
        } else {
            0
        };
        self.probe_update_offset = next_offset;

        // 如果 scene 提供了 depth + color view，则 dispatch compute 更新
        if let (Some(depth_view), Some(color_view)) = (scene.depth_view(), scene.color_view()) {
            let update_pipeline = self
                .update_pipeline
                .as_ref()
                .expect("Ddgi::new() must be called before update_probes()");
            let update_layout = self
                .update_layout
                .as_ref()
                .expect("Ddgi::new() must be called before update_probes()");
            let probe_buffer = self
                .probe_buffer
                .as_ref()
                .expect("Ddgi::new() must be called before update_probes()");
            let irradiance_view = self
                .irradiance_view
                .as_ref()
                .expect("Ddgi::new() must be called before update_probes()");

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ddgi update bind group"),
                layout: update_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: probe_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(irradiance_view),
                    },
                ],
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ddgi update encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("ddgi update pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(update_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(PROBES_PER_FRAME, 1, 1);
            }
            queue.submit(std::iter::once(encoder.finish()));
        }

        self.frame_index = self.frame_index.wrapping_add(1);
    }
    /// 渲染 DDGI 到目标
    ///
    /// 将 irradiance 叠加到 src_view，输出到 dst_view。
    /// 渲染时不使用场景深度，用 NDC z=0.5 估算 world position（简化）。
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_view: &TextureView,
        dst_view: &TextureView,
        camera: &Camera,
    ) {
        let render_pipeline = self
            .render_pipeline
            .as_ref()
            .expect("Ddgi::new() must be called before render()");
        let render_layout = self
            .render_layout
            .as_ref()
            .expect("Ddgi::new() must be called before render()");
        let uniform_buffer = self
            .uniform_buffer
            .as_ref()
            .expect("Ddgi::new() must be called before render()");
        let irradiance_view = self
            .irradiance_view
            .as_ref()
            .expect("Ddgi::new() must be called before render()");
        let depth_view = self
            .depth_view
            .as_ref()
            .expect("Ddgi::new() must be called before render()");
        let linear_sampler = self
            .linear_sampler
            .as_ref()
            .expect("Ddgi::new() must be called before render()");

        // 写入 uniform（基于 camera）
        let view_proj = camera.view_proj();
        let view_inv = view_proj.inverse();
        let uniform = self.build_uniform(
            view_proj.to_cols_array_2d(),
            view_inv.to_cols_array_2d(),
            1920.0,
            1080.0,
        );
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ddgi render bind group"),
            layout: render_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(irradiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ddgi render encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ddgi render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(render_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// 创建 update bind group（外部调用）
    pub fn create_update_bind_group(
        &self,
        device: &wgpu::Device,
        depth_view: &TextureView,
        color_view: &TextureView,
    ) -> Option<BindGroup> {
        let layout = self.update_layout.as_ref()?;
        let uniform_buffer = self.uniform_buffer.as_ref()?;
        let probe_buffer = self.probe_buffer.as_ref()?;
        let irradiance_view = self.irradiance_view.as_ref()?;

        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ddgi update bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: probe_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(irradiance_view),
                },
            ],
        }))
    }

    /// 创建 render bind group（外部调用）
    pub fn create_render_bind_group(
        &self,
        device: &wgpu::Device,
        src_view: &TextureView,
    ) -> Option<BindGroup> {
        let layout = self.render_layout.as_ref()?;
        let uniform_buffer = self.uniform_buffer.as_ref()?;
        let irradiance_view = self.irradiance_view.as_ref()?;
        let depth_view = self.depth_view.as_ref()?;
        let linear_sampler = self.linear_sampler.as_ref()?;

        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ddgi render bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(irradiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        }))
    }

    /// 设置 irradiance 强度
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    /// 获取 probe 总数
    pub fn probe_count(&self) -> u32 {
        self.probe_count
    }

    /// 获取当前帧索引
    pub fn frame_index(&self) -> u32 {
        self.frame_index
    }

    /// 获取当前 probe 更新偏移
    pub fn probe_update_offset(&self) -> u32 {
        self.probe_update_offset
    }
}
// ============ WGSL: 更新 Compute Shader ============

const DDGI_UPDATE_SHADER: &str = r#"
struct DdgiUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    grid_origin: vec4<f32>,
    grid_spacing: vec4<f32>,
    grid_count: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
    frame_index: vec4<f32>,
};

struct ProbeData {
    position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: DdgiUniform;
@group(0) @binding(1) var<storage, read> probes: array<ProbeData>;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var t_color: texture_2d<f32>;
@group(0) @binding(4) var t_irradiance: texture_storage_3d<rgba8unorm, write>;

fn dir_from_uv(du: f32, dv: f32) -> vec3<f32> {
    let phi = du * 6.28318530718;
    let theta = dv * 3.14159265359;
    return vec3<f32>(
        sin(theta) * cos(phi),
        cos(theta),
        sin(theta) * sin(phi),
    );
}

@compute @workgroup_size(8, 8, 1)
fn cs_update_probe(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let probe_idx = u32(u.frame_index.y) + wg.x;
    if (probe_idx >= u32(u.grid_count.w)) {
        return;
    }

    let probe_pos = probes[probe_idx].position.xyz;
    let texel = lid.xy;

    let du = f32(texel.x) / 8.0;
    let dv = f32(texel.y) / 8.0;
    let dir = dir_from_uv(du, dv);

    let sample_pos = probe_pos + dir * u.params.x;
    let clip = u.view_proj * vec4<f32>(sample_pos, 1.0);

    var irradiance = vec4<f32>(0.0);
    if (clip.w > 0.0) {
        let ndc = clip.xyz / clip.w;
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let dims = vec2<u32>(u.screen_size.xy);
            let clamped_dims = select(vec2<u32>(1u), dims, dims > vec2<u32>(0u));
            let pixel = clamp(vec2<u32>(uv * vec2<f32>(clamped_dims)), vec2<u32>(0u), clamped_dims - vec2<u32>(1u));
            let color = textureLoad(t_color, pixel, 0);
            irradiance = color;
        }
    }

    textureStore(t_irradiance, vec3<u32>(texel, probe_idx), irradiance);
}
"#;

// ============ WGSL: 渲染 Shader ============

const DDGI_RENDER_SHADER: &str = r#"
struct DdgiUniform {
    view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    grid_origin: vec4<f32>,
    grid_spacing: vec4<f32>,
    grid_count: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
    frame_index: vec4<f32>,
};

@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var t_irradiance: texture_3d<f32>;
@group(0) @binding(2) var t_probe_depth: texture_2d_array<f32>;
@group(0) @binding(3) var s_linear: sampler;
@group(0) @binding(4) var<uniform> u: DdgiUniform;

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

@fragment
fn fs_ddgi(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims_u = textureDimensions(t_src);
    let dims = vec2<f32>(dims_u);
    let uv = pos.xy / dims;

    let src_color = textureSampleLevel(t_src, s_linear, uv, 0.0);

    // 简化：用 NDC z=0.5 估算 world position（无场景深度）
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.5, 1.0);
    let world_h = u.view_inv * ndc;
    let world_pos = world_h.xyz / world_h.w;

    // 计算 probe 网格坐标
    let grid_f = (world_pos - u.grid_origin.xyz) / u.grid_spacing.xyz;
    let grid_i = floor(grid_f);
    let grid_frac = grid_f - grid_i;

    var irradiance_sum = vec3<f32>(0.0);
    var weight_sum = 0.0;

    for (var dz: i32 = 0; dz <= 1; dz = dz + 1) {
        for (var dy: i32 = 0; dy <= 1; dy = dy + 1) {
            for (var dx: i32 = 0; dx <= 1; dx = dx + 1) {
                let offset = vec3<i32>(dx, dy, dz);
                let probe_coord = grid_i + vec3<f32>(offset);

                if (any(probe_coord < vec3<f32>(0.0)) ||
                    any(probe_coord >= u.grid_count.xyz)) {
                    continue;
                }

                let probe_idx = probe_coord.x +
                    probe_coord.y * u.grid_count.x +
                    probe_coord.z * u.grid_count.x * u.grid_count.y;
                let probe_idx_u = u32(probe_idx);

                // 三线性权重
                let trilinear = mix(1.0 - grid_frac, grid_frac, vec3<f32>(offset));
                let weight = trilinear.x * trilinear.y * trilinear.z;

                // 采样 probe irradiance（3D 纹理中心）
                let probe_count_f = f32(u.grid_count.w);
                let probe_uvw = vec3<f32>(
                    0.5,
                    0.5,
                    (f32(probe_idx_u) + 0.5) / max(probe_count_f, 1.0),
                );
                let irradiance = textureSampleLevel(t_irradiance, s_linear, probe_uvw, 0.0).rgb;

                irradiance_sum = irradiance_sum + irradiance * weight;
                weight_sum = weight_sum + weight;
            }
        }
    }

    let ambient = irradiance_sum / max(weight_sum, 0.0001);
    let intensity = u.params.y;
    let result = src_color.rgb + ambient * intensity;
    return vec4<f32>(result, src_color.a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        assert_eq!(std::mem::size_of::<DdgiUniform>(), 224);
    }

    #[test]
    fn probe_data_size() {
        assert_eq!(std::mem::size_of::<ProbeData>(), 16);
    }

    #[test]
    fn default_values() {
        let d = Ddgi::default();
        assert_eq!(d.volume_count, [4, 4, 4]);
        assert!((d.probe_radius - 2.0).abs() < 1e-6);
        assert_eq!(d.irradiance_resolution, 8);
        assert_eq!(d.depth_resolution, 14);
        assert_eq!(d.probe_count, 64);
        assert!(d.update_pipeline.is_none());
        assert!(d.render_pipeline.is_none());
        assert!(!d.depth_initialized);
    }

    #[test]
    fn update_shader_contains_key_elements() {
        assert!(DDGI_UPDATE_SHADER.contains("cs_update_probe"));
        assert!(DDGI_UPDATE_SHADER.contains("texture_storage_3d"));
        assert!(DDGI_UPDATE_SHADER.contains("textureStore"));
        assert!(DDGI_UPDATE_SHADER.contains("workgroup_size"));
        assert!(DDGI_UPDATE_SHADER.contains("dir_from_uv"));
    }

    #[test]
    fn render_shader_contains_key_elements() {
        assert!(DDGI_RENDER_SHADER.contains("vs_fullscreen"));
        assert!(DDGI_RENDER_SHADER.contains("fs_ddgi"));
        assert!(DDGI_RENDER_SHADER.contains("texture_3d"));
        assert!(DDGI_RENDER_SHADER.contains("textureSampleLevel"));
        assert!(DDGI_RENDER_SHADER.contains("trilinear"));
        assert!(DDGI_RENDER_SHADER.contains("textureDimensions"));
    }

    #[test]
    fn generate_probe_positions_count() {
        let probes = Ddgi::generate_probe_positions([4, 4, 4], [0.0; 3], [4.0; 3]);
        assert_eq!(probes.len(), 64);
        // First probe at origin
        assert_eq!(probes[0].position, [0.0, 0.0, 0.0, 1.0]);
        // Last probe at (3*4, 3*4, 3*4) = (12, 12, 12)
        assert_eq!(probes[63].position, [12.0, 12.0, 12.0, 1.0]);
    }

    #[test]
    fn generate_probe_positions_custom_grid() {
        let probes = Ddgi::generate_probe_positions([2, 2, 2], [1.0; 3], [2.0; 3]);
        assert_eq!(probes.len(), 8);
        assert_eq!(probes[0].position, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(probes[7].position, [3.0, 3.0, 3.0, 1.0]);
    }

    #[test]
    fn build_uniform_values() {
        let mut d = Ddgi::default();
        d.grid_origin = [10.0, 20.0, 30.0];
        d.grid_spacing = [2.0, 4.0, 6.0];
        let u = d.build_uniform([[1.0; 4]; 4], [[2.0; 4]; 4], 1920.0, 1080.0);
        assert_eq!(u.grid_origin, [10.0, 20.0, 30.0, 0.0]);
        assert_eq!(u.grid_spacing, [2.0, 4.0, 6.0, 0.0]);
        assert_eq!(u.grid_count, [4.0, 4.0, 4.0, 64.0]);
        assert_eq!(u.screen_size, [1920.0, 1080.0, 1.0 / 1920.0, 1.0 / 1080.0]);
        assert_eq!(u.params[0], 2.0); // probe_radius
    }
}