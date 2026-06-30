//! Skinning - Dual Quaternion Skinning (DQS)
//!
//! 基于:
//! - Kavan, Collins, Zara, O'Sullivan. "Geometric Skinning with Approximate
//!   Dual Quaternion Blending." ACM TOG 2008.
//! - Ladislav Kavan. "Skinning with Dual Quaternions" (游戏开发实践).
//!
//! 核心思想:
//! 1. 传统线性混合蒙皮 (LBS): v' = Σ w_i * M_i * v
//!    - 问题: 矩阵加权在关节弯曲时导致"糖果包装"失真 (体积流失)
//! 2. 对偶四元数 (DQ): 把旋转和平移编码为对偶四元数
//!    - 实部 q_r = rotation quaternion
//!    - 对偶部 q_d = 0.5 * t * q_r  (t = translation 四元数)
//!    - DQ 在 SE(3) 上是刚体变换, 加权混合保持刚体性
//! 3. DQS: DQ_blend = Σ w_i * DQ_i (加权混合), 然后 v' = DQ_blend * v
//!    - 自然避免 LBS 的体积流失
//!    - 代价: 每顶点 8 个 float (vs LBS 的 16 个), 但需要 DQ 运算
//!
//! 对偶四元数代数:
//! - DQ = q_r + ε * q_d, 其中 ε² = 0
//! - DQ1 * DQ2 = (q_r1 * q_r2) + ε * (q_r1 * q_d2 + q_d1 * q_r2)
//! - 共轭: DQ* = q_r* + ε * q_d*
//! - 变换点 p: p' = q_r * p * q_r* + 2 * (q_d * q_r*).xyz

use glam::{Quat, Vec3, Vec4};
use serde::{Deserialize, Serialize};

// ============================================================
// 对偶四元数
// ============================================================

/// 对偶四元数 (实部 + 对偶部, 用 Vec4 存储以支持算术运算)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DualQuaternion {
    /// 实部 (旋转) - (x, y, z, w)
    pub real: Vec4,
    /// 对偶部 (平移编码) - (x, y, z, w)
    pub dual: Vec4,
}

impl DualQuaternion {
    pub const IDENTITY: Self = Self {
        real: Vec4::new(0.0, 0.0, 0.0, 1.0),
        dual: Vec4::ZERO,
    };

    /// 从旋转和平移构建对偶四元数
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        let q_r = quat_to_vec4(rotation);
        let t = Vec4::new(translation.x, translation.y, translation.z, 0.0);
        let q_d = quat_mul_vec4(t, q_r) * 0.5;
        Self { real: q_r, dual: q_d }
    }

    /// 单位化 (对偶四元数归一化)
    pub fn normalize(self) -> Self {
        let len = self.real.length();
        if len < 1e-10 {
            return Self::IDENTITY;
        }
        let real = self.real / len;
        let dual = self.dual / len;
        let dot = real.dot(dual);
        let dual = dual - real * dot;
        Self { real, dual }
    }

    /// 对偶四元数乘法 (组合变换)
    pub fn mul(self, other: Self) -> Self {
        Self {
            real: quat_mul_vec4(self.real, other.real),
            dual: quat_mul_vec4(self.real, other.dual) + quat_mul_vec4(self.dual, other.real),
        }
    }

    /// 共轭 (实部和对偶部都取共轭)
    pub fn conjugate(self) -> Self {
        Self {
            real: quat_conjugate(self.real),
            dual: quat_conjugate(self.dual),
        }
    }

    /// 变换一个点
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let p_quat = Vec4::new(p.x, p.y, p.z, 0.0);
        let rotated = quat_mul_vec4(quat_mul_vec4(self.real, p_quat), quat_conjugate(self.real));
        let trans_q = quat_mul_vec4(self.dual, quat_conjugate(self.real)) * 2.0;
        Vec3::new(rotated.x + trans_q.x, rotated.y + trans_q.y, rotated.z + trans_q.z)
    }

    /// 提取旋转
    pub fn rotation(self) -> Quat {
        vec4_to_quat(self.real)
    }

    /// 提取平移
    pub fn translation(self) -> Vec3 {
        let t = quat_mul_vec4(self.dual, quat_conjugate(self.real)) * 2.0;
        Vec3::new(t.x, t.y, t.z)
    }
}

impl Default for DualQuaternion {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// ============================================================
// Quat <-> Vec4 转换和 Vec4 四元数运算
// ============================================================

#[inline]
fn quat_to_vec4(q: Quat) -> Vec4 {
    Vec4::new(q.x, q.y, q.z, q.w)
}

#[inline]
fn vec4_to_quat(v: Vec4) -> Quat {
    Quat::from_xyzw(v.x, v.y, v.z, v.w)
}

#[inline]
fn quat_conjugate(q: Vec4) -> Vec4 {
    Vec4::new(-q.x, -q.y, -q.z, q.w)
}

#[inline]
fn quat_mul_vec4(a: Vec4, b: Vec4) -> Vec4 {
    // 四元数乘法: (a.w*b + b.w*a + a.xyz × b.xyz, a.w*b.w - a.xyz · b.xyz)
    let ax = Vec3::new(a.x, a.y, a.z);
    let aw = a.w;
    let bx = Vec3::new(b.x, b.y, b.z);
    let bw = b.w;
    let xyz = ax.cross(bx) + bx * aw + ax * bw;
    let w = aw * bw - ax.dot(bx);
    Vec4::new(xyz.x, xyz.y, xyz.z, w)
}

// ============================================================
// 对偶四元数混合 (DQB)
// ============================================================

/// 加权混合多个对偶四元数
/// 注意: 需要处理 "antipodal" 情况 (DQ 和 -DQ 表示同一变换)
pub fn dual_quaternion_blend(dqs: &[DualQuaternion], weights: &[f32]) -> DualQuaternion {
    debug_assert_eq!(dqs.len(), weights.len());
    if dqs.is_empty() {
        return DualQuaternion::IDENTITY;
    }
    let mut real_acc = Vec4::ZERO;
    let mut dual_acc = Vec4::ZERO;
    let ref_real = dqs[0].real;
    for (dq, w) in dqs.iter().zip(weights.iter()) {
        let mut dq = *dq;
        if ref_real.dot(dq.real) < 0.0 {
            dq.real = -dq.real;
            dq.dual = -dq.dual;
        }
        real_acc += dq.real * (*w);
        dual_acc += dq.dual * (*w);
    }
    DualQuaternion {
        real: real_acc,
        dual: dual_acc,
    }
    .normalize()
}

// ============================================================
// 蒙皮网格
// ============================================================

/// 骨骼姿态 (每个骨骼的对偶四元数)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonPose {
    /// 每个骨骼关节的世界变换 (对偶四元数)
    pub joint_transforms: Vec<DualQuaternion>,
}

impl SkeletonPose {
    pub fn new(joint_count: usize) -> Self {
        Self {
            joint_transforms: vec![DualQuaternion::IDENTITY; joint_count],
        }
    }

    /// 设置关节变换
    pub fn set_joint(&mut self, idx: usize, rotation: Quat, translation: Vec3) {
        if idx < self.joint_transforms.len() {
            self.joint_transforms[idx] = DualQuaternion::from_rotation_translation(rotation, translation);
        }
    }
}

/// 顶点蒙皮权重
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VertexSkinning {
    /// 影响该顶点的骨骼索引 (最多 4 个)
    pub joint_indices: [u32; 4],
    /// 对应权重 (应归一化为 1)
    pub weights: [f32; 4],
    /// 实际影响数 (1-4)
    pub bone_count: u8,
}

impl VertexSkinning {
    pub fn new(joints: &[u32], weights: &[f32]) -> Self {
        let mut joint_indices = [0u32; 4];
        let mut w = [0.0f32; 4];
        let n = joints.len().min(4);
        for i in 0..n {
            joint_indices[i] = joints[i];
            w[i] = weights[i];
        }
        // 归一化权重
        let sum: f32 = w.iter().sum();
        if sum > 1e-10 {
            for wi in &mut w {
                *wi /= sum;
            }
        }
        Self {
            joint_indices,
            weights: w,
            bone_count: n as u8,
        }
    }
}

/// 对一个顶点应用 DQS 蒙皮
pub fn skin_vertex(
    vertex: Vec3,
    skinning: &VertexSkinning,
    pose: &SkeletonPose,
    inverse_bind_pose: &[DualQuaternion], // 每个骨骼的逆绑定姿态
) -> Vec3 {
    let n = skinning.bone_count as usize;
    if n == 0 {
        return vertex;
    }
    // 收集每个骨骼的最终变换 (current * inverse_bind)
    let mut dqs = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    for i in 0..n {
        let joint_idx = skinning.joint_indices[i] as usize;
        if joint_idx >= pose.joint_transforms.len() {
            continue;
        }
        let current = pose.joint_transforms[joint_idx];
        let inv_bind = inverse_bind_pose.get(joint_idx).copied().unwrap_or(DualQuaternion::IDENTITY);
        // 最终变换 = current * inv_bind
        let final_dq = current.mul(inv_bind);
        dqs.push(final_dq);
        weights.push(skinning.weights[i]);
    }
    let blended = dual_quaternion_blend(&dqs, &weights);
    blended.transform_point(vertex)
}

/// 批量蒙皮 (整个网格)
pub fn skin_mesh(
    vertices: &[Vec3],
    skinning: &[VertexSkinning],
    pose: &SkeletonPose,
    inverse_bind_pose: &[DualQuaternion],
) -> Vec<Vec3> {
    vertices
        .iter()
        .zip(skinning.iter())
        .map(|(v, s)| skin_vertex(*v, s, pose, inverse_bind_pose))
        .collect()
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dq_identity() {
        let dq = DualQuaternion::IDENTITY;
        let p = Vec3::new(1.0, 2.0, 3.0);
        let p2 = dq.transform_point(p);
        assert!((p2 - p).length() < 1e-5, "identity should not change point");
    }

    #[test]
    fn test_dq_translation() {
        let t = Vec3::new(1.0, 2.0, 3.0);
        let dq = DualQuaternion::from_rotation_translation(Quat::IDENTITY, t);
        let p = Vec3::new(0.0, 0.0, 0.0);
        let p2 = dq.transform_point(p);
        assert!((p2 - t).length() < 1e-5, "translation: {} -> {}", p, p2);
    }

    #[test]
    fn test_dq_rotation() {
        let rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2); // 90° about y
        let dq = DualQuaternion::from_rotation_translation(rot, Vec3::ZERO);
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p2 = dq.transform_point(p);
        // 绕 y 轴 90°: (1,0,0) -> (0,0,-1)
        assert!((p2 - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4, "rotation: {:?} -> {:?}", p, p2);
    }

    #[test]
    fn test_dq_rotation_translation_combined() {
        let rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let t = Vec3::new(1.0, 0.0, 0.0);
        let dq = DualQuaternion::from_rotation_translation(rot, t);
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p2 = dq.transform_point(p);
        // 先旋转 (绕 z 90°: (1,0,0) -> (0,1,0)), 再平移 (+1,0,0) -> (1,1,0)
        assert!((p2 - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-4, "combined: {:?} -> {:?}", p, p2);
    }

    #[test]
    fn test_dq_multiplication() {
        // DQ1 * DQ2: 先应用 DQ2, 再应用 DQ1
        let t1 = Vec3::new(1.0, 0.0, 0.0);
        let t2 = Vec3::new(0.0, 1.0, 0.0);
        let dq1 = DualQuaternion::from_rotation_translation(Quat::IDENTITY, t1);
        let dq2 = DualQuaternion::from_rotation_translation(Quat::IDENTITY, t2);
        let combined = dq1.mul(dq2);
        let p = Vec3::ZERO;
        let p2 = combined.transform_point(p);
        // 平移叠加: (0,0,0) -> (1,1,0)
        assert!((p2 - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-4, "mul: {:?}", p2);
    }

    #[test]
    fn test_dq_extract_rotation_translation() {
        let rot = Quat::from_rotation_x(0.5);
        let t = Vec3::new(2.0, 3.0, 4.0);
        let dq = DualQuaternion::from_rotation_translation(rot, t);
        let extracted_rot = dq.rotation();
        let extracted_t = dq.translation();
        assert!((extracted_rot - rot).length() < 1e-4, "extracted rotation");
        assert!((extracted_t - t).length() < 1e-4, "extracted translation: {:?} vs {:?}", extracted_t, t);
    }

    #[test]
    fn test_dq_normalize() {
        // 缩放过的 DQ 归一化后应保持变换
        let rot = Quat::from_rotation_y(0.5);
        let t = Vec3::new(1.0, 2.0, 3.0);
        let dq = DualQuaternion::from_rotation_translation(rot, t);
        // 乘以 2.0 (非单位)
        let scaled = DualQuaternion {
            real: dq.real * 2.0,
            dual: dq.dual * 2.0,
        };
        let normalized = scaled.normalize();
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p_orig = dq.transform_point(p);
        let p_norm = normalized.transform_point(p);
        assert!((p_orig - p_norm).length() < 1e-4, "normalize preserves transform");
    }

    #[test]
    fn test_dq_blend_single() {
        let dq = DualQuaternion::from_rotation_translation(
            Quat::from_rotation_y(1.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        let blended = dual_quaternion_blend(&[dq], &[1.0]);
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p1 = dq.transform_point(p);
        let p2 = blended.transform_point(p);
        assert!((p1 - p2).length() < 1e-4, "single blend should match");
    }

    #[test]
    fn test_dq_blend_two_translations() {
        // 混合两个平移应给出中间平移
        let dq1 = DualQuaternion::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.0));
        let dq2 = DualQuaternion::from_rotation_translation(Quat::IDENTITY, Vec3::new(2.0, 0.0, 0.0));
        let blended = dual_quaternion_blend(&[dq1, dq2], &[0.5, 0.5]);
        let p = Vec3::ZERO;
        let p2 = blended.transform_point(p);
        // 中点: (1, 0, 0)
        assert!((p2 - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4, "mid translation: {:?}", p2);
    }

    #[test]
    fn test_dq_blend_rotations_no_collapse() {
        // DQS 关键测试: 两个相反旋转的混合不会塌缩 (LBS 会)
        let dq1 = DualQuaternion::from_rotation_translation(
            Quat::from_rotation_z(1.0),
            Vec3::ZERO,
        );
        let dq2 = DualQuaternion::from_rotation_translation(
            Quat::from_rotation_z(-1.0),
            Vec3::ZERO,
        );
        let blended = dual_quaternion_blend(&[dq1, dq2], &[0.5, 0.5]);
        // 混合后应接近 identity (两个相反旋转中点)
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p2 = blended.transform_point(p);
        // 应接近原点 (旋转抵消), 不应塌缩到原点 (LBS 会)
        assert!((p2 - p).length() < 0.1, "blend opposite rotations: {:?} -> {:?}", p, p2);
    }

    #[test]
    fn test_dq_antipodal_handling() {
        // q 和 -q 表示同一旋转, 混合应正确处理
        let rot = Quat::from_rotation_y(0.5);
        let dq1 = DualQuaternion::from_rotation_translation(rot, Vec3::ZERO);
        let dq2 = DualQuaternion {
            real: -dq1.real,
            dual: -dq1.dual,
        };
        // 两者表示同一变换
        let p = Vec3::new(1.0, 0.0, 0.0);
        let p1 = dq1.transform_point(p);
        let p2 = dq2.transform_point(p);
        assert!((p1 - p2).length() < 1e-4, "antipodal DQ same transform");
        // 混合也应给同一结果
        let blended = dual_quaternion_blend(&[dq1, dq2], &[0.5, 0.5]);
        let p3 = blended.transform_point(p);
        assert!((p1 - p3).length() < 1e-4, "antipodal blend");
    }

    #[test]
    fn test_vertex_skinning_creation() {
        let vs = VertexSkinning::new(&[0, 1], &[0.5, 0.5]);
        assert_eq!(vs.bone_count, 2);
        assert!((vs.weights[0] - 0.5).abs() < 1e-5);
        assert!((vs.weights[1] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_vertex_skinning_normalization() {
        // 权重未归一化时应自动归一化
        let vs = VertexSkinning::new(&[0, 1], &[2.0, 2.0]);
        let sum: f32 = vs.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "weights normalized: sum={}", sum);
    }

    #[test]
    fn test_skin_vertex_single_bone() {
        let mut pose = SkeletonPose::new(1);
        pose.set_joint(0, Quat::IDENTITY, Vec3::new(1.0, 0.0, 0.0));
        let inv_bind = vec![DualQuaternion::IDENTITY];
        let skinning = VertexSkinning::new(&[0], &[1.0]);
        let v = Vec3::new(0.0, 0.0, 0.0);
        let v2 = skin_vertex(v, &skinning, &pose, &inv_bind);
        assert!((v2 - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4, "single bone skin: {:?}", v2);
    }

    #[test]
    fn test_skin_vertex_two_bones_midpoint() {
        // 顶点受两个骨骼影响, 各 50%, 应变换到中点
        let mut pose = SkeletonPose::new(2);
        pose.set_joint(0, Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.0));
        pose.set_joint(1, Quat::IDENTITY, Vec3::new(2.0, 0.0, 0.0));
        let inv_bind = vec![DualQuaternion::IDENTITY; 2];
        let skinning = VertexSkinning::new(&[0, 1], &[0.5, 0.5]);
        let v = Vec3::ZERO;
        let v2 = skin_vertex(v, &skinning, &pose, &inv_bind);
        // 中点: (1, 0, 0)
        assert!((v2 - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4, "two bone midpoint: {:?}", v2);
    }

    #[test]
    fn test_skin_mesh_batch() {
        let mut pose = SkeletonPose::new(1);
        pose.set_joint(0, Quat::from_rotation_y(1.0), Vec3::ZERO);
        let inv_bind = vec![DualQuaternion::IDENTITY];
        let vertices = vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];
        let skinning = vec![
            VertexSkinning::new(&[0], &[1.0]),
            VertexSkinning::new(&[0], &[1.0]),
        ];
        let result = skin_mesh(&vertices, &skinning, &pose, &inv_bind);
        assert_eq!(result.len(), 2);
        // 验证第一个顶点被变换
        assert!((result[0] - Vec3::new(1.0, 0.0, 0.0)).length() > 0.1, "vertex transformed");
    }

    #[test]
    fn test_dq_volume_preservation() {
        // DQS 关键特性: 关节弯曲时体积保持 (LBS 会塌缩)
        // 模拟肘部弯曲: 上臂静止, 前臂旋转 90°
        let upper_arm = DualQuaternion::from_rotation_translation(Quat::IDENTITY, Vec3::ZERO);
        let forearm = DualQuaternion::from_rotation_translation(
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 0.0, 0.0), // 前臂起点在 (1,0,0)
        );
        // 肘部顶点 (在两骨骼交界), 50% 各
        let blended = dual_quaternion_blend(&[upper_arm, forearm], &[0.5, 0.5]);
        // 验证变换是刚体 (保持距离)
        let p1 = Vec3::new(0.5, 0.0, 0.0);
        let p2 = Vec3::new(0.5, 0.1, 0.0);
        let d_orig = (p2 - p1).length();
        let p1_t = blended.transform_point(p1);
        let p2_t = blended.transform_point(p2);
        let d_new = (p2_t - p1_t).length();
        // 距离应保持 (刚体变换)
        assert!((d_new - d_orig) / d_orig < 0.1, "volume preservation: orig={} new={}", d_orig, d_new);
    }
}
