use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGenRequest {
    pub prompt: String,
    pub output_format: ModelFormat,
    pub resolution: u32,
    pub style: GenerationStyle,
    pub seed: Option<u64>,
    pub num_inference_steps: u32,
    pub guidance_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    Obj,
    Glb,
    Fbx,
    Ply,
    Usd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationStyle {
    Realistic,
    Stylized,
    LowPoly,
    Voxel,
    Cad,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGenResult {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub materials: Vec<MaterialSlot>,
    pub metadata: ModelMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSlot {
    pub name: String,
    pub start_index: u32,
    pub index_count: u32,
    pub albedo: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounding_box: [[f32; 3]; 2],
    pub generation_time_ms: u64,
    pub model_name: String,
    pub lod_levels: u32,
}

pub struct MeshValidator;

impl MeshValidator {
    pub fn validate(result: &ModelGenResult) -> ValidationReport {
        let mut issues = Vec::new();
        if result.vertices.is_empty() {
            issues.push("empty vertex buffer".to_string());
        }
        if !result.indices.len().is_multiple_of(3) {
            issues.push(format!("index count {} not divisible by 3", result.indices.len()));
        }
        let max_idx = result.indices.iter().copied().max().unwrap_or(0);
        if max_idx as usize >= result.vertices.len() {
            issues.push(format!("max index {} >= vertex count {}", max_idx, result.vertices.len()));
        }
        for (i, tri) in result.indices.chunks(3).enumerate() {
            if tri.len() == 3 && (tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2]) {
                issues.push(format!("degenerate triangle at index {}", i * 3));
            }
        }
        let has_non_manifold = false;
        let mut bbox_min = [f32::MAX; 3];
        let mut bbox_max = [f32::MIN; 3];
        for v in &result.vertices {
            for i in 0..3 {
                bbox_min[i] = bbox_min[i].min(v[i]);
                bbox_max[i] = bbox_max[i].max(v[i]);
            }
        }
        ValidationReport {
            valid: issues.is_empty(),
            issues,
            vertex_count: result.vertices.len() as u32,
            triangle_count: (result.indices.len() / 3) as u32,
            bounding_box: [bbox_min, bbox_max],
            has_non_manifold,
            uv_channels: if result.uvs.is_empty() { 0 } else { 1 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<String>,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounding_box: [[f32; 3]; 2],
    pub has_non_manifold: bool,
    pub uv_channels: u32,
}

pub struct LodGenerator;

impl LodGenerator {
    pub fn generate_lods(
        vertices: &[[f32; 3]],
        indices: &[u32],
        levels: u32,
    ) -> Vec<ModelGenResult> {
        let mut lods = Vec::with_capacity(levels as usize);
        let mut current_verts = vertices.to_vec();
        let mut current_idx = indices.to_vec();
        for level in 0..levels {
            let ratio = 1.0 - (level as f32 / levels as f32) * 0.75;
            let target_tris = ((current_idx.len() / 3) as f32 * ratio) as usize;
            if target_tris < current_idx.len() / 3 && current_idx.len() >= 6 {
                let simplified = Self::simplify_mesh(&current_verts, &current_idx, target_tris);
                current_verts = simplified.vertices;
                current_idx = simplified.indices;
            }
            lods.push(ModelGenResult {
                vertices: current_verts.clone(),
                normals: vec![[0.0, 1.0, 0.0]; current_verts.len()],
                uvs: vec![[0.0, 0.0]; current_verts.len()],
                indices: current_idx.clone(),
                materials: vec![],
                metadata: ModelMetadata {
                    vertex_count: current_verts.len() as u32,
                    triangle_count: (current_idx.len() / 3) as u32,
                    bounding_box: [[0.0; 3]; 2],
                    generation_time_ms: 0,
                    model_name: format!("LOD{}", level),
                    lod_levels: 1,
                },
            });
        }
        lods
    }

    fn simplify_mesh(vertices: &[[f32; 3]], indices: &[u32], target_tris: usize) -> ModelGenResult {
        let current_tris = indices.len() / 3;
        if current_tris <= target_tris {
            return ModelGenResult {
                vertices: vertices.to_vec(),
                normals: vec![[0.0, 1.0, 0.0]; vertices.len()],
                uvs: vec![[0.0, 0.0]; vertices.len()],
                indices: indices.to_vec(),
                materials: vec![],
                metadata: ModelMetadata {
                    vertex_count: vertices.len() as u32,
                    triangle_count: current_tris as u32,
                    bounding_box: [[0.0; 3]; 2],
                    generation_time_ms: 0,
                    model_name: "simplified".into(),
                    lod_levels: 1,
                },
            };
        }
        let removal_count = (current_tris - target_tris) * 3;
        let keep = indices.len().saturating_sub(removal_count);
        ModelGenResult {
            vertices: vertices.to_vec(),
            normals: vec![[0.0, 1.0, 0.0]; vertices.len()],
            uvs: vec![[0.0, 0.0]; vertices.len()],
            indices: indices[..keep].to_vec(),
            materials: vec![],
            metadata: ModelMetadata {
                vertex_count: vertices.len() as u32,
                triangle_count: (keep / 3) as u32,
                bounding_box: [[0.0; 3]; 2],
                generation_time_ms: 0,
                model_name: "simplified".into(),
                lod_levels: 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_mesh() {
        let result = ModelGenResult {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.0, 0.0]; 3],
            indices: vec![0, 1, 2],
            materials: vec![],
            metadata: ModelMetadata {
                vertex_count: 3,
                triangle_count: 1,
                bounding_box: [[0.0; 3]; 2],
                generation_time_ms: 0,
                model_name: "test".into(),
                lod_levels: 1,
            },
        };
        let report = MeshValidator::validate(&result);
        assert!(report.valid);
        assert_eq!(report.triangle_count, 1);
    }

    #[test]
    fn test_validate_empty_mesh() {
        let result = ModelGenResult {
            vertices: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![],
            materials: vec![],
            metadata: ModelMetadata {
                vertex_count: 0,
                triangle_count: 0,
                bounding_box: [[0.0; 3]; 2],
                generation_time_ms: 0,
                model_name: "empty".into(),
                lod_levels: 1,
            },
        };
        let report = MeshValidator::validate(&result);
        assert!(!report.valid);
    }

    #[test]
    fn test_lod_generation() {
        let vertices: Vec<[f32; 3]> = (0..12)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 12.0;
                [angle.cos(), 0.0, angle.sin()]
            })
            .collect();
        let indices: Vec<u32> = (0..12)
            .flat_map(|i| {
                let next = (i + 1) % 12;
                vec![i as u32, next as u32, 6]
            })
            .collect();
        let lods = LodGenerator::generate_lods(&vertices, &indices, 3);
        assert_eq!(lods.len(), 3);
        assert!(lods[0].metadata.triangle_count >= lods[2].metadata.triangle_count);
    }
}
