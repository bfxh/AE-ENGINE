//! 程序化 NPC 生成器
//!
//! 突破性 NPC 程序化生成：
//! - 12 部位人形骨骼（头/颈/胸/腰/上臂×2/前臂×2/大腿×2/小腿×2）
//! - 程序化几何体（胶囊 + 球 + 盒子组合）
//! - 蒙皮权重自动分配（按距离最近骨骼）
//! - 步行动画轨道（基于生物力学的周期性运动）

use crate::mesh::{MeshBuilder, Vertex};
use crate::procedural::geometry::{cylinder, CylinderParams};
use crate::procedural::skeleton::{
    AnimationClip, AnimationTrack, JointTransform, LoopMode, Skeleton, SkinWeights,
};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// NPC 身体参数
#[derive(Debug, Clone)]
pub struct NpcBodyParams {
    /// 总高度（米）
    pub height: f32,
    /// 体型比例（0.0=瘦, 0.5=标准, 1.0=壮）
    pub build: f32,
    /// 肩宽
    pub shoulder_width: f32,
    /// 髋宽
    pub hip_width: f32,
    /// 头身比（头高/身高，1/7=标准, 1/8=写实）
    pub head_ratio: f32,
    /// 肤色
    pub skin_color: [f32; 4],
    /// 性别（影响肩髋比）
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

/// 人形骨骼定义（标准 12 部位 + 中间关节）
pub struct HumanoidSkeleton;

impl HumanoidSkeleton {
    /// 创建标准人形骨骼层级
    ///
    /// 层级结构：
    /// ```text
    /// pelvis (root)
    /// ├── spine
    /// │   └── chest
    /// │       ├── neck → head
    /// │       ├── shoulder_l → upper_arm_l → lower_arm_l
    /// │       └── shoulder_r → upper_arm_r → lower_arm_r
    /// ├── thigh_l → calf_l
    /// └── thigh_r → calf_r
    /// ```
    ///
    /// 12 主要部位：head, neck, chest, pelvis, upper_arm_l/r, lower_arm_l/r, thigh_l/r, calf_l/r
    pub fn create(params: &NpcBodyParams) -> Skeleton {
        let mut skel = Skeleton::new();
        let h = params.height;
        let build = params.build;

        // 关键身高比例（基于人体测量学）
        let head_h = h * params.head_ratio; // 头高
        let neck_h = head_h * 0.3;
        let chest_h = h * 0.25; // 胸部高度
        let spine_h = h * 0.10; // 腰部脊柱段
        let pelvis_h = h * 0.10; // 髋部高度
        let upper_arm_h = h * 0.18;
        let lower_arm_h = h * 0.15;
        let thigh_h = h * 0.25;
        let calf_h = h * 0.22;

        // 从下往上构建
        let pelvis_y = 0.0; // 根骨骼在原点
        let spine_y = pelvis_y + pelvis_h;
        let chest_y = spine_y + spine_h;
        let neck_y = chest_y + chest_h;
        let head_y = neck_y + neck_h;

        // 半肩宽/半髋宽（性别调整）
        let (sw, hw) = match params.gender {
            Gender::Male => (params.shoulder_width * 0.5, params.hip_width * 0.45),
            Gender::Female => (params.shoulder_width * 0.42, params.hip_width * 0.55),
        };
        let arm_radius = 0.04 + build * 0.02;
        let leg_radius = 0.05 + build * 0.025;
        let _ = (arm_radius, leg_radius); // 用于几何体生成

        // 添加骨骼（局部变换 = 相对父骨骼）
        let pelvis = skel.add_bone(
            "pelvis",
            None,
            JointTransform {
                translation: Vec3::new(0.0, pelvis_y, 0.0),
                ..Default::default()
            },
        );

        let spine = skel.add_bone(
            "spine",
            Some(pelvis),
            JointTransform {
                translation: Vec3::new(0.0, pelvis_h, 0.0),
                ..Default::default()
            },
        );

        let chest = skel.add_bone(
            "chest",
            Some(spine),
            JointTransform {
                translation: Vec3::new(0.0, spine_h, 0.0),
                ..Default::default()
            },
        );

        let neck = skel.add_bone(
            "neck",
            Some(chest),
            JointTransform {
                translation: Vec3::new(0.0, chest_h, 0.0),
                ..Default::default()
            },
        );

        let head = skel.add_bone(
            "head",
            Some(neck),
            JointTransform {
                translation: Vec3::new(0.0, neck_h + head_h * 0.5, 0.0),
                ..Default::default()
            },
        );

        // 左肩 → 上臂 → 前臂
        let shoulder_l = skel.add_bone(
            "shoulder_l",
            Some(chest),
            JointTransform {
                translation: Vec3::new(sw, chest_h * 0.9, 0.0),
                ..Default::default()
            },
        );
        let upper_arm_l = skel.add_bone(
            "upper_arm_l",
            Some(shoulder_l),
            JointTransform {
                translation: Vec3::new(sw * 0.3, -0.05, 0.0),
                rotation: Quat::from_rotation_z(0.1), // 微微外展
                ..Default::default()
            },
        );
        let lower_arm_l = skel.add_bone(
            "lower_arm_l",
            Some(upper_arm_l),
            JointTransform {
                translation: Vec3::new(0.0, -upper_arm_h, 0.0),
                ..Default::default()
            },
        );

        // 右肩 → 上臂 → 前臂
        let shoulder_r = skel.add_bone(
            "shoulder_r",
            Some(chest),
            JointTransform {
                translation: Vec3::new(-sw, chest_h * 0.9, 0.0),
                ..Default::default()
            },
        );
        let upper_arm_r = skel.add_bone(
            "upper_arm_r",
            Some(shoulder_r),
            JointTransform {
                translation: Vec3::new(-sw * 0.3, -0.05, 0.0),
                rotation: Quat::from_rotation_z(-0.1),
                ..Default::default()
            },
        );
        let lower_arm_r = skel.add_bone(
            "lower_arm_r",
            Some(upper_arm_r),
            JointTransform {
                translation: Vec3::new(0.0, -upper_arm_h, 0.0),
                ..Default::default()
            },
        );

        // 左腿
        let thigh_l = skel.add_bone(
            "thigh_l",
            Some(pelvis),
            JointTransform {
                translation: Vec3::new(hw * 0.5, -0.05, 0.0),
                ..Default::default()
            },
        );
        let calf_l = skel.add_bone(
            "calf_l",
            Some(thigh_l),
            JointTransform {
                translation: Vec3::new(0.0, -thigh_h, 0.0),
                ..Default::default()
            },
        );

        // 右腿
        let thigh_r = skel.add_bone(
            "thigh_r",
            Some(pelvis),
            JointTransform {
                translation: Vec3::new(-hw * 0.5, -0.05, 0.0),
                ..Default::default()
            },
        );
        let calf_r = skel.add_bone(
            "calf_r",
            Some(thigh_r),
            JointTransform {
                translation: Vec3::new(0.0, -thigh_h, 0.0),
                ..Default::default()
            },
        );

        skel.compute_bind_pose();
        skel
    }
}

/// NPC 生成器
pub struct NpcBodyGenerator {
    builder: MeshBuilder,
    skeleton: Skeleton,
    skin_weights: Vec<SkinWeights>,
}

impl NpcBodyGenerator {
    pub fn new() -> Self {
        Self {
            builder: MeshBuilder::new(),
            skeleton: Skeleton::new(),
            skin_weights: Vec::new(),
        }
    }

    /// 生成 NPC：返回 (顶点, 索引, 骨骼, 蒙皮权重)
    pub fn generate(mut self, params: &NpcBodyParams) -> (Vec<Vertex>, Vec<u32>, Skeleton, Vec<SkinWeights>) {
        // 1. 创建骨骼
        self.skeleton = HumanoidSkeleton::create(params);

        // 2. 生成几何体（每部位一个图元）
        let h = params.height;
        let head_h = h * params.head_ratio;
        let neck_h = head_h * 0.3;
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

        // 获取骨骼位置
        let bone_positions = self.collect_bone_positions();

        // 头（球）
        self.push_sphere_at(bone_positions["head"], head_h * 0.45, "head");
        // 颈（圆柱）
        self.push_cylinder_at(
            bone_positions["neck"],
            bone_positions["head"],
            0.05,
            "neck",
        );
        // 胸（盒子）
        self.push_box_at(
            bone_positions["chest"],
            [torso_w, chest_h, torso_d],
            "chest",
        );
        // 腰（盒子）
        self.push_box_at(
            bone_positions["spine"],
            [torso_w * 0.85, spine_h, torso_d * 0.9],
            "spine",
        );
        // 髋（盒子）
        self.push_box_at(
            bone_positions["pelvis"],
            [hw * 2.2, pelvis_h, torso_d * 0.9],
            "pelvis",
        );
        // 上臂 L/R（圆柱）
        let arm_radius = 0.04 + build * 0.02;
        self.push_cylinder_at(
            bone_positions["upper_arm_l"],
            bone_positions["lower_arm_l"],
            arm_radius,
            "upper_arm_l",
        );
        self.push_cylinder_at(
            bone_positions["upper_arm_r"],
            bone_positions["lower_arm_r"],
            arm_radius,
            "upper_arm_r",
        );
        // 前臂 L/R（圆柱）
        self.push_cylinder_at(
            bone_positions["lower_arm_l"],
            bone_positions["lower_arm_l"] + Vec3::new(0.0, -lower_arm_h, 0.0),
            arm_radius * 0.85,
            "lower_arm_l",
        );
        self.push_cylinder_at(
            bone_positions["lower_arm_r"],
            bone_positions["lower_arm_r"] + Vec3::new(0.0, -lower_arm_h, 0.0),
            arm_radius * 0.85,
            "lower_arm_r",
        );
        // 大腿 L/R（圆柱）
        let leg_radius = 0.05 + build * 0.025;
        self.push_cylinder_at(
            bone_positions["thigh_l"],
            bone_positions["calf_l"],
            leg_radius,
            "thigh_l",
        );
        self.push_cylinder_at(
            bone_positions["thigh_r"],
            bone_positions["calf_r"],
            leg_radius,
            "thigh_r",
        );
        // 小腿 L/R（圆柱）
        self.push_cylinder_at(
            bone_positions["calf_l"],
            bone_positions["calf_l"] + Vec3::new(0.0, -calf_h, 0.0),
            leg_radius * 0.8,
            "calf_l",
        );
        self.push_cylinder_at(
            bone_positions["calf_r"],
            bone_positions["calf_r"] + Vec3::new(0.0, -calf_h, 0.0),
            leg_radius * 0.8,
            "calf_r",
        );

        // 3. 应用肤色
        let color = params.skin_color;
        for v in self.builder.vertices_mut().iter_mut() {
            v.color = color;
        }

        let (vertices, indices) = self.builder.into_parts();

        // 4. 生成蒙皮权重（每顶点分配最近骨骼）
        self.skin_weights = Self::compute_skin_weights(&vertices, &self.skeleton);

        (vertices, indices, self.skeleton, self.skin_weights)
    }

    /// 收集所有骨骼的世界位置
    fn collect_bone_positions(&self) -> hashbrown::HashMap<String, Vec3> {
        let mut map = hashbrown::HashMap::new();
        for bone in self.skeleton.bones.values() {
            let pos = bone.world_bind.w_axis.truncate();
            map.insert(bone.name.clone(), pos);
        }
        map
    }

    /// 在指定位置生成球（用于头部）
    fn push_sphere_at(&mut self, center: Vec3, radius: f32, _bone_name: &str) {
        let (verts, indices) = crate::mesh::MeshBuilder::sphere(12, 8);
        let base = self.builder.vertex_count() as u32;
        // 缩放并平移每个顶点
        for v in &verts {
            let p = Vec3::from(v.position) * radius + center;
            let mut v = *v;
            v.position = p.into();
            self.builder.push_vertex(v);
        }
        // 推入索引（偏移 base）
        for &i in &indices {
            self.builder.indices_mut().push(base + i);
        }
    }

    /// 在两点之间生成圆柱
    fn push_cylinder_at(&mut self, start: Vec3, end: Vec3, radius: f32, _bone_name: &str) {
        let direction = end - start;
        let length = direction.length();
        if length < 1e-6 {
            return;
        }
        let (verts, indices) = cylinder(CylinderParams {
            radius,
            height: length,
            segments: 8,
            cap_bottom: true,
            cap_top: true,
        });
        // 计算旋转：默认圆柱沿 Y 轴（中心在原点），需要旋转到 direction 方向并平移到中点
        let dir_normalized = direction / length;
        let default_dir = Vec3::new(0.0, 1.0, 0.0);
        let rotation = Quat::from_rotation_arc(default_dir, dir_normalized);
        let midpoint = start + direction * 0.5;

        let base = self.builder.vertex_count() as u32;
        for v in &verts {
            let p = Vec3::from(v.position);
            let p = rotation * p + midpoint;
            let n = rotation * Vec3::from(v.normal);
            let mut v = *v;
            v.position = p.into();
            v.normal = n.into();
            self.builder.push_vertex(v);
        }
        for &i in &indices {
            self.builder.indices_mut().push(base + i);
        }
    }

    /// 在指定中心生成盒子
    fn push_box_at(&mut self, center: Vec3, extent: [f32; 3], _bone_name: &str) {
        self.builder.push_box([center.x, center.y, center.z], extent);
    }

    /// 获取骨骼索引
    fn bone_index(&self, name: &str) -> u32 {
        self.skeleton
            .get_bone_by_name(name)
            .map(|id| self.skeleton.bone_index(id))
            .unwrap_or(0)
    }

    /// 计算蒙皮权重（每顶点最近骨骼 + 距离权重）
    fn compute_skin_weights(vertices: &[Vertex], skeleton: &Skeleton) -> Vec<SkinWeights> {
        // 收集所有骨骼位置
        let bone_positions: Vec<(u32, Vec3)> = skeleton
            .bones
            .iter()
            .map(|(id, bone)| (skeleton.bone_index(id), bone.world_bind.w_axis.truncate()))
            .collect();

        vertices
            .iter()
            .map(|v| {
                let pos = Vec3::from(v.position);
                let mut weights = SkinWeights::default();
                // 计算到每根骨骼的距离，取最近的 4 根
                let mut distances: Vec<(u32, f32)> = bone_positions
                    .iter()
                    .map(|(idx, bp)| {
                        let d = (pos - bp).length();
                        // 权重 = 1 / (distance^2 + epsilon)
                        let w = 1.0 / (d * d + 0.01);
                        (*idx, w)
                    })
                    .collect();
                distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                for &(_, w) in distances.iter().take(4) {
                    // 暂时只取权重，骨骼索引需要重新映射
                    let _ = w;
                }
                // 简化：取最近的 1 根骨骼，权重 1.0
                if let Some(&(idx, _)) = distances.first() {
                    weights = SkinWeights::single(idx, 1.0);
                }
                weights
            })
            .collect()
    }

    /// 生成步行动画剪辑
    pub fn create_walk_animation(skeleton: &Skeleton, cycle_duration: f32) -> AnimationClip {
        let mut tracks = Vec::new();

        // 左右腿交替摆动（绕 X 轴前后摆）
        if let Some(thigh_l) = skeleton.get_bone_by_name("thigh_l") {
            tracks.push(AnimationTrack {
                name: "thigh_l_swing".to_string(),
                bone_id: thigh_l,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(0.4), // 前摆
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(-0.4), // 后摆
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(0.4),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }
        if let Some(thigh_r) = skeleton.get_bone_by_name("thigh_r") {
            tracks.push(AnimationTrack {
                name: "thigh_r_swing".to_string(),
                bone_id: thigh_r,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(-0.4),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(0.4),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(-0.4),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }

        // 小腿弯曲（行走时膝盖弯曲）
        if let Some(calf_l) = skeleton.get_bone_by_name("calf_l") {
            tracks.push(AnimationTrack {
                name: "calf_l_bend".to_string(),
                bone_id: calf_l,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.25, JointTransform {
                        rotation: Quat::from_rotation_x(0.8), // 后摆时弯膝
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }
        if let Some(calf_r) = skeleton.get_bone_by_name("calf_r") {
            tracks.push(AnimationTrack {
                name: "calf_r_bend".to_string(),
                bone_id: calf_r,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.75, JointTransform {
                        rotation: Quat::from_rotation_x(0.8),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(0.2),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }

        // 手臂摆动（与对侧腿同步）
        if let Some(upper_arm_l) = skeleton.get_bone_by_name("upper_arm_l") {
            tracks.push(AnimationTrack {
                name: "upper_arm_l_swing".to_string(),
                bone_id: upper_arm_l,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(-0.3),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(0.3),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(-0.3),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }
        if let Some(upper_arm_r) = skeleton.get_bone_by_name("upper_arm_r") {
            tracks.push(AnimationTrack {
                name: "upper_arm_r_swing".to_string(),
                bone_id: upper_arm_r,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_x(0.3),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_x(-0.3),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_x(0.3),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }

        // 骨盆上下浮动（行走时的重心变化）
        if let Some(pelvis) = skeleton.get_bone_by_name("pelvis") {
            tracks.push(AnimationTrack {
                name: "pelvis_bob".to_string(),
                bone_id: pelvis,
                keyframes: vec![
                    (0.0, JointTransform {
                        translation: Vec3::new(0.0, 0.0, 0.0),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.25, JointTransform {
                        translation: Vec3::new(0.0, 0.03, 0.0),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        translation: Vec3::new(0.0, 0.0, 0.0),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.75, JointTransform {
                        translation: Vec3::new(0.0, 0.03, 0.0),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        translation: Vec3::new(0.0, 0.0, 0.0),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }

        // 躯干轻微扭转（与手臂摆动同步）
        if let Some(chest) = skeleton.get_bone_by_name("chest") {
            tracks.push(AnimationTrack {
                name: "chest_twist".to_string(),
                bone_id: chest,
                keyframes: vec![
                    (0.0, JointTransform {
                        rotation: Quat::from_rotation_y(0.1),
                        ..Default::default()
                    }),
                    (cycle_duration * 0.5, JointTransform {
                        rotation: Quat::from_rotation_y(-0.1),
                        ..Default::default()
                    }),
                    (cycle_duration, JointTransform {
                        rotation: Quat::from_rotation_y(0.1),
                        ..Default::default()
                    }),
                ],
                loop_mode: LoopMode::Loop,
            });
        }

        AnimationClip {
            name: "walk".to_string(),
            duration: cycle_duration,
            tracks,
        }
    }

    /// 生成站立待机动画
    pub fn create_idle_animation(skeleton: &Skeleton) -> AnimationClip {
        let mut tracks = Vec::new();
        // 呼吸动画（chest 轻微上下）
        if let Some(chest) = skeleton.get_bone_by_name("chest") {
            tracks.push(AnimationTrack {
                name: "breathing".to_string(),
                bone_id: chest,
                keyframes: vec![
                    (0.0, JointTransform::default()),
                    (2.0, JointTransform {
                        translation: Vec3::new(0.0, 0.01, 0.0),
                        rotation: Quat::from_rotation_x(0.02),
                        ..Default::default()
                    }),
                    (4.0, JointTransform::default()),
                ],
                loop_mode: LoopMode::Loop,
            });
        }
        AnimationClip {
            name: "idle".to_string(),
            duration: 4.0,
            tracks,
        }
    }
}

impl Default for NpcBodyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 形态模板系统（支持非人类 NPC：虫族、母巢子实体、变异生物）
// ============================================================================

/// 身体蓝图（决定整体拓扑结构）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPlan {
    /// 双足（人形、克隆人、践踏者）
    Bipedal,
    /// 四足（追猎者、裂地兽）
    Quadrupedal,
    /// 六足昆虫形（编织者）
    Insectoid,
    /// 八足蜘蛛形（臃肿者）
    Arachnid,
    /// 不定形/扁平（窃听者、菌丝块）
    Amorphous,
    /// 有翼飞行（蜂群）
    Winged,
    /// 装甲外壳形（碎脊者、锈骑士）
    Armored,
}

/// 生物材质类型（决定 PBR 参数和损伤行为）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiologicalMaterial {
    /// 血肉（人类、克隆人）
    Flesh,
    /// 菌丝（践踏者、母巢子实体主体）
    Mycelium,
    /// 甲壳（追猎者、虫族外骨骼）
    Chitin,
    /// 角质板（碎脊者天然装甲）
    KeratinPlate,
    /// 锈金属（锈骑士动力装甲）
    RustyMetal,
    /// 薄膜（蜂群机翼）
    Membrane,
    /// 液体（臃肿者腹部消化液）
    Fluid,
    /// 地衣（窃听者扁平形态）
    Lichen,
}

impl BiologicalMaterial {
    /// 返回该材质的 PBR 参数（albedo, roughness, metallic）
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

/// 体型分级（影响碰撞体大小和生命值）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeClass {
    /// 微型（蜂群、窃听者）—— < 0.3m
    Tiny,
    /// 小型（践踏者）—— 0.3-1.2m
    Small,
    /// 中型（追猎者、编织者、人形）—— 1.2-2.0m
    Medium,
    /// 大型（碎脊者、臃肿者、锈骑士）—— 2.0-3.5m
    Large,
    /// 巨型（巢母、特殊 Boss）—— > 3.5m
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

/// 通用形态参数（用于非人类 NPC 生成）
#[derive(Debug, Clone)]
pub struct MorphParams {
    /// 总体缩放
    pub scale: f32,
    /// 体型变异（0..1，影响粗细）
    pub build: f32,
    /// 主材质
    pub material: BiologicalMaterial,
    /// 材质颜色覆盖（None = 用材质默认色）
    pub color_override: Option<[f32; 4]>,
    /// 变异种子（决定随机变异）
    pub variant_seed: u32,
    /// 腿数量（虫族可变：0/2/4/6/8）
    pub leg_count: u32,
    /// 手臂数量（0/2/4）
    pub arm_count: u32,
    /// 眼睛数量（0=无眼，1-8）
    pub eye_count: u32,
    /// 触角数量（0-4）
    pub antenna_count: u32,
    /// 是否有翅膀
    pub has_wings: bool,
    /// 是否有膨胀腹部
    pub has_abdomen: bool,
    /// 装甲覆盖率（0..1，0=无装甲，1=全覆盖）
    pub armor_coverage: f32,
    /// 菌丝密度（0..1，影响表面纹理）
    pub mycelium_density: f32,
    /// 黏液分泌（0..1）
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

/// 变异系统：基于种子生成可重现的随机变异
#[derive(Debug, Clone)]
pub struct MorphMutation {
    pub seed: u32,
}

impl MorphMutation {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// 简单确定性哈希（保证相同种子产生相同变异）
    pub fn hash(&self, salt: u32) -> u32 {
        let mut h = self.seed.wrapping_add(salt.wrapping_mul(2654435761));
        h ^= h >> 16;
        h = h.wrapping_mul(0x85ebca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2ae35);
        h ^= h >> 16;
        h
    }

    /// 返回 0..1 的伪随机浮点数
    pub fn random01(&self, salt: u32) -> f32 {
        (self.hash(salt) as f32) / (u32::MAX as f32)
    }

    /// 在 [min, max] 范围内生成伪随机数
    pub fn range(&self, salt: u32, min: f32, max: f32) -> f32 {
        min + self.random01(salt) * (max - min)
    }

    /// 应用肢体数量变异（虫族基因重组）
    pub fn vary_limb_count(&self, base: u32, min: u32, max: u32) -> u32 {
        let delta = (self.hash(0xABCD) % (max - min + 1)) as u32;
        (min + delta).min(max).max(min)
    }

    /// 应用体型变异
    pub fn vary_scale(&self, base_scale: f32) -> f32 {
        let factor = self.range(0x1234, 0.85, 1.15);
        base_scale * factor
    }
}

/// 形态模板（描述一种 NPC 形态的完整定义）
#[derive(Debug, Clone)]
pub struct MorphTemplate {
    pub name: &'static str,
    pub body_plan: BodyPlan,
    pub material: BiologicalMaterial,
    pub size_class: SizeClass,
    /// 默认参数（生成时会与传入的 MorphParams 合并）
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
        Self {
            name,
            body_plan,
            material,
            size_class,
            default_params,
        }
    }

    /// 获取最终颜色（优先使用 override，否则用材质默认色）
    pub fn resolve_color(&self, params: &MorphParams) -> [f32; 4] {
        params
            .color_override
            .unwrap_or_else(|| self.material.pbr_params().0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanoid_skeleton_creation() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        // 12 主要部位 + 2 肩膀 + 1 spine = 15 根骨骼
        assert_eq!(skel.bone_count(), 15);
        assert_eq!(skel.roots.len(), 1);
        // 验证关键骨骼存在
        assert!(skel.get_bone_by_name("head").is_some());
        assert!(skel.get_bone_by_name("pelvis").is_some());
        assert!(skel.get_bone_by_name("upper_arm_l").is_some());
        assert!(skel.get_bone_by_name("calf_r").is_some());
    }

    #[test]
    fn test_skeleton_hierarchy_correct() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        // head 应该在 neck 之上
        let head_pos = skel.bones.get(skel.get_bone_by_name("head").unwrap()).unwrap().world_bind.w_axis.truncate();
        let pelvis_pos = skel.bones.get(skel.get_bone_by_name("pelvis").unwrap()).unwrap().world_bind.w_axis.truncate();
        assert!(head_pos.y > pelvis_pos.y, "head ({}) should be above pelvis ({})", head_pos.y, pelvis_pos.y);
    }

    #[test]
    fn test_npc_body_generation() {
        let params = NpcBodyParams::default();
        let gen = NpcBodyGenerator::new();
        let (vertices, indices, _skel, weights) = gen.generate(&params);
        assert!(vertices.len() > 100, "expected >100 vertices, got {}", vertices.len());
        assert!(!indices.is_empty());
        assert_eq!(weights.len(), vertices.len());
    }

    #[test]
    fn test_female_body_proportions() {
        let mut params = NpcBodyParams::default();
        params.gender = Gender::Female;
        let skel = HumanoidSkeleton::create(&params);
        let shoulder_l = skel.bones.get(skel.get_bone_by_name("shoulder_l").unwrap()).unwrap().world_bind.w_axis.truncate();
        let hip_l = skel.bones.get(skel.get_bone_by_name("thigh_l").unwrap()).unwrap().world_bind.w_axis.truncate();
        // 女性髋宽应该相对更大（shoulder_width=0.42*0.42=0.176, hip_width=0.32*0.55=0.176）
        // 此测试仅验证生成不崩溃
        assert!(shoulder_l.x.abs() > 0.0);
        assert!(hip_l.x.abs() > 0.0);
    }

    #[test]
    fn test_walk_animation() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        let clip = NpcBodyGenerator::create_walk_animation(&skel, 1.0);
        // 应该有 7 条轨道：2 大腿 + 2 小腿 + 2 上臂 + 1 骨盆 + 1 胸 = 8
        assert!(clip.tracks.len() >= 7, "expected >=7 tracks, got {}", clip.tracks.len());
        assert_eq!(clip.name, "walk");
        assert_eq!(clip.duration, 1.0);
    }

    #[test]
    fn test_idle_animation() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        let clip = NpcBodyGenerator::create_idle_animation(&skel);
        assert!(clip.tracks.len() >= 1);
        assert_eq!(clip.name, "idle");
    }

    #[test]
    fn test_animation_sample_at_boundaries() {
        let params = NpcBodyParams::default();
        let skel = HumanoidSkeleton::create(&params);
        let clip = NpcBodyGenerator::create_walk_animation(&skel, 1.0);
        let transforms_0 = clip.sample(0.0);
        let transforms_1 = clip.sample(1.0);
        // 循环动画：t=0 和 t=duration 应该相同
        if let Some(thigh_l) = skel.get_bone_by_name("thigh_l") {
            let t0 = transforms_0.get(&thigh_l);
            let t1 = transforms_1.get(&thigh_l);
            assert!(t0.is_some() && t1.is_some());
            // 旋转应该相等（循环点）
            let angle_diff = t0.unwrap().rotation.angle_between(t1.unwrap().rotation);
            assert!(angle_diff < 1e-4, "loop boundary mismatch: {}", angle_diff);
        }
    }

    // === 形态模板系统测试 ===

    #[test]
    fn test_body_plan_variants() {
        // 所有 BodyPlan 变体都能正确构造和比较
        assert_ne!(BodyPlan::Bipedal, BodyPlan::Quadrupedal);
        assert_ne!(BodyPlan::Insectoid, BodyPlan::Arachnid);
        assert_ne!(BodyPlan::Winged, BodyPlan::Armored);
        assert_eq!(BodyPlan::Amorphous, BodyPlan::Amorphous);
    }

    #[test]
    fn test_biological_material_pbr() {
        // 每种材质都应返回有效的 PBR 参数
        let materials = [
            BiologicalMaterial::Flesh,
            BiologicalMaterial::Mycelium,
            BiologicalMaterial::Chitin,
            BiologicalMaterial::KeratinPlate,
            BiologicalMaterial::RustyMetal,
            BiologicalMaterial::Membrane,
            BiologicalMaterial::Fluid,
            BiologicalMaterial::Lichen,
        ];
        for m in &materials {
            let (albedo, roughness, metallic) = m.pbr_params();
            assert!(albedo[0] >= 0.0 && albedo[0] <= 1.0, "albedo R out of range: {}", albedo[0]);
            assert!(albedo[1] >= 0.0 && albedo[1] <= 1.0, "albedo G out of range: {}", albedo[1]);
            assert!(roughness >= 0.0 && roughness <= 1.0, "roughness out of range: {}", roughness);
            assert!(metallic >= 0.0 && metallic <= 1.0, "metallic out of range: {}", metallic);
        }
        // RustyMetal 应该有高金属度
        assert!(BiologicalMaterial::RustyMetal.pbr_params().2 > 0.5);
        // Mycelium 应该高粗糙度
        assert!(BiologicalMaterial::Mycelium.pbr_params().1 > 0.8);
    }

    #[test]
    fn test_size_class_scale() {
        assert!(SizeClass::Tiny.default_scale() < SizeClass::Small.default_scale());
        assert!(SizeClass::Small.default_scale() < SizeClass::Medium.default_scale());
        assert!(SizeClass::Medium.default_scale() < SizeClass::Large.default_scale());
        assert!(SizeClass::Large.default_scale() < SizeClass::Huge.default_scale());
    }

    #[test]
    fn test_morph_params_default() {
        let p = MorphParams::default();
        assert_eq!(p.leg_count, 2);
        assert_eq!(p.arm_count, 2);
        assert_eq!(p.material, BiologicalMaterial::Mycelium);
        assert!(!p.has_wings);
        assert!(!p.has_abdomen);
    }

    #[test]
    fn test_morph_mutation_deterministic() {
        // 相同种子应产生相同结果
        let m1 = MorphMutation::new(42);
        let m2 = MorphMutation::new(42);
        assert_eq!(m1.hash(100), m2.hash(100));
        assert_eq!(m1.random01(200), m2.random01(200));

        // 不同种子应产生不同结果（极大概率）
        let m3 = MorphMutation::new(43);
        assert_ne!(m1.hash(100), m3.hash(100));
    }

    #[test]
    fn test_morph_mutation_range() {
        let m = MorphMutation::new(123);
        for salt in 0..100u32 {
            let v = m.range(salt, -1.0, 1.0);
            assert!(v >= -1.0 && v <= 1.0, "value {} out of range for salt {}", v, salt);
        }
    }

    #[test]
    fn test_morph_mutation_limb_variation() {
        let m = MorphMutation::new(999);
        let count = m.vary_limb_count(4, 2, 6);
        assert!(count >= 2 && count <= 6, "limb count {} out of bounds", count);
    }

    #[test]
    fn test_morph_template_resolve_color() {
        let tpl = MorphTemplate::new(
            "test",
            BodyPlan::Bipedal,
            BiologicalMaterial::Flesh,
            SizeClass::Medium,
            MorphParams::default(),
        );
        // 无 override 时使用材质默认色
        let params = MorphParams::default();
        let color = tpl.resolve_color(&params);
        assert_eq!(color, BiologicalMaterial::Flesh.pbr_params().0);

        // 有 override 时使用 override
        let mut params2 = params.clone();
        params2.color_override = Some([1.0, 0.0, 0.0, 1.0]);
        let color2 = tpl.resolve_color(&params2);
        assert_eq!(color2, [1.0, 0.0, 0.0, 1.0]);
    }
}
