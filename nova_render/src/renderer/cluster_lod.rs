//! GPU-Driven Cluster LOD (P1)
//!
//! 实现 GPU-driven mesh cluster pipeline，借鉴 Nanite (Karis 2021) 设计：
//! - Cluster 预处理（CPU 一次性）：每个 mesh 分成 ~128 顶点的 cluster
//! - Cluster Culling（compute shader，每帧）：
//!   - Frustum cull（cluster bounding sphere vs 6 frustum planes）
//!   - LOD selection（根据屏幕空间误差选 LOD level）
//!   - Occlusion cull（可选，用 hierarchical-Z buffer）
//! - Indirect Draw（GPU-driven）：用 culling 结果填充 indirect draw arguments
//!
//! 由于 wgpu 24 不支持 mesh shader，用 indirect draw + compute culling 实现。
//!
//! 参考：
//! - Karis 2021 "Nanite" (UE5 SIGGRAPH)
//! - Lidgren 2018 "GPU-Driven Rendering Pipelines" (GPU Zen 2)
//! - Drobot 2014 "GPU-Driven Pipelines" (SIGGRAPH Advances in Real-Time Rendering)

use bytemuck::{Pod, Zeroable};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wgpu::{
    BindGroupLayout, Buffer, ComputePipeline, Device, Queue, RenderPass, TextureView,
};

use crate::scene::camera::Camera;

const WORKGROUP_SIZE: u32 = 64;

/// 一个 cluster（顶点组 + 包围球）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct Cluster {
    pub center: [f32; 4], // xyz=center, w=radius
    pub index_offset: u32,
    pub index_count: u32,
    pub lod: u32,
    pub mesh_id: u32,
}

/// Cluster LOD uniform（每帧更新）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct ClusterLodUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 4],
    pub screen_size: [f32; 4], // xy=size, zw=1/size
    pub params: [f32; 4],      // x=error_threshold, y=enable_occlusion, z=cluster_count, w=pad
}

/// Indirect draw argument（匹配 wgpu::util::DrawIndexedIndirectArgs 字节布局，20 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct DrawIndexedArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

/// Draw count + 统计（GPU 写入，CPU 读回，16 bytes）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct DrawCount {
    pub count: u32,   // atomic counter — compact indirect slot
    pub visible: u32, // visible clusters (after culling)
    pub culled: u32,  // culled clusters
    pub _pad: u32,
}

/// Cluster LOD 统计
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterLodStats {
    pub visible_clusters: u32,
    pub draw_calls: u32,
    pub culled_clusters: u32,
}

/// Cluster LOD 系统
pub struct ClusterLod {
    // GPU resources
    cull_pipeline: Option<ComputePipeline>,
    cull_layout: Option<BindGroupLayout>,
    cluster_buffer: Option<Buffer>,
    cluster_index_buffer: Option<Buffer>,
    indirect_buffer: Option<Buffer>,
    draw_count_buffer: Option<Buffer>,
    uniform_buffer: Option<Buffer>,
    stats_staging: Option<Buffer>,
    dummy_hzb_texture: Option<wgpu::Texture>,
    dummy_hzb_view: Option<TextureView>,
    hzb_sampler: Option<wgpu::Sampler>,

    // Config
    pub max_clusters: u32,
    pub max_draws: u32,
    pub screen_error_threshold: f32,
    pub enable_occlusion: bool,
    pub screen_size: [f32; 2],

    // Feature flags (detected at new())
    multi_draw_indirect_count: bool,
    multi_draw_indirect: bool,

    // Stats readback state
    stats_pending: bool,
    stats_mapped: Arc<AtomicBool>,
    cached_stats: ClusterLodStats,

    // Track uploaded cluster count + dummy hzb init
    cluster_count: u32,
    dummy_hzb_initialized: bool,
}

impl ClusterLod {
    /// 创建 ClusterLod 系统。max_clusters / max_draws 自动 clamp 到 >= 1。
    pub fn new(device: &Device, max_clusters: u32, max_draws: u32) -> Self {
        let max_clusters = max_clusters.max(1);
        let max_draws = max_draws.max(1);

        let features = device.features();
        let multi_draw_indirect_count = features.contains(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT);
        let multi_draw_indirect = features.contains(wgpu::Features::MULTI_DRAW_INDIRECT);

        // ---------- Buffers ----------
        let cluster_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: cluster metadata"),
            size: (max_clusters as u64) * std::mem::size_of::<Cluster>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cluster_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: cluster index buffer"),
            size: (max_clusters as u64) * 128 * 4, // 估每 cluster 最多 128 indices
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: indirect args"),
            size: (max_draws as u64) * std::mem::size_of::<DrawIndexedArgs>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let draw_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: draw count + stats"),
            size: std::mem::size_of::<DrawCount>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: uniform"),
            size: std::mem::size_of::<ClusterLodUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let stats_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cluster_lod: stats staging"),
            size: std::mem::size_of::<DrawCount>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---------- Dummy HZB texture (1x1 R32Float, init to 1.0 = far) ----------
        let dummy_hzb_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cluster_lod: dummy hzb"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let dummy_hzb_view =
            dummy_hzb_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ---------- HZB sampler ----------
        let hzb_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cluster_lod: hzb sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---------- Bind group layout ----------
        let cull_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cluster_lod: cull layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<Cluster>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<DrawIndexedArgs>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<DrawCount>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<ClusterLodUniform>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // ---------- Shader + pipeline ----------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cluster_lod: cull shader"),
            source: wgpu::ShaderSource::Wgsl(CULL_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cluster_lod: cull pipeline layout"),
            bind_group_layouts: &[&cull_layout],
            push_constant_ranges: &[],
        });

        let cull_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cluster_lod: cull pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_cull"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            cull_pipeline: Some(cull_pipeline),
            cull_layout: Some(cull_layout),
            cluster_buffer: Some(cluster_buffer),
            cluster_index_buffer: Some(cluster_index_buffer),
            indirect_buffer: Some(indirect_buffer),
            draw_count_buffer: Some(draw_count_buffer),
            uniform_buffer: Some(uniform_buffer),
            stats_staging: Some(stats_staging),
            dummy_hzb_texture: Some(dummy_hzb_texture),
            dummy_hzb_view: Some(dummy_hzb_view),
            hzb_sampler: Some(hzb_sampler),
            max_clusters,
            max_draws,
            screen_error_threshold: 8.0,
            enable_occlusion: false,
            screen_size: [1920.0, 1080.0],
            multi_draw_indirect_count,
            multi_draw_indirect,
            stats_pending: false,
            stats_mapped: Arc::new(AtomicBool::new(false)),
            cached_stats: ClusterLodStats::default(),
            cluster_count: 0,
            dummy_hzb_initialized: false,
        }
    }

    /// 上传 mesh cluster 数据（CPU 一次性预处理）
    /// - `clusters`: 所有 mesh 的 cluster 元数据
    /// - `indices`: 所有 cluster 引用的全局 index buffer（megabuffer 思路）
    pub fn upload_clusters(&mut self, queue: &Queue, clusters: &[Cluster], indices: &[u32]) {
        let count = clusters.len().min(self.max_clusters as usize);
        if count > 0 {
            queue.write_buffer(
                self.cluster_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&clusters[..count]),
            );
            self.cluster_count = count as u32;
        } else {
            self.cluster_count = 0;
        }

        // Upload global cluster index buffer (megabuffer)
        if !indices.is_empty() {
            let max_bytes = (self.max_clusters as usize) * 128 * 4;
            let bytes = bytemuck::cast_slice::<u32, u8>(indices);
            let n = bytes.len().min(max_bytes);
            queue.write_buffer(
                self.cluster_index_buffer.as_ref().unwrap(),
                0,
                &bytes[..n],
            );
        }

        // Initialize dummy HZB to 1.0 (far depth) — only once
        if !self.dummy_hzb_initialized {
            let init_bytes = 1.0f32.to_ne_bytes();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: self.dummy_hzb_texture.as_ref().unwrap(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &init_bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            self.dummy_hzb_initialized = true;
        }
    }

    /// 访问内部 cluster index buffer（用于 draw_indirect 时绑定）
    pub fn cluster_index_buffer(&self) -> Option<&Buffer> {
        self.cluster_index_buffer.as_ref()
    }

    /// 每帧执行 culling + LOD selection（compute shader）
    pub fn cull(
        &mut self,
        device: &Device,
        queue: &Queue,
        camera: &Camera,
        hzb_texture: Option<&TextureView>,
    ) {
        // 1. Try to consume previous frame's stats (non-blocking)
        self.try_consume_mapped_stats();

        if self.cluster_count == 0 {
            return;
        }

        // 2. Update uniform
        let view_proj = camera.view_proj();
        let (sw, sh) = (self.screen_size[0], self.screen_size[1]);
        let safe_w = sw.max(1.0);
        let safe_h = sh.max(1.0);
        let uniform = ClusterLodUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view_pos: [camera.position.x, camera.position.y, camera.position.z, 0.0],
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
            params: [
                self.screen_error_threshold,
                if self.enable_occlusion { 1.0 } else { 0.0 },
                self.cluster_count as f32,
                0.0,
            ],
        };
        queue.write_buffer(
            self.uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[uniform]),
        );

        // 3. Clear indirect + draw_count buffers (CPU-side zeroing via queue.write_buffer)
        let indirect_bytes =
            vec![0u8; (self.max_draws as usize) * std::mem::size_of::<DrawIndexedArgs>()];
        queue.write_buffer(
            self.indirect_buffer.as_ref().unwrap(),
            0,
            &indirect_bytes,
        );
        let count_zeros = [0u8; std::mem::size_of::<DrawCount>()];
        queue.write_buffer(self.draw_count_buffer.as_ref().unwrap(), 0, &count_zeros);

        // 4. Build bind group (recreate each frame to allow dynamic hzb)
        let hzb_view = hzb_texture
            .unwrap_or_else(|| self.dummy_hzb_view.as_ref().unwrap());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cluster_lod: cull bind group"),
            layout: self.cull_layout.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.cluster_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.indirect_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.draw_count_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.uniform_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(hzb_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(
                        self.hzb_sampler.as_ref().unwrap(),
                    ),
                },
            ],
        });

        // 5. Dispatch compute
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cluster_lod: cull encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cluster_lod: cull pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(self.cull_pipeline.as_ref().unwrap());
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (self.cluster_count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // 6. Copy stats to staging (only if previous stats already consumed)
        let schedule_map = !self.stats_pending;
        if schedule_map {
            encoder.copy_buffer_to_buffer(
                self.draw_count_buffer.as_ref().unwrap(),
                0,
                self.stats_staging.as_ref().unwrap(),
                0,
                std::mem::size_of::<DrawCount>() as u64,
            );
        }

        queue.submit(std::iter::once(encoder.finish()));

        // 7. Schedule async map of staging buffer (non-blocking)
        if schedule_map {
            let flag = self.stats_mapped.clone();
            self.stats_staging
                .as_ref()
                .unwrap()
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    if result.is_ok() {
                        flag.store(true, Ordering::Relaxed);
                    }
                });
            self.stats_pending = true;
        }
    }

    /// 间接绘制（在 render pass 中调用）
    ///
    /// 绑定外部 vertex + index buffer，根据 GPU 填充的 indirect args 发起 draw。
    /// - 若设备支持 MULTI_DRAW_INDIRECT_COUNT：使用 count_buffer 决定实际 draw 数（最优）
    /// - 否则若支持 MULTI_DRAW_INDIRECT：发起 max_draws 个 draw（空 slot instance_count=0，GPU 跳过）
    /// - 否则回退到逐个 draw_indexed_indirect 循环
    pub fn draw_indirect(
        &self,
        pass: &mut RenderPass,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
    ) {
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let indirect = self.indirect_buffer.as_ref().unwrap();
        let max = self.max_draws;

        if self.multi_draw_indirect_count {
            pass.multi_draw_indexed_indirect_count(
                indirect,
                0,
                self.draw_count_buffer.as_ref().unwrap(),
                0,
                max,
            );
        } else if self.multi_draw_indirect {
            pass.multi_draw_indexed_indirect(indirect, 0, max);
        } else {
            let stride = std::mem::size_of::<DrawIndexedArgs>() as u64;
            for i in 0..max {
                pass.draw_indexed_indirect(indirect, i as u64 * stride);
            }
        }
    }

    /// 读取统计（cluster 数 / draw call 数 / cull 数）
    ///
    /// 返回上一帧 cull() 完成后的统计（异步读回，可能延迟 1-2 帧）。
    pub fn read_stats(&self, _queue: &Queue) -> ClusterLodStats {
        self.cached_stats
    }

    /// Try to read + unmap the staging buffer if map_async completed
    fn try_consume_mapped_stats(&mut self) {
        if !self.stats_pending || !self.stats_mapped.load(Ordering::Relaxed) {
            return;
        }
        let staging = self.stats_staging.as_ref().unwrap();
        let view = staging.slice(..);
        let data = view.get_mapped_range();
        let dc_size = std::mem::size_of::<DrawCount>();
        if data.len() >= dc_size {
            let dc: &DrawCount = bytemuck::from_bytes(&data[..dc_size]);
            self.cached_stats = ClusterLodStats {
                visible_clusters: dc.visible,
                draw_calls: dc.count,
                culled_clusters: dc.culled,
            };
        }
        drop(data);
        staging.unmap();
        self.stats_mapped.store(false, Ordering::Relaxed);
        self.stats_pending = false;
    }
}

impl Default for ClusterLod {
    fn default() -> Self {
        unreachable!("ClusterLod requires device + max_clusters + max_draws arguments");
    }
}

// ============================================================================
// WGSL Shader
// ============================================================================

const CULL_SHADER: &str = r#"
struct Cluster {
    center: vec4<f32>,
    index_offset: u32,
    index_count: u32,
    lod: u32,
    mesh_id: u32,
}

struct ClusterLodUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
    screen_size: vec4<f32>,
    params: vec4<f32>,
}

struct DrawIndexedArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

struct DrawCount {
    count: atomic<u32>,
    visible: atomic<u32>,
    culled: atomic<u32>,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read_write> indirect: array<DrawIndexedArgs>;
@group(0) @binding(2) var<storage, read_write> draw_count: DrawCount;
@group(0) @binding(3) var<uniform> u: ClusterLodUniform;
@group(0) @binding(4) var hzb: texture_2d<f32>;
@group(0) @binding(5) var hzb_sampler: sampler;

// 6-plane frustum test (Gribb-Hartmann extraction from view_proj)
// Returns true if sphere (center.xyz, radius) intersects the frustum.
fn in_frustum(center: vec4<f32>, radius: f32) -> bool {
    let m = u.view_proj;
    // mat4x4 in WGSL is column-major: m[col][row]. Rows of M are:
    let r0 = vec4<f32>(m[0][0], m[1][0], m[2][0], m[3][0]);
    let r1 = vec4<f32>(m[0][1], m[1][1], m[2][1], m[3][1]);
    let r2 = vec4<f32>(m[0][2], m[1][2], m[2][2], m[3][2]);
    let r3 = vec4<f32>(m[0][3], m[1][3], m[2][3], m[3][3]);
    let p = vec4<f32>(center.xyz, 1.0);

    // 6 frustum planes: left, right, bottom, top, near, far
    let planes = array<vec4<f32>, 6>(
        r3 + r0,
        r3 - r0,
        r3 + r1,
        r3 - r1,
        r3 + r2,
        r3 - r2,
    );

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        let plane = planes[i];
        let n_len = length(plane.xyz);
        if (n_len < 0.0001) { continue; }
        let d = dot(plane, p) / n_len;
        if (d < -radius) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn cs_cull(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cluster_count = u32(u.params.z);
    if (gid.x >= cluster_count) { return; }

    let cluster = clusters[gid.x];
    let center = cluster.center;
    let radius = center.w;

    // 1. Frustum cull — bounding sphere vs 6 frustum planes
    if (!in_frustum(center, radius)) {
        atomicAdd(&draw_count.culled, 1u);
        return;
    }

    // 2. Project to clip space for LOD test
    let projected = u.view_proj * vec4<f32>(center.xyz, 1.0);
    if (projected.w <= 0.0001) {
        // Behind camera (defensive; should be caught by frustum cull)
        atomicAdd(&draw_count.culled, 1u);
        return;
    }

    // Screen-space radius (pixels) — approx using screen height
    let screen_radius = abs(radius / projected.w) * u.screen_size.y;

    // LOD selection: if cluster too small AND has higher-LOD parent, skip
    // (Simplified P1: cull if below threshold. Real impl would pick coarser LOD.)
    if (screen_radius < u.params.x && cluster.lod > 0u) {
        atomicAdd(&draw_count.culled, 1u);
        return;
    }

    // 3. Occlusion cull (optional) — sample HZB mip 0
    if (u.params.y > 0.5) {
        let inv_w = 1.0 / projected.w;
        let ndc = vec3<f32>(projected.xyz * inv_w);
        let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
        if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
            let hzb_depth = textureSampleLevel(hzb, hzb_sampler, uv, 0.0).x;
            let cluster_depth = ndc.z * 0.5 + 0.5;
            // Conservative bias based on projected radius (avoid false culling)
            let bias = max(0.01, screen_radius * 0.001);
            if (cluster_depth > hzb_depth + bias) {
                atomicAdd(&draw_count.culled, 1u);
                return;
            }
        }
    }

    // 4. Write compact indirect args via atomic counter
    let slot = atomicAdd(&draw_count.count, 1u);
    let max_draws = arrayLength(&indirect);
    if (slot < max_draws) {
        indirect[slot].index_count = cluster.index_count;
        indirect[slot].instance_count = 1u;
        indirect[slot].first_index = cluster.index_offset;
        indirect[slot].vertex_offset = 0;
        indirect[slot].first_instance = 0u;
    }
    atomicAdd(&draw_count.visible, 1u);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_sizes() {
        assert_eq!(std::mem::size_of::<Cluster>(), 32);
        assert_eq!(std::mem::size_of::<ClusterLodUniform>(), 112);
        assert_eq!(std::mem::size_of::<DrawIndexedArgs>(), 20);
        assert_eq!(std::mem::size_of::<DrawCount>(), 16);
    }

    #[test]
    fn test_shader_present() {
        assert!(CULL_SHADER.contains("cs_cull"));
        assert!(CULL_SHADER.contains("in_frustum"));
        assert!(CULL_SHADER.contains("DrawIndexedArgs"));
        assert!(CULL_SHADER.contains("textureSampleLevel"));
        assert!(CULL_SHADER.contains("atomicAdd"));
        assert!(CULL_SHADER.contains("@compute @workgroup_size(64)"));
    }

    #[test]
    fn test_cluster_default() {
        let c = Cluster::default();
        assert_eq!(c.center, [0.0; 4]);
        assert_eq!(c.index_offset, 0);
        assert_eq!(c.index_count, 0);
        assert_eq!(c.lod, 0);
        assert_eq!(c.mesh_id, 0);
    }
}
