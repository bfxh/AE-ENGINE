//! TAA (Temporal Anti-Aliasing) Module
//!
//! 时域抗锯齿：利用历史帧信息在时域上累积样本，配合亚像素抖动实现全屏抗锯齿。
//!
//! 流程：
//! 1. **Jitter**：每帧用 Halton 序列生成亚像素偏移，注入投影矩阵，场景渲染到 HDR
//! 2. **Reproject**：从当前帧深度重建世界坐标，用上一帧 view-projection 重新投影得到历史 UV
//! 3. **Neighborhood clamp**：采样当前帧 3x3 邻域计算 min/max 包围盒，将历史帧 clamp 进去（抑制鬼影/拖尾）
//! 4. **Blend**：`result = mix(history_clamped, current, blend_factor)`，速度越大 blend 越多当前帧
//!
//! 历史帧采用 ping-pong 双缓冲：
//! - `history_textures[history_index]` 本帧读取（上一帧 resolve 结果）
//! - `history_textures[1 - history_index]` 本帧写入（TAA 输出，下一帧的历史）
//! - 帧末调用 `swap_history()` 翻转索引，无需外部 copy
//!
//! WGSL 注意事项：
//! - mat4x4 不允许作为 vertex input/output，全部通过 uniform 传递
//! - 全屏三角形：3 个顶点覆盖 NDC，无 vertex buffer
//! - Depth32Float 不支持 filtering，使用 NonFiltering sampler + textureSampleLevel
//! - 循环内统一使用 textureSampleLevel 避免 non-uniform control flow 限制

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler, Texture, TextureView};

/// TAA Uniform（304 bytes = 4 × mat4x4 + 3 × vec4，符合 WGSL 16-byte 对齐）
///
/// 字段布局：
/// - `view_proj` / `view_proj_jitter`：当前帧 view-projection（含 jitter）
/// - `prev_view_proj`：上一帧 view-projection（含 jitter），用于重新投影
/// - `view_inv`：当前帧 view-projection 的逆（用于从深度重建世界坐标）
/// - `jitter`：xy=当前帧 jitter，zw=上一帧 jitter
/// - `params`：x=blend_factor(0.1), y=vel_scale, z=enable(1.0), w=unused
/// - `screen_size`：x=width, y=height, z=1/width, w=1/height
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct TaaUniform {
    /// 当前帧 view-projection（含 jitter）
    pub view_proj: [[f32; 4]; 4],
    /// 当前帧 view-projection（含 jitter，同上）
    pub view_proj_jitter: [[f32; 4]; 4],
    /// 上一帧 view-projection（含 jitter）
    pub prev_view_proj: [[f32; 4]; 4],
    /// 逆 view-projection（用于从 NDC+深度重建世界坐标）
    pub view_inv: [[f32; 4]; 4],
    /// xy=当前帧 jitter, zw=上一帧 jitter
    pub jitter: [f32; 4],
    /// x=blend_factor(0.1), y=vel_scale, z=enable(1.0), w=unused
    pub params: [f32; 4],
    /// x=width, y=height, z=1/width, w=1/height
    pub screen_size: [f32; 4],
}

impl TaaUniform {
    /// 创建默认参数 TAA uniform
    pub fn new(
        view_proj: [[f32; 4]; 4],
        prev_view_proj: [[f32; 4]; 4],
        view_inv: [[f32; 4]; 4],
        jitter: [f32; 2],
        prev_jitter: [f32; 2],
        width: f32,
        height: f32,
    ) -> Self {
        let safe_w = width.max(1.0);
        let safe_h = height.max(1.0);
        Self {
            view_proj,
            view_proj_jitter: view_proj,
            prev_view_proj,
            view_inv,
            jitter: [jitter[0], jitter[1], prev_jitter[0], prev_jitter[1]],
            params: [0.1, 100.0, 1.0, 0.0],
            screen_size: [safe_w, safe_h, 1.0 / safe_w, 1.0 / safe_h],
        }
    }
}

/// TAA Shader (WGSL)
///
/// 全屏三角形 vertex + TAA resolve fragment：
/// - 采样当前帧颜色（linear filter 利用硬件滤波抵消 jitter）
/// - 采样深度重建世界坐标，用 prev_view_proj 重新投影得到历史 UV
/// - 3x3 邻域 clamping 包围盒，将历史帧 clamp 进去
/// - 速度自适应 blend：速度越大 blend 越多当前帧
const TAA_SHADER: &str = r#"
struct TaaUniform {
    view_proj: mat4x4<f32>,
    view_proj_jitter: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    jitter: vec4<f32>,
    params: vec4<f32>,
    screen_size: vec4<f32>,
};

@group(0) @binding(0) var t_current: texture_2d<f32>;
@group(0) @binding(1) var t_history: texture_2d<f32>;
@group(0) @binding(2) var t_depth: texture_2d<f32>;
@group(0) @binding(3) var s_linear: sampler;
@group(0) @binding(4) var s_depth: sampler;
@group(0) @binding(5) var<uniform> u: TaaUniform;

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
fn fs_taa(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(u.screen_size.x, u.screen_size.y);
    let texel = vec2<f32>(u.screen_size.z, u.screen_size.w);
    let uv = pos.xy / dims;

    // 当前帧颜色（linear filter，硬件双线性近似抵消 jitter）
    let current_color = textureSampleLevel(t_current, s_linear, uv, 0.0).rgb;

    // TAA 关闭：直接返回当前帧
    if (u.params.z < 0.5) {
        return vec4<f32>(current_color, 1.0);
    }

    // 采样深度（Depth32Float 不支持 filtering，用 textureSampleLevel）
    let depth = textureSampleLevel(t_depth, s_depth, uv, 0.0).x;

    // 远平面/背景：无历史可参考，直接返回当前帧
    if (depth >= 1.0) {
        return vec4<f32>(current_color, 1.0);
    }

    // 重建世界坐标：UV+Depth -> NDC -> World
    // WebGPU NDC: x∈[-1,1] (右), y∈[-1,1] (上), z∈[0,1]
    // 纹理 UV: u∈[0,1] (左→右), v∈[0,1] (上→下)
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world_h = u.view_inv * ndc;
    let world_pos = world_h.xyz / world_h.w;

    // 重新投影到上一帧 clip-space
    let prev_clip = u.prev_view_proj * vec4<f32>(world_pos, 1.0);
    let prev_ndc = prev_clip.xyz / prev_clip.w;
    let prev_uv = vec2<f32>(
        prev_ndc.x * 0.5 + 0.5,
        0.5 - prev_ndc.y * 0.5,
    );

    // 历史 UV 是否在屏幕内
    let on_screen = all(prev_uv >= vec2<f32>(0.0, 0.0)) && all(prev_uv <= vec2<f32>(1.0, 1.0));

    // 采样历史帧（超出屏幕则回退到当前帧）
    var history_color = textureSampleLevel(t_history, s_linear, prev_uv, 0.0).rgb;
    if (!on_screen) {
        history_color = current_color;
    }

    // 邻域 clamping：3x3 包围盒（含中心）
    var neighbor_min = current_color;
    var neighbor_max = current_color;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let c = textureSampleLevel(t_current, s_linear, uv + offset, 0.0).rgb;
            neighbor_min = min(neighbor_min, c);
            neighbor_max = max(neighbor_max, c);
        }
    }

    // 将历史帧 clamp 到当前帧邻域包围盒内（抑制拖尾/鬼影）
    let history_clamped = clamp(history_color, neighbor_min, neighbor_max);

    // 速度计算（基于 reproject UV 偏移，乘以 vel_scale）
    let velocity = (prev_uv - uv) * u.params.y;
    let speed = length(velocity);
    // 速度越大 → blend 越多当前帧（减少高速运动拖尾）
    var blend_factor = clamp(u.params.x + speed * 2.0, 0.0, 1.0);
    if (!on_screen) {
        blend_factor = 1.0;
    }

    let result = mix(history_clamped, current_color, blend_factor);
    return vec4<f32>(result, 1.0);
}
"#;

/// TAA 渲染器：管理 TAA resolve 管线与 ping-pong 历史帧
///
/// 使用方式：
/// ```ignore
/// let mut taa = TaaRenderer::new(&device, hdr_format, depth_format);
/// taa.init_history(&device, width, height);
///
/// // 每帧：
/// taa.update_uniform(&queue, &taa_uniform);
/// let bg = taa.create_bind_group(&device, &hdr_view, taa.history_view().unwrap(), &depth_view);
/// // TAA pass: 渲染到 taa.current_output_view().unwrap()
/// taa.draw(&mut pass, &bg);
/// // 后续 bloom/tonemap 用 current_output_view()
/// // 帧末：
/// taa.swap_history();
/// ```
pub struct TaaRenderer {
    /// TAA resolve pipeline
    pub taa_pipeline: RenderPipeline,
    /// Uniform buffer
    pub uniform_buffer: Buffer,
    /// TAA bind group layout（含 texture/sampler/uniform 全部 binding）
    pub uniform_layout: BindGroupLayout,
    /// Linear sampler（history/current 颜色，Rgba16Float 支持 filtering）
    pub sampler: Sampler,
    /// Depth sampler（NonFiltering，Depth32Float 不支持 filtering）
    pub depth_sampler: Sampler,
    /// Ping-pong 历史帧纹理。`[history_index]` 本帧读，`[1-history_index]` 本帧写
    pub history_textures: [Option<Texture>; 2],
    /// Ping-pong 历史帧 view，与 `history_textures` 一一对应
    pub history_views: [Option<TextureView>; 2],
    /// 当前读的历史帧索引（0 或 1），写目标为 `1 - history_index`
    pub history_index: u32,
}

impl TaaRenderer {
    /// 创建 TAA 渲染器
    ///
    /// # 参数
    /// - `device`: wgpu 设备
    /// - `hdr_format`: HDR 纹理格式（应为 Rgba16Float，与历史帧一致）
    /// - `_depth_format`: 深度纹理格式（应为 Depth32Float，内部按非 filterable 处理）
    pub fn new(
        device: &Device,
        hdr_format: wgpu::TextureFormat,
        _depth_format: wgpu::TextureFormat,
    ) -> Self {
        // ---------- TAA bind group layout ----------
        // 0: current HDR (Rgba16Float, filterable)
        // 1: history (Rgba16Float, filterable)
        // 2: depth (Depth32Float, non-filterable)
        // 3: linear sampler (filtering)
        // 4: depth sampler (non-filtering)
        // 5: uniform buffer
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa bind group layout"),
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
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<TaaUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });

        // ---------- Uniform buffer ----------
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("taa uniform buffer"),
            size: std::mem::size_of::<TaaUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---------- Linear sampler（history/current 颜色）----------
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---------- Depth sampler（NonFiltering，Depth32Float）----------
        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa depth sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---------- Shader & pipeline ----------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("taa shader"),
            source: wgpu::ShaderSource::Wgsl(TAA_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("taa pipeline layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        let taa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("taa pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_taa"),
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

        Self {
            taa_pipeline,
            uniform_buffer,
            uniform_layout,
            sampler,
            depth_sampler,
            history_textures: [None, None],
            history_views: [None, None],
            history_index: 0,
        }
    }

    /// 上传 uniform 到 GPU
    pub fn update_uniform(&self, queue: &Queue, uniform: &TaaUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// 创建当前帧 bind group
    ///
    /// # 参数
    /// - `current_view`: 当前帧 HDR view（带 jitter 渲染的场景）
    /// - `history_view`: 历史帧 view（即 `history_view()` 返回值）
    /// - `depth_view`: 深度纹理 view（Depth32Float）
    pub fn create_bind_group(
        &self,
        device: &Device,
        current_view: &TextureView,
        history_view: &TextureView,
        depth_view: &TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("taa bind group"),
            layout: &self.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(current_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(history_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建历史帧纹理（Rgba16Float，与 HDR 一致）
    ///
    /// usage: RENDER_ATTACHMENT（TAA 写入）+ TEXTURE_BINDING（下一帧采样）+ COPY
    pub fn create_history_texture(device: &Device, width: u32, height: u32, label: &str) -> Texture {
        let safe_w = width.max(1);
        let safe_h = height.max(1);
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: safe_w,
                height: safe_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// 初始化历史帧纹理（ping-pong 双缓冲）
    pub fn init_history(&mut self, device: &Device, width: u32, height: u32) {
        let tex0 = Self::create_history_texture(device, width, height, "taa history 0");
        let tex1 = Self::create_history_texture(device, width, height, "taa history 1");
        let view0 = tex0.create_view(&wgpu::TextureViewDescriptor::default());
        let view1 = tex1.create_view(&wgpu::TextureViewDescriptor::default());
        self.history_textures = [Some(tex0), Some(tex1)];
        self.history_views = [Some(view0), Some(view1)];
        self.history_index = 0;
    }

    /// resize 时重建历史帧（历史信息丢失，可接受）
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        self.init_history(device, width, height);
    }

    /// 执行 TAA resolve（全屏三角形，3 顶点）
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &BindGroup) {
        pass.set_pipeline(&self.taa_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 交换历史帧（帧末调用，翻转 history_index）
    pub fn swap_history(&mut self) {
        self.history_index = 1 - self.history_index;
    }

    /// 获取当前帧要读的历史帧 view（用于 create_bind_group 的 history_view 参数）
    pub fn history_view(&self) -> Option<&TextureView> {
        self.history_views.get(self.history_index as usize)?.as_ref()
    }

    /// 获取当前帧 TAA 输出 view（即新的历史帧，TAA pass 的 color attachment）
    ///
    /// bloom/tonemap 应使用此 view 作为输入。
    pub fn current_output_view(&self) -> Option<&TextureView> {
        let write_idx = (1 - self.history_index) as usize;
        self.history_views.get(write_idx)?.as_ref()
    }
}

// ============================================================
// Jitter 生成函数
// ============================================================

/// 生成 Halton 序列抖动值
///
/// 返回 [0, 1) 范围的 quasi-random 值。`index` 从 1 开始有意义（index=0 返回 0）。
///
/// # 已知值
/// - halton(1, 2) = 0.5
/// - halton(2, 2) = 0.25
/// - halton(3, 2) = 0.75
/// - halton(1, 3) = 1/3
/// - halton(2, 3) = 2/3
pub fn halton_sequence(index: u32, base: u32) -> f32 {
    if base == 0 {
        return 0.0;
    }
    let mut f = 1.0_f32;
    let mut r = 0.0_f32;
    let mut i = index;
    let b = base as f32;
    while i > 0 {
        f /= b;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

/// 获取帧的 jitter 偏移（返回 [-0.5, 0.5] 范围的 xy）
///
/// 使用 Halton(2) 和 Halton(3) 序列，分别作为 x/y 方向抖动。
/// `frame_index` 从 0 开始。
pub fn frame_jitter(frame_index: u32) -> [f32; 2] {
    let x = halton_sequence(frame_index + 1, 2) - 0.5;
    let y = halton_sequence(frame_index + 1, 3) - 0.5;
    [x, y]
}

/// 应用 jitter 到投影矩阵（返回带 jitter 的投影矩阵）
///
/// jitter 为像素空间偏移 [-0.5, 0.5]，转换为 NDC 偏移后以平移矩阵左乘投影矩阵，
/// 使 clip-space 结果偏移 (jx/width, jy/height)。
///
/// `jitter = [0, 0]` 时直接返回原矩阵（early return，精确相等）。
pub fn apply_jitter_to_projection(proj: Mat4, jitter: [f32; 2], width: f32, height: f32) -> Mat4 {
    if jitter[0] == 0.0 && jitter[1] == 0.0 {
        return proj;
    }
    let safe_w = width.max(1.0);
    let safe_h = height.max(1.0);
    let ndc_x = jitter[0] / safe_w;
    let ndc_y = jitter[1] / safe_h;
    // 平移矩阵左乘投影：T * P * v = T * clip = clip + (ndc_x, ndc_y, 0, 0)
    let jitter_mat = Mat4::from_translation(Vec3::new(ndc_x, ndc_y, 0.0));
    jitter_mat * proj
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_size() {
        // 4 × mat4 (64) + 3 × vec4 (16) = 256 + 48 = 304 bytes（16 的倍数）
        assert_eq!(std::mem::size_of::<TaaUniform>(), 304);
    }

    #[test]
    fn uniform_default() {
        let u = TaaUniform::default();
        assert_eq!(u.jitter, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(u.params, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(u.screen_size, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(u.view_proj, [[0.0; 4]; 4]);
        assert_eq!(u.view_proj_jitter, [[0.0; 4]; 4]);
        assert_eq!(u.prev_view_proj, [[0.0; 4]; 4]);
        assert_eq!(u.view_inv, [[0.0; 4]; 4]);
    }

    #[test]
    fn uniform_field_offsets() {
        // 验证 #[repr(C)] 内存布局
        assert_eq!(std::mem::offset_of!(TaaUniform, view_proj), 0);
        assert_eq!(std::mem::offset_of!(TaaUniform, view_proj_jitter), 64);
        assert_eq!(std::mem::offset_of!(TaaUniform, prev_view_proj), 128);
        assert_eq!(std::mem::offset_of!(TaaUniform, view_inv), 192);
        assert_eq!(std::mem::offset_of!(TaaUniform, jitter), 256);
        assert_eq!(std::mem::offset_of!(TaaUniform, params), 272);
        assert_eq!(std::mem::offset_of!(TaaUniform, screen_size), 288);
    }

    #[test]
    fn halton_known_values() {
        // Halton base-2
        assert!((halton_sequence(1, 2) - 0.5).abs() < 1e-6);
        assert!((halton_sequence(2, 2) - 0.25).abs() < 1e-6);
        assert!((halton_sequence(3, 2) - 0.75).abs() < 1e-6);
        assert!((halton_sequence(4, 2) - 0.125).abs() < 1e-6);
        // Halton base-3
        assert!((halton_sequence(1, 3) - 1.0 / 3.0).abs() < 1e-6);
        assert!((halton_sequence(2, 3) - 2.0 / 3.0).abs() < 1e-6);
        assert!((halton_sequence(3, 3) - 1.0 / 9.0).abs() < 1e-6);
        // index=0 返回 0
        assert!((halton_sequence(0, 2) - 0.0).abs() < 1e-6);
        // base=0 安全返回 0
        assert!((halton_sequence(1, 0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn frame_jitter_range() {
        for i in 0..32 {
            let j = frame_jitter(i);
            assert!(j[0] >= -0.5 && j[0] <= 0.5, "x out of range at {}: {}", i, j[0]);
            assert!(j[1] >= -0.5 && j[1] <= 0.5, "y out of range at {}: {}", i, j[1]);
        }
        // frame_jitter(0) = [halton(1,2)-0.5, halton(1,3)-0.5] = [0.0, -1/6]
        let j0 = frame_jitter(0);
        assert!((j0[0] - 0.0).abs() < 1e-6);
        assert!((j0[1] - (-1.0 / 6.0)).abs() < 1e-6);
    }

    #[test]
    fn apply_jitter_zero_returns_original() {
        let proj = Mat4::IDENTITY;
        let result = apply_jitter_to_projection(proj, [0.0, 0.0], 1920.0, 1080.0);
        // jitter=[0,0] early return，精确相等
        assert_eq!(result.to_cols_array(), proj.to_cols_array());
    }

    #[test]
    fn apply_jitter_nonzero_modifies_matrix() {
        let proj = Mat4::IDENTITY;
        let result = apply_jitter_to_projection(proj, [0.5, -0.5], 1920.0, 1080.0);
        // 非零 jitter 应改变矩阵（平移列变化）
        assert_ne!(result.to_cols_array(), proj.to_cols_array());
    }

    #[test]
    fn history_index_swap_logic() {
        // 模拟 ping-pong swap 逻辑（无需 GPU）
        let mut idx: u32 = 0;
        // 初始：读 0，写 1
        assert_eq!(idx, 0);
        assert_eq!(1 - idx, 1);
        // swap 后：读 1，写 0
        idx = 1 - idx;
        assert_eq!(idx, 1);
        assert_eq!(1 - idx, 0);
        // 再 swap：读 0，写 1
        idx = 1 - idx;
        assert_eq!(idx, 0);
        assert_eq!(1 - idx, 1);
    }

    #[test]
    fn taa_uniform_new_builder() {
        let vp = [[1.0; 4]; 4];
        let pvp = [[2.0; 4]; 4];
        let vi = [[3.0; 4]; 4];
        let u = TaaUniform::new(vp, pvp, vi, [0.25, -0.25], [0.1, 0.1], 1920.0, 1080.0);
        assert_eq!(u.view_proj, vp);
        assert_eq!(u.view_proj_jitter, vp);
        assert_eq!(u.prev_view_proj, pvp);
        assert_eq!(u.view_inv, vi);
        assert_eq!(u.jitter, [0.25, -0.25, 0.1, 0.1]);
        assert_eq!(u.params, [0.1, 100.0, 1.0, 0.0]);
        assert_eq!(u.screen_size, [1920.0, 1080.0, 1.0 / 1920.0, 1.0 / 1080.0]);
    }
}
