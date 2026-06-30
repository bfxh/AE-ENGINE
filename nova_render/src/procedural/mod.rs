//! 程序化生成系统（从 v1 ae_render 移植 + 重构为 nova 架构）
//!
//! 模块组织（对应 v1 procedural/）：
//! - `building`：参数化废土建筑（窗框/人字屋顶/阳台/钢筋 + 承重图 + 老化状态）
//! - `character`：12 部位 NPC 程序化生成 + 简化骨骼层级 + 蒙皮权重
//! - `creature`：母巢子实体 8 种异形生物（虫族/追猎者/碎脊者/锈骑士/蜂群/臃肿者/窃听者/编织者）
//! - `damage`：四级损伤系统 + 生理地图 + 血液流动 + 骨骼破坏
//!
//! 与 v1 的差异：
//! - 输出 `assets::MeshData` 而非 `(Vec<Vertex>, Vec<u32>)` 元组
//! - 错误用 `anyhow::Result`（保留 v1 panic 风格的内部断言用于测试）
//! - 随机源统一用 `rand` crate（v1 用自研 `MorphMutation` 哈希）
//! - 通过 `ProceduralGenerator` trait + `GeneratorParams` 统一入口

pub mod building;
pub mod character;
pub mod creature;
pub mod damage;

pub use building::{
    BalconyParams, BuildingGenerator, BuildingParams, BuildingSemantics, BuildingType,
    DecayState, FunctionZone, GableRoofParams, LoadBearingGraph, RebarParams, RebarPattern,
    StructuralElement, StructuralType, WindowParams, ZoneType,
};
pub use character::{
    BiologicalMaterial, BodyPlan, CharacterGenerator, Gender, HumanoidSkeleton, MorphMutation,
    MorphParams, MorphTemplate, NpcBodyParams, NpcBodyGenerator, SizeClass, Skeleton, SkinWeights,
    Bone, BoneId, JointTransform,
};
pub use creature::{
    all_templates, bloated_template, crusher_template, hunter_template, listener_template,
    rust_knight_template, stalker_template, swarm_template, weaver_template, CreatureGenerator,
};
pub use damage::{
    BleedingSource, BodyRegion, BodyRegionId, DamageEvent, DamageGenerator, DamageLevel,
    DamageType, ForeignBody, FractureType, OrganState, OrganType, PhysiologicalMap, TissueLayer,
};

use crate::assets::{MeshData, Vertex};

// ============================================================================
// 统一接口（借鉴 Fyrox Prefab 继承 + nova SceneNode）
// ============================================================================

/// 程序化生成器统一 trait
///
/// 每个生成器实现此 trait，通过 `GeneratorParams` 接收统一参数（seed/style/lod/...）。
/// 各生成器还可暴露专用的 `generate_with_params(&SpecificParams)` 方法以获得细粒度控制。
pub trait ProceduralGenerator {
    /// 生成输出类型（通常是 `MeshData` 或包含 `MeshData` 的复合结构）
    type Output;

    /// 用统一参数驱动生成
    fn generate(&self, params: &GeneratorParams) -> Self::Output;
}

/// 统一生成参数
#[derive(Debug, Clone)]
pub struct GeneratorParams {
    /// 随机种子（决定可重现的随机变异）
    pub seed: u64,
    /// 风格（决定生成器分发与默认参数）
    pub style: ProceduralStyle,
    /// 材质调色板（材质 Handle 索引列表，生成器据此分区分配材质）
    pub material_palette: Vec<usize>,
    /// LOD 等级（0=最高精度，255=最低精度）
    pub lod: u8,
    /// 影响生成的实体特征（如 "ae"、"ruined"、"infected"）
    pub seed_entities: Vec<String>,
}

impl Default for GeneratorParams {
    fn default() -> Self {
        Self {
            seed: 0,
            style: ProceduralStyle::WastelandBuilding,
            material_palette: vec![0],
            lod: 0,
            seed_entities: Vec::new(),
        }
    }
}

/// 程序化风格（决定生成器分发与语义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProceduralStyle {
    /// 废土建筑（功能分区 + 承重图 + 老化）
    WastelandBuilding,
    /// 旧世界废墟（更高老化、局部坍塌）
    OldWorldRuins,
    /// 角色（12 部位人形骨骼 + 蒙皮）
    Character,
    /// 异形生物（母巢子实体 8 种形态）
    Creature,
    /// 损伤（叠加在现有 Mesh 之上）
    Damage,
}

// ============================================================================
// MeshBuilder：CPU 端网格构建器（nova 没有，procedural 内部使用）
// ============================================================================

/// CPU 端网格构建器（借鉴 v1 MeshBuilder，输出 nova `MeshData`）
pub struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn with_capacity(v: usize, i: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(v),
            indices: Vec::with_capacity(i),
        }
    }

    pub fn push_vertex(&mut self, v: Vertex) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(v);
        idx
    }

    pub fn push_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.extend_from_slice(&[i0, i1, i2]);
    }

    pub fn push_quad(&mut self, i0: u32, i1: u32, i2: u32, i3: u32) {
        // (i0, i1, i2) + (i0, i2, i3)
        self.indices.extend_from_slice(&[i0, i1, i2, i0, i2, i3]);
    }

    /// 推入一个三角形面（带法线），返回顶点索引三元组
    pub fn push_triangle_face(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        normal: [f32; 3],
    ) {
        let i0 = self.push_vertex(Vertex::new(v0).with_normal(normal).with_uv([0.0, 0.0]));
        let i1 = self.push_vertex(Vertex::new(v1).with_normal(normal).with_uv([1.0, 0.0]));
        let i2 = self.push_vertex(Vertex::new(v2).with_normal(normal).with_uv([0.5, 1.0]));
        self.push_triangle(i0, i1, i2);
    }

    /// 推入一个四边形面（带法线 + UV 尺度）
    pub fn push_quad_face(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
        normal: [f32; 3],
        uv_size: [f32; 2],
    ) {
        let i0 = self.push_vertex(
            Vertex::new(v0)
                .with_normal(normal)
                .with_uv([0.0, 0.0]),
        );
        let i1 = self.push_vertex(
            Vertex::new(v1)
                .with_normal(normal)
                .with_uv([uv_size[0], 0.0]),
        );
        let i2 = self.push_vertex(
            Vertex::new(v2)
                .with_normal(normal)
                .with_uv([uv_size[0], uv_size[1]]),
        );
        let i3 = self.push_vertex(
            Vertex::new(v3)
                .with_normal(normal)
                .with_uv([0.0, uv_size[1]]),
        );
        self.push_quad(i0, i1, i2, i3);
    }

    /// 推入一个轴对齐盒子（中心 + 尺寸，6 面 12 三角形）
    pub fn push_box(&mut self, center: [f32; 3], extent: [f32; 3]) {
        let cx = center[0];
        let cy = center[1];
        let cz = center[2];
        let hx = extent[0] * 0.5;
        let hy = extent[1] * 0.5;
        let hz = extent[2] * 0.5;

        let v = [
            [cx - hx, cy - hy, cz - hz], // 0
            [cx + hx, cy - hy, cz - hz], // 1
            [cx + hx, cy + hy, cz - hz], // 2
            [cx - hx, cy + hy, cz - hz], // 3
            [cx - hx, cy - hy, cz + hz], // 4
            [cx + hx, cy - hy, cz + hz], // 5
            [cx + hx, cy + hy, cz + hz], // 6
            [cx - hx, cy + hy, cz + hz], // 7
        ];

        let uv = [extent[0], extent[1]];
        // -Z 面
        self.push_quad_face(v[0], v[1], v[2], v[3], [0.0, 0.0, -1.0], uv);
        // +Z 面
        self.push_quad_face(v[5], v[4], v[7], v[6], [0.0, 0.0, 1.0], uv);
        // -X 面
        self.push_quad_face(v[4], v[0], v[3], v[7], [-1.0, 0.0, 0.0], [extent[2], extent[1]]);
        // +X 面
        self.push_quad_face(v[1], v[5], v[6], v[2], [1.0, 0.0, 0.0], [extent[2], extent[1]]);
        // -Y 面
        self.push_quad_face(v[4], v[5], v[1], v[0], [0.0, -1.0, 0.0], [extent[0], extent[2]]);
        // +Y 面
        self.push_quad_face(v[3], v[2], v[6], v[7], [0.0, 1.0, 0.0], [extent[0], extent[2]]);
    }

    /// 推入参数化圆柱（中心在 (0, height/2, 0)，Y 轴向上）
    pub fn push_cylinder(&mut self, radius: f32, height: f32, segments: u32, cap: bool) {
        let half_h = height * 0.5;
        let seg = segments as f32;
        let base = self.vertices.len() as u32;

        // 侧面顶点（底环 + 顶环）
        for i in 0..=segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / seg;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            let u = i as f32 / seg;
            self.push_vertex(
                Vertex::new([cos_a * radius, -half_h, sin_a * radius])
                    .with_normal([cos_a, 0.0, sin_a])
                    .with_uv([u, 0.0]),
            );
            self.push_vertex(
                Vertex::new([cos_a * radius, half_h, sin_a * radius])
                    .with_normal([cos_a, 0.0, sin_a])
                    .with_uv([u, 1.0]),
            );
        }
        // 侧面索引
        for i in 0..segments {
            let i0 = base + i * 2;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;
            self.push_quad(i0, i1, i3, i2);
        }

        if cap {
            // 底盖
            let center_b = self.push_vertex(
                Vertex::new([0.0, -half_h, 0.0])
                    .with_normal([0.0, -1.0, 0.0])
                    .with_uv([0.5, 0.5]),
            );
            for i in 0..segments {
                let a0 = 2.0 * std::f32::consts::PI * i as f32 / seg;
                let a1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / seg;
                let p0 = self.push_vertex(
                    Vertex::new([a0.cos() * radius, -half_h, a0.sin() * radius])
                        .with_normal([0.0, -1.0, 0.0]),
                );
                let p1 = self.push_vertex(
                    Vertex::new([a1.cos() * radius, -half_h, a1.sin() * radius])
                        .with_normal([0.0, -1.0, 0.0]),
                );
                self.push_triangle(center_b, p1, p0);
            }
            // 顶盖
            let center_t = self.push_vertex(
                Vertex::new([0.0, half_h, 0.0])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_uv([0.5, 0.5]),
            );
            for i in 0..segments {
                let a0 = 2.0 * std::f32::consts::PI * i as f32 / seg;
                let a1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / seg;
                let p0 = self.push_vertex(
                    Vertex::new([a0.cos() * radius, half_h, a0.sin() * radius])
                        .with_normal([0.0, 1.0, 0.0]),
                );
                let p1 = self.push_vertex(
                    Vertex::new([a1.cos() * radius, half_h, a1.sin() * radius])
                        .with_normal([0.0, 1.0, 0.0]),
                );
                self.push_triangle(center_t, p0, p1);
            }
        }
    }

    /// 推入 UV 球（中心 + 半径）
    pub fn push_sphere(&mut self, center: [f32; 3], radius: f32, seg_h: u32, seg_v: u32) {
        let base = self.vertices.len() as u32;
        for j in 0..=seg_v {
            let v = j as f32 / seg_v as f32;
            let phi = v * std::f32::consts::PI; // 0..π
            let y = radius * phi.cos();
            let r = radius * phi.sin();
            for i in 0..=seg_h {
                let u = i as f32 / seg_h as f32;
                let theta = u * 2.0 * std::f32::consts::PI;
                let x = r * theta.cos();
                let z = r * theta.sin();
                let pos = [center[0] + x, center[1] + y, center[2] + z];
                // 法线 = 单位方向
                let nlen = (x * x + y * y + z * z).sqrt().max(1e-6);
                let normal = [x / nlen, y / nlen, z / nlen];
                self.push_vertex(Vertex::new(pos).with_normal(normal).with_uv([u, v]));
            }
        }
        // 索引
        let ring = seg_h + 1;
        for j in 0..seg_v {
            for i in 0..seg_h {
                let a = base + j * ring + i;
                let b = a + 1;
                let c = a + ring;
                let d = c + 1;
                self.push_quad(a, b, d, c);
            }
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn vertices_mut(&mut self) -> &mut [Vertex] {
        &mut self.vertices
    }

    pub fn indices_mut(&mut self) -> &mut Vec<u32> {
        &mut self.indices
    }

    /// 应用统一变换（平移 + 旋转四元数 + 缩放）到所有顶点
    pub fn transform(&mut self, translation: [f32; 3], rotation: [f32; 4], scale: f32) {
        use glam::{Quat, Vec3};
        let t = Vec3::from(translation);
        let r = Quat::from_vec4(rotation.into());
        let s = scale;
        for v in &mut self.vertices {
            let p = Vec3::from(v.position);
            let p = r * (p * s) + t;
            v.position = p.into();
            let n = Vec3::from(v.normal);
            let n = r * n;
            v.normal = n.into();
        }
    }

    /// 追加另一个 builder 的全部顶点/索引（索引偏移）
    pub fn append(&mut self, other: &MeshBuilder) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        for &i in &other.indices {
            self.indices.push(base + i);
        }
    }

    /// 消耗 builder，返回 `MeshData`
    pub fn into_mesh_data(self) -> MeshData {
        MeshData::new(self.vertices, self.indices)
    }

    /// 消耗 builder，返回 `(vertices, indices)` 元组（v1 兼容）
    pub fn into_parts(self) -> (Vec<Vertex>, Vec<u32>) {
        (self.vertices, self.indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_builder_box() {
        let mut b = MeshBuilder::new();
        b.push_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let md = b.into_mesh_data();
        assert_eq!(md.vertices.len(), 24); // 6 faces × 4 verts
        assert_eq!(md.indices.len(), 36);  // 6 faces × 6 indices
    }

    #[test]
    fn test_mesh_builder_cylinder() {
        let mut b = MeshBuilder::new();
        b.push_cylinder(1.0, 2.0, 8, true);
        let md = b.into_mesh_data();
        assert!(md.vertices.len() > 0);
        assert!(!md.indices.is_empty());
    }

    #[test]
    fn test_mesh_builder_sphere() {
        let mut b = MeshBuilder::new();
        b.push_sphere([0.0, 0.0, 0.0], 1.0, 8, 4);
        let md = b.into_mesh_data();
        assert!(md.vertices.len() > 0);
        assert!(!md.indices.is_empty());
    }

    #[test]
    fn test_generator_params_default() {
        let p = GeneratorParams::default();
        assert_eq!(p.style, ProceduralStyle::WastelandBuilding);
        assert_eq!(p.lod, 0);
    }
}
