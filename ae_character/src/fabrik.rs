//! FABRIK — Forward And Backward Reaching Inverse Kinematics
//!
//! 基于:
//! - Aristidou, A., & Lasenby, J. (2011). "FABRIK: A fast, iterative solver for
//!   the Inverse Kinematics problem." Graphical Models, 73(5), 243-260.
//! - Aristidou, A., et al. (2018). "Inverse Kinematics Techniques in Computer
//!   Graphics: A Survey." Computer Graphics Forum, 37(6), 35-58.
//! - Aristidou, A. (2010). "Tracking and Modelling Motion for Biomechanical
//!   Analysis." PhD thesis, University of Cambridge.
//!
//! 核心思想:
//! FABRIK 用"前后向到达"迭代求解 IK:
//! 1. Forward: 从末端执行器向根节点迭代, 把每个关节拉到下一个关节的目标位置
//! 2. Backward: 从根节点向末端迭代, 把每个关节拉到上一个关节的目标位置
//! 3. 重复直到收敛或达到最大迭代次数
//!
//! 优势 (vs CCD / Jacobian):
//! - 收敛速度快 (通常 10-20 次迭代)
//! - 视觉自然 (关节角度变化平滑)
//! - 计算量小 (只需 Vec3 运算, 无矩阵求逆)
//! - 支持多端执行器和角度约束
//!
//! 局限:
//! - 角度约束需要额外处理 (本实现用锥角约束, 简化版)
//! - 末端朝向约束需要扩展 (本实现用简单后处理)

use glam::{Vec3, Quat};

const FABRIK_MAX_ITER: usize = 20;
const FABRIK_TOLERANCE: f32 = 1e-4;

// ============================================================
// 锥角约束
// ============================================================

/// 锥角约束: 限制关节处的偏转角度
///
/// 约束作用于 joint_index 处的关节, 限制该关节之后的链方向
/// 相对于该关节之前的链方向的最大夹角
#[derive(Debug, Clone, Copy)]
pub struct ConeConstraint {
    /// 约束的关节索引 (限制 (joints[i-1]→joints[i]) 与 (joints[i]→joints[i+1]) 间的夹角)
    pub joint_index: usize,
    /// 最大偏转角度 (弧度)
    pub max_angle: f32,
}

impl ConeConstraint {
    pub fn new(joint_index: usize, max_angle_rad: f32) -> Self {
        Self { joint_index, max_angle: max_angle_rad }
    }
}

// ============================================================
// FABRIK 链
// ============================================================

/// FABRIK 单链
#[derive(Debug, Clone)]
pub struct FabrikChain {
    /// 关节位置 (joints[0] = 根, joints[n-1] = 末端执行器)
    pub joints: Vec<Vec3>,
    /// 相邻关节间的原始距离 (distances[i] = joints[i] 到 joints[i+1] 的距离)
    /// 长度 = joints.len() - 1
    pub distances: Vec<f32>,
    /// 根节点固定位置 (backward pass 起始点)
    pub root: Vec3,
}

impl FabrikChain {
    /// 从关节位置创建 FABRIK 链
    pub fn new(joints: Vec<Vec3>) -> Self {
        assert!(joints.len() >= 2, "FABRIK chain needs at least 2 joints");
        let distances: Vec<f32> = joints.windows(2)
            .map(|w| (w[1] - w[0]).length())
            .collect();
        let root = joints[0];
        Self { joints, distances, root }
    }

    /// 末端执行器位置
    #[inline]
    pub fn end_effector(&self) -> Vec3 {
        *self.joints.last().unwrap()
    }

    /// 链的总长
    #[inline]
    pub fn total_length(&self) -> f32 {
        self.distances.iter().sum()
    }

    /// 求解 IK: 把末端执行器移动到 target
    /// 返回是否收敛到容差内
    pub fn solve(&mut self, target: Vec3) -> bool {
        self.solve_with_constraints(target, &[])
    }

    /// 求解 IK (带角度约束)
    pub fn solve_with_constraints(
        &mut self,
        target: Vec3,
        constraints: &[ConeConstraint],
    ) -> bool {
        let n = self.joints.len();
        if n < 2 { return false; }

        // 检查目标可达性
        let total_length = self.total_length();
        let root_to_target = (target - self.root).length();
        if root_to_target > total_length {
            // 不可达: 拉直朝向目标
            let dir = (target - self.root).normalize_or_zero();
            if dir.length_squared() > 1e-12 {
                for i in 1..n {
                    self.joints[i] = self.joints[i - 1] + dir * self.distances[i - 1];
                }
            }
            return false;
        }

        let mut iter = 0;
        while iter < FABRIK_MAX_ITER {
            let end = self.end_effector();
            if (end - target).length() < FABRIK_TOLERANCE {
                return true;
            }

            // ---- Forward pass: 从末端向根 ----
            // 把末端固定在 target, 然后逐个调整前面的关节保持段长
            self.joints[n - 1] = target;
            for i in (0..n - 1).rev() {
                let diff = self.joints[i] - self.joints[i + 1];
                let dir = diff.normalize_or_zero();
                self.joints[i] = self.joints[i + 1] + dir * self.distances[i];
            }

            // ---- Backward pass: 从根向末端 ----
            // 把根固定在 root, 然后逐个调整后面的关节保持段长
            self.joints[0] = self.root;
            for i in 1..n {
                let diff = self.joints[i] - self.joints[i - 1];
                let dir = diff.normalize_or_zero();
                self.joints[i] = self.joints[i - 1] + dir * self.distances[i - 1];
            }

            // ---- 应用角度约束 ----
            if !constraints.is_empty() {
                self.apply_constraints(constraints);
            }

            iter += 1;
        }

        // 最终检查
        let end = self.end_effector();
        (end - target).length() < FABRIK_TOLERANCE * 100.0
    }

    /// 应用锥角约束 (简化版: 在 FABRIK 迭代后调整)
    ///
    /// 注意: 这会改变被约束关节之后的段长, 但下次 FABRIK 迭代会修复
    fn apply_constraints(&mut self, constraints: &[ConeConstraint]) {
        let n = self.joints.len();
        for c in constraints {
            let i = c.joint_index;
            if i == 0 || i >= n - 1 { continue; }

            let ref_dir = (self.joints[i] - self.joints[i - 1]).normalize_or_zero();
            let current_dir = (self.joints[i + 1] - self.joints[i]).normalize_or_zero();
            if ref_dir.length_squared() < 1e-12 || current_dir.length_squared() < 1e-12 {
                continue;
            }

            let dot = current_dir.dot(ref_dir).clamp(-1.0, 1.0);
            let angle = dot.acos();
            if angle <= c.max_angle {
                continue;  // 在约束内
            }

            // 把 current_dir 旋转回 max_angle 内
            // 旋转轴 = ref_dir × current_dir (右手法则, 正旋转方向从 ref 到 current)
            let axis = ref_dir.cross(current_dir);
            if axis.length_squared() < 1e-12 {
                continue;  // 共线, 无需约束
            }
            let axis = axis.normalize();
            // 旋转 ref_dir 朝 current_dir 方向 max_angle 角度
            let rot = Quat::from_axis_angle(axis, c.max_angle);
            let new_dir = rot * ref_dir;
            self.joints[i + 1] = self.joints[i] + new_dir * self.distances[i];
        }
    }

    /// 求解 IK (带末端朝向约束)
    ///
    /// target: 末端位置
    /// target_orientation: 末端最后一节的目标方向 (单位向量)
    ///
    /// 实现: 先用 FABRIK 到达位置, 然后强制调整最后一节方向
    /// (简化版, 不保证所有段长严格不变)
    pub fn solve_with_orientation(
        &mut self,
        target: Vec3,
        target_orientation: Vec3,
    ) -> bool {
        let n = self.joints.len();
        if n < 3 { return self.solve(target); }

        // 先用普通 FABRIK 到达目标位置
        let _ = self.solve(target);

        // 强制调整最后一节方向
        let ori_dir = target_orientation.normalize_or_zero();
        if ori_dir.length_squared() > 1e-12 {
            let last_dist = self.distances[n - 2];
            self.joints[n - 1] = target;
            self.joints[n - 2] = target - ori_dir * last_dist;
        }

        true
    }
}

// ============================================================
// FABRIK 树 (多端执行器)
// ============================================================

/// FABRIK 树: 共享根节点的多条链
///
/// 用于人形 IK (双腿 + 双臂 + 头部共享髋部/腰部根节点)
#[derive(Debug, Clone)]
pub struct FabrikTree {
    /// 共享的根节点位置
    pub root: Vec3,
    /// 多条链
    pub chains: Vec<FabrikChain>,
}

impl FabrikTree {
    pub fn new(root: Vec3) -> Self {
        Self { root, chains: Vec::new() }
    }

    /// 添加一条独立链 (链的 joints[0] 应为 self.root 的子节点)
    pub fn add_chain(&mut self, joints: Vec<Vec3>) -> usize {
        let idx = self.chains.len();
        let mut chain = FabrikChain::new(joints);
        // 把链的根固定到 tree 的根
        chain.root = self.root;
        chain.joints[0] = self.root;
        self.chains.push(chain);
        idx
    }

    /// 求解多端 IK
    ///
    /// targets: [(chain_idx, target_position), ...]
    /// 返回所有链是否都到达目标
    pub fn solve(&mut self, targets: &[(usize, Vec3)]) -> bool {
        for _iter in 0..FABRIK_MAX_ITER {
            // 对每条链独立求解
            for &(chain_idx, target) in targets {
                if chain_idx >= self.chains.len() { continue; }
                self.chains[chain_idx].solve(target);
            }
            // 把所有链的根节点拉回 self.root (松弛迭代)
            for c in &mut self.chains {
                c.joints[0] = self.root;
                c.root = self.root;
                // 重新 backward 调整以保持段长
                for i in 1..c.joints.len() {
                    let diff = c.joints[i] - c.joints[i - 1];
                    let dir = diff.normalize_or_zero();
                    c.joints[i] = c.joints[i - 1] + dir * c.distances[i - 1];
                }
            }
        }

        // 检查所有目标是否到达
        let mut all_ok = true;
        for &(chain_idx, target) in targets {
            if chain_idx >= self.chains.len() { continue; }
            let end = self.chains[chain_idx].end_effector();
            if (end - target).length() > FABRIK_TOLERANCE * 100.0 {
                all_ok = false;
            }
        }
        all_ok
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_reach() {
        // 简单 3 关节链, 沿 X 轴
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);

        // 目标在 (1.5, 0.5, 0) - 可达
        let target = Vec3::new(1.5, 0.5, 0.0);
        let ok = chain.solve(target);
        assert!(ok, "should converge");
        let end = chain.end_effector();
        assert!((end - target).length() < 1e-2, "end effector should reach target: {:?}", end);
    }

    #[test]
    fn test_unreachable_target() {
        // 3 关节链, 总长 2, 目标在距离 5 处 (不可达)
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        let target = Vec3::new(5.0, 0.0, 0.0);
        let ok = chain.solve(target);
        assert!(!ok, "unreachable should return false");
        // 链应该被拉直朝向目标
        let end = chain.end_effector();
        assert!((end - target).length() < 0.5, "should be stretched toward target: {:?}", end);
    }

    #[test]
    fn test_preserve_segment_lengths() {
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        let original_distances: Vec<f32> = joints.windows(2)
            .map(|w| (w[1] - w[0]).length())
            .collect();
        let mut chain = FabrikChain::new(joints);
        chain.solve(Vec3::new(1.5, 1.5, 0.5));

        // 验证所有段长不变
        for i in 0..chain.joints.len() - 1 {
            let d = (chain.joints[i + 1] - chain.joints[i]).length();
            assert!((d - original_distances[i]).abs() < 1e-3,
                "segment {} length changed: {} vs {}", i, d, original_distances[i]);
        }
    }

    #[test]
    fn test_root_fixed() {
        let root = Vec3::new(1.0, 2.0, 3.0);
        let joints = vec![
            root,
            root + Vec3::new(1.0, 0.0, 0.0),
            root + Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        chain.solve(root + Vec3::new(1.5, 0.5, 0.0));
        // 根节点不应移动
        assert!((chain.joints[0] - root).length() < 1e-4,
            "root should be fixed: {:?}", chain.joints[0]);
    }

    #[test]
    fn test_cone_constraint() {
        // 4 关节链沿 X 轴
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);

        // 在关节 1 处限制偏转角 <= 30°
        let constraints = vec![ConeConstraint::new(1, std::f32::consts::PI / 6.0)];

        // 目标在斜上方, 要求大角度弯曲
        let target = Vec3::new(1.0, 2.0, 0.0);
        chain.solve_with_constraints(target, &constraints);

        // 验证关节 1 处的偏转角 <= 30° + 容差
        let ref_dir = (chain.joints[1] - chain.joints[0]).normalize();
        let cur_dir = (chain.joints[2] - chain.joints[1]).normalize();
        let dot = ref_dir.dot(cur_dir).clamp(-1.0, 1.0);
        let angle = dot.acos();
        assert!(angle <= std::f32::consts::PI / 6.0 + 1e-2,
            "joint angle {} should be <= 30°", angle.to_degrees());
    }

    #[test]
    fn test_long_chain_convergence() {
        // 10 关节长链
        let mut joints = vec![Vec3::ZERO];
        for i in 1..10 {
            joints.push(Vec3::new(i as f32, 0.0, 0.0));
        }
        let mut chain = FabrikChain::new(joints);

        // 目标在链可达范围内
        let target = Vec3::new(5.0, 3.0, 0.0);
        let total_len = chain.total_length();
        assert!((target - chain.root).length() < total_len, "target should be reachable");

        let ok = chain.solve(target);
        assert!(ok, "should converge");
        let end = chain.end_effector();
        assert!((end - target).length() < 1e-2, "end should reach target: {:?}", end);
    }

    #[test]
    fn test_zero_length_segment() {
        // 包含零长度段的链 (退化情况)
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),  // 零长度
            Vec3::new(1.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        // 不应 panic
        let _ = chain.solve(Vec3::new(0.5, 0.5, 0.0));
    }

    #[test]
    fn test_target_at_root() {
        // 目标就是根节点
        let root = Vec3::new(0.0, 0.0, 0.0);
        let joints = vec![
            root,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        let _ = chain.solve(root);
        let end = chain.end_effector();
        // 末端应该接近根
        assert!((end - root).length() < 0.5, "end should be near root: {:?}", end);
    }

    #[test]
    fn test_3d_target() {
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        let target = Vec3::new(1.0, 2.0, 1.0);
        let ok = chain.solve(target);
        assert!(ok, "should converge in 3D");
        let end = chain.end_effector();
        assert!((end - target).length() < 1e-2, "3D target: {:?}", end);
    }

    #[test]
    fn test_solve_with_orientation() {
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        let target = Vec3::new(2.0, 1.0, 0.0);
        let _ = chain.solve_with_orientation(target, Vec3::new(0.0, 1.0, 0.0));

        // 验证末端在目标位置
        let end = chain.end_effector();
        assert!((end - target).length() < 1e-3, "end at target: {:?}", end);

        // 验证最后一节方向接近 +Y
        let last_dir = (chain.joints[3] - chain.joints[2]).normalize();
        let dot = last_dir.dot(Vec3::new(0.0, 1.0, 0.0));
        assert!(dot > 0.95, "last segment should point +Y: dot={}, dir={:?}", dot, last_dir);
    }

    #[test]
    fn test_multiple_chains() {
        // 树: 根节点连接两条链
        let root = Vec3::new(0.0, 0.0, 0.0);
        let mut tree = FabrikTree::new(root);

        // 链 0: 向 +X
        tree.add_chain(vec![
            root,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ]);
        // 链 1: 向 +Y
        tree.add_chain(vec![
            root,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ]);

        let targets = vec![
            (0, Vec3::new(1.5, 0.5, 0.0)),
            (1, Vec3::new(0.5, 1.5, 0.0)),
        ];
        let _ = tree.solve(&targets);

        // 两条链的末端应接近各自目标
        let end0 = tree.chains[0].end_effector();
        let end1 = tree.chains[1].end_effector();
        assert!((end0 - targets[0].1).length() < 0.5, "chain 0 end: {:?}", end0);
        assert!((end1 - targets[1].1).length() < 0.5, "chain 1 end: {:?}", end1);

        // 两条链的根节点应保持一致
        assert!((tree.chains[0].joints[0] - tree.chains[1].joints[0]).length() < 1e-3,
            "chain roots should match");
    }

    #[test]
    fn test_chain_distances_preserved() {
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        ];
        let original_dists: Vec<f32> = joints.windows(2)
            .map(|w| (w[1] - w[0]).length())
            .collect();
        let mut chain = FabrikChain::new(joints);

        // 多次求解不同目标
        chain.solve(Vec3::new(2.0, 2.0, 0.0));
        chain.solve(Vec3::new(0.0, 3.0, 0.0));
        chain.solve(Vec3::new(3.0, 0.0, 0.0));

        for i in 0..chain.joints.len() - 1 {
            let d = (chain.joints[i + 1] - chain.joints[i]).length();
            assert!((d - original_dists[i]).abs() < 1e-2,
                "distance {} changed: {} vs {}", i, d, original_dists[i]);
        }
    }

    #[test]
    fn test_already_at_target() {
        let joints = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let mut chain = FabrikChain::new(joints);
        // 目标就是当前末端位置
        let target = Vec3::new(2.0, 0.0, 0.0);
        let ok = chain.solve(target);
        assert!(ok, "already at target should converge");
        let end = chain.end_effector();
        assert!((end - target).length() < 1e-3);
    }
}
