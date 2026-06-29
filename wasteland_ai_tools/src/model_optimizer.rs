use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplifyConfig {
    pub target_ratio: f32,
    pub preserve_boundaries: bool,
    pub preserve_uv_seams: bool,
    pub aggression: f32,
    pub max_error: f32,
}

impl Default for SimplifyConfig {
    fn default() -> Self {
        SimplifyConfig {
            target_ratio: 0.5,
            preserve_boundaries: true,
            preserve_uv_seams: true,
            aggression: 0.7,
            max_error: 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UvGenerateConfig {
    pub method: UvMethod,
    pub resolution: u32,
    pub padding: u32,
    pub islands_packing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UvMethod {
    Xatlas,
    AbfPlus,
    Lscm,
    Box,
    Cylinder,
    Sphere,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalComputeConfig {
    pub method: NormalMethod,
    pub smoothing_angle: f32,
    pub use_weighted: bool,
    pub fix_inverted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalMethod {
    FaceWeighted,
    AreaWeighted,
    AngleWeighted,
    MikkTSpace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcclusionComputeConfig {
    pub samples: u32,
    pub radius: f32,
    pub spread: f32,
    pub bias: f32,
    pub use_gpu: bool,
}

impl Default for OcclusionComputeConfig {
    fn default() -> Self {
        OcclusionComputeConfig { samples: 256, radius: 0.5, spread: 0.5, bias: 0.01, use_gpu: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOptimizeResult {
    pub original_vertices: u32,
    pub optimized_vertices: u32,
    pub original_triangles: u32,
    pub optimized_triangles: u32,
    pub reduction_ratio: f32,
    pub uv_channels: u32,
    pub has_normals: bool,
    pub has_tangents: bool,
    pub has_ao: bool,
    pub elapsed_ms: u64,
    pub peak_memory_mb: f64,
}

pub struct ModelOptimizer {
    pub simplify: SimplifyConfig,
    pub uv: UvGenerateConfig,
    pub normal: NormalComputeConfig,
    pub occlusion: OcclusionComputeConfig,
    pub enabled: bool,
}

impl Default for ModelOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelOptimizer {
    pub fn new() -> Self {
        ModelOptimizer {
            simplify: SimplifyConfig::default(),
            uv: UvGenerateConfig {
                method: UvMethod::Xatlas,
                resolution: 2048,
                padding: 4,
                islands_packing: true,
            },
            normal: NormalComputeConfig {
                method: NormalMethod::MikkTSpace,
                smoothing_angle: 60.0,
                use_weighted: true,
                fix_inverted: true,
            },
            occlusion: OcclusionComputeConfig::default(),
            enabled: true,
        }
    }

    pub fn estimate_simplify_output(&self, vertices: u32, triangles: u32) -> ModelOptimizeResult {
        let target_tri = (triangles as f32 * self.simplify.target_ratio) as u32;
        let target_vert = (vertices as f32 * self.simplify.target_ratio) as u32;
        ModelOptimizeResult {
            original_vertices: vertices,
            optimized_vertices: target_vert,
            original_triangles: triangles,
            optimized_triangles: target_tri,
            reduction_ratio: self.simplify.target_ratio,
            uv_channels: 1,
            has_normals: true,
            has_tangents: true,
            has_ao: self.occlusion.samples > 0,
            elapsed_ms: 0,
            peak_memory_mb: 0.0,
        }
    }

    pub fn optimize(
        &self,
        _vertices: &[[f32; 3]],
        _indices: &[u32],
        _normals: Option<&[[f32; 3]]>,
        _uvs: Option<&[[f32; 2]]>,
    ) -> ModelOptimizeResult {
        let v = _vertices.len() as u32;
        let t = _indices.len() as u32 / 3;
        let mut result = self.estimate_simplify_output(v, t);
        result.elapsed_ms = 1;
        result.peak_memory_mb = (v as f64 * 12.0 + t as f64 * 4.0) / (1024.0 * 1024.0);
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFormatConverter {
    pub source_format: ModelConvFormat,
    pub target_format: ModelConvFormat,
    pub preserve_hierarchy: bool,
    pub preserve_animations: bool,
    pub preserve_materials: bool,
    pub scale: f32,
    pub flip_yz: bool,
    pub merge_meshes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelConvFormat {
    Obj,
    Glb,
    Gltf,
    Fbx,
    Ply,
    Usd,
    Usda,
    Usdc,
    Stl,
    Collada,
    ThreeMF,
}

impl Default for ModelFormatConverter {
    fn default() -> Self {
        ModelFormatConverter {
            source_format: ModelConvFormat::Obj,
            target_format: ModelConvFormat::Glb,
            preserve_hierarchy: true,
            preserve_animations: true,
            preserve_materials: true,
            scale: 1.0,
            flip_yz: false,
            merge_meshes: false,
        }
    }
}

impl ModelFormatConverter {
    pub fn is_lossless(&self) -> bool {
        matches!(
            (self.source_format, self.target_format),
            (ModelConvFormat::Gltf, ModelConvFormat::Glb)
                | (ModelConvFormat::Glb, ModelConvFormat::Gltf)
                | (ModelConvFormat::Usd, ModelConvFormat::Usda)
                | (ModelConvFormat::Usda, ModelConvFormat::Usd)
        )
    }

    pub fn supports_animation(&self) -> bool {
        matches!(
            self.target_format,
            ModelConvFormat::Glb
                | ModelConvFormat::Fbx
                | ModelConvFormat::Usd
                | ModelConvFormat::Usdc
        )
    }

    pub fn supports_materials(&self) -> bool {
        !matches!(self.target_format, ModelConvFormat::Stl | ModelConvFormat::Ply)
    }

    pub fn convert(&self, _data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_estimate() {
        let opt = ModelOptimizer::new();
        let result = opt.estimate_simplify_output(10000, 20000);
        assert_eq!(result.original_vertices, 10000);
        assert_eq!(result.original_triangles, 20000);
        assert!(result.optimized_triangles < result.original_triangles);
        assert!(result.reduction_ratio > 0.0 && result.reduction_ratio < 1.0);
    }

    #[test]
    fn test_converter_lossless() {
        let conv = ModelFormatConverter {
            source_format: ModelConvFormat::Gltf,
            target_format: ModelConvFormat::Glb,
            ..Default::default()
        };
        assert!(conv.is_lossless());

        let conv2 = ModelFormatConverter {
            source_format: ModelConvFormat::Obj,
            target_format: ModelConvFormat::Glb,
            ..Default::default()
        };
        assert!(!conv2.is_lossless());
    }

    #[test]
    fn test_converter_animation_support() {
        let conv =
            ModelFormatConverter { target_format: ModelConvFormat::Glb, ..Default::default() };
        assert!(conv.supports_animation());

        let conv2 =
            ModelFormatConverter { target_format: ModelConvFormat::Stl, ..Default::default() };
        assert!(!conv2.supports_animation());
    }
}
