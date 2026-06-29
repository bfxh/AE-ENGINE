//! Rigid Body Dynamics — 刚体动力学
//!
//! 基于:
//! - Baraff. "An Introduction to Physically Based Modeling: Rigid Body Simulation."
//!   SIGGRAPH Course Notes 2001.
//! - Bullet Physics SDK 的脉冲法碰撞响应
//! - Catto. "Iterative Dynamics with Temporal Coherence." GDC 2005.
//! - Mirtich. "Impulse-based Dynamic Simulation of Rigid Body Systems." PhD thesis 1996.
//!
//! 核心思想:
//! 1. 半隐式 Euler 积分: v += (F/m)·dt; x += v·dt
//! 2. 四元数更新: q' = q + 0.5·ω·q·dt (然后归一化), ω 为世界坐标系角速度
//! 3. 脉冲法碰撞响应 (impulse-based):
//!    - 法向脉冲 j_n = -(1+e)·v_rel·n / (1/m_a + 1/m_b + (r_a × n)·I_a⁻¹·(r_a × n) + (r_b × n)·I_b⁻¹·(r_b × n))
//!    - 摩擦脉冲 (Coulomb 模型): j_t = -v_rel·t / denom, 限制在 |j_t| ≤ μ·j_n
//! 4. 位置修正 (Baumgarte stabilization): 防止穿透累积
//!
//! 约定: Contact.normal 从 A 指向 B (即把 B 推开的方向), A 沿 -normal 方向被推开

use glam::{Vec3, Quat, Mat3};

const PERCENT: f32 = 0.2;  // 位置修正比例 (Baumgarte)
const SLOP: f32 = 0.01;    // 穿透容差 (避免抖动)

// ============================================================
// 接触信息
// ============================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct Contact {
    /// 世界坐标系下的接触点
    pub point: Vec3,
    /// 接触法线 (从 A 指向 B, 即 B 应被推开的方向)
    pub normal: Vec3,
    /// 穿透深度 (>0 表示相交)
    pub penetration: f32,
}

// ============================================================
// 刚体
// ============================================================

#[derive(Debug, Clone)]
pub struct RigidBody {
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    /// 局部坐标系下的逆惯性张量 (常数, 不随旋转改变)
    pub inv_inertia_local: Mat3,
    pub force_accum: Vec3,
    pub torque_accum: Vec3,
    pub restitution: f32,
    pub friction: f32,
    pub is_static: bool,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl RigidBody {
    pub fn new(mass: f32, inv_inertia: Mat3) -> Self {
        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass,
            inv_mass,
            inv_inertia_local: inv_inertia,
            force_accum: Vec3::ZERO,
            torque_accum: Vec3::ZERO,
            restitution: 0.3,
            friction: 0.5,
            is_static: false,
            linear_damping: 0.01,
            angular_damping: 0.01,
        }
    }

    pub fn new_static() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass: 0.0,
            inv_mass: 0.0,
            inv_inertia_local: Mat3::ZERO,
            force_accum: Vec3::ZERO,
            torque_accum: Vec3::ZERO,
            restitution: 0.3,
            friction: 0.5,
            is_static: true,
            linear_damping: 0.0,
            angular_damping: 0.0,
        }
    }

    /// 球体刚体 (inertia = (2/5)·m·r²)
    pub fn sphere(mass: f32, radius: f32) -> Self {
        let inertia = 0.4 * mass * radius * radius;
        let inv_inertia = Mat3::from_diagonal(Vec3::splat(1.0 / inertia));
        Self::new(mass, inv_inertia)
    }

    /// 盒体刚体
    /// Ixx = (1/3)·m·(hy² + hz²), Iyy = (1/3)·m·(hx² + hz²), Izz = (1/3)·m·(hx² + hy²)
    pub fn box_body(mass: f32, half_extents: Vec3) -> Self {
        let hx = half_extents.x;
        let hy = half_extents.y;
        let hz = half_extents.z;
        let ixx = (1.0 / 3.0) * mass * (hy * hy + hz * hz);
        let iyy = (1.0 / 3.0) * mass * (hx * hx + hz * hz);
        let izz = (1.0 / 3.0) * mass * (hx * hx + hy * hy);
        let inv_inertia = Mat3::from_diagonal(Vec3::new(1.0 / ixx, 1.0 / iyy, 1.0 / izz));
        Self::new(mass, inv_inertia)
    }

    /// 世界坐标系下的逆惯性张量: R·I_local⁻¹·Rᵀ
    #[inline]
    pub fn world_inv_inertia(&self) -> Mat3 {
        if self.is_static || self.inv_mass == 0.0 {
            return Mat3::ZERO;
        }
        let r = Mat3::from_quat(self.rotation);
        r * self.inv_inertia_local * r.transpose()
    }

    pub fn apply_force(&mut self, force: Vec3) {
        if self.is_static { return; }
        self.force_accum += force;
    }

    pub fn apply_force_at_point(&mut self, force: Vec3, point: Vec3) {
        if self.is_static { return; }
        self.force_accum += force;
        let torque = (point - self.position).cross(force);
        self.torque_accum += torque;
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.is_static { return; }
        self.linear_velocity += impulse * self.inv_mass;
    }

    pub fn apply_impulse_at_point(&mut self, impulse: Vec3, point: Vec3) {
        if self.is_static { return; }
        self.linear_velocity += impulse * self.inv_mass;
        let r = point - self.position;
        let angular_impulse = r.cross(impulse);
        // ω += I⁻¹·(r × J)
        let inv_i = self.world_inv_inertia();
        self.angular_velocity += inv_i * angular_impulse;
    }

    /// 接触点处的线速度 (含角速度贡献): v = v_linear + ω × r
    #[inline]
    pub fn velocity_at_point(&self, point: Vec3) -> Vec3 {
        let r = point - self.position;
        self.linear_velocity + self.angular_velocity.cross(r)
    }

    /// 半隐式 Euler 积分 (一阶精度, 但比显式 Euler 稳定)
    pub fn integrate(&mut self, dt: f32) {
        if self.is_static || self.inv_mass == 0.0 {
            return;
        }
        // 线性运动
        let acceleration = self.force_accum * self.inv_mass;
        self.linear_velocity += acceleration * dt;
        // 阻尼 (按时间指数衰减)
        let linear_decay = (1.0 - self.linear_damping * dt).max(0.0);
        self.linear_velocity *= linear_decay;
        self.position += self.linear_velocity * dt;

        // 角运动
        let inv_i = self.world_inv_inertia();
        let ang_acc = inv_i * self.torque_accum;
        self.angular_velocity += ang_acc * dt;
        let angular_decay = (1.0 - self.angular_damping * dt).max(0.0);
        self.angular_velocity *= angular_decay;

        // 四元数更新: q' = q + 0.5·ω·q·dt (然后归一化避免数值漂移)
        let omega_q = Quat::from_xyzw(
            self.angular_velocity.x,
            self.angular_velocity.y,
            self.angular_velocity.z,
            0.0,
        );
        let dq = omega_q * self.rotation;
        let new_q = Quat::from_xyzw(
            self.rotation.x + 0.5 * dq.x * dt,
            self.rotation.y + 0.5 * dq.y * dt,
            self.rotation.z + 0.5 * dq.z * dt,
            self.rotation.w + 0.5 * dq.w * dt,
        );
        self.rotation = new_q.normalize();

        // 清空力/力矩累积器
        self.force_accum = Vec3::ZERO;
        self.torque_accum = Vec3::ZERO;
    }
}

// ============================================================
// 碰撞响应
// ============================================================

/// 接触点处 B 相对 A 的速度
#[inline]
fn relative_velocity(a: &RigidBody, b: &RigidBody, contact: &Contact) -> Vec3 {
    let va = a.velocity_at_point(contact.point);
    let vb = b.velocity_at_point(contact.point);
    vb - va
}

/// 法向有效逆质量: 1/m + (r × n)·I⁻¹·(r × n)
#[inline]
fn effective_mass_normal(body: &RigidBody, r: Vec3, n: Vec3) -> f32 {
    if body.is_static { return 0.0; }
    let r_cross_n = r.cross(n);
    let inv_i = body.world_inv_inertia();
    body.inv_mass + r_cross_n.dot(inv_i * r_cross_n)
}

/// 切向有效逆质量
#[inline]
fn effective_mass_tangent(body: &RigidBody, r: Vec3, t: Vec3) -> f32 {
    if body.is_static { return 0.0; }
    let r_cross_t = r.cross(t);
    let inv_i = body.world_inv_inertia();
    body.inv_mass + r_cross_t.dot(inv_i * r_cross_t)
}

/// 脉冲法碰撞响应 (含 Coulomb 摩擦)
///
/// 输入: A, B 两个刚体 (至少一个非静态), 接触信息
/// 输出: 修改 A, B 的线速度和角速度
pub fn resolve_contact(a: &mut RigidBody, b: &mut RigidBody, contact: &Contact) {
    if a.is_static && b.is_static { return; }

    let r_a = contact.point - a.position;
    let r_b = contact.point - b.position;

    let v_rel = relative_velocity(a, b, contact);
    let vn = v_rel.dot(contact.normal);

    // vn > 0 表示分离中, 无需施加脉冲
    if vn > 0.0 { return; }

    // 法向有效质量
    let inv_mass_sum = effective_mass_normal(a, r_a, contact.normal)
                     + effective_mass_normal(b, r_b, contact.normal);
    if inv_mass_sum < 1e-12 { return; }

    // 法向脉冲大小 j_n (>=0)
    // 用 max 混合 restitution (Box2D 约定): 弹性大的材料主导, 对静态体更直观
    let e = a.restitution.max(b.restitution);
    let j_n = -(1.0 + e) * vn / inv_mass_sum;
    if j_n <= 0.0 { return; }

    // 法向脉冲向量 (沿 +normal 方向, 作用在 B 上; -normal 作用在 A 上)
    let impulse_n = contact.normal * j_n;
    a.apply_impulse_at_point(-impulse_n, contact.point);
    b.apply_impulse_at_point(impulse_n, contact.point);

    // ---- 摩擦脉冲 (Coulomb 模型) ----
    // 重新计算相对速度 (法向脉冲已应用)
    let v_rel_new = relative_velocity(a, b, contact);
    let v_tangent = v_rel_new - contact.normal * v_rel_new.dot(contact.normal);
    let t_len = v_tangent.length();
    if t_len < 1e-6 { return; }
    let tangent = v_tangent / t_len;

    let inv_mass_t = effective_mass_tangent(a, r_a, tangent)
                   + effective_mass_tangent(b, r_b, tangent);
    if inv_mass_t < 1e-12 { return; }

    // 切向脉冲大小
    let j_t = -v_rel_new.dot(tangent) / inv_mass_t;
    // Coulomb 锥: |j_t| ≤ μ·j_n
    let mu = (a.friction * b.friction).sqrt();
    let max_friction = j_n * mu;
    let clamped_j_t = if j_t > max_friction {
        max_friction
    } else if j_t < -max_friction {
        -max_friction
    } else {
        j_t
    };

    let friction_impulse = tangent * clamped_j_t;
    a.apply_impulse_at_point(-friction_impulse, contact.point);
    b.apply_impulse_at_point(friction_impulse, contact.point);
}

/// 位置修正 (Baumgarte stabilization)
///
/// 直接调整位置以解决穿透, 防止穿透累积导致物体"陷入"彼此
pub fn position_correction(a: &mut RigidBody, b: &mut RigidBody, contact: &Contact) {
    let inv_mass_sum = a.inv_mass + b.inv_mass;
    if inv_mass_sum < 1e-12 { return; }
    let correction_mag = (contact.penetration - SLOP).max(0.0) / inv_mass_sum * PERCENT;
    let correction = contact.normal * correction_mag;
    // normal 从 A 指向 B: B 沿 +normal 移开, A 沿 -normal 移开
    if !a.is_static {
        a.position -= correction * a.inv_mass;
    }
    if !b.is_static {
        b.position += correction * b.inv_mass;
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_inertia() {
        // I = (2/5)·m·r² = (2/5)·1·4 = 1.6, inv = 0.625
        let body = RigidBody::sphere(1.0, 2.0);
        let inv_i = body.inv_inertia_local;
        assert!((inv_i.x_axis.x - 0.625).abs() < 1e-4, "sphere inv inertia xx: {}", inv_i.x_axis.x);
        assert!((inv_i.y_axis.y - 0.625).abs() < 1e-4, "sphere inv inertia yy: {}", inv_i.y_axis.y);
        assert!((inv_i.z_axis.z - 0.625).abs() < 1e-4, "sphere inv inertia zz: {}", inv_i.z_axis.z);
    }

    #[test]
    fn test_box_inertia() {
        // Ixx = (1/3)·1·(1+1) = 0.6667, inv = 1.5
        let body = RigidBody::box_body(1.0, Vec3::new(1.0, 1.0, 1.0));
        let inv_i = body.inv_inertia_local;
        assert!((inv_i.x_axis.x - 1.5).abs() < 1e-4, "box inv inertia xx: {}", inv_i.x_axis.x);
        assert!((inv_i.y_axis.y - 1.5).abs() < 1e-4, "box inv inertia yy: {}", inv_i.y_axis.y);
    }

    #[test]
    fn test_static_body_immovable() {
        let mut body = RigidBody::new_static();
        assert!(body.is_static);
        assert_eq!(body.inv_mass, 0.0);
        body.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(body.linear_velocity, Vec3::ZERO, "static body should not move");
        body.apply_force(Vec3::new(0.0, 10.0, 0.0));
        body.integrate(1.0);
        assert_eq!(body.linear_velocity, Vec3::ZERO);
        assert_eq!(body.position, Vec3::ZERO);
    }

    #[test]
    fn test_gravity_integration() {
        let mut body = RigidBody::sphere(1.0, 1.0);
        body.apply_force(Vec3::new(0.0, -9.8, 0.0));
        body.integrate(1.0);
        // v = -9.8·1·(1-0.01·1) ≈ -9.702
        assert!(body.linear_velocity.y > -10.0 && body.linear_velocity.y < -9.0,
            "velocity y: {}", body.linear_velocity.y);
        // position = v·dt ≈ -9.7
        assert!(body.position.y < -5.0 && body.position.y > -10.5,
            "position y: {}", body.position.y);
    }

    #[test]
    fn test_impulse_changes_velocity() {
        let mut body = RigidBody::sphere(1.0, 1.0);
        body.apply_impulse(Vec3::new(5.0, 0.0, 0.0));
        assert!((body.linear_velocity.x - 5.0).abs() < 1e-4);
        assert_eq!(body.angular_velocity, Vec3::ZERO, "central impulse should not spin");
    }

    #[test]
    fn test_torque_from_offcenter_force() {
        let mut body = RigidBody::box_body(1.0, Vec3::new(1.0, 1.0, 1.0));
        body.apply_force_at_point(Vec3::new(0.0, 0.0, 10.0), Vec3::new(1.0, 0.0, 0.0));
        // torque = r × F = (1,0,0) × (0,0,10) = (0·10-0·0, 0·0-1·10, 1·0-0·0) = (0, -10, 0)
        assert!((body.torque_accum.y + 10.0).abs() < 1e-4, "torque y: {}", body.torque_accum.y);
    }

    #[test]
    fn test_head_on_elastic_collision() {
        // 两个等质量球, 相向运动, 完全弹性碰撞 -> 速度交换
        let mut a = RigidBody::sphere(1.0, 1.0);
        a.position = Vec3::new(-1.5, 0.0, 0.0);
        a.linear_velocity = Vec3::new(2.0, 0.0, 0.0);
        a.restitution = 1.0;

        let mut b = RigidBody::sphere(1.0, 1.0);
        b.position = Vec3::new(1.5, 0.0, 0.0);
        b.linear_velocity = Vec3::new(-2.0, 0.0, 0.0);
        b.restitution = 1.0;

        let contact = Contact {
            point: Vec3::ZERO,
            normal: Vec3::new(1.0, 0.0, 0.0), // A → B = +x
            penetration: 0.5,
        };
        resolve_contact(&mut a, &mut b, &contact);

        // 等质量完全弹性碰撞: 速度交换 (A: 2→-2, B: -2→2)
        assert!((a.linear_velocity.x + 2.0).abs() < 0.1, "a velocity x: {}", a.linear_velocity.x);
        assert!((b.linear_velocity.x - 2.0).abs() < 0.1, "b velocity x: {}", b.linear_velocity.x);
    }

    #[test]
    fn test_collision_with_static_ground() {
        // 静态地面 (A) 在下, 球 (B) 在上, normal 从地面指向球 = +y
        let mut ground = RigidBody::new_static();
        ground.position = Vec3::new(0.0, -1.0, 0.0);

        let mut ball = RigidBody::sphere(1.0, 1.0);
        ball.position = Vec3::new(0.0, 0.5, 0.0);
        ball.linear_velocity = Vec3::new(0.0, -5.0, 0.0);
        ball.restitution = 1.0;

        let contact = Contact {
            point: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0), // 地面 → 球 = 向上
            penetration: 0.5,
        };
        resolve_contact(&mut ground, &mut ball, &contact);

        // 球完全反弹 (vy 从 -5 → +5)
        assert!((ball.linear_velocity.y - 5.0).abs() < 0.1, "bounce velocity y: {}", ball.linear_velocity.y);
        // 地面不动
        assert_eq!(ground.linear_velocity, Vec3::ZERO);
    }

    #[test]
    fn test_inelastic_collision() {
        // 完全非弹性碰撞 (e=0), 等质量相向 -> 两球停止
        let mut a = RigidBody::sphere(1.0, 1.0);
        a.linear_velocity = Vec3::new(1.0, 0.0, 0.0);
        a.restitution = 0.0;

        let mut b = RigidBody::sphere(1.0, 1.0);
        b.linear_velocity = Vec3::new(-1.0, 0.0, 0.0);
        b.restitution = 0.0;

        let contact = Contact {
            point: Vec3::ZERO,
            normal: Vec3::new(1.0, 0.0, 0.0),
            penetration: 0.1,
        };
        resolve_contact(&mut a, &mut b, &contact);

        // 两球速度都接近 0
        assert!(a.linear_velocity.x.abs() < 0.1, "a v x: {}", a.linear_velocity.x);
        assert!(b.linear_velocity.x.abs() < 0.1, "b v x: {}", b.linear_velocity.x);
    }

    #[test]
    fn test_position_correction() {
        let mut a = RigidBody::sphere(1.0, 1.0);
        a.position = Vec3::new(-0.5, 0.0, 0.0);

        let mut b = RigidBody::sphere(1.0, 1.0);
        b.position = Vec3::new(0.5, 0.0, 0.0);

        let contact = Contact {
            point: Vec3::ZERO,
            normal: Vec3::new(1.0, 0.0, 0.0), // A → B = +x
            penetration: 1.0,
        };
        position_correction(&mut a, &mut b, &contact);

        // 等质量, A 沿 -x, B 沿 +x
        // correction_mag = (1.0 - 0.01) / 2 · 0.2 ≈ 0.099
        assert!(a.position.x < -0.5, "a should move -x: {}", a.position.x);
        assert!(b.position.x > 0.5, "b should move +x: {}", b.position.x);
    }

    #[test]
    fn test_friction_deceleration() {
        // 静态地面 (A), 滑块 (B) 在地面上滑动, 摩擦应使速度减小
        // 注意: 摩擦脉冲 = μ·j_n, 需要 j_n > 0, 即需要法向相对速度 vn < 0
        // 真实场景中重力每帧产生向下速度, 接触响应消耗它并产生 j_n
        let mut ground = RigidBody::new_static();
        ground.position = Vec3::new(0.0, -1.0, 0.0);
        ground.friction = 0.5;
        ground.restitution = 0.0;  // 避免反弹干扰

        let mut block = RigidBody::box_body(1.0, Vec3::new(1.0, 1.0, 1.0));
        block.position = Vec3::new(0.0, 1.0, 0.0);
        block.linear_velocity = Vec3::new(10.0, 0.0, 0.0);
        block.friction = 0.5;
        block.restitution = 0.0;

        let contact = Contact {
            point: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0), // 地面 → 滑块 = 向上
            penetration: 0.1,
        };

        let v_before = block.linear_velocity.x;

        // 多步: 重力 → 积分 → 接触响应 (模拟真实物理循环)
        for _ in 0..5 {
            block.apply_force(Vec3::new(0.0, -9.8, 0.0));
            block.integrate(0.1);
            resolve_contact(&mut ground, &mut block, &contact);
        }

        assert!(block.linear_velocity.x < v_before,
            "friction should slow down: {} vs {}", block.linear_velocity.x, v_before);
    }

    #[test]
    fn test_angular_impulse() {
        // 球体, 给一个不在质心的脉冲, 应产生角速度
        let mut body = RigidBody::sphere(1.0, 2.0); // inv_inertia = 0.625·I
        body.apply_impulse_at_point(Vec3::new(0.0, 0.0, 10.0), Vec3::new(1.0, 0.0, 0.0));
        // r × J = (1,0,0) × (0,0,10) = (0, -10, 0)
        // ω = I⁻¹·(r × J) = 0.625·(0, -10, 0) = (0, -6.25, 0)
        assert!((body.angular_velocity.y + 6.25).abs() < 0.1,
            "angular vel y: {}", body.angular_velocity.y);
    }

    #[test]
    fn test_rotation_integration() {
        // 给定初始角速度, 积分后四元数应改变
        let mut body = RigidBody::sphere(1.0, 1.0);
        body.angular_velocity = Vec3::new(0.0, 1.0, 0.0); // 绕 Y 轴
        let q_before = body.rotation;
        body.integrate(0.1);
        let q_after = body.rotation;
        // 四元数的 y 分量应有变化 (绕 Y 旋转)
        assert!((q_after.y - q_before.y).abs() > 1e-4,
            "rotation should change: q_before={:?} q_after={:?}", q_before, q_after);
        // 四元数应保持单位长度
        assert!((q_after.length() - 1.0).abs() < 1e-4, "quat should be normalized");
    }

    #[test]
    fn test_velocity_at_point() {
        let mut body = RigidBody::sphere(1.0, 1.0);
        body.linear_velocity = Vec3::new(1.0, 0.0, 0.0);
        body.angular_velocity = Vec3::new(0.0, 1.0, 0.0); // 绕 Y
        // 在 (0,0,1) 点: v = v_linear + ω × r = (1,0,0) + (0,1,0)×(0,0,1)
        // (0,1,0) × (0,0,1) = (1·1-0·0, 0·0-0·1, 0·0-1·0) = (1, 0, 0)
        // v = (1,0,0) + (1,0,0) = (2,0,0)
        let v = body.velocity_at_point(Vec3::new(0.0, 0.0, 1.0));
        assert!((v.x - 2.0).abs() < 1e-4, "v at point: {:?}", v);
    }

    #[test]
    fn test_world_inertia_rotation() {
        // 盒体 (2,1,1): Ixx=0.667 (inv=1.5), Iyy=Izz=1.667 (inv=0.6)
        let mut body = RigidBody::box_body(1.0, Vec3::new(2.0, 1.0, 1.0));
        body.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let wi = body.world_inv_inertia();
        // 旋转 90° 绕 Y: 局部 X 轴 → 世界 -Z (或 +Z 取决于约定)
        // 期望: 世界 xx ≈ 0.6 (原 z), 世界 zz ≈ 1.5 (原 x)
        assert!((wi.x_axis.x - 0.6).abs() < 0.05, "world inv xx: {}", wi.x_axis.x);
        assert!((wi.z_axis.z - 1.5).abs() < 0.05, "world inv zz: {}", wi.z_axis.z);
    }

    #[test]
    fn test_static_body_no_response() {
        let mut a = RigidBody::new_static();
        let mut b = RigidBody::new_static();
        let contact = Contact {
            point: Vec3::ZERO,
            normal: Vec3::new(1.0, 0.0, 0.0),
            penetration: 1.0,
        };
        resolve_contact(&mut a, &mut b, &contact);
        assert_eq!(a.linear_velocity, Vec3::ZERO);
        assert_eq!(b.linear_velocity, Vec3::ZERO);
        assert_eq!(a.angular_velocity, Vec3::ZERO);
        assert_eq!(b.angular_velocity, Vec3::ZERO);
    }

    #[test]
    fn test_separating_contact_ignored() {
        // 两球正在分离 (vn > 0), 不应施加脉冲
        let mut a = RigidBody::sphere(1.0, 1.0);
        a.linear_velocity = Vec3::new(-1.0, 0.0, 0.0); // 向左 (远离 B)

        let mut b = RigidBody::sphere(1.0, 1.0);
        b.linear_velocity = Vec3::new(1.0, 0.0, 0.0); // 向右 (远离 A)

        let v_a_before = a.linear_velocity;
        let v_b_before = b.linear_velocity;

        let contact = Contact {
            point: Vec3::ZERO,
            normal: Vec3::new(1.0, 0.0, 0.0), // A → B
            penetration: 0.1,
        };
        resolve_contact(&mut a, &mut b, &contact);

        // v_rel = v_b - v_a = (1,0,0) - (-1,0,0) = (2,0,0), vn = 2 > 0 -> 分离中
        assert_eq!(a.linear_velocity, v_a_before, "separating contact should not affect a");
        assert_eq!(b.linear_velocity, v_b_before, "separating contact should not affect b");
    }
}
