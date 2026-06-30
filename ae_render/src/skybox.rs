//! Skybox Renderer: Procedural Sky + Sun Uniform
//!
//! Provides:
//! - Procedural sky gradient (no external texture dependency)
//! - Sun uniform (direction, color, intensity) shared with lighting
//! - Fullscreen triangle rendering (no vertex buffer)
//!
//! Design:
//! - Sky drawn as fullscreen triangle at depth=1 (behind everything)
//! - Sun uniform exposes sun direction/intensity to other renderers
//! - Gradient sky color + sun disk glow approximation

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline};

/// Sun Uniform: sun direction + color + intensity
///
/// 16 bytes (vec4 direction) + 16 bytes (vec4 color) = 32 bytes,
/// 符合 WGSL 16-byte 对齐。
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SunUniform {
    /// xyz: sun direction (normalized), w: intensity
    pub direction: [f32; 4],
    /// rgb: sun color, a: unused
    pub color: [f32; 4],
}

impl SunUniform {
    /// 构造 SunUniform，自动归一化方向向量
    pub fn new(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        let len = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
        .sqrt();
        let dir = if len > 1e-6 {
            [direction[0] / len, direction[1] / len, direction[2] / len]
        } else {
            [0.0, -1.0, 0.0]
        };
        Self {
            direction: [dir[0], dir[1], dir[2], intensity],
            color: [color[0], color[1], color[2], 0.0],
        }
    }

    /// 默认正午阳光：从上方斜照，暖白色
    pub fn default_sun() -> Self {
        Self::new([0.5, -1.0, 0.3], [1.0, 0.95, 0.85], 1.5)
    }
}

/// SkyboxRenderer: 程序化天空 + 太阳
pub struct SkyboxRenderer {
    pub pipeline: RenderPipeline,
    pub sun_buffer: Buffer,
    pub sun_bind_group: BindGroup,
    pub sun_layout: BindGroupLayout,
}

impl SkyboxRenderer {
    /// 创建天空渲染器
    pub fn new(device: &Device, color_format: wgpu::TextureFormat) -> Self {
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
            bind_group_layouts: &[&sun_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skybox shader"),
            source: wgpu::ShaderSource::Wgsl(SKY_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skybox pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
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
            sun_buffer,
            sun_bind_group,
            sun_layout,
        }
    }

    /// 上传太阳 uniform 到 GPU
    pub fn update_sun(&self, queue: &Queue, sun: &SunUniform) {
        queue.write_buffer(&self.sun_buffer, 0, bytemuck::cast_slice(&[*sun]));
    }

    /// 在渲染 pass 中绘制天空（应在场景绘制前调用）
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.sun_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// 获取 sun bind group（供其他 renderer 共享）
    pub fn sun_bind_group(&self) -> &BindGroup {
        &self.sun_bind_group
    }

    /// 获取 sun layout（供其他 pipeline 共享）
    pub fn sun_layout(&self) -> &BindGroupLayout {
        &self.sun_layout
    }
}

const SKY_SHADER: &str = r#"
struct SunUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> sun: SunUniform;

@vertex
fn vs_sky(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    // 全屏三角形：3 个顶点覆盖 NDC 全屏，无需 vertex buffer
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    // z=1.0 保证天空在最远处（depth_write_enabled=false, depth_compare=LessEqual）
    return vec4<f32>(positions[vid], 1.0, 1.0);
}

@fragment
fn fs_sky(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // 基于屏幕 Y 的天空渐变
    let dims = vec2<f32>(800.0, 600.0);
    let uv = pos.xy / dims;

    // 天空颜色渐变：地平线浅蓝 → 天顶深蓝
    let horizon = vec3<f32>(0.7, 0.85, 1.0);
    let zenith = vec3<f32>(0.25, 0.45, 0.85);
    let sky_color = mix(horizon, zenith, clamp(uv.y, 0.0, 1.0));

    // 太阳盘片近似：根据屏幕坐标与太阳方向投影的距离
    let sun_dir = normalize(sun.direction.xyz);
    // 将太阳方向投影到屏幕（简化：用方向 xy 作为屏幕位置偏移）
    let sun_screen = sun_dir.xy * 0.5 + vec2<f32>(0.5);
    let dist = distance(uv, sun_screen);
    let sun_disk = smoothstep(0.05, 0.02, dist);
    let sun_glow = smoothstep(0.3, 0.05, dist) * 0.5;

    let color = sky_color
        + sun.color.rgb * sun.direction.w * sun_disk
        + sun.color.rgb * sun.direction.w * sun_glow * 0.3;

    return vec4<f32>(color, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_uniform_size() {
        // 2 × vec4 = 32 bytes
        assert_eq!(std::mem::size_of::<SunUniform>(), 32);
    }

    #[test]
    fn sun_default_has_normalized_direction() {
        let sun = SunUniform::default_sun();
        let dir = sun.direction;
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "direction should be normalized, got len={}",
            len
        );
    }

    #[test]
    fn sun_zero_direction_uses_default() {
        let sun = SunUniform::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 1.0);
        assert_eq!(sun.direction[0], 0.0);
        assert_eq!(sun.direction[1], -1.0);
        assert_eq!(sun.direction[2], 0.0);
    }

    #[test]
    fn sun_intensity_stored_in_w() {
        let sun = SunUniform::new([0.0, -1.0, 0.0], [1.0, 1.0, 1.0], 2.5);
        assert!((sun.direction[3] - 2.5).abs() < 1e-4);
    }

    #[test]
    fn shader_has_vertex_and_fragment() {
        assert!(SKY_SHADER.contains("@vertex"));
        assert!(SKY_SHADER.contains("@fragment"));
        assert!(SKY_SHADER.contains("vs_sky"));
        assert!(SKY_SHADER.contains("fs_sky"));
    }

    #[test]
    fn shader_references_sun_uniform() {
        assert!(SKY_SHADER.contains("SunUniform"));
        assert!(SKY_SHADER.contains("sun.direction"));
    }
}
