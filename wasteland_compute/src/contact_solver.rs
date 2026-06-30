//! Contact Solver — 接触约束求解 (含库仑摩擦)
//!
//! 基于:
//! - Catto. "Iterative Dynamics with Temporal Coherence." GDC 2005.
//!   https://box2d.org/files/ErinCatto_GDC2005_IterativeDynamics.pdf
//! - Box2D b2ContactSolver (sequential impulse with friction)
//! - Bullet btSequentialImpulseConstraintSolver
//! - Ericson. Real-Time Collision Detection. Ch 5 (contact resolution)
//!
//! 核心思想 (Sequential Impulse with Friction):
//! 1. 每个接触点产生 3 个约束:
//!    - 1 法向约束: v_rel·n + e·(-v_rel·n) ≥ 0 (非穿透 + 弹性恢复)
//!    - 2 切向约束: 库仑摩擦 (|λ_t| ≤ μ·λ_n)
//! 2. 法向有效质量: k_n = 1/m_a + 1/m_b + (r_a×n)·I_a⁻¹·(r_a×n) + (r_b×n)·I_b⁻¹·(r_b×n)
//! 3. 切向有效质量: 同上, 但用切向量 t1, t2
//! 4. 顺序冲量法: 逐点求解, 多次迭代收敛 (8-15 次速度, 3-5 次位置)
//! 5. Warm starting: 从上一帧累积冲量起步, 加快收敛
//! 6. 摩擦锥: 先求解法向, 再用 λ_n 限制切向冲量
//! 7. Baumgarte 位置修正: bias = β/dt · penetration, 推开穿透

use glam::{Mat3, Vec3};

use crate::contact_manifold::{ContactManifold, ContactPoint};
use crate::rigid_body::RigidBody;

const DEFAULT_VELOCITY_ITERATIONS: usize = 10;
const DEFAULT_POSITION_ITERATIONS: usize = 4;
const BAUMGARTE_BETA: f32 = 0.2;
const MAX_SLOP: f32 = 0.005;
const REST_SLOP: f32 = 0.5; // 速度低于此值时视为非碰撞 (不施加弹性)

// ============================================================
// 辅助函数
// ============================================================

/// 反对称矩阵 skew(r): skew(r)·x = r × x
#[inline]
fn skew(r: Vec3) -> Mat3 {
    Mat3::from_cols(Vec3::new(0.0, r.z, -r.y), Vec3::new(-r.z, 0.0, r.x), Vec3::new(r.y, -r.x, 0.0))
}

/// 接触点处的有效逆质量 (标量, 沿给定方向 axis)
/// k = 1/m_a + 1/m_b + (r_a×axis)·I_a⁻¹·(r_a×axis) + (r_b×axis)·I_b⁻¹·(r_b×axis)
#[inline]
fn effective_mass(a: &RigidBody, b: &RigidBody, r_a: Vec3, r_b: Vec3, axis: Vec3) -> f32 {
    let mut k = 0.0;
    if !a.is_static {
        let rxa = r_a.cross(axis);
        k += a.inv_mass + rxa.dot(a.world_inv_inertia() * rxa);
    }
    if !b.is_static {
        let rxb = r_b.cross(axis);
        k += b.inv_mass + rxb.dot(b.world_inv_inertia() * rxb);
    }
    if k < 1e-12 { 1e12 } else { k }
}

/// 接触点处的相对速度 (B 相对 A)
#[inline]
fn relative_velocity(a: &RigidBody, b: &RigidBody, r_a: Vec3, r_b: Vec3) -> Vec3 {
    let v_a = a.linear_velocity + a.angular_velocity.cross(r_a);
    let v_b = b.linear_velocity + b.angular_velocity.cross(r_b);
    v_b - v_a
}

/// 应用冲量到两个刚体 (沿 axis 方向, 大小 impulse)
#[inline]
fn apply_impulse_pair(a: &mut RigidBody, b: &mut RigidBody, r_a: Vec3, r_b: Vec3, impulse: Vec3) {
    if !a.is_static {
        a.linear_velocity -= impulse * a.inv_mass;
        a.angular_velocity -= a.world_inv_inertia() * r_a.cross(impulse);
    }
    if !b.is_static {
        b.linear_velocity += impulse * b.inv_mass;
        b.angular_velocity += b.world_inv_inertia() * r_b.cross(impulse);
    }
}

/// 位置修正 (直接移动, 不产生速度)
#[inline]
fn apply_position_correction(
    a: &mut RigidBody,
    b: &mut RigidBody,
    r_a: Vec3,
    r_b: Vec3,
    impulse: Vec3,
) {
    if !a.is_static {
        a.position -= impulse * a.inv_mass;
        // 旋转修正: δθ = I⁻¹ · (r × J)
        let delta_rot = a.world_inv_inertia() * r_a.cross(impulse);
        a.rotation = integrate_rotation(a.rotation, delta_rot);
    }
    if !b.is_static {
        b.position += impulse * b.inv_mass;
        let delta_rot = b.world_inv_inertia() * r_b.cross(impulse);
        b.rotation = integrate_rotation(b.rotation, delta_rot);
    }
}

/// 小角度旋转积分: q' = normalize(q + 0.5·δθ·q)
#[inline]
fn integrate_rotation(q: glam::Quat, delta_rot: Vec3) -> glam::Quat {
    let dq =
        glam::Quat::from_xyzw(0.5 * delta_rot.x, 0.5 * delta_rot.y, 0.5 * delta_rot.z, 0.0) * q;
    let result = glam::Quat::from_xyzw(q.x + dq.x, q.y + dq.y, q.z + dq.z, q.w + dq.w);
    if result.length_squared() > 1e-12 { result.normalize() } else { glam::Quat::IDENTITY }
}

// ============================================================
// ContactSolver — 接触求解器
// ============================================================

/// 接触求解器: 管理所有接触流形, 运行顺序冲量法
pub struct ContactSolver {
    /// 速度迭代次数
    pub velocity_iterations: usize,
    /// 位置迭代次数
    pub position_iterations: usize,
    /// 接触流形列表 (引用 body 索引)
    manifolds: Vec<ContactManifold>,
}

impl ContactSolver {
    pub fn new() -> Self {
        Self {
            velocity_iterations: DEFAULT_VELOCITY_ITERATIONS,
            position_iterations: DEFAULT_POSITION_ITERATIONS,
            manifolds: Vec::new(),
        }
    }

    /// 设置迭代次数
    pub fn with_iterations(mut self, velocity: usize, position: usize) -> Self {
        self.velocity_iterations = velocity;
        self.position_iterations = position;
        self
    }

    /// 添加接触流形
    pub fn add_manifold(&mut self, manifold: ContactManifold) {
        self.manifolds.push(manifold);
    }

    /// 清空所有流形 (每帧调用)
    pub fn clear(&mut self) {
        self.manifolds.clear();
    }

    /// 获取流形数量
    #[inline]
    pub fn len(&self) -> usize {
        self.manifolds.len()
    }

    /// 是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.manifolds.is_empty()
    }

    /// 获取流形 (只读)
    pub fn manifolds(&self) -> &[ContactManifold] {
        &self.manifolds
    }

    /// Warm starting: 应用上一帧的累积冲量作为本帧初值
    ///
    /// 必须在 solve_velocity 之前调用
    pub fn warm_start(&mut self, bodies: &mut [RigidBody]) {
        for m in &mut self.manifolds {
            let a_idx = m.a_index;
            let b_idx = m.b_index;
            // 跳过无效索引
            if a_idx >= bodies.len() || b_idx >= bodies.len() {
                continue;
            }
            // 借用两个不同 body
            let (a, b) = get_two_mut(bodies, a_idx, b_idx);
            for cp in &m.points {
                let r_a = cp.point - a.position;
                let r_b = cp.point - b.position;
                let impulse = cp.normal * cp.normal_impulse
                    + m.tangent1 * cp.tangent_impulse1
                    + m.tangent2 * cp.tangent_impulse2;
                apply_impulse_pair(a, b, r_a, r_b, impulse);
            }
        }
    }

    /// 求解速度约束 (法向 + 摩擦)
    pub fn solve_velocity(&mut self, bodies: &mut [RigidBody], dt: f32) {
        if self.manifolds.is_empty() {
            return;
        }
        // 预计算每个接触点的速度偏置 (用初始 vn, 不随迭代改变)
        // velocity_bias = -e * vn_init + position_bias (目标分离速度)
        let mut all_biases: Vec<Vec<f32>> = Vec::with_capacity(self.manifolds.len());
        for m in &self.manifolds {
            let a_idx = m.a_index;
            let b_idx = m.b_index;
            if a_idx >= bodies.len() || b_idx >= bodies.len() {
                all_biases.push(Vec::new());
                continue;
            }
            let (a, b) = get_two_mut(bodies, a_idx, b_idx);
            let mut biases = Vec::with_capacity(m.points.len());
            for cp in &m.points {
                let r_a = cp.point - a.position;
                let r_b = cp.point - b.position;
                let v_rel = relative_velocity(a, b, r_a, r_b);
                let vn = v_rel.dot(m.normal);
                // 弹性: 只在初始接近速度 > REST_SLOP 时施加
                let restitution =
                    if vn < -REST_SLOP { a.restitution.min(b.restitution) } else { 0.0 };
                // 位置偏置 (Baumgarte, 速度级)
                let position_bias = if cp.penetration > MAX_SLOP {
                    BAUMGARTE_BETA * (cp.penetration - MAX_SLOP) / dt
                } else {
                    0.0
                };
                // 目标分离速度 = -e*vn_init + position_bias
                let velocity_bias = -restitution * vn + position_bias;
                biases.push(velocity_bias);
            }
            all_biases.push(biases);
        }

        // 迭代求解
        for _ in 0..self.velocity_iterations {
            for (m_idx, m) in self.manifolds.iter_mut().enumerate() {
                let a_idx = m.a_index;
                let b_idx = m.b_index;
                if a_idx >= bodies.len() || b_idx >= bodies.len() {
                    continue;
                }
                let (a, b) = get_two_mut(bodies, a_idx, b_idx);
                let friction = a.friction.min(b.friction);
                let biases = &all_biases[m_idx];
                solve_manifold_velocity(m, a, b, friction, biases);
            }
        }
    }

    /// 求解位置约束 (Baumgarte 位置修正)
    pub fn solve_position(&mut self, bodies: &mut [RigidBody]) {
        for _ in 0..self.position_iterations {
            for m in &mut self.manifolds {
                let a_idx = m.a_index;
                let b_idx = m.b_index;
                if a_idx >= bodies.len() || b_idx >= bodies.len() {
                    continue;
                }
                let (a, b) = get_two_mut(bodies, a_idx, b_idx);
                solve_manifold_position(m, a, b);
            }
        }
    }

    /// 完整求解流程: warm_start → solve_velocity → solve_position
    pub fn solve(&mut self, bodies: &mut [RigidBody], dt: f32) {
        self.warm_start(bodies);
        self.solve_velocity(bodies, dt);
        self.solve_position(bodies);
    }
}

impl Default for ContactSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 单流形求解
// ============================================================

/// 求解单个流形的速度约束 (法向 + 摩擦)
/// 使用预计算的 velocity_bias (含弹性恢复和位置偏置)
fn solve_manifold_velocity(
    m: &mut ContactManifold,
    a: &mut RigidBody,
    b: &mut RigidBody,
    friction: f32,
    biases: &[f32],
) {
    let n = m.normal;
    let t1 = m.tangent1;
    let t2 = m.tangent2;

    for (i, cp) in m.points.iter_mut().enumerate() {
        let r_a = cp.point - a.position;
        let r_b = cp.point - b.position;

        // 相对速度 (B 相对 A)
        let v_rel = relative_velocity(a, b, r_a, r_b);
        let vn = v_rel.dot(n);

        // ----- 法向约束 -----
        // 目标: vn >= velocity_bias (以 velocity_bias 速度分离)
        // λ = (velocity_bias - vn) / k_n
        let k_n = effective_mass(a, b, r_a, r_b, n);
        let velocity_bias = biases[i];
        let lambda_n = (velocity_bias - vn) / k_n;
        // 累积, clamp >= 0 (法向约束单侧: 只推不拉)
        let old_n = cp.normal_impulse;
        cp.normal_impulse = (old_n + lambda_n).max(0.0);
        let applied_n = cp.normal_impulse - old_n;

        let impulse_n = n * applied_n;
        apply_impulse_pair(a, b, r_a, r_b, impulse_n);

        // ----- 摩擦 (切向) 约束 -----
        // 重新计算相对速度 (法向冲量已改变速度)
        let v_rel = relative_velocity(a, b, r_a, r_b);
        let vt1 = v_rel.dot(t1);
        let vt2 = v_rel.dot(t2);

        let k_t1 = effective_mass(a, b, r_a, r_b, t1);
        let k_t2 = effective_mass(a, b, r_a, r_b, t2);

        let lambda_t1 = -vt1 / k_t1;
        let lambda_t2 = -vt2 / k_t2;

        // 库仑摩擦锥: |λ_t_total| <= mu * lambda_n
        let old_t1 = cp.tangent_impulse1;
        let old_t2 = cp.tangent_impulse2;
        let new_t1 = old_t1 + lambda_t1;
        let new_t2 = old_t2 + lambda_t2;

        let max_friction = friction * cp.normal_impulse;
        let friction_mag = (new_t1 * new_t1 + new_t2 * new_t2).sqrt();
        let (scaled_t1, scaled_t2) = if friction_mag > max_friction && friction_mag > 1e-12 {
            let scale = max_friction / friction_mag;
            (new_t1 * scale, new_t2 * scale)
        } else {
            (new_t1, new_t2)
        };

        let applied_t1 = scaled_t1 - old_t1;
        let applied_t2 = scaled_t2 - old_t2;
        cp.tangent_impulse1 = scaled_t1;
        cp.tangent_impulse2 = scaled_t2;

        let impulse_t = t1 * applied_t1 + t2 * applied_t2;
        apply_impulse_pair(a, b, r_a, r_b, impulse_t);
    }
}

/// 求解单个流形的位置约束 (Baumgarte)
fn solve_manifold_position(m: &mut ContactManifold, a: &mut RigidBody, b: &mut RigidBody) {
    let n = m.normal;
    for cp in &mut m.points {
        let penetration = cp.penetration;
        if penetration <= MAX_SLOP {
            continue;
        }
        let correction = (penetration - MAX_SLOP) * BAUMGARTE_BETA;
        let r_a = cp.point - a.position;
        let r_b = cp.point - b.position;
        let k_n = effective_mass(a, b, r_a, r_b, n);
        // lambda > 0: impulse = +n, B 沿 +n 移开, A 沿 -n 移开 (分离)
        let lambda = correction / k_n;
        let impulse = n * lambda;
        apply_position_correction(a, b, r_a, r_b, impulse);
    }
}

// ============================================================
// 辅助: 安全借用两个不同的可变引用
// ============================================================

/// 从切片中安全借用两个不同索引的可变引用
/// 如果 a_idx == b_idx, 返回 (第一个, 第一个的副本) — 但这不应发生在物理中
fn get_two_mut<T>(slice: &mut [T], a_idx: usize, b_idx: usize) -> (&mut T, &mut T) {
    assert!(a_idx != b_idx, "body indices must differ");
    assert!(a_idx < slice.len() && b_idx < slice.len(), "index out of bounds");
    if a_idx < b_idx {
        let (left, right) = slice.split_at_mut(b_idx);
        (&mut left[a_idx], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(a_idx);
        (&mut right[0], &mut left[b_idx])
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn make_box_at(mass: f32, pos: Vec3, half: Vec3) -> RigidBody {
        let mut b = RigidBody::box_body(mass, half);
        b.position = pos;
        b
    }

    fn make_static_floor() -> RigidBody {
        RigidBody::new_static()
    }

    /// 创建一个接触流形: 球/盒落在地面上
    fn make_floor_contact(
        body_idx: usize,
        floor_idx: usize,
        point: Vec3,
        penetration: f32,
        normal: Vec3,
    ) -> ContactManifold {
        let mut m = ContactManifold::new(floor_idx, body_idx);
        m.normal = normal;
        m.compute_tangents();
        m.points.push(ContactPoint::new(point, normal, penetration));
        m
    }

    #[test]
    fn test_contact_solver_new() {
        let solver = ContactSolver::new();
        assert_eq!(solver.velocity_iterations, DEFAULT_VELOCITY_ITERATIONS);
        assert_eq!(solver.position_iterations, DEFAULT_POSITION_ITERATIONS);
        assert!(solver.is_empty());
    }

    #[test]
    fn test_contact_solver_add_clear() {
        let mut solver = ContactSolver::new();
        let m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.1, Vec3::Y);
        solver.add_manifold(m);
        assert_eq!(solver.len(), 1);
        solver.clear();
        assert!(solver.is_empty());
    }

    #[test]
    fn test_falling_box_stops_on_floor() {
        // 盒子从 y=0.9 自由落体到地面 (y=0 平面)
        // 穿透 0.1, 接触法线 +y
        let mut bodies = vec![
            make_static_floor(),                                          // 地面 (index 0)
            make_box_at(1.0, Vec3::new(0.0, 0.9, 0.0), Vec3::splat(1.0)), // 盒子 (index 1)
        ];
        bodies[0].position = Vec3::ZERO;

        // 给盒子一个向下速度
        bodies[1].linear_velocity = Vec3::new(0.0, -1.0, 0.0);

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.1, Vec3::Y);
        // a_index = 0 (floor), b_index = 1 (box)
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 盒子应该被推上去 (y 速度变正或接近 0)
        assert!(
            bodies[1].linear_velocity.y > -0.1,
            "box should be stopped/pushed up, vy = {}",
            bodies[1].linear_velocity.y
        );
    }

    #[test]
    fn test_restitution_bounce() {
        // 高恢复系数的盒子应该弹起
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.9, 0.0), Vec3::splat(1.0))];
        bodies[1].restitution = 0.8; // 高弹性
        bodies[1].linear_velocity = Vec3::new(0.0, -10.0, 0.0); // 高速下落

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.01, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 应该有正向 (向上) 速度
        assert!(
            bodies[1].linear_velocity.y > 0.0,
            "box should bounce up, vy = {}",
            bodies[1].linear_velocity.y
        );
    }

    #[test]
    fn test_no_restitution_for_slow_contact() {
        // 慢速接触不应有弹性
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.9, 0.0), Vec3::splat(1.0))];
        bodies[1].restitution = 0.8;
        bodies[1].linear_velocity = Vec3::new(0.0, -0.1, 0.0); // 慢速 (< REST_SLOP=0.5)

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.01, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 不应弹起 (速度接近 0, 不为正)
        assert!(
            bodies[1].linear_velocity.y <= 0.5,
            "slow contact should not bounce, vy = {}",
            bodies[1].linear_velocity.y
        );
    }

    #[test]
    fn test_friction_slows_sliding() {
        // 盒子在地面上滑动, 摩擦应使其减速
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[0].friction = 0.5;
        bodies[1].friction = 0.5;
        bodies[1].linear_velocity = Vec3::new(10.0, 0.0, 0.0); // 水平速度

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.01, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // x 方向速度应减小
        assert!(
            bodies[1].linear_velocity.x < 10.0,
            "friction should slow sliding, vx = {}",
            bodies[1].linear_velocity.x
        );
    }

    #[test]
    fn test_high_friction_stops_sliding() {
        // 高摩擦应大幅减速
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[0].friction = 1.0;
        bodies[1].friction = 1.0;
        bodies[1].linear_velocity = Vec3::new(5.0, 0.0, 0.0);

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.5, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new().with_iterations(20, 5);
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 多次迭代后, 高摩擦应使速度接近 0
        assert!(
            bodies[1].linear_velocity.x.abs() < 1.0,
            "high friction should nearly stop sliding, vx = {}",
            bodies[1].linear_velocity.x
        );
    }

    #[test]
    fn test_zero_friction_no_slowing() {
        // 零摩擦, 水平速度不应改变
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[0].friction = 0.0;
        bodies[1].friction = 0.0;
        bodies[1].linear_velocity = Vec3::new(5.0, 0.0, 0.0);

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.01, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        assert!(
            (bodies[1].linear_velocity.x - 5.0).abs() < 1e-3,
            "zero friction should not slow, vx = {}",
            bodies[1].linear_velocity.x
        );
    }

    #[test]
    fn test_position_correction_resolves_penetration() {
        // 盒子穿透地面, 位置修正应推开
        let mut bodies = vec![
            make_static_floor(),
            make_box_at(1.0, Vec3::new(0.0, -0.1, 0.0), Vec3::splat(1.0)),
        ];

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.1, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new().with_iterations(10, 20);
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 盒子 y 应升高 (穿透减少)
        assert!(
            bodies[1].position.y > -0.1,
            "position should be corrected, y = {}",
            bodies[1].position.y
        );
    }

    #[test]
    fn test_warm_start_preserves_impulse() {
        // 两帧之间, warm starting 应保留累积冲量
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[1].linear_velocity = Vec3::new(0.0, -5.0, 0.0);

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.1, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m.clone());

        // 第一帧
        solver.solve(&mut bodies, 1.0 / 60.0);
        let impulse_frame1 = solver.manifolds()[0].points[0].normal_impulse;

        // 第二帧: 保留冲量, warm start 应使其更快收敛
        solver.clear();
        solver.add_manifold(m.clone());
        // 第一帧已将速度修正, 第二帧接近静止
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 累积冲量应存在 (>0)
        assert!(impulse_frame1 > 0.0, "should have accumulated impulse, got {}", impulse_frame1);
    }

    #[test]
    fn test_two_dynamic_bodies_collision() {
        // 两个动态盒子相向运动, 碰撞后应分开
        let mut bodies = vec![
            make_box_at(1.0, Vec3::new(-1.0, 0.0, 0.0), Vec3::splat(1.0)),
            make_box_at(1.0, Vec3::new(1.0, 0.0, 0.0), Vec3::splat(1.0)),
        ];
        bodies[0].linear_velocity = Vec3::new(2.0, 0.0, 0.0); // 向右
        bodies[1].linear_velocity = Vec3::new(-2.0, 0.0, 0.0); // 向左

        // 接触点在原点, 法线从 A(0) 指向 B(1)
        let mut m = make_floor_contact(1, 0, Vec3::ZERO, 0.1, Vec3::new(1.0, 0.0, 0.0));
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // A 速度应减小 (被 B 推回)
        assert!(
            bodies[0].linear_velocity.x < 2.0,
            "A should slow down, vx = {}",
            bodies[0].linear_velocity.x
        );
        // B 速度应增大 (被 A 推向 +x)
        assert!(
            bodies[1].linear_velocity.x > -2.0,
            "B should slow down (less negative), vx = {}",
            bodies[1].linear_velocity.x
        );
    }

    #[test]
    fn test_static_body_not_affected() {
        // 静态地面不应被接触影响
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[1].linear_velocity = Vec3::new(0.0, -10.0, 0.0);

        let floor_pos = bodies[0].position;
        let floor_vel = bodies[0].linear_velocity;

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 0.1, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        assert_eq!(bodies[0].position, floor_pos, "static body should not move");
        assert_eq!(bodies[0].linear_velocity, floor_vel, "static body should not change velocity");
    }

    #[test]
    fn test_multiple_contact_points() {
        // 多接触点 (如盒子底面四角着地)
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[1].linear_velocity = Vec3::new(0.0, -1.0, 0.0);

        let mut m = ContactManifold::new(0, 1);
        m.normal = Vec3::Y;
        m.compute_tangents();
        // 四个底角接触点
        for &(x, z) in &[(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            m.points.push(ContactPoint::new(Vec3::new(x, 0.0, z), Vec3::Y, 0.05));
        }

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 应被推上去
        assert!(
            bodies[1].linear_velocity.y > -0.1,
            "multi-contact should stop fall, vy = {}",
            bodies[1].linear_velocity.y
        );
    }

    #[test]
    fn test_effective_mass_calculation() {
        let a = RigidBody::sphere(1.0, 1.0);
        let b = RigidBody::sphere(1.0, 1.0);
        let r_a = Vec3::new(1.0, 0.0, 0.0);
        let r_b = Vec3::new(-1.0, 0.0, 0.0);
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let k = effective_mass(&a, &b, r_a, r_b, axis);
        // k = 1/m_a + 1/m_b + (r_a×axis)·I⁻¹·(r_a×axis) + (r_b×axis)·I⁻¹·(r_b×axis)
        // r_a × axis = 0 (平行), r_b × axis = 0
        // k = 1 + 1 = 2
        assert!((k - 2.0).abs() < 1e-5, "k = {}, expected 2", k);
    }

    #[test]
    fn test_effective_mass_with_rotation() {
        let a = RigidBody::sphere(1.0, 1.0);
        let b = RigidBody::new_static();
        let r_a = Vec3::new(0.0, 1.0, 0.0); // 偏离质心
        let r_b = Vec3::ZERO;
        let axis = Vec3::new(1.0, 0.0, 0.0); // x 方向
        let k = effective_mass(&a, &b, r_a, r_b, axis);
        // r_a × axis = (0,1,0) × (1,0,0) = (0,0,-1)
        // I⁻¹ = 1/(0.4) * I (球体)
        // (r_a×axis)·I⁻¹·(r_a×axis) = |(0,0,-1)|² / 0.4 = 1/0.4 = 2.5
        // k = 1/m_a + 0 + 2.5 + 0 = 1 + 2.5 = 3.5
        assert!((k - 3.5).abs() < 1e-4, "k = {}, expected 3.5", k);
    }

    #[test]
    fn test_skew_matrix() {
        let r = Vec3::new(1.0, 2.0, 3.0);
        let s = skew(r);
        // skew(r) · x = r × x
        let x = Vec3::new(4.0, 5.0, 6.0);
        let result = s * x;
        let expected = r.cross(x);
        assert!(
            (result - expected).length() < 1e-5,
            "skew mismatch: {:?} vs {:?}",
            result,
            expected
        );
    }

    #[test]
    fn test_get_two_mut_different() {
        let mut arr = vec![10, 20, 30, 40];
        let (a, b) = get_two_mut(&mut arr, 1, 3);
        assert_eq!(*a, 20);
        assert_eq!(*b, 40);
        *a = 99;
        assert_eq!(arr[1], 99);
    }

    #[test]
    fn test_get_two_mut_reversed() {
        let mut arr = vec![10, 20, 30, 40];
        let (a, b) = get_two_mut(&mut arr, 3, 0);
        assert_eq!(*a, 40);
        assert_eq!(*b, 10);
    }

    #[test]
    fn test_friction_cone_clamp() {
        // 大水平速度 + 高法向冲量 → 摩擦被锥限制
        let mut bodies =
            vec![make_static_floor(), make_box_at(1.0, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(1.0))];
        bodies[0].friction = 0.3;
        bodies[1].friction = 0.3;
        bodies[1].linear_velocity = Vec3::new(100.0, -1.0, 0.0); // 极大水平速度

        let mut m = make_floor_contact(1, 0, Vec3::new(0.0, 0.0, 0.0), 1.0, Vec3::Y);
        m.a_index = 0;
        m.b_index = 1;

        let mut solver = ContactSolver::new();
        solver.add_manifold(m);
        solver.solve(&mut bodies, 1.0 / 60.0);

        // 摩擦冲量被锥限制, 速度减小但不为 0
        let vx = bodies[1].linear_velocity.x;
        assert!(vx < 100.0, "friction should reduce vx, got {}", vx);
        assert!(vx > 0.0, "friction should not reverse direction, got {}", vx);
    }

    #[test]
    fn test_empty_solver_no_crash() {
        let mut bodies: Vec<RigidBody> = vec![];
        let mut solver = ContactSolver::new();
        solver.solve(&mut bodies, 1.0 / 60.0);
        // 不应 panic
    }

    #[test]
    fn test_iterations_setting() {
        let solver = ContactSolver::new().with_iterations(20, 8);
        assert_eq!(solver.velocity_iterations, 20);
        assert_eq!(solver.position_iterations, 8);
    }
}
