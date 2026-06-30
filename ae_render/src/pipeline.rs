//! 渲染管线缓存

use crate::material::MaterialType;
use crate::mesh::Vertex;
use crate::shader::{ShaderId, ShaderLibrary};
use hashbrown::HashMap;
use wgpu::{Device, RenderPipeline, RenderPipelineDescriptor};

/// 管线配置键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub material_type: MaterialType,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: wgpu::TextureFormat,
    pub sample_count: u32,
    pub two_sided: bool,
    pub wireframe: bool,
}

/// 管线缓存
pub struct RenderPipelineCache {
    pipelines: HashMap<PipelineKey, RenderPipeline>,
    shaders: ShaderLibrary,
}

impl Default for RenderPipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderPipelineCache {
    pub fn new() -> Self {
        Self { pipelines: HashMap::new(), shaders: ShaderLibrary::new() }
    }

    pub fn get(
        &mut self,
        device: &Device,
        key: PipelineKey,
        camera_layout: &wgpu::BindGroupLayout,
        model_layout: &wgpu::BindGroupLayout,
        material_layout: &wgpu::BindGroupLayout,
    ) -> &RenderPipeline {
        self.pipelines.entry(key).or_insert_with(|| {
            create_pipeline(
                device,
                &mut self.shaders,
                key,
                camera_layout,
                model_layout,
                material_layout,
            )
        })
    }

    pub fn clear(&mut self) {
        self.pipelines.clear();
    }

    pub fn len(&self) -> usize {
        self.pipelines.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

fn create_pipeline(
    device: &Device,
    shaders: &mut ShaderLibrary,
    key: PipelineKey,
    camera_layout: &wgpu::BindGroupLayout,
    model_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> RenderPipeline {
    let (shader_id, entry_point) = match key.material_type {
        MaterialType::Pbr | MaterialType::Unlit => (ShaderId::Pbr, "vs_main"),
        MaterialType::Wireframe => (ShaderId::Pbr, "vs_main"),
        MaterialType::Sky => (ShaderId::Pbr, "vs_main"),
        MaterialType::Terrain => (ShaderId::Pbr, "vs_main"),
        MaterialType::PostProcess => (ShaderId::Pbr, "vs_main"),
    };

    let shader = shaders.get(device, shader_id);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render pipeline layout"),
        bind_group_layouts: &[camera_layout, model_layout, material_layout],
        push_constant_ranges: &[],
    });

    let targets = vec![Some(wgpu::ColorTargetState {
        format: key.color_format,
        blend: Some(if key.material_type == MaterialType::Unlit {
            wgpu::BlendState::ALPHA_BLENDING
        } else {
            wgpu::BlendState::REPLACE
        }),
        write_mask: wgpu::ColorWrites::ALL,
    })];

    let _ = targets.len();

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("render pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Vertex::LAYOUT],
        },
        primitive: wgpu::PrimitiveState {
            topology: if key.wireframe {
                wgpu::PrimitiveTopology::LineList
            } else {
                wgpu::PrimitiveTopology::TriangleList
            },
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: if key.two_sided { None } else { Some(wgpu::Face::Back) },
            unclipped_depth: false,
            polygon_mode: if key.wireframe {
                wgpu::PolygonMode::Line
            } else {
                wgpu::PolygonMode::Fill
            },
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: key.depth_format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: key.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_key_equality() {
        let k1 = PipelineKey {
            material_type: MaterialType::Pbr,
            color_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            depth_format: wgpu::TextureFormat::Depth32Float,
            sample_count: 1,
            two_sided: false,
            wireframe: false,
        };
        let k2 = k1;
        let k3 = PipelineKey { two_sided: true, ..k1 };
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn cache_starts_empty() {
        let cache = RenderPipelineCache::new();
        assert!(cache.is_empty());
    }
}
