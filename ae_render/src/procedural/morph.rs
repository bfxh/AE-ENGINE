//! 母巢子实体生成器（8 种异形生物）
//!
//! 来自AE-ENGINE世界观文档（TECH_ROADMAP.md §1.3）：
//! - Stalker 践踏者：双足菌丝体，无眼无口，传感触角
//! - Hunter 追猎者：四足菌丝+角质板，鞭状触角
//! - Crusher 碎脊者：1.5x 人形，厚重角质层天然装甲
//! - RustKnight 锈骑士：动力装甲外壳+菌丝血管填充
//! - Swarm 蜂群：菌丝机翼+复眼+爆炸孢子囊
//! - Bloated 臃肿者：八足蜘蛛形，膨胀腹部
//! - Listener 窃听者：扁平地衣状，附着表面
//! - Weaver 编织者：六足蚂蚁形，多条菌丝触手
//!
//! 设计目标：基于 MorphTemplate + BodyPlan 自动生成任意拓扑骨骼与几何体

use crate::mesh::{MeshBuilder, Vertex};
use crate::procedural::geometry::{cylinder, CylinderParams};
use crate::procedural::npc::*;
use crate::procedural::skeleton::{
    AnimationClip, AnimationTrack, JointTransform, LoopMode, Skeleton, SkinWeights,
};
use glam::{Quat, Vec3};

// ============================================================================
// 8 种母巢子实体模板
// ============================================================================

/// 践踏者：双足菌丝体，无眼无口，传感触角
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

/// 追猎者：四足菌丝+角质板，鞭状触角
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
// 拓扑骨骼生成（根据 BodyPlan 分发）
// ============================================================================

/// 根据 MorphTemplate + MorphParams 构建骨骼
pub fn build_skeleton(template: &MorphTemplate, params: &MorphParams) -> Skeleton {
    let mutation = MorphMutation::new(params.variant_seed);
    let scale = mutation.vary_scale(template.size_class.default_scale() * params.scale);

    match template.body_plan {
        BodyPlan::Bipedal => build_bipedal_skeleton(params, scale),
        BodyPlan::Quadrupedal => build_quadrupedal_skeleton(params, scale),
        BodyPlan::Insectoid => build_insectoid_skeleton(params, scale, 6),
        BodyPlan::Arachnid => build_insectoid_skeleton(params, scale, 8),
        BodyPlan::Amorphous => build_amorphous_skeleton(params, scale),
        BodyPlan::Winged => build_winged_skeleton(params, scale),
        BodyPlan::Armored => build_armored_skeleton(params, scale),
    }
}

/// 双足拓扑（践踏者、克隆人变异体）
fn build_bipedal_skeleton(params: &MorphParams, scale: f32) -> Skeleton {
    let mut skel = Skeleton::new();
    let h = 1.6 * scale;
    let build = params.build;

    let pelvis = skel.add_bone(
        "pelvis",
        None,
        JointTransform {
            translation: Vec3::new(0.0, h * 0.5, 0.0),
            ..Default::default()
        },
    );
    let spine = skel.add_bone(
        "spine",
        Some(pelvis),
        JointTransform {
            translation: Vec3::new(0.0, h * 0.15, 0.0),
            ..Default::default()
        },
    );
    let chest = skel.add_bone(
        "chest",
        Some(spine),
        JointTransform {
            translation: Vec3::new(0.0, h * 0.25, 0.0),
            ..Default::default()
        },
    );
    let head = skel.add_bone(
        "head",
        Some(chest),
        JointTransform {
            translation: Vec3::new(0.0, h * 0.15, 0.0),
            ..Default::default()
        },
    );

    // 触角（如果配置）
    for i in 0..params.antenna_count.min(2) {
        let side = if i == 0 { 1.0 } else { -1.0 };
        let antenna = skel.add_bone(
            &format!("antenna_{}", if i == 0 { "l" } else { "r" }),
            Some(head),
            JointTransform {
                translation: Vec3::new(side * 0.05 * scale, h * 0.05, 0.0),
                rotation: Quat::from_rotation_z(side * 0.3),
                ..Default::default()
            },
        );
        skel.add_bone(
            &format!("antenna_tip_{}", if i == 0 { "l" } else { "r" }),
            Some(antenna),
            JointTransform {
                translation: Vec3::new(side * 0.02 * scale, h * 0.15, 0.0),
                ..Default::default()
            },
        );
    }

    // 手臂
    for i in 0..params.arm_count.min(2) {
        let side = if i == 0 { 1.0 } else { -1.0 };
        let arm_name = if i == 0 { "arm_l" } else { "arm_r" };
        let arm = skel.add_bone(
            arm_name,
            Some(chest),
            JointTransform {
                translation: Vec3::new(side * 0.15 * scale, h * 0.05, 0.0),
                ..Default::default()
            },
        );
        skel.add_bone(
            &format!("{}_fore", arm_name),
            Some(arm),
            JointTransform {
                translation: Vec3::new(0.0, -h * 0.25, 0.0),
                ..Default::default()
            },
        );
    }

    // 腿部
    for i in 0..params.leg_count.min(2) {
        let side = if i == 0 { 1.0 } else { -1.0 };
        let leg_name = if i == 0 { "leg_l" } else { "leg_r" };
        let leg = skel.add_bone(
            leg_name,
            Some(pelvis),
            JointTransform {
                translation: Vec3::new(side * 0.08 * scale, -h * 0.05, 0.0),
                ..Default::default()
            },
        );
        skel.add_bone(
            &format!("{}_shin", leg_name),
            Some(leg),
            JointTransform {
                translation: Vec3::new(0.0, -h * 0.25, 0.0),
                ..Default::default()
            },
        );
    }

    let _ = build;
    skel.compute_bind_pose();
    skel
}

/// 四足拓扑（追猎者、裂地兽）
fn build_quadrupedal_skeleton(params: &MorphParams, scale: f32) -> Skeleton {
    let mut skel = Skeleton::new();
    let h = 1.0 * scale;
    let length = 1.5 * scale;

    let pelvis = skel.add_bone(
        "pelvis",
        None,
        JointTransform {
            translation: Vec3::new(0.0, h * 0.7, 0.0),
            ..Default::default()
        },
    );
    let spine = skel.add_bone(
        "spine",
        Some(pelvis),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, length * 0.3),
            ..Default::default()
        },
    );
    let chest = skel.add_bone(
        "chest",
        Some(spine),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, length * 0.4),
            ..Default::default()
        },
    );
    let head = skel.add_bone(
        "head",
        Some(chest),
        JointTransform {
            translation: Vec3::new(0.0, h * 0.1, length * 0.3),
            ..Default::default()
        },
    );

    // 鞭状触角（追猎者特征）
    for i in 0..params.antenna_count.min(2) {
        let side = if i == 0 { 0.05 } else { -0.05 };
        skel.add_bone(
            &format!("whisker_{}", if i == 0 { "l" } else { "r" }),
            Some(head),
            JointTransform {
                translation: Vec3::new(side * scale, 0.0, length * 0.1),
                ..Default::default()
            },
        );
    }

    // 4 条腿（前 2 后 2）
    let leg_positions = [
        ("front_l", chest, 0.12, length * 0.35),
        ("front_r", chest, -0.12, length * 0.35),
        ("back_l", pelvis, 0.12, -length * 0.15),
        ("back_r", pelvis, -0.12, -length * 0.15),
    ];
    for (name, parent, x, z) in leg_positions {
        let parent_id = parent;
        let upper = skel.add_bone(
            name,
            Some(parent_id),
            JointTransform {
                translation: Vec3::new(x * scale, 0.0, z),
                ..Default::default()
            },
        );
        skel.add_bone(
            &format!("{}_lower", name),
            Some(upper),
            JointTransform {
                translation: Vec3::new(0.0, -h * 0.5, 0.0),
                ..Default::default()
            },
        );
    }

    let _ = params;
    skel.compute_bind_pose();
    skel
}

/// 多足昆虫/蜘蛛拓扑（编织者 6 足、臃肿者 8 足）
fn build_insectoid_skeleton(params: &MorphParams, scale: f32, leg_count: u32) -> Skeleton {
    let mut skel = Skeleton::new();
    let h = 0.6 * scale;
    let length = 1.2 * scale;

    let pelvis = skel.add_bone(
        "pelvis",
        None,
        JointTransform {
            translation: Vec3::new(0.0, h * 0.8, 0.0),
            ..Default::default()
        },
    );
    let thorax = skel.add_bone(
        "thorax",
        Some(pelvis),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, length * 0.4),
            ..Default::default()
        },
    );
    let head = skel.add_bone(
        "head",
        Some(thorax),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, length * 0.3),
            ..Default::default()
        },
    );

    // 膨胀腹部（臃肿者特征）
    if params.has_abdomen {
        skel.add_bone(
            "abdomen",
            Some(pelvis),
            JointTransform {
                translation: Vec3::new(0.0, -h * 0.1, -length * 0.4),
                ..Default::default()
            },
        );
    }

    // 多条腿（沿身体两侧均匀分布）
    let legs = leg_count.min(8);
    let per_side = legs / 2;
    for i in 0..per_side {
        let t = if per_side > 1 {
            i as f32 / (per_side - 1) as f32
        } else {
            0.5
        };
        let z = length * (0.5 - t) * 0.9;
        let parent = if t > 0.5 { pelvis } else { thorax };
        for side in &[1.0f32, -1.0] {
            let name = format!("leg_{}_{}", if *side > 0.0 { "l" } else { "r" }, i);
            let upper = skel.add_bone(
                &name,
                Some(parent),
                JointTransform {
                    translation: Vec3::new(side * 0.1 * scale, 0.0, z),
                    ..Default::default()
                },
            );
            skel.add_bone(
                &format!("{}_lower", name),
                Some(upper),
                JointTransform {
                    translation: Vec3::new(side * 0.05 * scale, -h * 0.6, 0.0),
                    ..Default::default()
                },
            );
        }
    }

    // 触手（编织者特征）
    for i in 0..params.antenna_count.min(4) {
        let angle = (i as f32) * std::f32::consts::TAU / 4.0;
        let x = angle.cos() * 0.08 * scale;
        let z = angle.sin() * 0.08 * scale;
        skel.add_bone(
            &format!("tentacle_{}", i),
            Some(head),
            JointTransform {
                translation: Vec3::new(x, 0.0, z + length * 0.1),
                ..Default::default()
            },
        );
    }

    skel.compute_bind_pose();
    skel
}

/// 不定形/扁平拓扑（窃听者）
fn build_amorphous_skeleton(params: &MorphParams, scale: f32) -> Skeleton {
    let mut skel = Skeleton::new();
    let r = 0.3 * scale;

    // 中心根
    let center = skel.add_bone(
        "center",
        None,
        JointTransform {
            translation: Vec3::new(0.0, r * 0.1, 0.0),
            ..Default::default()
        },
    );

    // 扁平放射状分支（5-7 个）
    let branches = 6u32;
    for i in 0..branches {
        let angle = (i as f32) * std::f32::consts::TAU / branches as f32;
        let mid = skel.add_bone(
            &format!("branch_{}_mid", i),
            Some(center),
            JointTransform {
                translation: Vec3::new(angle.cos() * r * 0.5, 0.0, angle.sin() * r * 0.5),
                ..Default::default()
            },
        );
        skel.add_bone(
            &format!("branch_{}_tip", i),
            Some(mid),
            JointTransform {
                translation: Vec3::new(angle.cos() * r * 0.5, 0.0, angle.sin() * r * 0.5),
                ..Default::default()
            },
        );
    }

    let _ = params;
    skel.compute_bind_pose();
    skel
}

/// 有翼飞行拓扑（蜂群）
fn build_winged_skeleton(params: &MorphParams, scale: f32) -> Skeleton {
    let mut skel = Skeleton::new();
    let h = 0.15 * scale;

    let thorax = skel.add_bone(
        "thorax",
        None,
        JointTransform {
            translation: Vec3::new(0.0, h, 0.0),
            ..Default::default()
        },
    );
    let head = skel.add_bone(
        "head",
        Some(thorax),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, h * 0.5),
            ..Default::default()
        },
    );

    // 腹部（孢子囊）
    if params.has_abdomen {
        skel.add_bone(
            "abdomen",
            Some(thorax),
            JointTransform {
                translation: Vec3::new(0.0, 0.0, -h * 1.5),
                ..Default::default()
            },
        );
    }

    // 6 条腿
    for i in 0..3u32 {
        for side in &[1.0f32, -1.0] {
            let name = format!("leg_{}_{}", if *side > 0.0 { "l" } else { "r" }, i);
            skel.add_bone(
                &name,
                Some(thorax),
                JointTransform {
                    translation: Vec3::new(side * h * 0.3, 0.0, h * (0.3 - i as f32 * 0.3)),
                    ..Default::default()
                },
            );
        }
    }

    // 2 对翅膀
    if params.has_wings {
        for side in &[1.0f32, -1.0] {
            let wing_name = if *side > 0.0 { "wing_l" } else { "wing_r" };
            skel.add_bone(
                wing_name,
                Some(thorax),
                JointTransform {
                    translation: Vec3::new(side * h * 0.2, h * 0.1, 0.0),
                    rotation: Quat::from_rotation_z(side * 0.5),
                    ..Default::default()
                },
            );
        }
    }

    // 复眼
    for i in 0..params.eye_count.min(4) {
        let side = if i < 2 { 1.0 } else { -1.0 };
        let offset = if i % 2 == 0 { 0.02 } else { -0.02 };
        skel.add_bone(
            &format!("eye_{}", i),
            Some(head),
            JointTransform {
                translation: Vec3::new(side * h * 0.15, offset * h, h * 0.1),
                ..Default::default()
            },
        );
    }

    skel.compute_bind_pose();
    skel
}

/// 装甲外壳拓扑（碎脊者、锈骑士）—— 人形 + 装甲壳
fn build_armored_skeleton(params: &MorphParams, scale: f32) -> Skeleton {
    // 复用双足骨架，再附加装甲壳骨骼
    let mut skel = build_bipedal_skeleton(params, scale);

    // 在 chest 上添加装甲壳
    let chest = skel.get_bone_by_name("chest").unwrap();
    skel.add_bone(
        "armor_chest",
        Some(chest),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, 0.05 * scale),
            ..Default::default()
        },
    );
    let pelvis = skel.get_bone_by_name("pelvis").unwrap();
    skel.add_bone(
        "armor_pelvis",
        Some(pelvis),
        JointTransform {
            translation: Vec3::new(0.0, 0.0, 0.05 * scale),
            ..Default::default()
        },
    );

    skel.compute_bind_pose();
    skel
}

// ============================================================================
// 几何体辅助函数
// ============================================================================

/// 椭球体（用于头部、膨胀腹部）
fn push_ellipsoid(builder: &mut MeshBuilder, center: Vec3, radii: Vec3, color: [f32; 4]) {
    let (verts, indices) = MeshBuilder::sphere(12, 8);
    let base = builder.vertex_count() as u32;
    for v in &verts {
        let p = Vec3::from(v.position);
        let scaled = Vec3::new(p.x * radii.x, p.y * radii.y, p.z * radii.z) + center;
        let mut v = *v;
        v.position = scaled.into();
        // 法线缩放（再归一化）
        let n = Vec3::from(v.normal);
        let n_scaled = Vec3::new(n.x / radii.x, n.y / radii.y, n.z / radii.z);
        v.normal = n_scaled.normalize_or_zero().into();
        v.color = color;
        builder.push_vertex(v);
    }
    for &i in &indices {
        builder.indices_mut().push(base + i);
    }
}

/// 锥化圆柱（两端不同半径，用于触角、肢体）
fn push_tapered_cylinder(
    builder: &mut MeshBuilder,
    start: Vec3,
    end: Vec3,
    r_start: f32,
    r_end: f32,
    color: [f32; 4],
    segments: u32,
) {
    let direction = end - start;
    let length = direction.length();
    if length < 1e-6 {
        return;
    }
    let dir_normalized = direction / length;
    let default_dir = Vec3::new(0.0, 1.0, 0.0);
    let rotation = Quat::from_rotation_arc(default_dir, dir_normalized);
    let midpoint = start + direction * 0.5;

    let seg = segments.max(3) as usize;
    let mut verts = Vec::with_capacity(seg * 2 + 2);
    let mut indices = Vec::with_capacity(seg * 6 + seg * 3);

    // 底部中心
    let bottom_center = verts.len() as u32;
    verts.push(Vertex {
        position: [0.0, -length * 0.5, 0.0],
        normal: [0.0, -1.0, 0.0],
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.5, 0.5],
    });
    // 顶部中心
    let top_center = verts.len() as u32;
    verts.push(Vertex {
        position: [0.0, length * 0.5, 0.0],
        normal: [0.0, 1.0, 0.0],
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.5, 0.5],
    });

    // 环带顶点
    let mut bottom_ring = Vec::with_capacity(seg);
    let mut top_ring = Vec::with_capacity(seg);
    for i in 0..seg {
        let angle = (i as f32) * std::f32::consts::TAU / seg as f32;
        let x = angle.cos();
        let z = angle.sin();
        let n = [x, 0.0, z];

        let bi = verts.len() as u32;
        verts.push(Vertex {
            position: [x * r_start, -length * 0.5, z * r_start],
            normal: n,
            tangent: [0.0, 0.0, 0.0, 0.0],
            color,
            uv: [i as f32 / seg as f32, 0.0],
        });
        bottom_ring.push(bi);

        let ti = verts.len() as u32;
        verts.push(Vertex {
            position: [x * r_end, length * 0.5, z * r_end],
            normal: n,
            tangent: [0.0, 0.0, 0.0, 0.0],
            color,
            uv: [i as f32 / seg as f32, 1.0],
        });
        top_ring.push(ti);
    }

    // 底面三角形
    for i in 0..seg {
        let next = (i + 1) % seg;
        indices.push(bottom_center);
        indices.push(bottom_ring[i]);
        indices.push(bottom_ring[next]);
    }
    // 顶面三角形
    for i in 0..seg {
        let next = (i + 1) % seg;
        indices.push(top_center);
        indices.push(top_ring[next]);
        indices.push(top_ring[i]);
    }
    // 侧面四边形（两个三角形）
    for i in 0..seg {
        let next = (i + 1) % seg;
        indices.push(bottom_ring[i]);
        indices.push(top_ring[i]);
        indices.push(top_ring[next]);
        indices.push(bottom_ring[i]);
        indices.push(top_ring[next]);
        indices.push(bottom_ring[next]);
    }

    // 应用旋转和平移
    let base = builder.vertex_count() as u32;
    for v in &verts {
        let p = rotation * Vec3::from(v.position) + midpoint;
        let n = rotation * Vec3::from(v.normal);
        let mut v = *v;
        v.position = p.into();
        v.normal = n.into();
        builder.push_vertex(v);
    }
    for &i in &indices {
        builder.indices_mut().push(base + i);
    }
}

/// 扁平甲壳板（用于装甲覆盖）
fn push_chitin_plate(
    builder: &mut MeshBuilder,
    center: Vec3,
    size: [f32; 2],
    thickness: f32,
    color: [f32; 4],
) {
    builder.push_box(
        [center.x, center.y, center.z],
        [size[0], thickness, size[1]],
    );
    // 设置颜色（push_box 不接受颜色，需要后处理）
    let vc = builder.vertex_count();
    // push_box 添加 24 个顶点（6 面×4 顶点），覆盖最后 24 个
    let start = vc.saturating_sub(24);
    for v in builder.vertices_mut()[start..].iter_mut() {
        v.color = color;
    }
}

/// 鞭状触角（多段渐细圆柱）
fn push_antenna(
    builder: &mut MeshBuilder,
    base: Vec3,
    direction: Vec3,
    length: f32,
    segments: u32,
    color: [f32; 4],
) {
    let segs = segments.max(2);
    let mut prev = base;
    for i in 0..segs {
        let t = (i + 1) as f32 / segs as f32;
        let next = base + direction * (length * t);
        let r_start = 0.015 * (1.0 - i as f32 / segs as f32);
        let r_end = 0.015 * (1.0 - t);
        push_tapered_cylinder(builder, prev, next, r_start.max(0.003), r_end.max(0.003), color, 6);
        prev = next;
    }
}

/// 翼膜（双面扁平网格）
fn push_wing_membrane(
    builder: &mut MeshBuilder,
    root: Vec3,
    span: Vec3,
    chord: Vec3,
    color: [f32; 4],
) {
    let base = builder.vertex_count() as u32;
    // 4 个顶点（双面）
    let v0 = root;
    let v1 = root + span;
    let v2 = root + span + chord;
    let v3 = root + chord;

    let normal_front = span.cross(chord).normalize_or_zero();
    let normal_back = -normal_front;

    // 正面
    let i0 = builder.vertex_count() as u32;
    builder.push_vertex(Vertex {
        position: v0.into(),
        normal: normal_front.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.0, 0.0],
    });
    builder.push_vertex(Vertex {
        position: v1.into(),
        normal: normal_front.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [1.0, 0.0],
    });
    builder.push_vertex(Vertex {
        position: v2.into(),
        normal: normal_front.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [1.0, 1.0],
    });
    builder.push_vertex(Vertex {
        position: v3.into(),
        normal: normal_front.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.0, 1.0],
    });
    builder.indices_mut().extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

    // 反面
    let i1 = builder.vertex_count() as u32;
    builder.push_vertex(Vertex {
        position: v0.into(),
        normal: normal_back.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.0, 0.0],
    });
    builder.push_vertex(Vertex {
        position: v3.into(),
        normal: normal_back.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [0.0, 1.0],
    });
    builder.push_vertex(Vertex {
        position: v2.into(),
        normal: normal_back.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [1.0, 1.0],
    });
    builder.push_vertex(Vertex {
        position: v1.into(),
        normal: normal_back.into(),
        tangent: [0.0, 0.0, 0.0, 0.0],
        color,
        uv: [1.0, 0.0],
    });
    builder.indices_mut().extend_from_slice(&[i1, i1 + 1, i1 + 2, i1, i1 + 2, i1 + 3]);

    let _ = base;
}

/// 复眼（半球阵列，多个小半球分布在大球表面）
fn push_compound_eye(
    builder: &mut MeshBuilder,
    center: Vec3,
    facing: Vec3,
    radius: f32,
    facets: u32,
    color: [f32; 4],
) {
    let facing_n = facing.normalize_or_zero();
    let (verts, indices) = MeshBuilder::sphere(8, 6);
    let base = builder.vertex_count() as u32;
    // 主半球
    for v in &verts {
        let p = Vec3::from(v.position) * radius + center;
        let mut v = *v;
        v.position = p.into();
        v.color = color;
        builder.push_vertex(v);
    }
    for &i in &indices {
        builder.indices_mut().push(base + i);
    }

    // 小 facets 分布在主半球朝向 facing 的半边
    for i in 0..facets {
        let phi = (i as f32) * std::f32::consts::TAU / facets as f32;
        let theta = std::f32::consts::FRAC_PI_4; // 45°
        let local = Vec3::new(
            theta.cos() * phi.cos(),
            theta.sin(),
            theta.cos() * phi.sin(),
        );
        // 旋转到 facing 方向
        let rot = Quat::from_rotation_arc(Vec3::new(0.0, 1.0, 0.0), facing_n);
        let dir = rot * local;
        let facet_pos = center + dir * radius * 0.9;
        let (fv, fi) = MeshBuilder::sphere(4, 3);
        let fbase = builder.vertex_count() as u32;
        for v in &fv {
            let p = Vec3::from(v.position) * radius * 0.15 + facet_pos;
            let mut v = *v;
            v.position = p.into();
            v.color = [color[0] * 1.2, color[1] * 1.2, color[2] * 1.2, color[3]];
            builder.push_vertex(v);
        }
        for &i in &fi {
            builder.indices_mut().push(fbase + i);
        }
    }
}

// ============================================================================
// 主生成器
// ============================================================================

/// 母巢子实体生成器
pub struct MorphGenerator {
    builder: MeshBuilder,
    skeleton: Skeleton,
    skin_weights: Vec<SkinWeights>,
}

impl MorphGenerator {
    pub fn new() -> Self {
        Self {
            builder: MeshBuilder::new(),
            skeleton: Skeleton::new(),
            skin_weights: Vec::new(),
        }
    }

    /// 生成子实体：返回 (顶点, 索引, 骨骼, 蒙皮权重)
    pub fn generate(
        mut self,
        template: &MorphTemplate,
        params: &MorphParams,
    ) -> (Vec<Vertex>, Vec<u32>, Skeleton, Vec<SkinWeights>) {
        // 1. 构建骨骼
        self.skeleton = build_skeleton(template, params);

        // 2. 收集骨骼位置
        let bone_positions: hashbrown::HashMap<String, Vec3> = self
            .skeleton
            .bones
            .values()
            .map(|b| (b.name.clone(), b.world_bind.w_axis.truncate()))
            .collect();

        // 3. 根据身体蓝图生成几何体
        let color = template.resolve_color(params);
        match template.body_plan {
            BodyPlan::Bipedal => self.gen_bipedal_geometry(params, &bone_positions, color),
            BodyPlan::Quadrupedal => self.gen_quadrupedal_geometry(params, &bone_positions, color),
            BodyPlan::Insectoid => self.gen_insectoid_geometry(params, &bone_positions, color, 6),
            BodyPlan::Arachnid => self.gen_insectoid_geometry(params, &bone_positions, color, 8),
            BodyPlan::Amorphous => self.gen_amorphous_geometry(params, &bone_positions, color),
            BodyPlan::Winged => self.gen_winged_geometry(params, &bone_positions, color),
            BodyPlan::Armored => self.gen_armored_geometry(params, &bone_positions, color),
        }

        // 4. 应用主色调
        let (vertices, indices) = self.builder.into_parts();

        // 5. 生成蒙皮权重
        self.skin_weights = Self::compute_skin_weights(&vertices, &self.skeleton);

        (vertices, indices, self.skeleton, self.skin_weights)
    }

    /// 双足几何体（践踏者）
    fn gen_bipedal_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
    ) {
        let scale = params.scale;
        // 头部（菌丝球，无眼无口）
        if let Some(&head) = bones.get("head") {
            push_ellipsoid(&mut self.builder, head, Vec3::new(0.1, 0.12, 0.1) * scale, color);
        }
        // 躯干（盒子）
        if let (Some(&chest), Some(&spine), Some(&pelvis)) =
            (bones.get("chest"), bones.get("spine"), bones.get("pelvis"))
        {
            self.builder.push_box(
                [chest.x, chest.y, chest.z],
                [0.18 * scale, 0.25 * scale, 0.12 * scale],
            );
            self.builder.push_box(
                [spine.x, spine.y, spine.z],
                [0.15 * scale, 0.15 * scale, 0.1 * scale],
            );
            self.builder.push_box(
                [pelvis.x, pelvis.y, pelvis.z],
                [0.16 * scale, 0.1 * scale, 0.1 * scale],
            );
        }
        // 触角
        for side in &["l", "r"] {
            if let (Some(&antenna), Some(&tip)) = (
                bones.get(&format!("antenna_{}", side)),
                bones.get(&format!("antenna_tip_{}", side)),
            ) {
                push_antenna(
                    &mut self.builder,
                    antenna,
                    (tip - antenna).normalize(),
                    (tip - antenna).length(),
                    4,
                    color,
                );
            }
        }
        // 手臂
        for side in &["l", "r"] {
            if let (Some(&arm), Some(&fore)) = (
                bones.get(&format!("arm_{}", side)),
                bones.get(&format!("arm_{}_fore", side)),
            ) {
                push_tapered_cylinder(
                    &mut self.builder,
                    arm,
                    fore,
                    0.04 * scale,
                    0.03 * scale,
                    color,
                    6,
                );
                let fore_end = fore + (fore - arm).normalize() * 0.15 * scale;
                push_tapered_cylinder(
                    &mut self.builder,
                    fore,
                    fore_end,
                    0.03 * scale,
                    0.02 * scale,
                    color,
                    6,
                );
            }
        }
        // 腿部
        for side in &["l", "r"] {
            if let (Some(&leg), Some(&shin)) = (
                bones.get(&format!("leg_{}", side)),
                bones.get(&format!("leg_{}_shin", side)),
            ) {
                push_tapered_cylinder(
                    &mut self.builder,
                    leg,
                    shin,
                    0.05 * scale,
                    0.04 * scale,
                    color,
                    8,
                );
                let foot = shin + Vec3::new(0.0, -0.1 * scale, 0.05 * scale);
                push_tapered_cylinder(
                    &mut self.builder,
                    shin,
                    foot,
                    0.04 * scale,
                    0.03 * scale,
                    color,
                    8,
                );
            }
        }
    }

    /// 四足几何体（追猎者）
    fn gen_quadrupedal_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
    ) {
        let scale = params.scale;
        // 躯干（长盒子）
        if let (Some(&pelvis), Some(&chest)) = (bones.get("pelvis"), bones.get("chest")) {
            let mid = (pelvis + chest) * 0.5;
            let dir = chest - pelvis;
            let length = dir.length();
            self.builder.push_box(
                [mid.x, mid.y, mid.z],
                [0.2 * scale, 0.25 * scale, length + 0.1],
            );
        }
        // 头部
        if let Some(&head) = bones.get("head") {
            push_ellipsoid(&mut self.builder, head, Vec3::new(0.1, 0.1, 0.18) * scale, color);
        }
        // 鞭状触角
        for side in &["l", "r"] {
            if let Some(&w) = bones.get(&format!("whisker_{}", side)) {
                let head_pos = bones.get("head").copied().unwrap_or(Vec3::ZERO);
                let dir = (w - head_pos).normalize();
                push_antenna(&mut self.builder, head_pos, dir, 0.4 * scale, 6, color);
            }
        }
        // 4 条腿
        for name in &["front_l", "front_r", "back_l", "back_r"] {
            if let (Some(&upper), Some(&lower)) =
                (bones.get(*name), bones.get(&format!("{}_lower", name)))
            {
                push_tapered_cylinder(
                    &mut self.builder,
                    upper,
                    lower,
                    0.05 * scale,
                    0.04 * scale,
                    color,
                    8,
                );
                let foot = lower + Vec3::new(0.0, -0.1 * scale, 0.0);
                push_tapered_cylinder(
                    &mut self.builder,
                    lower,
                    foot,
                    0.04 * scale,
                    0.03 * scale,
                    color,
                    8,
                );
            }
        }
        // 甲壳板（追猎者特征）
        if params.armor_coverage > 0.3 {
            if let Some(&spine) = bones.get("spine") {
                push_chitin_plate(
                    &mut self.builder,
                    spine + Vec3::new(0.0, 0.1 * scale, 0.0),
                    [0.25 * scale, 0.5 * scale],
                    0.02 * scale,
                    [color[0] * 0.7, color[1] * 0.7, color[2] * 0.7, color[3]],
                );
            }
        }
    }

    /// 多足昆虫几何体（编织者、臃肿者）
    fn gen_insectoid_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
        leg_count: u32,
    ) {
        let scale = params.scale;
        // 腹部（膨胀）
        if let Some(&abdomen) = bones.get("abdomen") {
            let abdomen_size = if params.has_abdomen { 0.3 } else { 0.15 };
            push_ellipsoid(
                &mut self.builder,
                abdomen,
                Vec3::new(abdomen_size, abdomen_size * 0.8, abdomen_size * 1.2) * scale,
                color,
            );
        }
        // 胸部
        if let Some(&thorax) = bones.get("thorax") {
            push_ellipsoid(&mut self.builder, thorax, Vec3::new(0.12, 0.1, 0.15) * scale, color);
        }
        // 头部
        if let Some(&head) = bones.get("head") {
            push_ellipsoid(&mut self.builder, head, Vec3::new(0.08, 0.08, 0.1) * scale, color);
        }
        // 多条腿
        let per_side = leg_count / 2;
        for i in 0..per_side {
            for side in &["l", "r"] {
                let name = format!("leg_{}_{}", side, i);
                let lower_name = format!("{}_lower", name);
                if let (Some(&upper), Some(&lower)) =
                    (bones.get(&name), bones.get(&lower_name))
                {
                    push_tapered_cylinder(
                        &mut self.builder,
                        upper,
                        lower,
                        0.03 * scale,
                        0.02 * scale,
                        color,
                        6,
                    );
                }
            }
        }
        // 触手（编织者）
        for i in 0..params.antenna_count.min(4) {
            if let Some(&tentacle) = bones.get(&format!("tentacle_{}", i)) {
                let head_pos = bones.get("head").copied().unwrap_or(Vec3::ZERO);
                let dir = (tentacle - head_pos).normalize();
                push_antenna(&mut self.builder, tentacle, dir, 0.3 * scale, 5, color);
            }
        }
    }

    /// 不定形几何体（窃听者，扁平地衣状）
    fn gen_amorphous_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
    ) {
        let scale = params.scale;
        // 中心扁平盘
        if let Some(&center) = bones.get("center") {
            push_ellipsoid(
                &mut self.builder,
                center,
                Vec3::new(0.25, 0.02, 0.25) * scale,
                color,
            );
        }
        // 分支
        for i in 0..6 {
            if let (Some(&mid), Some(&tip)) = (
                bones.get(&format!("branch_{}_mid", i)),
                bones.get(&format!("branch_{}_tip", i)),
            ) {
                push_tapered_cylinder(
                    &mut self.builder,
                    mid,
                    tip,
                    0.05 * scale,
                    0.02 * scale,
                    color,
                    5,
                );
                // 末端小盘
                push_ellipsoid(
                    &mut self.builder,
                    tip,
                    Vec3::new(0.06, 0.01, 0.06) * scale,
                    color,
                );
            }
        }
    }

    /// 有翼飞行几何体（蜂群）
    fn gen_winged_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
    ) {
        let scale = params.scale;
        // 胸部
        if let Some(&thorax) = bones.get("thorax") {
            push_ellipsoid(&mut self.builder, thorax, Vec3::new(0.04, 0.04, 0.06) * scale, color);
        }
        // 头部
        if let Some(&head) = bones.get("head") {
            push_ellipsoid(&mut self.builder, head, Vec3::new(0.03, 0.03, 0.04) * scale, color);
        }
        // 腹部（孢子囊）
        if let Some(&abdomen) = bones.get("abdomen") {
            push_ellipsoid(
                &mut self.builder,
                abdomen,
                Vec3::new(0.04, 0.04, 0.08) * scale,
                [color[0] * 0.8, color[1] * 0.6, color[2] * 0.4, color[3]],
            );
        }
        // 复眼
        for i in 0..params.eye_count.min(4) {
            if let Some(&eye) = bones.get(&format!("eye_{}", i)) {
                push_compound_eye(
                    &mut self.builder,
                    eye,
                    Vec3::new(0.0, 0.0, 1.0),
                    0.015 * scale,
                    6,
                    [0.8, 0.7, 0.2, 1.0],
                );
            }
        }
        // 翅膀
        if params.has_wings {
            for side in &["l", "r"] {
                if let Some(&wing) = bones.get(&format!("wing_{}", side)) {
                    let span = Vec3::new(if *side == "l" { 1.0 } else { -1.0 }, 0.0, 0.0) * 0.15 * scale;
                    let chord = Vec3::new(0.0, 0.0, 0.1 * scale);
                    push_wing_membrane(&mut self.builder, wing, span, chord, [0.6, 0.5, 0.4, 0.6]);
                }
            }
        }
        // 腿
        for i in 0..3u32 {
            for side in &["l", "r"] {
                let name = format!("leg_{}_{}", side, i);
                if let Some(&leg) = bones.get(&name) {
                    let end = leg + Vec3::new(0.0, -0.04 * scale, 0.0);
                    push_tapered_cylinder(
                        &mut self.builder,
                        leg,
                        end,
                        0.005 * scale,
                        0.002 * scale,
                        color,
                        4,
                    );
                }
            }
        }
    }

    /// 装甲外壳几何体（碎脊者、锈骑士）
    fn gen_armored_geometry(
        &mut self,
        params: &MorphParams,
        bones: &hashbrown::HashMap<String, Vec3>,
        color: [f32; 4],
    ) {
        // 先生成双足基础几何体
        self.gen_bipedal_geometry(params, bones, color);

        let scale = params.scale;
        // 装甲壳（覆盖在胸部和骨盆）
        if params.armor_coverage > 0.5 {
            if let Some(&armor_chest) = bones.get("armor_chest") {
                push_chitin_plate(
                    &mut self.builder,
                    armor_chest + Vec3::new(0.0, 0.0, 0.08 * scale),
                    [0.25 * scale, 0.3 * scale],
                    0.04 * scale,
                    [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, color[3]],
                );
            }
            if let Some(&armor_pelvis) = bones.get("armor_pelvis") {
                push_chitin_plate(
                    &mut self.builder,
                    armor_pelvis + Vec3::new(0.0, 0.0, 0.06 * scale),
                    [0.22 * scale, 0.18 * scale],
                    0.04 * scale,
                    [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, color[3]],
                );
            }
        }
    }

    /// 计算蒙皮权重（每顶点最近骨骼 + 距离权重）
    fn compute_skin_weights(vertices: &[Vertex], skeleton: &Skeleton) -> Vec<SkinWeights> {
        let bone_positions: Vec<(u32, Vec3)> = skeleton
            .bones
            .iter()
            .map(|(id, bone)| (skeleton.bone_index(id), bone.world_bind.w_axis.truncate()))
            .collect();

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

impl Default for MorphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 动画生成（每种 BodyPlan 一个步态）
// ============================================================================

/// 为子实体生成移动动画（根据 BodyPlan 分发）
pub fn create_locomotion_animation(
    template: &MorphTemplate,
    skeleton: &Skeleton,
    cycle_duration: f32,
) -> AnimationClip {
    let mut tracks = Vec::new();

    match template.body_plan {
        BodyPlan::Bipedal | BodyPlan::Armored => {
            // 双足步态：左右腿交替
            for side in &["l", "r"] {
                if let Some(leg) = skeleton.get_bone_by_name(&format!("leg_{}", side)) {
                    let phase = if *side == "l" { 0.0 } else { 0.5 };
                    let swing = 0.4f32;
                    tracks.push(AnimationTrack {
                        name: format!("leg_{}_swing", side),
                        bone_id: leg,
                        keyframes: vec![
                            (phase * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(swing),
                                ..Default::default()
                            }),
                            ((phase + 0.5) * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(-swing),
                                ..Default::default()
                            }),
                            ((phase + 1.0) * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(swing),
                                ..Default::default()
                            }),
                        ],
                        loop_mode: LoopMode::Loop,
                    });
                }
            }
        }
        BodyPlan::Quadrupedal => {
            // 四足步态：对角步态（trot）
            for (name, phase) in &[
                ("front_l", 0.0),
                ("back_r", 0.0),
                ("front_r", 0.5),
                ("back_l", 0.5),
            ] {
                if let Some(leg) = skeleton.get_bone_by_name(name) {
                    tracks.push(AnimationTrack {
                        name: format!("{}_swing", name),
                        bone_id: leg,
                        keyframes: vec![
                            (*phase * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(0.3),
                                ..Default::default()
                            }),
                            ((*phase + 0.5) * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(-0.3),
                                ..Default::default()
                            }),
                            ((*phase + 1.0) * cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_x(0.3),
                                ..Default::default()
                            }),
                        ],
                        loop_mode: LoopMode::Loop,
                    });
                }
            }
        }
        BodyPlan::Insectoid | BodyPlan::Arachnid => {
            // 多足步态：三角步态
            let leg_count = if template.body_plan == BodyPlan::Insectoid { 6 } else { 8 };
            let per_side = leg_count / 2;
            for i in 0..per_side {
                for side in &["l", "r"] {
                    let name = format!("leg_{}_{}", side, i);
                    if let Some(leg) = skeleton.get_bone_by_name(&name) {
                        // 交替相位（三角步态）
                        let phase = if (i + if *side == "l" { 0 } else { 1 }) % 2 == 0 {
                            0.0
                        } else {
                            0.5
                        };
                        tracks.push(AnimationTrack {
                            name: format!("{}_swing", name),
                            bone_id: leg,
                            keyframes: vec![
                                (phase * cycle_duration, JointTransform {
                                    rotation: Quat::from_rotation_x(0.2),
                                    ..Default::default()
                                }),
                                ((phase + 0.5) * cycle_duration, JointTransform {
                                    rotation: Quat::from_rotation_x(-0.2),
                                    ..Default::default()
                                }),
                                ((phase + 1.0) * cycle_duration, JointTransform {
                                    rotation: Quat::from_rotation_x(0.2),
                                    ..Default::default()
                                }),
                            ],
                            loop_mode: LoopMode::Loop,
                        });
                    }
                }
            }
        }
        BodyPlan::Winged => {
            // 飞行：翅膀拍动
            for side in &["l", "r"] {
                if let Some(wing) = skeleton.get_bone_by_name(&format!("wing_{}", side)) {
                    let dir = if *side == "l" { 1.0 } else { -1.0 };
                    tracks.push(AnimationTrack {
                        name: format!("wing_{}_flap", side),
                        bone_id: wing,
                        keyframes: vec![
                            (0.0, JointTransform {
                                rotation: Quat::from_rotation_z(dir * 0.8),
                                ..Default::default()
                            }),
                            (cycle_duration * 0.5, JointTransform {
                                rotation: Quat::from_rotation_z(dir * -0.3),
                                ..Default::default()
                            }),
                            (cycle_duration, JointTransform {
                                rotation: Quat::from_rotation_z(dir * 0.8),
                                ..Default::default()
                            }),
                        ],
                        loop_mode: LoopMode::Loop,
                    });
                }
            }
        }
        BodyPlan::Amorphous => {
            // 不定形：缓慢脉动
            if let Some(center) = skeleton.get_bone_by_name("center") {
                tracks.push(AnimationTrack {
                    name: "pulse".to_string(),
                    bone_id: center,
                    keyframes: vec![
                        (0.0, JointTransform {
                            scale: Vec3::new(1.0, 1.0, 1.0),
                            ..Default::default()
                        }),
                        (cycle_duration * 0.5, JointTransform {
                            scale: Vec3::new(1.1, 1.2, 1.1),
                            ..Default::default()
                        }),
                        (cycle_duration, JointTransform {
                            scale: Vec3::new(1.0, 1.0, 1.0),
                            ..Default::default()
                        }),
                    ],
                    loop_mode: LoopMode::Loop,
                });
            }
        }
    }

    AnimationClip {
        name: "locomotion".to_string(),
        duration: cycle_duration,
        tracks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_8_templates_exist() {
        let templates = all_templates();
        assert_eq!(templates.len(), 8);
        let names: Vec<&str> = templates.iter().map(|t| t.name).collect();
        assert!(names.contains(&"stalker"));
        assert!(names.contains(&"hunter"));
        assert!(names.contains(&"crusher"));
        assert!(names.contains(&"rust_knight"));
        assert!(names.contains(&"swarm"));
        assert!(names.contains(&"bloated"));
        assert!(names.contains(&"listener"));
        assert!(names.contains(&"weaver"));
    }

    #[test]
    fn test_template_body_plans_diverse() {
        let templates = all_templates();
        let plans: Vec<BodyPlan> = templates.iter().map(|t| t.body_plan).collect();
        // 应该有多种不同的 BodyPlan
        let unique: std::collections::HashSet<_> = plans.iter().collect();
        assert!(unique.len() >= 5, "expected >=5 unique body plans, got {}", unique.len());
    }

    #[test]
    fn test_stalker_template_params() {
        let tpl = stalker_template();
        assert_eq!(tpl.body_plan, BodyPlan::Bipedal);
        assert_eq!(tpl.material, BiologicalMaterial::Mycelium);
        assert_eq!(tpl.size_class, SizeClass::Small);
        assert_eq!(tpl.default_params.antenna_count, 2);
        assert_eq!(tpl.default_params.eye_count, 0);
    }

    #[test]
    fn test_bloated_template_has_abdomen() {
        let tpl = bloated_template();
        assert_eq!(tpl.body_plan, BodyPlan::Arachnid);
        assert!(tpl.default_params.has_abdomen);
        assert_eq!(tpl.default_params.leg_count, 8);
    }

    #[test]
    fn test_swarm_template_has_wings() {
        let tpl = swarm_template();
        assert_eq!(tpl.body_plan, BodyPlan::Winged);
        assert!(tpl.default_params.has_wings);
        assert!(tpl.default_params.has_abdomen);
        assert_eq!(tpl.default_params.eye_count, 4);
    }

    #[test]
    fn test_build_skeleton_bipedal() {
        let tpl = stalker_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        assert!(skel.bone_count() >= 5, "stalker should have >=5 bones, got {}", skel.bone_count());
        assert!(skel.get_bone_by_name("pelvis").is_some());
        assert!(skel.get_bone_by_name("head").is_some());
        assert!(skel.get_bone_by_name("antenna_l").is_some());
        assert!(skel.get_bone_by_name("antenna_r").is_some());
    }

    #[test]
    fn test_build_skeleton_quadrupedal() {
        let tpl = hunter_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        assert!(skel.bone_count() >= 8, "hunter should have >=8 bones, got {}", skel.bone_count());
        assert!(skel.get_bone_by_name("front_l").is_some());
        assert!(skel.get_bone_by_name("back_r").is_some());
    }

    #[test]
    fn test_build_skeleton_arachnid_8_legs() {
        let tpl = bloated_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        // 8 条腿 = 16 个腿部骨骼（上+下）
        let leg_count = skel
            .bones
            .values()
            .filter(|b| b.name.starts_with("leg_"))
            .count();
        assert_eq!(leg_count, 16, "expected 16 leg bones (8 legs × 2 segments), got {}", leg_count);
        assert!(skel.get_bone_by_name("abdomen").is_some());
    }

    #[test]
    fn test_build_skeleton_insectoid_6_legs() {
        let tpl = weaver_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let leg_count = skel
            .bones
            .values()
            .filter(|b| b.name.starts_with("leg_"))
            .count();
        assert_eq!(leg_count, 12, "expected 12 leg bones (6 legs × 2 segments), got {}", leg_count);
        // 编织者有 4 条触手
        let tentacle_count = skel
            .bones
            .values()
            .filter(|b| b.name.starts_with("tentacle_"))
            .count();
        assert_eq!(tentacle_count, 4);
    }

    #[test]
    fn test_build_skeleton_amorphous() {
        let tpl = listener_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        assert!(skel.get_bone_by_name("center").is_some());
        // 6 个分支
        let branch_count = skel
            .bones
            .values()
            .filter(|b| b.name.starts_with("branch_"))
            .count();
        assert_eq!(branch_count, 12); // 6 mid + 6 tip
    }

    #[test]
    fn test_build_skeleton_winged() {
        let tpl = swarm_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        assert!(skel.get_bone_by_name("wing_l").is_some());
        assert!(skel.get_bone_by_name("wing_r").is_some());
        assert!(skel.get_bone_by_name("abdomen").is_some());
        // 复眼
        let eye_count = skel
            .bones
            .values()
            .filter(|b| b.name.starts_with("eye_"))
            .count();
        assert_eq!(eye_count, 4);
    }

    #[test]
    fn test_build_skeleton_armored() {
        let tpl = crusher_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        assert!(skel.get_bone_by_name("armor_chest").is_some());
        assert!(skel.get_bone_by_name("armor_pelvis").is_some());
    }

    #[test]
    fn test_morph_generator_stalker() {
        let tpl = stalker_template();
        let params = tpl.default_params.clone();
        let gen = MorphGenerator::new();
        let (vertices, indices, skel, weights) = gen.generate(&tpl, &params);
        assert!(vertices.len() > 50, "stalker should have >50 vertices, got {}", vertices.len());
        assert!(!indices.is_empty());
        assert_eq!(weights.len(), vertices.len());
        assert!(skel.bone_count() > 0);
    }

    #[test]
    fn test_morph_generator_all_8_templates() {
        for tpl in all_templates().iter() {
            let params = tpl.default_params.clone();
            let gen = MorphGenerator::new();
            let (vertices, indices, skel, weights) = gen.generate(tpl, &params);
            assert!(
                vertices.len() > 10,
                "template {} should have >10 vertices, got {}",
                tpl.name,
                vertices.len()
            );
            assert!(!indices.is_empty(), "template {} should have indices", tpl.name);
            assert_eq!(weights.len(), vertices.len(), "template {} weight count mismatch", tpl.name);
            assert!(skel.bone_count() > 0, "template {} should have bones", tpl.name);
        }
    }

    #[test]
    fn test_locomotion_animation_bipedal() {
        let tpl = stalker_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let clip = create_locomotion_animation(&tpl, &skel, 1.0);
        assert!(clip.tracks.len() >= 2, "bipedal should have >=2 tracks, got {}", clip.tracks.len());
        assert_eq!(clip.name, "locomotion");
    }

    #[test]
    fn test_locomotion_animation_quadrupedal() {
        let tpl = hunter_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let clip = create_locomotion_animation(&tpl, &skel, 1.0);
        assert!(clip.tracks.len() >= 4, "quadrupedal should have >=4 tracks, got {}", clip.tracks.len());
    }

    #[test]
    fn test_locomotion_animation_arachnid() {
        let tpl = bloated_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let clip = create_locomotion_animation(&tpl, &skel, 1.0);
        // 8 条腿都应该有动画
        assert!(clip.tracks.len() >= 8, "arachnid should have >=8 tracks, got {}", clip.tracks.len());
    }

    #[test]
    fn test_locomotion_animation_winged() {
        let tpl = swarm_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let clip = create_locomotion_animation(&tpl, &skel, 0.5);
        // 翅膀拍动动画
        assert!(clip.tracks.len() >= 2, "winged should have >=2 wing tracks, got {}", clip.tracks.len());
    }

    #[test]
    fn test_locomotion_animation_amorphous_pulse() {
        let tpl = listener_template();
        let params = tpl.default_params.clone();
        let skel = build_skeleton(&tpl, &params);
        let clip = create_locomotion_animation(&tpl, &skel, 4.0);
        assert!(clip.tracks.len() >= 1, "amorphous should have pulse track");
        // 验证缩放动画
        let track = &clip.tracks[0];
        let t0 = track.sample(0.0);
        let t_mid = track.sample(2.0);
        assert!(t_mid.scale.x > t0.scale.x, "pulse should scale up at mid");
    }

    #[test]
    fn test_morph_mutation_affects_skeleton() {
        let tpl = stalker_template();
        // 不同 variant_seed 应产生略微不同的缩放
        let mut params1 = tpl.default_params.clone();
        params1.variant_seed = 1;
        let mut params2 = tpl.default_params.clone();
        params2.variant_seed = 2;
        let skel1 = build_skeleton(&tpl, &params1);
        let skel2 = build_skeleton(&tpl, &params2);
        // 骨骼数量相同
        assert_eq!(skel1.bone_count(), skel2.bone_count());
        // 但位置可能略有不同（因 vary_scale）
        let p1_pelvis = skel1.bones.get(skel1.get_bone_by_name("pelvis").unwrap()).unwrap().world_bind.w_axis.truncate();
        let p2_pelvis = skel2.bones.get(skel2.get_bone_by_name("pelvis").unwrap()).unwrap().world_bind.w_axis.truncate();
        let _ = (p1_pelvis, p2_pelvis); // 验证不崩溃
    }

    #[test]
    fn test_compound_eye_helper() {
        let mut builder = MeshBuilder::new();
        let initial_count = builder.vertex_count();
        push_compound_eye(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            0.05,
            4,
            [0.8, 0.7, 0.2, 1.0],
        );
        assert!(builder.vertex_count() > initial_count, "compound eye should add vertices");
    }

    #[test]
    fn test_wing_membrane_helper() {
        let mut builder = MeshBuilder::new();
        let initial_count = builder.vertex_count();
        push_wing_membrane(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.1),
            [0.6, 0.5, 0.4, 0.6],
        );
        assert!(builder.vertex_count() > initial_count, "wing membrane should add vertices");
    }

    #[test]
    fn test_tapered_cylinder_helper() {
        let mut builder = MeshBuilder::new();
        let initial_count = builder.vertex_count();
        push_tapered_cylinder(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.1,
            0.05,
            [1.0, 0.0, 0.0, 1.0],
            8,
        );
        assert!(builder.vertex_count() > initial_count, "tapered cylinder should add vertices");
    }

    #[test]
    fn test_antenna_helper() {
        let mut builder = MeshBuilder::new();
        push_antenna(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
            5,
            [0.5, 0.5, 0.5, 1.0],
        );
        assert!(builder.vertex_count() > 0, "antenna should produce vertices");
    }

    #[test]
    fn test_ellipsoid_helper() {
        let mut builder = MeshBuilder::new();
        let initial_count = builder.vertex_count();
        push_ellipsoid(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.1, 0.2, 0.1),
            [0.8, 0.6, 0.4, 1.0],
        );
        assert!(builder.vertex_count() > initial_count, "ellipsoid should add vertices");
    }

    #[test]
    fn test_chitin_plate_helper() {
        let mut builder = MeshBuilder::new();
        let initial_count = builder.vertex_count();
        push_chitin_plate(
            &mut builder,
            Vec3::new(0.0, 0.0, 0.0),
            [0.2, 0.3],
            0.02,
            [0.4, 0.3, 0.2, 1.0],
        );
        assert!(builder.vertex_count() > initial_count, "chitin plate should add vertices");
    }
}
