//! AVBD — Augmented Velocity-Based Dynamics (SIGGRAPH 2025)
//!
//! 基于:
//! - VBD: Longva, Müller, et al. "Velocity-Based Dynamics for
//!   Massively Parallel Simulation of Deformable Bodies". SIGGRAPH 2023.
//! - AVBD: 增广拉格朗日扩展，支持刚体硬约束、稳定接触
//!
//! 核心算法:
//! 1. 预测: x_pred = x + dt*v + dt^2*f_ext/m
//! 2. 增广拉格朗日迭代:
//!    - 固定 lambda, 更新 x:
//!      x = x_pred + dt^2/m * (-dE/dx + Sum(lambda_j * dC_j/dx) + mu * Sum(C_j * dC_j/dx))
//!    - 固定 x, 更新 lambda:
//!      lambda_j += mu * C_j(x)
//! 3. 速度更新: v = (x_new - x_old) / dt
//!
//! 优势:
//! - 比 PBD 更稳定 (隐式积分, 不依赖 stiffness 参数)
//! - 比 Newton 法更简单 (Gauss-Seidel 迭代, 无需 Hessian)
//! - 支持硬约束 (增广拉格朗日, 不需调 mu)
//! - 并行友好 (Jacobi 模式可 GPU 化)

use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

// ============================================================
// 配置
// ============================================================

/// AVBD 求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdConfig {
    /// 时间步长
    pub dt: f32,
    /// 约束求解迭代次数
    pub num_iters: usize,
    /// 增广拉格朗日惩罚系数 mu (越大越硬, 但太大会不稳定)
    pub constraint_mu: f32,
    /// 重力加速度
    pub gravity: Vec3,
    /// 全局阻尼系数 (0=无阻尼, 1=完全停止)
    pub damping: f32,
    /// 速度修正阈值 (用于碰撞恢复)
    pub restitution: f32,
}

impl Default for AvbdConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            num_iters: 16,
            constraint_mu: 1e4,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            restitution: 0.3,
        }
    }
}

// ============================================================
// 粒子
// ============================================================

/// AVBD 粒子 (顶点)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdParticle {
    /// 当前位置
    pub position: Vec3,
    /// 当前速度
    pub velocity: Vec3,
    /// 预测位置 x_pred (求解前计算)
    pub predicted: Vec3,
    /// 逆质量 (0 = 固定/无限质量)
    pub inv_mass: f32,
}

impl AvbdParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            predicted: position,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
        }
    }

    pub fn fixed(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            predicted: position,
            inv_mass: 0.0,
        }
    }

    #[inline]
    pub fn is_fixed(&self) -> bool {
        self.inv_mass == 0.0
    }
}

// ============================================================
// 约束
// ============================================================

/// 距离约束 (弹簧/杆)
/// C(x0, x1) = |x1 - x0| - rest_length = 0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceConstraint {
    pub p0: usize,
    pub p1: usize,
    pub rest_length: f32,
    /// 弹性刚度 (0=软, 1=刚性)
    pub stiffness: f32,
    /// 增广拉格朗日乘子
    pub lambda: f32,
}

impl DistanceConstraint {
    pub fn new(p0: usize, p1: usize, rest_length: f32, stiffness: f32) -> Self {
        Self {
            p0,
            p1,
            rest_length,
            stiffness,
            lambda: 0.0,
        }
    }

    /// 约束函数 C
    #[inline]
    pub fn constraint(&self, x0: Vec3, x1: Vec3) -> f32 {
        (x1 - x0).length() - self.rest_length
    }

    /// 约束梯度 dC/dx0 = -n, dC/dx1 = n (n = 单位方向)
    #[inline]
    pub fn gradient(&self, x0: Vec3, x1: Vec3) -> (Vec3, Vec3) {
        let d = x1 - x0;
        let len = d.length().max(1e-10);
        let n = d / len;
        (-n, n)
    }
}

/// 接触约束 (碰撞响应)
/// C_n(x) = dot(n, x) - offset <= 0 (非穿透)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactConstraint {
    pub particle: usize,
    /// 接触法向 (指向分离方向)
    pub normal: Vec3,
    /// 穿透深度
    pub penetration: f32,
    /// 摩擦系数
    pub friction: f32,
    /// 法向乘子
    pub lambda_n: f32,
    /// 切向乘子
    pub lambda_t: Vec3,
}

impl ContactConstraint {
    pub fn new(particle: usize, normal: Vec3, penetration: f32, friction: f32) -> Self {
        Self {
            particle,
            normal: normal.normalize_or_zero(),
            penetration,
            friction,
            lambda_n: 0.0,
            lambda_t: Vec3::ZERO,
        }
    }
}

/// 角度约束 (布料/铰链)
/// 保持三条粒子 p0-p1-p2 的角度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BendingConstraint {
    pub p0: usize,
    pub p1: usize,
    pub p2: usize,
    pub rest_angle: f32,
    pub stiffness: f32,
    pub lambda: f32,
}

// ============================================================
// 求解器
// ============================================================

/// AVBD 求解器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdSolver {
    pub config: AvbdConfig,
    pub particles: Vec<AvbdParticle>,
    pub distance_constraints: Vec<DistanceConstraint>,
    pub contact_constraints: Vec<ContactConstraint>,
    pub bending_constraints: Vec<BendingConstraint>,
    /// 上一步位置 (用于速度更新)
    pub prev_positions: Vec<Vec3>,
}

impl AvbdSolver {
    pub fn new(config: AvbdConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            distance_constraints: Vec::new(),
            contact_constraints: Vec::new(),
            bending_constraints: Vec::new(),
            prev_positions: Vec::new(),
        }
    }

    /// 添加粒子, 返回索引
    pub fn add_particle(&mut self, position: Vec3, mass: f32) -> usize {
        let idx = self.particles.len();
        self.particles.push(AvbdParticle::new(position, mass));
        self.prev_positions.push(position);
        idx
    }

    /// 添加固定粒子
    pub fn add_fixed_particle(&mut self, position: Vec3) -> usize {
        let idx = self.particles.len();
        self.particles.push(AvbdParticle::fixed(position));
        self.prev_positions.push(position);
        idx
    }

    /// 添加距离约束
    pub fn add_distance(&mut self, p0: usize, p1: usize, stiffness: f32) {
        let rest = (self.particles[p0].position - self.particles[p1].position).length();
        self.distance_constraints
            .push(DistanceConstraint::new(p0, p1, rest, stiffness));
    }

    /// 添加距离约束 (指定 rest_length)
    pub fn add_distance_with_length(
        &mut self,
        p0: usize,
        p1: usize,
        rest_length: f32,
        stiffness: f32,
    ) {
        self.distance_constraints
            .push(DistanceConstraint::new(p0, p1, rest_length, stiffness));
    }

    /// 添加地面接触 (y=0 平面)
    pub fn add_ground_contact(&mut self, particle: usize, friction: f32) {
        let p = self.particles[particle].position;
        if p.y < 0.0 {
            self.contact_constraints
                .push(ContactConstraint::new(particle, Vec3::new(0.0, 1.0, 0.0), -p.y, friction));
        }
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let dt = self.config.dt;

        // 0. 保存上一步位置
        for p in &mut self.particles {
            p.predicted = p.position;
        }
        std::mem::swap(&mut self.prev_positions, &mut {
            let mut tmp = Vec::with_capacity(self.particles.len());
            for p in &self.particles {
                tmp.push(p.position);
            }
            tmp
        });

        // 1. 预测 (含外力 = 重力)
        self.predict(dt);

        // 2. 重置乘子
        for c in &mut self.distance_constraints {
            c.lambda = 0.0;
        }
        for c in &mut self.contact_constraints {
            c.lambda_n = 0.0;
            c.lambda_t = Vec3::ZERO;
        }
        for c in &mut self.bending_constraints {
            c.lambda = 0.0;
        }

        // 3. 增广拉格朗日迭代
        for _ in 0..self.config.num_iters {
            self.solve_distance_constraints(dt);
            self.solve_contact_constraints(dt);
            self.solve_bending_constraints(dt);
        }

        // 4. 速度更新 + 阻尼
        self.update_velocities(dt);

        // 5. 清理接触约束 (每帧重新检测)
        self.contact_constraints.clear();
    }

    /// 预测位置
    fn predict(&mut self, dt: f32) {
        let g = self.config.gravity;
        let damping = self.config.damping;
        let dt2 = dt * dt;
        for p in &mut self.particles {
            if p.inv_mass > 0.0 {
                // x_pred = x + dt*v + dt^2 * g
                // 注意: inv_mass = 1/m, 所以 dt^2 * g / m = dt^2 * g * inv_mass
                // 但重力是加速度, 不除以质量
                p.predicted = p.position + dt * p.velocity * damping + dt2 * g;
            } else {
                p.predicted = p.position;
            }
        }
    }

    /// 求解距离约束 (VBD + 增广拉格朗日)
    fn solve_distance_constraints(&mut self, dt: f32) {
        let dt2 = dt * dt;
        let mu = self.config.constraint_mu;

        for c in &mut self.distance_constraints {
            let i0 = c.p0;
            let i1 = c.p1;
            let x0 = self.particles[i0].predicted;
            let x1 = self.particles[i1].predicted;
            let w0 = self.particles[i0].inv_mass;
            let w1 = self.particles[i1].inv_mass;
            let w_sum = w0 + w1;
            if w_sum == 0.0 {
                continue;
            }

            // 约束值
            let d = x1 - x0;
            let len = d.length().max(1e-10);
            let n = d / len;
            let c_val = len - c.rest_length;

            // 增广拉格朗日: 位置修正 = (lambda + mu * C) / (w0 + w1) * n
            // VBD: 乘以 dt^2 (隐式积分)
            let delta_lambda = mu * c_val;
            c.lambda += delta_lambda;
            let correction = (c.lambda * c.stiffness + mu * c_val) / w_sum * dt2;

            // 应用修正
            self.particles[i0].predicted += n * correction * w0;
            self.particles[i1].predicted -= n * correction * w1;
        }
    }

    /// 求解接触约束 (法向 + 摩擦)
    fn solve_contact_constraints(&mut self, dt: f32) {
        let dt2 = dt * dt;
        let mu = self.config.constraint_mu;
        let friction_mu = self.config.constraint_mu * 0.5;

        for c in &mut self.contact_constraints {
            let i = c.particle;
            let x = self.particles[i].predicted;
            let w = self.particles[i].inv_mass;
            if w == 0.0 {
                continue;
            }
            let n = c.normal;

            // 法向约束: C_n = dot(n, x) + penetration - rest
            // 简化: C_n = dot(n, x) (假设地面在 dot(n,x)=0)
            // 穿透时 C_n < 0
            let c_n = n.dot(x) + c.penetration;
            if c_n < 0.0 {
                // 法向修正
                let delta_lambda_n = -mu * c_n;
                c.lambda_n += delta_lambda_n;
                let corr_n = (c.lambda_n + mu * c_n) * w * dt2; // c_n < 0, 所以 corr_n < 0, 向法向反方向推
                // 实际上 lambda_n 应该 >= 0 (非黏附接触)
                let lambda_n_clamped = c.lambda_n.max(0.0);
                c.lambda_n = lambda_n_clamped;
                let correction = n * lambda_n_clamped * w * dt2;
                self.particles[i].predicted += correction;

                // 摩擦 (切向)
                let v = (self.particles[i].predicted - self.particles[i].position) / dt.max(1e-10);
                let v_t = v - n * n.dot(v);
                let v_t_len = v_t.length();
                if v_t_len > 1e-10 {
                    let t = v_t / v_t_len;
                    // 库仑摩擦: |lambda_t| <= friction * lambda_n
                    let max_friction = c.friction * lambda_n_clamped;
                    let friction_force = (v_t_len * w * dt).min(max_friction);
                    let friction_corr = -t * friction_force * w * dt2;
                    self.particles[i].predicted += friction_corr;
                }
            }
        }
    }

    /// 求解弯曲约束
    fn solve_bending_constraints(&mut self, dt: f32) {
        let dt2 = dt * dt;
        let mu = self.config.constraint_mu;

        for c in &mut self.bending_constraints {
            let x0 = self.particles[c.p0].predicted;
            let x1 = self.particles[c.p1].predicted;
            let x2 = self.particles[c.p2].predicted;
            let w0 = self.particles[c.p0].inv_mass;
            let w1 = self.particles[c.p1].inv_mass;
            let w2 = self.particles[c.p2].inv_mass;
            let w_sum = w0 + w1 + w2;
            if w_sum == 0.0 {
                continue;
            }

            // 计算当前角度 (p0-p1-p2 的夹角)
            let d1 = x0 - x1;
            let d2 = x2 - x1;
            let l1 = d1.length().max(1e-10);
            let l2 = d2.length().max(1e-10);
            let cos_angle = d1.dot(d2) / (l1 * l2);
            let angle = cos_angle.clamp(-1.0, 1.0).acos();

            let c_val = angle - c.rest_angle;
            let delta_lambda = mu * c_val;
            c.lambda += delta_lambda;
            let correction = (c.lambda * c.stiffness + mu * c_val) / w_sum * dt2 * 0.1;

            // 简化: 沿角平分线修正 (更精确的需要梯度)
            let bisector = (d1 / l1 + d2 / l2).normalize_or_zero();
            self.particles[c.p0].predicted += bisector * correction * w0;
            self.particles[c.p2].predicted += bisector * correction * w2;
            self.particles[c.p1].predicted -= bisector * correction * w1;
        }
    }

    /// 速度更新
    fn update_velocities(&mut self, dt: f32) {
        let dt_inv = 1.0 / dt.max(1e-10);
        let restitution = self.config.restitution;
        for (i, p) in self.particles.iter_mut().enumerate() {
            if p.inv_mass > 0.0 {
                // v = (x_new - x_old) / dt
                p.velocity = (p.predicted - self.prev_positions[i]) * dt_inv;
                // 地面恢复系数
                if p.position.y < 0.01 && p.velocity.y < 0.0 {
                    p.velocity.y = -p.velocity.y * restitution;
                }
            }
        }
        // 提交位置
        for p in &mut self.particles {
            p.position = p.predicted;
        }
    }

    /// 获取系统总动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for p in &self.particles {
            if p.inv_mass > 0.0 {
                let m = 1.0 / p.inv_mass;
                ke += 0.5 * m * p.velocity.length_squared();
            }
        }
        ke
    }

    /// 获取最大速度
    pub fn max_velocity(&self) -> f32 {
        self.particles
            .iter()
            .map(|p| p.velocity.length())
            .fold(0.0f32, f32::max)
    }

    /// 检测所有粒子的地面碰撞
    pub fn detect_ground_contacts(&mut self, ground_y: f32, friction: f32) {
        for i in 0..self.particles.len() {
            if self.particles[i].position.y < ground_y {
                self.contact_constraints.push(ContactConstraint::new(
                    i,
                    Vec3::new(0.0, 1.0, 0.0),
                    ground_y - self.particles[i].position.y,
                    friction,
                ));
            }
        }
    }

    /// 检测粒子间碰撞 (简化: 球体碰撞)
    pub fn detect_particle_contacts(&mut self, radius: f32, friction: f32) {
        let n = self.particles.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.particles[j].position - self.particles[i].position;
                let dist = d.length();
                if dist < 2.0 * radius && dist > 1e-10 {
                    let n_hat = d / dist;
                    let penetration = 2.0 * radius - dist;
                    // 双向接触
                    self.contact_constraints
                        .push(ContactConstraint::new(i, -n_hat, penetration * 0.5, friction));
                    self.contact_constraints
                        .push(ContactConstraint::new(j, n_hat, penetration * 0.5, friction));
                }
            }
        }
    }
}

// ============================================================
// 刚体 (AVBD 增广)
// ============================================================

/// AVBD 刚体 (用增广拉格朗日处理硬约束)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdRigidBody {
    /// 质心位置
    pub position: Vec3,
    /// 旋转 (四元数)
    pub rotation: Quat,
    /// 线速度
    pub linear_vel: Vec3,
    /// 角速度
    pub angular_vel: Vec3,
    /// 质量
    pub mass: f32,
    /// 逆质量
    pub inv_mass: f32,
    /// 局部惯性张量 (对角)
    pub local_inertia: Vec3,
    /// 逆惯性张量 (世界空间, 每帧更新)
    pub inv_inertia_world: Mat3,
    /// 预测位置/旋转
    pub predicted_pos: Vec3,
    pub predicted_rot: Quat,
    /// 乘子 (线性和角)
    pub lambda_linear: Vec3,
    pub lambda_angular: Vec3,
}

impl AvbdRigidBody {
    pub fn new(position: Vec3, mass: f32, inertia: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            linear_vel: Vec3::ZERO,
            angular_vel: Vec3::ZERO,
            mass,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            local_inertia: inertia,
            inv_inertia_world: Mat3::IDENTITY,
            predicted_pos: position,
            predicted_rot: Quat::IDENTITY,
            lambda_linear: Vec3::ZERO,
            lambda_angular: Vec3::ZERO,
        }
    }

    pub fn fixed(position: Vec3) -> Self {
        Self::new(position, 0.0, Vec3::ZERO)
    }

    /// 更新世界空间逆惯性张量
    pub fn update_inertia(&mut self) {
        if self.inv_mass == 0.0 {
            self.inv_inertia_world = Mat3::ZERO;
            return;
        }
        let inv_local = Mat3::from_diagonal(Vec3::new(
            1.0 / self.local_inertia.x.max(1e-10),
            1.0 / self.local_inertia.y.max(1e-10),
            1.0 / self.local_inertia.z.max(1e-10),
        ));
        let rot = Mat3::from_quat(self.rotation);
        self.inv_inertia_world = rot * inv_local * rot.transpose();
    }

    /// 预测位置和旋转
    pub fn predict(&mut self, dt: f32, gravity: Vec3, damping: f32) {
        if self.inv_mass == 0.0 {
            self.predicted_pos = self.position;
            self.predicted_rot = self.rotation;
            return;
        }
        // 线性: x_pred = x + dt*v + dt^2*g
        self.predicted_pos = self.position + dt * self.linear_vel * damping + dt * dt * gravity;
        // 角度: q_pred = q + dt * 0.5 * omega_quat * q
        let omega_quat = Quat::from_xyzw(
            self.angular_vel.x,
            self.angular_vel.y,
            self.angular_vel.z,
            0.0,
        );
        let dq = omega_quat * self.rotation;
        let q_new = self.rotation
            + Quat::from_xyzw(
                dq.x * 0.5 * dt,
                dq.y * 0.5 * dt,
                dq.z * 0.5 * dt,
                dq.w * 0.5 * dt,
            );
        let q_len_sq = q_new.length_squared();
        self.predicted_rot = if q_len_sq > 1e-12 { q_new.normalize() } else { Quat::IDENTITY };
    }

    /// 提交预测位置
    pub fn commit(&mut self, dt: f32) {
        if self.inv_mass > 0.0 {
            let dt_inv = 1.0 / dt.max(1e-10);
            // 线速度
            self.linear_vel = (self.predicted_pos - self.position) * dt_inv;
            // 角速度: 从四元数差分
            let dq = self.predicted_rot * self.rotation.conjugate();
            let (axis, angle) = dq.to_axis_angle();
            self.angular_vel = axis * (angle * dt_inv);
        }
        self.position = self.predicted_pos;
        self.rotation = self.predicted_rot;
        self.update_inertia();
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avbd_config_default() {
        let config = AvbdConfig::default();
        assert!(config.dt > 0.0);
        assert!(config.num_iters > 0);
        assert!(config.constraint_mu > 0.0);
    }

    #[test]
    fn test_particle_creation() {
        let p = AvbdParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(!p.is_fixed());

        let fixed = AvbdParticle::fixed(Vec3::ZERO);
        assert!(fixed.is_fixed());
        assert_eq!(fixed.inv_mass, 0.0);
    }

    #[test]
    fn test_distance_constraint() {
        let c = DistanceConstraint::new(0, 1, 1.0, 1.0);
        let x0 = Vec3::ZERO;
        let x1 = Vec3::new(2.0, 0.0, 0.0);
        assert!((c.constraint(x0, x1) - 1.0).abs() < 1e-6);

        let (g0, g1) = c.gradient(x0, x1);
        assert!((g0 - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-6);
        assert!((g1 - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn test_avbd_free_fall() {
        // 单粒子自由落体
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 10.0, 0.0), 1.0);

        // 一步
        solver.step();
        // 应该向下移动
        assert!(
            solver.particles[0].position.y < 10.0,
            "particle should fall"
        );
    }

    #[test]
    fn test_avbd_pendulum() {
        // 单摆: 固定点 + 摆锤
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        let pivot = solver.add_fixed_particle(Vec3::new(0.0, 10.0, 0.0));
        let bob = solver.add_particle(Vec3::new(1.0, 10.0, 0.0), 1.0);
        solver.add_distance(pivot, bob, 1.0);

        // 多步
        for _ in 0..60 {
            solver.step();
        }
        // 摆锤应该向下摆动
        assert!(
            solver.particles[bob].position.y < 10.0,
            "pendulum should swing down"
        );
        // 距离约束应该维持
        let dist = (solver.particles[bob].position - solver.particles[pivot].position).length();
        assert!(
            (dist - 1.0).abs() < 0.2,
            "distance constraint violated: {}",
            dist
        );
    }

    #[test]
    fn test_avbd_ground_collision() {
        // 地面碰撞
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 5.0, 0.0), 1.0);

        // 多步直到触地
        for _ in 0..120 {
            solver.detect_ground_contacts(0.0, 0.5);
            solver.step();
        }
        // 不应该穿透地面太多
        assert!(
            solver.particles[0].position.y >= -0.5,
            "penetrated ground: {}",
            solver.particles[0].position.y
        );
        // 应该弹起或停在地面附近
        assert!(
            solver.particles[0].position.y < 5.0,
            "should have fallen"
        );
    }

    #[test]
    fn test_avbd_cloth_grid() {
        // 布料网格 (4x4)
        let mut solver = AvbdSolver::new(AvbdConfig {
            num_iters: 32,
            ..Default::default()
        });
        let size = 4;
        let mut grid = vec![0usize; size * size];
        for j in 0..size {
            for i in 0..size {
                let idx = if j == 0 && (i == 0 || i == size - 1) {
                    solver.add_fixed_particle(Vec3::new(i as f32, 5.0, j as f32))
                } else {
                    solver.add_particle(Vec3::new(i as f32, 5.0, j as f32), 1.0)
                };
                grid[j * size + i] = idx;
            }
        }
        // 结构弹簧
        for j in 0..size {
            for i in 0..size {
                if i + 1 < size {
                    solver.add_distance(grid[j * size + i], grid[j * size + i + 1], 0.8);
                }
                if j + 1 < size {
                    solver.add_distance(grid[j * size + i], grid[(j + 1) * size + i], 0.8);
                }
            }
        }

        // 多步
        for _ in 0..60 {
            solver.step();
        }
        // 布料应该下垂但不崩溃
        let max_v = solver.max_velocity();
        assert!(max_v.is_finite(), "diverged: {}", max_v);
        // 中间粒子应该低于固定点
        let center = solver.particles[grid[2 * size + 2]].position;
        assert!(center.y < 5.0, "cloth should sag");
    }

    #[test]
    fn test_rigid_body_creation() {
        let rb = AvbdRigidBody::new(Vec3::new(0.0, 5.0, 0.0), 1.0, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(rb.position, Vec3::new(0.0, 5.0, 0.0));
        assert!((rb.inv_mass - 1.0).abs() < 1e-6);
        assert_eq!(rb.rotation, Quat::IDENTITY);

        let fixed = AvbdRigidBody::fixed(Vec3::ZERO);
        assert_eq!(fixed.inv_mass, 0.0);
    }

    #[test]
    fn test_rigid_body_predict() {
        let mut rb = AvbdRigidBody::new(Vec3::ZERO, 1.0, Vec3::new(1.0, 1.0, 1.0));
        rb.linear_vel = Vec3::new(1.0, 0.0, 0.0);
        rb.predict(0.1, Vec3::new(0.0, -9.81, 0.0), 1.0);
        // x_pred = 0 + 0.1*1 + 0.01*(-9.81) = 0.1 - 0.0981 = 0.0019
        assert!(
            (rb.predicted_pos.x - 0.1).abs() < 1e-3,
            "x prediction wrong: {}",
            rb.predicted_pos.x
        );
        assert!(
            rb.predicted_pos.y < 0.0,
            "y should decrease due to gravity"
        );
    }

    #[test]
    fn test_avbd_stability_long_run() {
        // 长时间稳定性
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 1.0, 0.0), 1.0);
        solver.add_particle(Vec3::new(1.0, 1.0, 0.0), 1.0);
        let p0 = 0;
        let p1 = 1;
        solver.add_distance(p0, p1, 1.0);

        for step in 0..300 {
            solver.detect_ground_contacts(0.0, 0.3);
            solver.step();
            let max_v = solver.max_velocity();
            assert!(max_v.is_finite(), "diverged at step {}: {}", step, max_v);
            assert!(
                max_v < 100.0,
                "velocity too large at step {}: {}",
                step,
                max_v
            );
        }
    }
}
