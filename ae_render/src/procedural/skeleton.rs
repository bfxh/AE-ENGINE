//! 骨骼层级 + 蒙皮权重系统
//!
//! 突破 AnimatedCharacter 的扁平 Vec<BoneTransform> 限制：
//! - 完整骨骼层级（parent-child 关系）
//! - 蒙皮权重（每顶点最多 4 个骨骼影响）
//! - 关键帧动画数据 + 插值
//! - 正向运动学（FK）计算世界变换

use crate::mesh::Vertex;
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

slotmap::new_key_type! { pub struct BoneId; }

/// 骨骼定义（层级 + 局部变换）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    pub parent: Option<BoneId>,
    /// 绑定姿态下的局部变换（相对父骨骼）
    pub local_bind: JointTransform,
    /// 绑定姿态下的世界变换（缓存，由 compute_bind_pose 填充）
    pub world_bind: Mat4,
    /// 逆绑定矩阵（用于蒙皮）
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

    pub fn interpolate(&self, other: &JointTransform, t: f32) -> JointTransform {
        JointTransform {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

/// 骨骼系统（层级结构 + 蒙皮）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skeleton {
    pub bones: slotmap::SlotMap<BoneId, Bone>,
    /// 根骨骼（无 parent 的骨骼，通常只有 1 个）
    pub roots: Vec<BoneId>,
    /// 名称 → BoneId 映射
    pub name_to_id: hashbrown::HashMap<String, BoneId>,
    /// BoneId → 数组索引（用于 SkinWeights 的 bone_indices）
    /// 在 compute_bind_pose 时填充
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

    /// 添加骨骼（返回 BoneId）
    pub fn add_bone(&mut self, name: &str, parent: Option<BoneId>, local_bind: JointTransform) -> BoneId {
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

    /// 获取骨骼的数组索引（用于 SkinWeights）
    pub fn bone_index(&self, id: BoneId) -> u32 {
        self.bone_id_to_index.get(&id).copied().unwrap_or(0)
    }

    /// 计算绑定姿态的世界变换 + 逆绑定矩阵。
    /// 必须在所有骨骼添加完成后调用。同时填充 bone_id_to_index 映射。
    pub fn compute_bind_pose(&mut self) {
        // 填充 bone_id_to_index 映射（按 SlotMap 迭代顺序）
        self.bone_id_to_index.clear();
        for (idx, bone_id) in self.bones.keys().enumerate() {
            self.bone_id_to_index.insert(bone_id, idx as u32);
        }
        // 拓扑排序：循环处理直到所有骨骼都访问完毕
        // 每次循环处理所有 parent 已访问（或无 parent）的未访问骨骼
        let total = self.bones.len();
        let mut visited: hashbrown::HashMap<BoneId, ()> = hashbrown::HashMap::with_capacity(total);
        while visited.len() < total {
            let mut progressed = false;
            let bone_ids: Vec<BoneId> = self.bones.keys().collect();
            for bone_id in bone_ids {
                if visited.contains_key(&bone_id) {
                    continue;
                }
                let parent = self.bones.get(bone_id).unwrap().parent;
                let parent_ready = match parent {
                    None => true,
                    Some(pid) => visited.contains_key(&pid),
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
                visited.insert(bone_id, ());
                progressed = true;
            }
            if !progressed {
                // 防止死循环（例如循环依赖，理论上不应发生）
                break;
            }
        }
    }

    /// 计算所有骨骼的当前世界变换（基于局部变换输入）。
    /// `local_transforms` 必须按 BoneId 顺序提供（或使用 HashMap）。
    pub fn compute_world_transforms(
        &self,
        local_transforms: &hashbrown::HashMap<BoneId, JointTransform>,
    ) -> Vec<(BoneId, Mat4)> {
        let mut result = Vec::with_capacity(self.bones.len());
        let mut cache: hashbrown::HashMap<BoneId, Mat4> = hashbrown::HashMap::new();

        fn compute(
            bone_id: BoneId,
            skeleton: &Skeleton,
            local_transforms: &hashbrown::HashMap<BoneId, JointTransform>,
            cache: &mut hashbrown::HashMap<BoneId, Mat4>,
        ) -> Mat4 {
            if let Some(&m) = cache.get(&bone_id) {
                return m;
            }
            let bone = skeleton.bones.get(bone_id).unwrap();
            let parent_world = if let Some(parent_id) = bone.parent {
                compute(parent_id, skeleton, local_transforms, cache)
            } else {
                Mat4::IDENTITY
            };
            let local = local_transforms
                .get(&bone_id)
                .cloned()
                .unwrap_or(bone.local_bind);
            let world = parent_world * local.to_mat4();
            cache.insert(bone_id, world);
            world
        }

        for bone_id in self.bones.keys() {
            let world = compute(bone_id, self, local_transforms, &mut cache);
            result.push((bone_id, world));
        }
        result
    }

    /// 计算蒙皮矩阵（final = world * inverse_bind）。
    /// 用于顶点着色器：skinned_pos = skin_matrix * bind_pos
    pub fn compute_skin_matrices(
        &self,
        world_transforms: &[(BoneId, Mat4)],
    ) -> Vec<(BoneId, Mat4)> {
        world_transforms
            .iter()
            .map(|(id, world)| {
                let bone = self.bones.get(*id).unwrap();
                (*id, *world * bone.inverse_bind)
            })
            .collect()
    }

    /// 获取骨骼数量
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// 按名称获取 BoneId
    pub fn get_bone_by_name(&self, name: &str) -> Option<BoneId> {
        self.name_to_id.get(name).copied()
    }
}

/// 蒙皮权重（每顶点最多 4 个骨骼影响）
#[derive(Debug, Clone, Copy, Default)]
pub struct SkinWeights {
    pub bone_indices: [u32; 4], // BoneId 的索引（在骨骼数组中的位置）
    pub weights: [f32; 4],
}

impl SkinWeights {
    /// 创建单骨骼权重
    pub fn single(bone_idx: u32, weight: f32) -> Self {
        let mut s = Self::default();
        s.bone_indices[0] = bone_idx;
        s.weights[0] = weight;
        s
    }

    /// 添加骨骼影响（保留权重最大的 4 个）
    pub fn add(&mut self, bone_idx: u32, weight: f32) {
        // 找到权重最小的位置
        let mut min_idx = 0;
        for i in 1..4 {
            if self.weights[i] < self.weights[min_idx] {
                min_idx = i;
            }
        }
        if weight > self.weights[min_idx] {
            self.bone_indices[min_idx] = bone_idx;
            self.weights[min_idx] = weight;
        }
    }

    /// 归一化权重
    pub fn normalize(&mut self) {
        let sum: f32 = self.weights.iter().sum();
        if sum > 1e-6 {
            for w in &mut self.weights {
                *w /= sum;
            }
        }
    }

    /// 应用蒙皮变换到顶点位置
    pub fn skin_position(&self, skin_matrices: &[Mat4], pos: Vec3) -> Vec3 {
        let mut result = Vec3::ZERO;
        for i in 0..4 {
            if self.weights[i] > 0.0 {
                let m = &skin_matrices[self.bone_indices[i] as usize];
                let transformed = m.transform_point3(pos);
                result += transformed * self.weights[i];
            }
        }
        result
    }

    /// 应用蒙皮变换到顶点法线（使用矩阵的 3x3 部分）
    pub fn skin_normal(&self, skin_matrices: &[Mat4], normal: Vec3) -> Vec3 {
        let mut result = Vec3::ZERO;
        for i in 0..4 {
            if self.weights[i] > 0.0 {
                let m = &skin_matrices[self.bone_indices[i] as usize];
                // 取 3x3 部分变换法线（忽略平移）
                let transformed = Mat3::from_mat4(*m) * normal;
                result += transformed * self.weights[i];
            }
        }
        // 重新归一化（蒙皮后法线可能非单位长度）
        result.normalize_or_zero()
    }
}

/// 将蒙皮权重应用到顶点列表，生成蒙皮后的顶点。
/// 用于离线烘焙或 CPU 端预计算。
pub fn apply_skinning(
    vertices: &[Vertex],
    weights: &[SkinWeights],
    skin_matrices: &[Mat4],
) -> Vec<Vertex> {
    vertices
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| {
            let pos = Vec3::from(v.position);
            let normal = Vec3::from(v.normal);
            let skinned_pos = w.skin_position(skin_matrices, pos);
            let skinned_normal = w.skin_normal(skin_matrices, normal);
            let mut v = *v;
            v.position = skinned_pos.into();
            v.normal = skinned_normal.into();
            v
        })
        .collect()
}

/// 关键帧动画数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationTrack {
    pub name: String,
    pub bone_id: BoneId,
    /// (time, transform) 关键帧（按时间升序）
    pub keyframes: Vec<(f32, JointTransform)>,
    pub loop_mode: LoopMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    Once,
    Loop,
    PingPong,
}

impl AnimationTrack {
    /// 在指定时间采样动画（返回该骨骼的局部变换）
    pub fn sample(&self, time: f32) -> JointTransform {
        if self.keyframes.is_empty() {
            return JointTransform::default();
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].1;
        }
        // 处理循环
        let duration = self.keyframes.last().unwrap().0;
        let t = if duration > 0.0 {
            match self.loop_mode {
                LoopMode::Once => time.min(duration),
                LoopMode::Loop => time.rem_euclid(duration),
                LoopMode::PingPong => {
                    let cycle = duration * 2.0;
                    let phase = time.rem_euclid(cycle);
                    if phase > duration {
                        cycle - phase
                    } else {
                        phase
                    }
                }
            }
        } else {
            0.0
        };
        // 找到包围 t 的两个关键帧
        let mut idx = 0;
        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.0 <= t {
                idx = i;
            } else {
                break;
            }
        }
        if idx + 1 >= self.keyframes.len() {
            return self.keyframes[idx].1;
        }
        let (t0, transform0) = &self.keyframes[idx];
        let (t1, transform1) = &self.keyframes[idx + 1];
        let alpha = if t1 - t0 > 1e-6 {
            (t - t0) / (t1 - t0)
        } else {
            0.0
        };
        transform0.interpolate(transform1, alpha)
    }
}

/// 动画剪辑（多个骨骼轨道的集合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub tracks: Vec<AnimationTrack>,
    pub duration: f32,
}

impl AnimationClip {
    /// 在指定时间采样所有轨道，返回每骨骼的局部变换
    pub fn sample(&self, time: f32) -> hashbrown::HashMap<BoneId, JointTransform> {
        self.tracks
            .iter()
            .map(|track| (track.bone_id, track.sample(time)))
            .collect()
    }
}

use glam::Mat3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_hierarchy() {
        let mut skel = Skeleton::new();
        let root = skel.add_bone("root", None, JointTransform::default());
        let child = skel.add_bone(
            "child",
            Some(root),
            JointTransform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
        );
        let grandchild = skel.add_bone(
            "grandchild",
            Some(child),
            JointTransform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
        );
        skel.compute_bind_pose();
        assert_eq!(skel.bone_count(), 3);
        assert_eq!(skel.roots.len(), 1);
        // grandchild 世界位置应为 (0, 2, 0)
        let gc = skel.bones.get(grandchild).unwrap();
        let world_pos = gc.world_bind.w_axis.truncate();
        assert!((world_pos - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_skin_weights_normalize() {
        let mut w = SkinWeights::default();
        w.add(0, 0.3);
        w.add(1, 0.3);
        w.add(2, 0.3);
        w.normalize();
        let sum: f32 = w.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_skin_weights_keep_top_4() {
        let mut w = SkinWeights::default();
        w.add(0, 0.1);
        w.add(1, 0.2);
        w.add(2, 0.3);
        w.add(3, 0.4);
        w.add(4, 0.5); // 应该替换掉 0.1
        // 找到权重 0.5 是否存在
        assert!(w.weights.contains(&0.5));
        assert!(!w.weights.contains(&0.1));
    }

    #[test]
    fn test_animation_track_sample() {
        let bone_id = BoneId::default();
        let track = AnimationTrack {
            name: "test".to_string(),
            bone_id,
            keyframes: vec![
                (0.0, JointTransform {
                    translation: Vec3::ZERO,
                    ..Default::default()
                }),
                (1.0, JointTransform {
                    translation: Vec3::new(1.0, 0.0, 0.0),
                    ..Default::default()
                }),
            ],
            loop_mode: LoopMode::Once,
        };
        let t0 = track.sample(0.0);
        let t05 = track.sample(0.5);
        let t1 = track.sample(1.0);
        assert!((t0.translation - Vec3::ZERO).length() < 1e-5);
        assert!((t05.translation - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-5);
        assert!((t1.translation - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_animation_loop_mode() {
        let bone_id = BoneId::default();
        let track = AnimationTrack {
            name: "test".to_string(),
            bone_id,
            keyframes: vec![
                (0.0, JointTransform {
                    translation: Vec3::ZERO,
                    ..Default::default()
                }),
                (1.0, JointTransform {
                    translation: Vec3::new(1.0, 0.0, 0.0),
                    ..Default::default()
                }),
            ],
            loop_mode: LoopMode::Loop,
        };
        // 1.5 秒应该循环回 0.5 秒位置
        let t = track.sample(1.5);
        assert!((t.translation - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_apply_skinning() {
        let vertices = vec![Vertex::new([0.0, 0.0, 0.0])];
        let weights = vec![SkinWeights::single(0, 1.0)];
        let skin_matrices = vec![Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))];
        let result = apply_skinning(&vertices, &weights, &skin_matrices);
        assert!((result[0].position[0] - 1.0).abs() < 1e-5);
    }
}
