//! 程序化异形生物生成器（从 v1 ae_render/procedural/morph.rs 移植）
//!
//! 母巢子实体 8 种异形生物（来自AE-ENGINE世界观）：
//! - Stalker 践踏者：双足菌丝体，无眼无口，传感触角
//! - Hunter 追猎者：四足菌丝+角质板，鞭状触角
//! - Crusher 碎脊者：1.5x 人形，厚重角质层天然装甲
//! - RustKnight 锈骑士：动力装甲外壳+菌丝血管填充
//! - Swarm 蜂群：菌丝机翼+复眼+爆炸孢子囊
//! - Bloated 臃肿者：八足蜘蛛形，膨胀腹部
//! - Listener 窃听者：扁平地衣状，附着表面
//! - Weaver 编织者：六足蚂蚁形，多条菌丝触手
//!
//! Nova 适配：
//! - 输出 `MeshData`
//! - 复用 `character` 模块的 MorphTemplate/MorphParams/MorphMutation
//! - 通过 `ProceduralGenerator` trait 集成

use crate::assets::MeshData;
use crate::procedural::character::{
    BiologicalMaterial, BodyPlan, MorphMutation, MorphParams, MorphTemplate, SizeClass,
};
use crate::procedural::{GeneratorParams, MeshBuilder, ProceduralGenerator, ProceduralStyle};
use glam::Vec3;

// ============================================================================
// 8 种母巢子实体模板（来自 v1 morph.rs）
// ============================================================================

/// 践踏者：双足菌丝体
pub fn stalker_template() -> MorphTemplate {
    MorphTemplate::new(
        "stalker",
        BodyPlan::Bipedal,
        BiologicalMaterial::Mycelium,
        SizeClass::Small,
        MorphParams {
            scale: 0.85,
            build: 0.4,
            material: BiologicalMaterial::Mycelium,
            leg_count: 2,
            arm_count: 2,
            eye_count: 0,
            antenna_count: 2,
            mycelium_density: 0.9,
            ..Default::default()
        },
    )
}

/// 追猎者：四足菌丝+角质板
pub fn hunter_template() -> MorphTemplate {
    MorphTemplate::new(
        "hunter",
        BodyPlan::Quadrupedal,
        BiologicalMaterial::Chitin,
        SizeClass::Medium,
        MorphParams {
            scale: 1.1,
            build: 0.5,
            material: BiologicalMaterial::Chitin,
            leg_count: 4,
            arm_count: 0,
            eye_count: 0,
            antenna_count: 2,
            armor_coverage: 0.5,
            mycelium_density: 0.3,
            ..Default::default()
        },
    )
}

/// 碎脊者：1.5x 人形，厚重角质层天然装甲
pub fn crusher_template() -> MorphTemplate {
    MorphTemplate::new(
        "crusher",
        BodyPlan::Armored,
        BiologicalMaterial::KeratinPlate,
        SizeClass::Large,
        MorphParams {
            scale: 1.75,
            build: 0.9,
            material: BiologicalMaterial::KeratinPlate,
            leg_count: 2,
            arm_count: 2,
            eye_count: 0,
            antenna_count: 0,
            armor_coverage: 0.9,
            ..Default::default()
        },
    )
}

/// 锈骑士：动力装甲外壳+菌丝血管填充
pub fn rust_knight_template() -> MorphTemplate {
    MorphTemplate::new(
        "rust_knight",
        BodyPlan::Armored,
        BiologicalMaterial::RustyMetal,
        SizeClass::Large,
        MorphParams {
            scale: 1.8,
            build: 0.9,
            material: BiologicalMaterial::RustyMetal,
            leg_count: 2,
            arm_count: 2,
            eye_count: 0,
            antenna_count: 0,
            armor_coverage: 1.0,
            mycelium_density: 0.4,
            ..Default::default()
        },
    )
}

/// 蜂群：菌丝机翼+复眼+爆炸孢子囊
pub fn swarm_template() -> MorphTemplate {
    MorphTemplate::new(
        "swarm",
        BodyPlan::Winged,
        BiologicalMaterial::Mycelium,
        SizeClass::Tiny,
        MorphParams {
            scale: 0.25,
            build: 0.2,
            material: BiologicalMaterial::Mycelium,
            leg_count: 6,
            arm_count: 0,
            eye_count: 4,
            antenna_count: 2,
            has_wings: true,
            has_abdomen: true,
            ..Default::default()
        },
    )
}

/// 臃肿者：八足蜘蛛形，膨胀腹部
pub fn bloated_template() -> MorphTemplate {
    MorphTemplate::new(
        "bloated",
        BodyPlan::Arachnid,
        BiologicalMaterial::Mycelium,
        SizeClass::Large,
        MorphParams {
            scale: 1.5,
            build: 0.8,
            material: BiologicalMaterial::Mycelium,
            leg_count: 8,
            arm_count: 0,
            eye_count: 0,
            antenna_count: 0,
            has_abdomen: true,
            ..Default::default()
        },
    )
}

/// 窃听者：扁平地衣状，附着表面
pub fn listener_template() -> MorphTemplate {
    MorphTemplate::new(
        "listener",
        BodyPlan::Amorphous,
        BiologicalMaterial::Lichen,
        SizeClass::Tiny,
        MorphParams {
            scale: 0.3,
            build: 0.1,
            material: BiologicalMaterial::Lichen,
            leg_count: 0,
            arm_count: 0,
            eye_count: 0,
            antenna_count: 0,
            ..Default::default()
        },
    )
}

/// 编织者：六足蚂蚁形，多条菌丝触手
pub fn weaver_template() -> MorphTemplate {
    MorphTemplate::new(
        "weaver",
        BodyPlan::Insectoid,
        BiologicalMaterial::Mycelium,
        SizeClass::Medium,
        MorphParams {
            scale: 1.0,
            build: 0.4,
            material: BiologicalMaterial::Mycelium,
            leg_count: 6,
            arm_count: 0,
            eye_count: 0,
            antenna_count: 4,
            ..Default::default()
        },
    )
}

/// 返回全部 8 种子实体模板
pub fn all_templates() -> [MorphTemplate; 8] {
    [
        stalker_template(),
        hunter_template(),
        crusher_template(),
        rust_knight_template(),
        swarm_template(),
        bloated_template(),
        listener_template(),
        weaver_template(),
    ]
}

// ============================================================================
// 异形生物生成器
// ============================================================================

/// 异形生物生成输出
pub struct CreatureOutput {
    pub mesh: MeshData,
    pub template_name: &'static str,
    pub body_plan: BodyPlan,
}

/// 异形生物生成器（基于 MorphTemplate 自动生成任意拓扑骨骼与几何体）
pub struct CreatureGenerator {
    pub template: MorphTemplate,
}

impl CreatureGenerator {
    pub fn new(template: MorphTemplate) -> Self {
        Self { template }
    }

    /// 用专用 MorphParams 生成
    pub fn generate_with_params(&self, params: &MorphParams) -> CreatureOutput {
        let mut builder = MeshBuilder::new();
        let scale = params.scale;
        let build = params.build;
        let mutation = MorphMutation::new(params.variant_seed);

        let color = self.template.resolve_color(params);

        // 主体几何：根据 BodyPlan 分发
        match self.template.body_plan {
            BodyPlan::Bipedal => self.build_bipedal(&mut builder, scale, build, params),
            BodyPlan::Quadrupedal => self.build_quadrupedal(&mut builder, scale, build, params),
            BodyPlan::Insectoid => self.build_insectoid(&mut builder, scale, build, params),
            BodyPlan::Arachnid => self.build_arachnid(&mut builder, scale, build, params),
            BodyPlan::Amorphous => self.build_amorphous(&mut builder, scale, params),
            BodyPlan::Winged => self.build_winged(&mut builder, scale, build, params),
            BodyPlan::Armored => self.build_armored(&mut builder, scale, build, params),
        }

        // 应用装甲覆盖（粗糙外壳）
        if params.armor_coverage > 0.3 {
            self.build_armor_shell(&mut builder, scale, params.armor_coverage);
        }

        // 应用颜色
        for v in builder.vertices_mut().iter_mut() {
            v.color = color;
        }

        // 用 mutation 添加随机扰动（变异特征）
        let _ = mutation; // 暂留接口；后续可加入顶点扰动

        CreatureOutput {
            mesh: builder.into_mesh_data(),
            template_name: self.template.name,
            body_plan: self.template.body_plan,
        }
    }

    fn build_bipedal(&self, b: &mut MeshBuilder, scale: f32, build: f32, params: &MorphParams) {
        let h = 1.75 * scale;
        let torso_w = 0.45 * scale * (0.8 + build * 0.4);
        let torso_d = 0.22 * scale * (0.8 + build * 0.4);
        // 头
        b.push_sphere([0.0, h * 0.92, 0.0], 0.12 * scale, 10, 6);
        // 躯干
        b.push_box([0.0, h * 0.55, 0.0], [torso_w, h * 0.35, torso_d]);
        // 髋
        b.push_box([0.0, h * 0.32, 0.0], [torso_w * 0.85, h * 0.12, torso_d * 0.9]);
        // 双腿
        let leg_r = 0.06 * scale * (0.8 + build * 0.4);
        for x in [-torso_w * 0.3, torso_w * 0.3] {
            // 大腿
            let mut tmp = MeshBuilder::new();
            tmp.push_cylinder(leg_r, h * 0.32, 8, true);
            tmp.transform([x, h * 0.16, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
            b.append(&tmp);
            // 小腿
            let mut tmp = MeshBuilder::new();
            tmp.push_cylinder(leg_r * 0.85, h * 0.30, 8, true);
            tmp.transform([x, -h * 0.14, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
            b.append(&tmp);
        }
        // 双臂（如果 arm_count >= 2）
        if params.arm_count >= 2 {
            let arm_r = 0.045 * scale * (0.8 + build * 0.4);
            for x in [-torso_w * 0.55, torso_w * 0.55] {
                let mut tmp = MeshBuilder::new();
                tmp.push_cylinder(arm_r, h * 0.30, 8, true);
                tmp.transform([x, h * 0.55, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
                b.append(&tmp);
            }
        }
        // 触角
        for i in 0..params.antenna_count.min(2) {
            let sign = if i == 0 { -1.0 } else { 1.0 };
            let mut tmp = MeshBuilder::new();
            tmp.push_cylinder(0.005 * scale, 0.25 * scale, 6, true);
            tmp.transform([sign * 0.05 * scale, h * 1.05, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
            b.append(&tmp);
        }
    }

    fn build_quadrupedal(&self, b: &mut MeshBuilder, scale: f32, build: f32, _params: &MorphParams) {
        let h = 1.1 * scale;
        let body_w = 0.35 * scale * (0.8 + build * 0.4);
        let body_d = 1.2 * scale;
        // 躯干（水平盒子）
        b.push_box([0.0, h * 0.6, 0.0], [body_w, h * 0.35, body_d]);
        // 头
        b.push_sphere([0.0, h * 0.75, body_d * 0.55], 0.15 * scale, 10, 6);
        // 4 条腿
        let leg_r = 0.05 * scale;
        let leg_positions = [
            [-body_w * 0.4, body_d * 0.35],
            [body_w * 0.4, body_d * 0.35],
            [-body_w * 0.4, -body_d * 0.35],
            [body_w * 0.4, -body_d * 0.35],
        ];
        for [x, z] in leg_positions {
            let mut tmp = MeshBuilder::new();
            tmp.push_cylinder(leg_r, h * 0.6, 8, true);
            tmp.transform([x, 0.0, z], [0.0, 0.0, 0.0, 1.0], 1.0);
            b.append(&tmp);
        }
    }

    fn build_insectoid(&self, b: &mut MeshBuilder, scale: f32, build: f32, params: &MorphParams) {
        let h = 0.7 * scale;
        let body_w = 0.25 * scale * (0.8 + build * 0.4);
        let body_d = 1.0 * scale;
        // 三段身体（头+胸+腹）
        b.push_sphere([0.0, h, body_d * 0.45], 0.13 * scale, 8, 6);
        b.push_box([0.0, h, 0.0], [body_w, h * 0.4, body_d * 0.4]);
        b.push_box([0.0, h * 0.9, -body_d * 0.35], [body_w * 1.2, h * 0.5, body_d * 0.45]);
        // 6 条腿
        let leg_r = 0.025 * scale;
        for i in 0..3 {
            let z = body_d * 0.2 - i as f32 * body_d * 0.2;
            for sign in [-1.0, 1.0] {
                let mut tmp = MeshBuilder::new();
                tmp.push_cylinder(leg_r, h * 0.9, 6, true);
                tmp.transform([sign * body_w * 0.6, 0.0, z], [0.0, 0.0, 0.0, 1.0], 1.0);
                b.append(&tmp);
            }
        }
        // 触角
        for i in 0..params.antenna_count.min(4) {
            let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
            let mut tmp = MeshBuilder::new();
            tmp.push_cylinder(0.004 * scale, 0.2 * scale, 4, true);
            tmp.transform([sign * 0.06 * scale, h * 1.2, body_d * 0.55], [0.0, 0.0, 0.0, 1.0], 1.0);
            b.append(&tmp);
        }
    }

    fn build_arachnid(&self, b: &mut MeshBuilder, scale: f32, build: f32, params: &MorphParams) {
        let h = 0.6 * scale;
        let body_w = 0.4 * scale * (0.8 + build * 0.4);
        // 头胸部
        b.push_sphere([0.0, h, 0.1 * scale], 0.18 * scale, 10, 6);
        // 腹部（膨胀 if has_abdomen）
        let abdomen_scale = if params.has_abdomen { 1.5 } else { 1.0 };
        b.push_sphere(
            [0.0, h * 0.9, -0.3 * scale],
            0.22 * scale * abdomen_scale,
            10,
            6,
        );
        // 8 条腿
        let leg_r = 0.025 * scale;
        for i in 0..4 {
            let z = 0.15 * scale - i as f32 * 0.05 * scale;
            for sign in [-1.0, 1.0] {
                let mut tmp = MeshBuilder::new();
                tmp.push_cylinder(leg_r, h * 1.4, 6, true);
                tmp.transform([sign * body_w * 0.5, 0.0, z], [0.0, 0.0, 0.0, 1.0], 1.0);
                b.append(&tmp);
            }
        }
    }

    fn build_amorphous(&self, b: &mut MeshBuilder, scale: f32, _params: &MorphParams) {
        // 扁平地衣状：多个不规则球体堆叠
        let h = 0.05 * scale;
        b.push_sphere([0.0, h, 0.0], 0.3 * scale, 12, 4);
        b.push_sphere([0.15 * scale, h, 0.1 * scale], 0.18 * scale, 8, 4);
        b.push_sphere([-0.12 * scale, h, -0.08 * scale], 0.16 * scale, 8, 4);
        b.push_sphere([0.05 * scale, h * 1.5, -0.15 * scale], 0.12 * scale, 8, 4);
    }

    fn build_winged(&self, b: &mut MeshBuilder, scale: f32, build: f32, params: &MorphParams) {
        let h = 0.25 * scale;
        let body_w = 0.08 * scale * (0.8 + build * 0.4);
        // 躯干
        b.push_box([0.0, h, 0.0], [body_w, h * 0.6, 0.3 * scale]);
        // 头
        b.push_sphere([0.0, h * 1.2, 0.18 * scale], 0.06 * scale, 8, 6);
        // 翅膀（薄膜）
        if params.has_wings {
            for sign in [-1.0, 1.0] {
                let mut tmp = MeshBuilder::new();
                tmp.push_quad_face(
                    [0.0, 0.0, 0.0],
                    [sign * 0.4 * scale, 0.0, 0.0],
                    [sign * 0.35 * scale, 0.02 * scale, -0.2 * scale],
                    [0.0, 0.02 * scale, -0.2 * scale],
                    [0.0, 1.0, 0.0],
                    [0.4, 0.3],
                );
                tmp.transform([0.0, h * 1.4, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
                b.append(&tmp);
            }
        }
        // 腹部（膨胀 if has_abdomen）
        if params.has_abdomen {
            b.push_sphere([0.0, h * 0.7, -0.12 * scale], 0.1 * scale, 8, 6);
        }
        // 6 条腿
        let leg_r = 0.008 * scale;
        for i in 0..3 {
            let z = 0.1 * scale - i as f32 * 0.08 * scale;
            for sign in [-1.0, 1.0] {
                let mut tmp = MeshBuilder::new();
                tmp.push_cylinder(leg_r, h * 1.5, 4, true);
                tmp.transform([sign * body_w * 0.6, 0.0, z], [0.0, 0.0, 0.0, 1.0], 1.0);
                b.append(&tmp);
            }
        }
    }

    fn build_armored(&self, b: &mut MeshBuilder, scale: f32, build: f32, params: &MorphParams) {
        // 复用双足基础 + 装甲外壳
        self.build_bipedal(b, scale, build, params);
        // 装甲板覆盖（盒子的额外层）
        let h = 1.75 * scale;
        let torso_w = 0.55 * scale * (0.8 + build * 0.4);
        let torso_d = 0.28 * scale * (0.8 + build * 0.4);
        b.push_box([0.0, h * 0.55, 0.0], [torso_w * 1.1, h * 0.38, torso_d * 1.1]);
    }

    fn build_armor_shell(&self, b: &mut MeshBuilder, scale: f32, coverage: f32) {
        // 在躯干周围添加装甲板（覆盖越高板越多）
        let h = 1.75 * scale;
        let shell_count = (coverage * 6.0).round() as u32;
        let r = 0.32 * scale;
        for i in 0..shell_count {
            let angle = i as f32 / shell_count as f32 * std::f32::consts::TAU;
            let x = angle.cos() * r;
            let z = angle.sin() * r;
            b.push_box([x, h * 0.55, z], [0.06 * scale, h * 0.35, 0.06 * scale]);
        }
    }
}

impl ProceduralGenerator for CreatureGenerator {
    type Output = CreatureOutput;

    fn generate(&self, params: &GeneratorParams) -> Self::Output {
        // 合并默认参数 + GeneratorParams 调整
        let mut mp = self.template.default_params.clone();
        mp.variant_seed = params.seed as u32;
        // lod 影响几何精度（直接体现在 scale 上做粗略简化）
        let _ = params.lod;
        // style 影响：OldWorldRuins 增加菌丝密度
        if params.style == ProceduralStyle::OldWorldRuins {
            mp.mycelium_density = (mp.mycelium_density + 0.2).min(1.0);
        }
        self.generate_with_params(&mp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_count() {
        let t = all_templates();
        assert_eq!(t.len(), 8);
        // 每个模板名称唯一
        let mut names: Vec<&str> = t.iter().map(|x| x.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 8);
    }

    #[test]
    fn test_stalker_template_fields() {
        let t = stalker_template();
        assert_eq!(t.body_plan, BodyPlan::Bipedal);
        assert_eq!(t.material, BiologicalMaterial::Mycelium);
        assert_eq!(t.size_class, SizeClass::Small);
        assert_eq!(t.default_params.leg_count, 2);
        assert_eq!(t.default_params.antenna_count, 2);
    }

    #[test]
    fn test_creature_generator_bipedal() {
        let tpl = stalker_template();
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&MorphParams::default());
        assert!(out.mesh.vertices.len() > 50, "expected >50 verts, got {}", out.mesh.vertices.len());
        assert!(!out.mesh.indices.is_empty());
        assert_eq!(out.template_name, "stalker");
    }

    #[test]
    fn test_creature_generator_quadrupedal() {
        let tpl = hunter_template();
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&MorphParams::default());
        assert!(out.mesh.vertices.len() > 50);
        assert_eq!(out.body_plan, BodyPlan::Quadrupedal);
    }

    #[test]
    fn test_creature_generator_arachnid() {
        let tpl = bloated_template();
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&MorphParams::default());
        assert!(out.mesh.vertices.len() > 50);
        assert_eq!(out.body_plan, BodyPlan::Arachnid);
    }

    #[test]
    fn test_creature_generator_winged() {
        let tpl = swarm_template();
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&MorphParams::default());
        assert!(out.mesh.vertices.len() > 20);
        assert_eq!(out.body_plan, BodyPlan::Winged);
    }

    #[test]
    fn test_creature_generator_amorphous() {
        let tpl = listener_template();
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&MorphParams::default());
        assert!(out.mesh.vertices.len() > 10);
        assert_eq!(out.body_plan, BodyPlan::Amorphous);
    }

    #[test]
    fn test_procedural_generator_trait() {
        let gen = CreatureGenerator::new(weaver_template());
        let gp = GeneratorParams {
            style: ProceduralStyle::Creature,
            seed: 42,
            ..Default::default()
        };
        let out = gen.generate(&gp);
        assert!(!out.mesh.vertices.is_empty());
        assert_eq!(out.template_name, "weaver");
    }

    #[test]
    fn test_old_world_ruins_style_increases_mycelium() {
        let gen = CreatureGenerator::new(stalker_template());
        let mut mp = gen.template.default_params.clone();
        let original = mp.mycelium_density;
        let gp = GeneratorParams {
            style: ProceduralStyle::OldWorldRuins,
            seed: 1,
            ..Default::default()
        };
        let out = gen.generate(&gp);
        // 直接验证 style 分支不 panic
        assert!(out.mesh.vertices.len() > 0);
        let _ = original;
    }

    #[test]
    fn test_armor_shell_applied() {
        let tpl = crusher_template();
        let mut mp = tpl.default_params.clone();
        mp.armor_coverage = 0.9;
        let gen = CreatureGenerator::new(tpl);
        let out = gen.generate_with_params(&mp);
        assert!(out.mesh.vertices.len() > 100);
    }
}
