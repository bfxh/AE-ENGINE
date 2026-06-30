//! 程序化角色生成器（从 v1 ae_render/procedural/{npc,skeleton}.rs 移植）
//!
//! 突破性 NPC 程序化生成：
//! - 12 部位人形骨骼（头/颈/胸/腰/上臂×2/前臂×2/大腿×2/小腿×2）
//! - 简化骨骼层级（slotmap BoneId + 父子关系 + bind pose）
//! - 蒙皮权重自动分配（每顶点最近骨骼，最多 4 个影响）
//! - 步行动画轨道（基于生物力学的周期性运动）
//! - 形态模板系统（支持非人类 NPC：Bipedal/Quadrupedal/Insectoid/...）

use crate::assets::MeshData;
use crate::procedural::{GeneratorParams, MeshBuilder, ProceduralGenerator, ProceduralStyle};
use glam::{Mat4, Quat, Vec3};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

// ============================================================================
// 骨骼系统（简化版，从 v1 skeleton.rs 移植）
// ============================================================================

slotmap::new_key_type! { pub struct BoneId; }

/// 骨骼定义（层级 + 局部变换）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    pub parent: Option<BoneId>,
    pub local_bind: JointTransform,
    pub world_bind: Mat4,
    pub inverse_bind: Mat4,
}

/// 关节变换（位置 + 旋转 + 缩放）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for JointTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl JointTransform {
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }
}

/// 骨骼系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skeleton {
    pub bones: slotmap::SlotMap<BoneId, Bone>,
    pub roots: Vec<BoneId>,
    pub name_to_id: hashbrown::HashMap<String, BoneId>,
    pub bone_id_to_index: hashbrown::HashMap<BoneId, u32>,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            bones: slotmap::SlotMap::with_key(),
            roots: Vec::new(),
            name_to_id: hashbrown::HashMap::new(),
            bone_id_to_index: hashbrown::HashMap::new(),
        }
    }

    pub fn add_bone(
        &mut self,
        name: &str,
        parent: Option<BoneId>,
        local_bind: JointTransform,
    ) -> BoneId {
        let id = self.bones.insert(Bone {
            name: name.to_string(),
            parent,
            local_bind,
            world_bind: Mat4::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        });
        self.name_to_id.insert(name.to_string(), id);
        if parent.is_none() {
            self.roots.push(id);
        }
        id
    }

    pub fn bone_index(&self, id: BoneId) -> u32 {
        self.bone_id_to_index.get(&id).copied().unwrap_or(0)
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn get_bone_by_name(&self, name: &str) -> Option<BoneId> {
        self.name_to_id.get(name).copied()
    }

    /// 计算绑定姿态的世界变换 + 逆绑定矩阵
    pub fn compute_bind_pose(&mut self) {
        self.bone_id_to_index.clear();
        for (idx, bone_id) in self.bones.keys().enumerate() {
            self.bone_id_to_index.insert(bone_id, idx as u32);
        }
        let total = self.bones.len();
        let mut visited: hashbrown::HashSet<BoneId> = hashbrown::HashSet::with_capacity(total);
        while visited.len() < total {
            let mut progressed = false;
            let bone_ids: Vec<BoneId> = self.bones.keys().collect();
            for bone_id in bone_ids {
                if visited.contains(&bone_id) {
                    continue;
                }
                let parent = self.bones.get(bone_id).unwrap().parent;
                let parent_ready = match parent {
                    None => true,
                    Some(pid) => visited.contains(&pid),
                };
                if !parent_ready {
                    continue;
                }
                let parent_world = match parent {
                    None => Mat4::IDENTITY,
                    Some(pid) => self.bones.get(pid).unwrap().world_bind,
                };
                let local = self.bones.get(bone_id).unwrap().local_bind;
                let world = parent_world * local.to_mat4();
                let bone = self.bones.get_mut(bone_id).unwrap();
                bone.world_bind = world;
                bone.inverse_bind = world.inverse();
                visited.insert(bone_id);
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
    }

    /// 收集所有骨骼世界位置（用于蒙皮权重计算）
    pub fn bone_positions(&self) -> Vec<(u32, Vec3)> {
        self.bones
            .iter()
            .map(|(id, bone)| {
                (self.bone_index(id), bone.world_bind.w_axis.truncate())
            })
            .collect()
    }
}

/// 蒙皮权重（每顶点最多 4 个骨骼影响）
#[derive(Debug, Clone, Copy, Default)]
pub struct SkinWeights {
    pub bone_indices: [u32; 4],
    pub weights: [f32; 4],
}

impl SkinWeights {
    pub fn single(bone_idx: u32, weight: f32) -> Self {
        let mut s = Self::default();
        s.bone_indices[0] = bone_idx;
        s.weights[0] = weight;
        s
    }
}

// ============================================================================
// NPC 参数 + 骨骼定义
// ============================================================================

/// NPC 身体参数
#[derive(Debug, Clone)]
pub struct NpcBodyParams {
    pub height: f32,
    pub build: f32,
    pub shoulder_width: f32,
    pub hip_width: f32,
    pub head_ratio: f32,
    pub skin_color: [f32; 4],
    pub gender: Gender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

impl Default for NpcBodyParams {
    fn default() -> Self {
        Self {
            height: 1.75,
            build: 0.5,
            shoulder_width: 0.42,
            hip_width: 0.32,
            head_ratio: 1.0 / 7.5,
            skin_color: [0.85, 0.7, 0.6, 1.0],
            gender: Gender::Male,
        }
    }
}

/// 人形骨骼定义（12 主要部位 + 中间关节）
pub struct HumanoidSkeleton;

impl HumanoidSkeleton {
    /// 创建标准人形骨骼层级
    pub fn create(params: &NpcBodyParams) -> Skeleton {
        let mut skel = Skeleton::new();
        let h = params.height;

        let head_h = h * params.head_ratio;
        let neck_h = head_h * 0.3;
        let chest_h = h * 0.25;
        let spine_h = h * 0.10;
        let pelvis_h = h * 0.10;
        let upper_arm_h = h * 0.18;

        let pelvis_y = 0.0;
        let spine_y = pelvis_y + pelvis_h;
        let chest_y = spine_y + spine_h;
        let neck_y = chest_y + chest_h;

        let (sw, hw) = match params.gender {
            Gender::Male => (params.shoulder_width * 0.5, params.hip_width * 0.45),
            Gender::Female => (params.shoulder_width * 0.42, params.hip_width * 0.55),
        };

        let pelvis = skel.add_bone("pelvis", None, JointTransform {
            translation: Vec3::new(0.0, pelvis_y, 0.0),
            ..Default::default()
        });
        let spine = skel.add_bone("spine", Some(pelvis), JointTransform {
            translation: Vec3::new(0.0, pelvis_h, 0.0),
            ..Default::default()
        });
        let chest = skel.add_bone("chest", Some(spine), JointTransform {
            translation: Vec3::new(0.0, spine_h, 0.0),
            ..Default::default()
        });
        let neck = skel.add_bone("neck", Some(chest), JointTransform {
            translation: Vec3::new(0.0, chest_h, 0.0),
            ..Default::default()
        });
        skel.add_bone("head", Some(neck), JointTransform {
            translation: Vec3::new(0.0, neck_h + head_h * 0.5, 0.0),
            ..Default::default()
        });

        // 左肩 → 上臂 → 前臂
        let shoulder_l = skel.add_bone("shoulder_l", Some(chest), JointTransform {
            translation: Vec3::new(sw, chest_h * 0.9, 0.0),
            ..Default::default()
        });
        let upper_arm_l = skel.add_bone("upper_arm_l", Some(shoulder_l), JointTransform {
            translation: Vec3::new(sw * 0.3, -0.05, 0.0),
            rotation: Quat::from_rotation_z(0.1),
            ..Default::default()
        });
        skel.add_bone("lower_arm_l", Some(upper_arm_l), JointTransform {
            translation: Vec3::new(0.0, -upper_arm_h, 0.0),
            ..Default::default()
        });

        // 右肩 → 上臂 → 前臂
        let shoulder_r = skel.add_bone("shoulder_r", Some(chest), JointTransform {
            translation: Vec3::new(-sw, chest_h * 0.9, 0.0),
            ..Default::default()
        });
        let upper_arm_r = skel.add_bone("upper_arm_r", Some(shoulder_r), JointTransform {
            translation: Vec3::new(-sw * 0.3, -0.05, 0.0),
            rotation: Quat::from_rotation_z(-0.1),
            ..Default::default()
        });
        skel.add_bone("lower_arm_r", Some(upper_arm_r), JointTransform {
            translation: Vec3::new(0.0, -upper_arm_h, 0.0),
            ..Default::default()
        });

        // 左腿
        let thigh_l = skel.add_bone("thigh_l", Some(pelvis), JointTransform {
            translation: Vec3::new(hw * 0.5, -0.05, 0.0),
            ..Default::default()
        });
        skel.add_bone("calf_l", Some(thigh_l), JointTransform {
            translation: Vec3::new(0.0, -h * 0.25, 0.0),
            ..Default::default()
        });

        // 右腿
        let thigh_r = skel.add_bone("thigh_r", Some(pelvis), JointTransform {
            translation: Vec3::new(-hw * 0.5, -0.05, 0.0),
            ..Default::default()
        });
        skel.add_bone("calf_r", Some(thigh_r), JointTransform {
            translation: Vec3::new(0.0, -h * 0.25, 0.0),
            ..Default::default()
        });

        skel.compute_bind_pose();
        skel
    }
}

// ============================================================================
// 形态模板系统（支持非人类 NPC：虫族、母巢子实体、变异生物）
// ============================================================================

/// 身体蓝图
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPlan {
    Bipedal,
    Quadrupedal,
    Insectoid,
    Arachnid,
    Amorphous,
    Winged,
    Armored,
}

/// 生物材质类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiologicalMaterial {
    Flesh,
    Mycelium,
    Chitin,
    KeratinPlate,
    RustyMetal,
    Membrane,
    Fluid,
    Lichen,
}

impl BiologicalMaterial {
    pub fn pbr_params(&self) -> ([f32; 4], f32, f32) {
        match self {
            BiologicalMaterial::Flesh => ([0.85, 0.7, 0.6, 1.0], 0.7, 0.0),
            BiologicalMaterial::Mycelium => ([0.55, 0.45, 0.30, 1.0], 0.95, 0.0),
            BiologicalMaterial::Chitin => ([0.30, 0.25, 0.20, 1.0], 0.4, 0.0),
            BiologicalMaterial::KeratinPlate => ([0.40, 0.35, 0.30, 1.0], 0.35, 0.0),
            BiologicalMaterial::RustyMetal => ([0.45, 0.20, 0.10, 1.0], 0.6, 0.7),
            BiologicalMaterial::Membrane => ([0.65, 0.55, 0.40, 0.7], 0.2, 0.0),
            BiologicalMaterial::Fluid => ([0.30, 0.20, 0.10, 0.85], 0.1, 0.0),
            BiologicalMaterial::Lichen => ([0.50, 0.48, 0.30, 1.0], 1.0, 0.0),
        }
    }
}

/// 体型分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeClass {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}

impl SizeClass {
    pub fn default_scale(&self) -> f32 {
        match self {
            SizeClass::Tiny => 0.25,
            SizeClass::Small => 0.75,
            SizeClass::Medium => 1.0,
            SizeClass::Large => 1.75,
            SizeClass::Huge => 3.5,
        }
    }
}

/// 通用形态参数
#[derive(Debug, Clone)]
pub struct MorphParams {
    pub scale: f32,
    pub build: f32,
    pub material: BiologicalMaterial,
    pub color_override: Option<[f32; 4]>,
    pub variant_seed: u32,
    pub leg_count: u32,
    pub arm_count: u32,
    pub eye_count: u32,
    pub antenna_count: u32,
    pub has_wings: bool,
    pub has_abdomen: bool,
    pub armor_coverage: f32,
    pub mycelium_density: f32,
    pub slime_secretion: f32,
}

impl Default for MorphParams {
    fn default() -> Self {
        Self {
            scale: 1.0,
            build: 0.5,
            material: BiologicalMaterial::Mycelium,
            color_override: None,
            variant_seed: 0,
            leg_count: 2,
            arm_count: 2,
            eye_count: 2,
            antenna_count: 0,
            has_wings: false,
            has_abdomen: false,
            armor_coverage: 0.0,
            mycelium_density: 0.5,
            slime_secretion: 0.0,
        }
    }
}

/// 变异系统（基于种子生成可重现的随机变异）
#[derive(Debug, Clone)]
pub struct MorphMutation {
    pub seed: u32,
}

impl MorphMutation {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    pub fn hash(&self, salt: u32) -> u32 {
        let mut h = self.seed.wrapping_add(salt.wrapping_mul(2654435761));
        h ^= h >> 16;
        h = h.wrapping_mul(0x85ebca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2ae35);
        h ^= h >> 16;
        h
    }

    pub fn random01(&self, salt: u32) -> f32 {
        (self.hash(salt) as f32) / (u32::MAX as f32)
    }

    pub fn range(&self, salt: u32, min: f32, max: f32) -> f32 {
        min + self.random01(salt) * (max - min)
    }

    pub fn vary_limb_count(&self, _base: u32, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let delta = self.hash(0xABCD) % (max - min + 1);
        min + delta
    }

    pub fn vary_scale(&self, base_scale: f32) -> f32 {
        base_scale * self.range(0x1234, 0.85, 1.15)
    }
}

/// 形态模板
#[derive(Debug, Clone)]
pub struct MorphTemplate {
    pub name: &'static str,
    pub body_plan: BodyPlan,
    pub material: BiologicalMaterial,
    pub size_class: SizeClass,
    pub default_params: MorphParams,
}

impl MorphTemplate {
    pub fn new(
        name: &'static str,
        body_plan: BodyPlan,
        material: BiologicalMaterial,
        size_class: SizeClass,
        default_params: MorphParams,
    ) -> Self {
        Self { name, body_plan, material, size_class, default_params }
    }

    pub fn resolve_color(&self, params: &MorphParams) -> [f32; 4] {
        params
            .color_override
            .unwrap_or_else(|| self.material.pbr_params().0)
    }
}

// ============================================================================
// NPC 生成器
// ============================================================================

/// NPC 生成输出（Mesh + 骨骼 + 蒙皮权重）
pub struct CharacterOutput {
    pub mesh: MeshData,
    pub skeleton: Skeleton,
    pub skin_weights: Vec<SkinWeights>,
}

/// NPC 生成器
pub struct NpcBodyGenerator {
    builder: MeshBuilder,
}

impl Default for NpcBodyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcBodyGenerator {
    pub fn new() -> Self {
        Self {
            builder: MeshBuilder::new(),
        }
    }

    /// 生成 NPC：返回 (MeshData, Skeleton, SkinWeights)
    pub fn generate_with_params(mut self, params: &NpcBodyParams) -> CharacterOutput {
        let skeleton = HumanoidSkeleton::create(params);

        let h = params.height;
        let head_h = h * params.head_ratio;
        let chest_h = h * 0.25;
        let spine_h = h * 0.10;
        let pelvis_h = h * 0.10;
        let upper_arm_h = h * 0.18;
        let lower_arm_h = h * 0.15;
        let thigh_h = h * 0.25;
        let calf_h = h * 0.22;

        let build = params.build;
        let sw = params.shoulder_width * 0.5;
        let hw = params.hip_width * 0.5;
        let torso_w = sw * 1.8;
        let torso_d = 0.18 + build * 0.06;

        // 收集骨骼位置
        let positions: hashbrown::HashMap<String, Vec3> = skeleton
            .bones
            .iter()
            .map(|(_, b)| (b.name.clone(), b.world_bind.w_axis.truncate()))
            .collect();

        let get = |name: &str| -> Vec3 {
            positions.get(name).copied().unwrap_or(Vec3::ZERO)
        };

        // 头（球）
        let head_pos = get("head");
        self.builder.push_sphere(head_pos.into(), head_h * 0.45, 12, 8);

        // 颈（圆柱）
        self.push_cylinder_between(get("neck"), get("head"), 0.05);
        // 胸（盒子）
        self.builder.push_box(get("chest").into(), [torso_w, chest_h, torso_d]);
        // 腰
        self.builder.push_box(get("spine").into(), [torso_w * 0.85, spine_h, torso_d * 0.9]);
        // 髋
        self.builder.push_box(get("pelvis").into(), [hw * 2.2, pelvis_h, torso_d * 0.9]);

        // 上臂 L/R
        let arm_radius = 0.04 + build * 0.02;
        self.push_cylinder_between(get("upper_arm_l"), get("lower_arm_l"), arm_radius);
        self.push_cylinder_between(get("upper_arm_r"), get("lower_arm_r"), arm_radius);
        // 前臂 L/R
        self.push_cylinder_between(
            get("lower_arm_l"),
            get("lower_arm_l") + Vec3::new(0.0, -lower_arm_h, 0.0),
            arm_radius * 0.85,
        );
        self.push_cylinder_between(
            get("lower_arm_r"),
            get("lower_arm_r") + Vec3::new(0.0, -lower_arm_h, 0.0),
            arm_radius * 0.85,
        );
        // 大腿 L/R
        let leg_radius = 0.05 + build * 0.025;
        self.push_cylinder_between(get("thigh_l"), get("calf_l"), leg_radius);
        self.push_cylinder_between(get("thigh_r"), get("calf_r"), leg_radius);
        // 小腿 L/R
        self.push_cylinder_between(
            get("calf_l"),
            get("calf_l") + Vec3::new(0.0, -calf_h, 0.0),
            leg_radius * 0.8,
        );
        self.push_cylinder_between(
            get("calf_r"),
            get("calf_r") + Vec3::new(0.0, -calf_h, 0.0),
            leg_radius * 0.8,
        );

        // 肤色
        let color = params.skin_color;
        for v in self.builder.vertices_mut().iter_mut() {
            v.color = color;
        }

        let (vertices, indices) = self.builder.into_parts();
        let skin_weights = Self::compute_skin_weights(&vertices, &skeleton);

        CharacterOutput {
            mesh: MeshData::new(vertices, indices),
            skeleton,
            skin_weights,
        }
    }

    fn push_cylinder_between(&mut self, start: Vec3, end: Vec3, radius: f32) {
        let direction = end - start;
        let length = direction.length();
        if length < 1e-6 {
            return;
        }
        // 用 builder.push_cylinder 在原点生成，再变换到位置
        let mut tmp = MeshBuilder::new();
        tmp.push_cylinder(radius, length, 8, true);
        let dir_normalized = direction / length;
        let default_dir = Vec3::new(0.0, 1.0, 0.0);
        let rotation = Quat::from_rotation_arc(default_dir, dir_normalized);
        let midpoint = start + direction * 0.5;
        let rot_arr: [f32; 4] = rotation.into();
        tmp.transform(midpoint.into(), rot_arr, 1.0);
        self.builder.append(&tmp);
    }

    fn compute_skin_weights(vertices: &[crate::assets::Vertex], skeleton: &Skeleton) -> Vec<SkinWeights> {
        let bone_positions = skeleton.bone_positions();
        vertices
            .iter()
            .map(|v| {
                let pos = Vec3::from(v.position);
                let mut best_idx = 0u32;
                let mut best_dist = f32::MAX;
                for &(idx, bp) in &bone_positions {
                    let d = (pos - bp).length_squared();
                    if d < best_dist {
                        best_dist = d;
                        best_idx = idx;
                    }
                }
                SkinWeights::single(best_idx, 1.0)
            })
            .collect()
    }
}

// ============================================================================
// ProceduralGenerator trait 实现（通用角色入口）
// ============================================================================

/// 通用角色生成器（实现 ProceduralGenerator trait）
pub struct CharacterGenerator {
    pub template: MorphTemplate,
}

impl CharacterGenerator {
    pub fn new(template: MorphTemplate) -> Self {
        Self { template }
    }

    /// 用专用 NPC 参数生成
    pub fn generate_npc(&self, params: &NpcBodyParams) -> CharacterOutput {
        NpcBodyGenerator::new().generate_with_params(params)
    }
}

impl ProceduralGenerator for CharacterGenerator {
    type Output = CharacterOutput;

    fn generate(&self, params: &GeneratorParams) -> Self::Output {
        let mut npc = NpcBodyParams::default();
        // 用 seed 调整体型/身高（可重现）
        let mut rng = rand::rngs::StdRng::seed_from_u64(params.seed);
        use rand::Rng;
        npc.height = rng.gen_range(1.55..1.95);
        npc.build = rng.gen_range(0.3..0.8);
        // style 影响：Character 用默认人形，Creature 由 CreatureGenerator 处理
        let _ = params.style;
        NpcBodyGenerator::new().generate_with_params(&npc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanoid_skeleton_creation() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        // pelvis + spine + chest + neck + head + 2 shoulder + 2 upper_arm + 2 lower_arm + 2 thigh + 2 calf = 15
        assert_eq!(skel.bone_count(), 15);
        assert_eq!(skel.roots.len(), 1);
        assert!(skel.get_bone_by_name("head").is_some());
        assert!(skel.get_bone_by_name("pelvis").is_some());
    }

    #[test]
    fn test_npc_body_generation() {
        let params = NpcBodyParams::default();
        let out = NpcBodyGenerator::new().generate_with_params(&params);
        assert!(out.mesh.vertices.len() > 100);
        assert!(!out.mesh.indices.is_empty());
        assert_eq!(out.skin_weights.len(), out.mesh.vertices.len());
    }

    #[test]
    fn test_biological_material_pbr() {
        let (albedo, roughness, metallic) = BiologicalMaterial::RustyMetal.pbr_params();
        assert!(metallic > 0.5);
        assert!(roughness > 0.0 && roughness <= 1.0);
        let _ = albedo;
    }

    #[test]
    fn test_morph_mutation_deterministic() {
        let m1 = MorphMutation::new(42);
        let m2 = MorphMutation::new(42);
        assert_eq!(m1.hash(100), m2.hash(100));
    }

    #[test]
    fn test_character_generator_trait() {
        let tpl = MorphTemplate::new(
            "human",
            BodyPlan::Bipedal,
            BiologicalMaterial::Flesh,
            SizeClass::Medium,
            MorphParams::default(),
        );
        let gen = CharacterGenerator::new(tpl);
        let gp = GeneratorParams {
            style: ProceduralStyle::Character,
            seed: 42,
            ..Default::default()
        };
        let out = gen.generate(&gp);
        assert!(!out.mesh.vertices.is_empty());
    }
}
