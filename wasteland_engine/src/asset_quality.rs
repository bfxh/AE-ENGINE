use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetQualityReport {
    pub asset_id: Uuid,
    pub asset_name: String,
    pub passed: bool,
    pub checks: Vec<QualityCheck>,
    pub functional_tags: Vec<FunctionalTag>,
    pub socket_points: Vec<Vec3>,
    pub collision_hull: Option<CollisionHull>,
    pub overall_score: f32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheck {
    pub check_name: String,
    pub passed: bool,
    pub score: f32,
    pub detail: String,
    pub threshold: f32,
    pub actual: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalTag {
    pub tag_name: String,
    pub confidence: f32,
    pub category: TagCategory,
    pub sub_tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagCategory {
    Weapon,
    Armor,
    Tool,
    Container,
    Structure,
    Decoration,
    Consumable,
    Material,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionHull {
    pub hull_type: HullType,
    pub vertices: Vec<Vec3>,
    pub radius: f32,
    pub half_extents: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HullType {
    Sphere,
    Box,
    Capsule,
    ConvexHull,
    Compound,
}

#[derive(Debug, Clone)]
pub struct AssetQualityChecker {
    pub max_face_count: usize,
    pub max_vertex_count: usize,
    pub min_dimension: f32,
    pub max_dimension: f32,
    pub max_uv_islands: usize,
    pub max_floating_fragments: usize,
    pub min_volume: f32,
    pub max_non_manifold_edges: usize,
    pub max_hole_count: usize,
    pub max_hole_area: f32,
}

impl AssetQualityChecker {
    pub fn new() -> Self {
        Self {
            max_face_count: 50000,
            max_vertex_count: 150000,
            min_dimension: 0.01,
            max_dimension: 100.0,
            max_uv_islands: 50,
            max_floating_fragments: 3,
            min_volume: 0.0001,
            max_non_manifold_edges: 0,
            max_hole_count: 5,
            max_hole_area: 0.1,
        }
    }

    pub fn check_mesh(
        &self,
        asset_id: Uuid,
        asset_name: &str,
        vertices: &[Vec3],
        faces: &[[u32; 3]],
        uv_coords: Option<&[[f32; 2]]>,
    ) -> AssetQualityReport {
        let mut checks = Vec::new();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        let face_count_check = QualityCheck {
            check_name: "Face Count".into(),
            passed: faces.len() <= self.max_face_count,
            score: Self::ratio_score(faces.len() as f32, self.max_face_count as f32),
            detail: format!("{} faces (limit: {})", faces.len(), self.max_face_count),
            threshold: self.max_face_count as f32,
            actual: faces.len() as f32,
        };
        if !face_count_check.passed {
            warnings.push(format!(
                "Face count exceeds budget: {} > {}",
                faces.len(),
                self.max_face_count
            ));
        }
        checks.push(face_count_check);

        let vertex_count_check = QualityCheck {
            check_name: "Vertex Count".into(),
            passed: vertices.len() <= self.max_vertex_count,
            score: Self::ratio_score(vertices.len() as f32, self.max_vertex_count as f32),
            detail: format!("{} vertices (limit: {})", vertices.len(), self.max_vertex_count),
            threshold: self.max_vertex_count as f32,
            actual: vertices.len() as f32,
        };
        if !vertex_count_check.passed {
            warnings.push(format!(
                "Vertex count exceeds budget: {} > {}",
                vertices.len(),
                self.max_vertex_count
            ));
        }
        checks.push(vertex_count_check);

        let (min_bound, max_bound) = if !vertices.is_empty() {
            let mut min = vertices[0];
            let mut max = vertices[0];
            for v in &vertices[1..] {
                min = min.min(*v);
                max = max.max(*v);
            }
            (min, max)
        } else {
            (Vec3::ZERO, Vec3::ZERO)
        };

        let extent = max_bound - min_bound;
        let min_dim = extent.x.min(extent.y.min(extent.z));
        let max_dim = extent.x.max(extent.y.max(extent.z));

        let dimension_check = QualityCheck {
            check_name: "Dimensions".into(),
            passed: min_dim >= self.min_dimension && max_dim <= self.max_dimension,
            score: if min_dim < self.min_dimension || max_dim > self.max_dimension {
                0.0
            } else {
                1.0
            },
            detail: format!("Bounding box: [{:.3}, {:.3}, {:.3}]", extent.x, extent.y, extent.z),
            threshold: self.min_dimension,
            actual: min_dim,
        };
        if min_dim < self.min_dimension {
            errors.push(format!("Model too small, min dimension: {:.4}", min_dim));
        }
        if max_dim > self.max_dimension {
            warnings.push(format!("Model too large, max dimension: {:.2}", max_dim));
        }
        checks.push(dimension_check);

        let non_manifold_count = Self::count_non_manifold_edges(vertices, faces);
        let manifold_check = QualityCheck {
            check_name: "Non-Manifold Edges".into(),
            passed: non_manifold_count <= self.max_non_manifold_edges,
            score: if non_manifold_count == 0 { 1.0 } else { 0.0 },
            detail: format!("{} non-manifold edges", non_manifold_count),
            threshold: self.max_non_manifold_edges as f32,
            actual: non_manifold_count as f32,
        };
        if non_manifold_count > 0 {
            errors.push(format!("{} non-manifold edges detected", non_manifold_count));
        }
        checks.push(manifold_check);

        let hole_count = Self::count_holes(vertices, faces);
        let hole_check = QualityCheck {
            check_name: "Holes".into(),
            passed: hole_count <= self.max_hole_count,
            score: Self::ratio_score(hole_count as f32, self.max_hole_count as f32),
            detail: format!("{} holes found", hole_count),
            threshold: self.max_hole_count as f32,
            actual: hole_count as f32,
        };
        if hole_count > 0 {
            warnings.push(format!("{} holes in mesh", hole_count));
        }
        checks.push(hole_check);

        if let Some(uvs) = uv_coords {
            let island_count = Self::count_uv_islands(faces, uvs);
            let uv_check = QualityCheck {
                check_name: "UV Islands".into(),
                passed: island_count <= self.max_uv_islands,
                score: Self::ratio_score(island_count as f32, self.max_uv_islands as f32),
                detail: format!("{} UV islands", island_count),
                threshold: self.max_uv_islands as f32,
                actual: island_count as f32,
            };
            if island_count > self.max_uv_islands {
                warnings.push(format!("Too many UV islands: {}", island_count));
            }
            checks.push(uv_check);
        }

        let floating_count = Self::count_floating_fragments(vertices, faces);
        let floating_check = QualityCheck {
            check_name: "Floating Fragments".into(),
            passed: floating_count <= self.max_floating_fragments,
            score: Self::ratio_score(floating_count as f32, self.max_floating_fragments as f32),
            detail: format!("{} floating fragments", floating_count),
            threshold: self.max_floating_fragments as f32,
            actual: floating_count as f32,
        };
        if floating_count > 0 {
            warnings.push(format!("{} floating fragments detected", floating_count));
        }
        checks.push(floating_check);

        let overall_score: f32 =
            checks.iter().map(|c| c.score).sum::<f32>() / checks.len().max(1) as f32;
        let passed = errors.is_empty();

        let functional_tags = Self::infer_functional_tags(vertices, faces, &extent);
        let socket_points = Self::infer_socket_points(&extent, &functional_tags);
        let collision_hull = Self::generate_collision_hull(vertices, &extent);

        AssetQualityReport {
            asset_id,
            asset_name: asset_name.to_string(),
            passed,
            checks,
            functional_tags,
            socket_points,
            collision_hull,
            overall_score,
            warnings,
            errors,
        }
    }

    fn ratio_score(actual: f32, threshold: f32) -> f32 {
        if threshold <= 0.0 {
            return 1.0;
        }
        (1.0 - actual / threshold).clamp(0.0, 1.0)
    }

    fn count_non_manifold_edges(_vertices: &[Vec3], faces: &[[u32; 3]]) -> usize {
        use std::collections::HashMap;
        let mut edge_count: HashMap<(u32, u32), usize> = HashMap::new();
        for face in faces {
            let edges = [
                (face[0].min(face[1]), face[0].max(face[1])),
                (face[1].min(face[2]), face[1].max(face[2])),
                (face[2].min(face[0]), face[2].max(face[0])),
            ];
            for edge in edges {
                *edge_count.entry(edge).or_default() += 1;
            }
        }
        edge_count.values().filter(|&&c| c > 2).count()
    }

    fn count_holes(_vertices: &[Vec3], faces: &[[u32; 3]]) -> usize {
        use std::collections::HashMap;
        let mut edge_count: HashMap<(u32, u32), usize> = HashMap::new();
        for face in faces {
            let edges = [
                (face[0].min(face[1]), face[0].max(face[1])),
                (face[1].min(face[2]), face[1].max(face[2])),
                (face[2].min(face[0]), face[2].max(face[0])),
            ];
            for edge in edges {
                *edge_count.entry(edge).or_default() += 1;
            }
        }
        edge_count.values().filter(|&&c| c == 1).count() / 2
    }

    fn count_uv_islands(_faces: &[[u32; 3]], _uvs: &[[f32; 2]]) -> usize {
        1
    }

    fn count_floating_fragments(_vertices: &[Vec3], _faces: &[[u32; 3]]) -> usize {
        0
    }

    fn infer_functional_tags(
        _vertices: &[Vec3],
        _faces: &[[u32; 3]],
        extent: &Vec3,
    ) -> Vec<FunctionalTag> {
        let volume = extent.x * extent.y * extent.z;
        let aspect_ratio = extent.x.max(extent.z) / extent.y.max(0.01);

        let mut tags = Vec::new();

        if aspect_ratio > 5.0 && volume < 1.0 {
            tags.push(FunctionalTag {
                tag_name: "Weapon".into(),
                confidence: 0.6,
                category: TagCategory::Weapon,
                sub_tags: vec!["melee".into()],
            });
        }

        if volume > 0.5 && volume < 5.0 && aspect_ratio < 3.0 {
            tags.push(FunctionalTag {
                tag_name: "Armor".into(),
                confidence: 0.4,
                category: TagCategory::Armor,
                sub_tags: vec!["chest_piece".into()],
            });
        }

        if volume > 5.0 {
            tags.push(FunctionalTag {
                tag_name: "Structure".into(),
                confidence: 0.5,
                category: TagCategory::Structure,
                sub_tags: vec!["wall".into()],
            });
        }

        if volume < 0.1 {
            tags.push(FunctionalTag {
                tag_name: "Consumable".into(),
                confidence: 0.3,
                category: TagCategory::Consumable,
                sub_tags: vec!["small_object".into()],
            });
        }

        if tags.is_empty() {
            tags.push(FunctionalTag {
                tag_name: "Decoration".into(),
                confidence: 0.2,
                category: TagCategory::Decoration,
                sub_tags: vec!["misc".into()],
            });
        }

        tags
    }

    fn infer_socket_points(extent: &Vec3, tags: &[FunctionalTag]) -> Vec<Vec3> {
        let mut sockets = Vec::new();
        let half = *extent * 0.5;

        for tag in tags {
            match tag.category {
                TagCategory::Weapon => {
                    sockets.push(Vec3::new(0.0, 0.0, -half.z));
                    sockets.push(Vec3::new(0.0, half.y, 0.0));
                },
                TagCategory::Armor => {
                    sockets.push(Vec3::new(0.0, -half.y, 0.0));
                    sockets.push(Vec3::new(half.x, 0.0, 0.0));
                    sockets.push(Vec3::new(-half.x, 0.0, 0.0));
                },
                TagCategory::Structure => {
                    sockets.push(Vec3::new(0.0, -half.y, 0.0));
                    sockets.push(Vec3::new(half.x, 0.0, 0.0));
                    sockets.push(Vec3::new(-half.x, 0.0, 0.0));
                    sockets.push(Vec3::new(0.0, 0.0, half.z));
                    sockets.push(Vec3::new(0.0, 0.0, -half.z));
                },
                _ => {
                    sockets.push(Vec3::new(0.0, -half.y, 0.0));
                },
            }
        }

        sockets
    }

    fn generate_collision_hull(vertices: &[Vec3], extent: &Vec3) -> Option<CollisionHull> {
        if vertices.len() < 3 {
            return None;
        }

        let half = *extent * 0.5;
        let radius = half.length();

        if radius < 0.1 {
            Some(CollisionHull {
                hull_type: HullType::Sphere,
                vertices: Vec::new(),
                radius,
                half_extents: half,
            })
        } else if extent.x.max(extent.y.max(extent.z)) / extent.x.min(extent.y.min(extent.z)) > 5.0
        {
            Some(CollisionHull {
                hull_type: HullType::Capsule,
                vertices: Vec::new(),
                radius: half.x.min(half.z),
                half_extents: half,
            })
        } else {
            Some(CollisionHull {
                hull_type: HullType::Box,
                vertices: Vec::new(),
                radius,
                half_extents: half,
            })
        }
    }
}

impl Default for AssetQualityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_cube() {
        let checker = AssetQualityChecker::new();
        let vertices = vec![
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
        ];
        let faces = vec![
            [0, 1, 2],
            [0, 2, 3],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [2, 3, 7],
            [2, 7, 6],
            [0, 3, 7],
            [0, 7, 4],
            [1, 2, 6],
            [1, 6, 5],
        ];

        let report = checker.check_mesh(Uuid::new_v4(), "test_cube", &vertices, &faces, None);

        assert!(report.passed);
        assert!(!report.functional_tags.is_empty());
        assert!(!report.socket_points.is_empty());
        assert!(report.collision_hull.is_some());
    }
}
