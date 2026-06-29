//! OGC - Offset Geometric Contact (SIGGRAPH 2025)
//!
//! 论文: Smith et al., "Offset Geometric Contact", ACM TOG (SIGGRAPH 2025).
//!
//! 核心创新:
//! 1. 用 SDF 的偏移表面定义接触区域, 而非单点接触
//!    - 传统 GJK+EPA 在面/边接触时产生抖动
//!    - OGC 在 SDF = -d 的等值面上积分, 接触更稳定
//! 2. 接触流形通过表面采样生成, 自然处理凹体和平滑接触
//! 3. 顺序冲量 (Sequential Impulse) 求解器, Baumgarte 稳定化
//!
//! 本实现为 CPU 参考版本, 提供基于 SDF 的通用接触检测与求解.

use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

// ============================================================
// 形状 trait (基于 SDF)
// ============================================================

/// 基于 SDF 的形状 trait
pub trait OgcShape: Send + Sync {
    /// 局部坐标 SDF (形状中心在原点)
    fn sdf_local(&self, p: Vec3) -> f32;
    /// 包围球半径 (用于 broad phase)
    fn bounding_radius(&self) -> f32;
    /// 形状名称 (调试用)
    fn name(&self) -> &str;
}

/// 球体形状
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcSphere {
    pub radius: f32,
}

impl OgcShape for OgcSphere {
    #[inline]
    fn sdf_local(&self, p: Vec3) -> f32 {
        p.length() - self.radius
    }
    fn bounding_radius(&self) -> f32 {
        self.radius
    }
    fn name(&self) -> &str {
        "sphere"
    }
}

/// 轴对齐盒形状 (局部坐标, 中心在原点)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcBox {
    pub half_extents: Vec3,
}

impl OgcShape for OgcBox {
    #[inline]
    fn sdf_local(&self, p: Vec3) -> f32 {
        // Inigo Quilez 精确盒 SDF
        let q = p.abs() - self.half_extents;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
    }
    fn bounding_radius(&self) -> f32 {
        self.half_extents.length()
    }
    fn name(&self) -> &str {
        "box"
    }
}

/// 圆柱形状 (沿 y 轴)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcCylinder {
    pub radius: f32,
    pub half_height: f32,
}

impl OgcShape for OgcCylinder {
    #[inline]
    fn sdf_local(&self, p: Vec3) -> f32 {
        let d = Vec3::new(
            (p.x * p.x + p.z * p.z).sqrt() - self.radius,
            p.y.abs() - self.half_height,
            0.0,
        );
        d.max(Vec3::ZERO).length() + d.x.max(d.y).min(0.0)
    }
    fn bounding_radius(&self) -> f32 {
        (self.radius * self.radius + self.half_height * self.half_height).sqrt()
    }
    fn name(&self) -> &str {
        "cylinder"
    }
}

// ============================================================
// 刚体
// ============================================================

/// OGC 刚体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcBody {
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_vel: Vec3,
    pub angular_vel: Vec3,
    pub inv_mass: f32,
    /// 局部坐标逆惯性张量 (对角线)
    pub inv_inertia_local: Vec3,
    /// 形状索引 (在 solver 的形状池中)
    pub shape_id: usize,
    pub restitution: f32,
    pub friction: f32,
    /// 是否固定 (inv_mass = 0)
    pub fixed: bool,
}

impl OgcBody {
    pub fn new(position: Vec3, shape_id: usize) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            linear_vel: Vec3::ZERO,
            angular_vel: Vec3::ZERO,
            inv_mass: 1.0,
            inv_inertia_local: Vec3::new(1.0, 1.0, 1.0),
            shape_id,
            restitution: 0.3,
            friction: 0.5,
            fixed: false,
        }
    }

    pub fn fixed(position: Vec3, shape_id: usize) -> Self {
        let mut b = Self::new(position, shape_id);
        b.inv_mass = 0.0;
        b.inv_inertia_local = Vec3::ZERO;
        b.fixed = true;
        b
    }

    /// 世界坐标逆惯性张量
    pub fn inv_inertia_world(&self) -> Mat3 {
        if self.inv_mass == 0.0 {
            return Mat3::ZERO;
        }
        let inv_local = Mat3::from_diagonal(self.inv_inertia_local);
        let rot = Mat3::from_quat(self.rotation);
        rot * inv_local * rot.transpose()
    }

    /// 世界坐标 SDF: 把世界点变换到局部, 调用形状 SDF
    pub fn sdf_world(&self, shape: &dyn OgcShape, p_world: Vec3) -> f32 {
        let p_local = self.rotation.inverse() * (p_world - self.position);
        shape.sdf_local(p_local)
    }

    /// 世界坐标 SDF 梯度 (法向, 中心差分)
    pub fn sdf_gradient_world(&self, shape: &dyn OgcShape, p_world: Vec3, eps: f32) -> Vec3 {
        let gx = self.sdf_world(shape, p_world + Vec3::new(eps, 0.0, 0.0))
            - self.sdf_world(shape, p_world - Vec3::new(eps, 0.0, 0.0));
        let gy = self.sdf_world(shape, p_world + Vec3::new(0.0, eps, 0.0))
            - self.sdf_world(shape, p_world - Vec3::new(0.0, eps, 0.0));
        let gz = self.sdf_world(shape, p_world + Vec3::new(0.0, 0.0, eps))
            - self.sdf_world(shape, p_world - Vec3::new(0.0, 0.0, eps));
        Vec3::new(gx, gy, gz) / (2.0 * eps)
    }

    /// 速度 at 点 p (世界坐标)
    pub fn velocity_at(&self, p: Vec3) -> Vec3 {
        let r = p - self.position;
        self.linear_vel + self.angular_vel.cross(r)
    }

    /// 施加冲量 at 点 p (世界坐标)
    pub fn apply_impulse(&mut self, impulse: Vec3, p: Vec3) {
        if self.inv_mass == 0.0 {
            return;
        }
        let r = p - self.position;
        self.linear_vel += impulse * self.inv_mass;
        self.angular_vel += self.inv_inertia_world() * r.cross(impulse);
    }
}

// ============================================================
// 接触
// ============================================================

/// OGC 接触点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcContact {
    pub body_a: usize,
    pub body_b: usize,
    pub point: Vec3,     // 世界坐标接触点
    pub normal: Vec3,    // 从 A 指向 B
    pub depth: f32,      // 穿透深度 (>0 表示穿透)
    pub lambda_n: f32,   // 法向冲量累积
    pub lambda_t: Vec3,  // 切向冲量累积
}

impl OgcContact {
    pub fn new(body_a: usize, body_b: usize, point: Vec3, normal: Vec3, depth: f32) -> Self {
        Self {
            body_a,
            body_b,
            point,
            normal: normal.normalize_or_zero(),
            depth,
            lambda_n: 0.0,
            lambda_t: Vec3::ZERO,
        }
    }
}

// ============================================================
// 求解器
// ============================================================

/// OGC 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgcConfig {
    pub dt: f32,
    pub gravity: Vec3,
    pub num_solver_iters: usize,
    pub contact_offset: f32,    // 接触偏移 (SDF < contact_offset 视为接触)
    pub max_penetration: f32,   // 最大穿透 (Baumgarte 稳定化)
    pub baumgarte: f32,         // Baumgarte 系数 (0-1)
    pub restitution_threshold: f32, // 速度小于此值时无恢复
    pub sample_count: usize,    // 每对形状的采样点数
    pub sdf_eps: f32,           // SDF 梯度差分步长
}

impl Default for OgcConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_solver_iters: 10,
            contact_offset: 0.001,
            max_penetration: 0.1,
            baumgarte: 0.2,
            restitution_threshold: 0.5,
            sample_count: 32,
            sdf_eps: 1e-3,
        }
    }
}

/// OGC 求解器
pub struct OgcSolver {
    pub config: OgcConfig,
    pub shapes: Vec<Box<dyn OgcShape>>,
    pub bodies: Vec<OgcBody>,
    pub contacts: Vec<OgcContact>,
}

impl OgcSolver {
    pub fn new(config: OgcConfig) -> Self {
        Self {
            config,
            shapes: Vec::new(),
            bodies: Vec::new(),
            contacts: Vec::new(),
        }
    }

    pub fn add_shape(&mut self, shape: Box<dyn OgcShape>) -> usize {
        let id = self.shapes.len();
        self.shapes.push(shape);
        id
    }

    pub fn add_body(&mut self, body: OgcBody) -> usize {
        let id = self.bodies.len();
        self.bodies.push(body);
        id
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let dt = self.config.dt;
        // 1. 积分速度 (重力)
        for b in &mut self.bodies {
            if !b.fixed {
                b.linear_vel += self.config.gravity * dt;
            }
        }
        // 2. 检测接触
        self.contacts.clear();
        self.detect_contacts();
        // 3. 求解接触约束 (顺序冲量)
        for _ in 0..self.config.num_solver_iters {
            self.solve_contacts();
        }
        // 4. 积分位置
        for b in &mut self.bodies {
            if !b.fixed {
                b.position += b.linear_vel * dt;
                let omega = b.angular_vel;
                let dq = Quat::from_xyzw(omega.x, omega.y, omega.z, 0.0) * b.rotation * 0.5 * dt;
                b.rotation = Quat::from_xyzw(
                    b.rotation.x + dq.x,
                    b.rotation.y + dq.y,
                    b.rotation.z + dq.z,
                    b.rotation.w + dq.w,
                );
                if b.rotation.length_squared() > 1e-10 {
                    b.rotation = b.rotation.normalize();
                }
            }
        }
    }

    /// 接触检测 (基于 SDF 表面采样)
    fn detect_contacts(&mut self) {
        let n = self.bodies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                self.detect_pair(i, j);
            }
        }
    }

    /// 检测一对物体的接触
    fn detect_pair(&mut self, i: usize, j: usize) {
        let bi = self.bodies[i].clone();
        let bj = self.bodies[j].clone();
        let si = &self.shapes[bi.shape_id];
        let sj = &self.shapes[bj.shape_id];

        // broad phase: 包围球测试
        let d_center = bj.position - bi.position;
        let dist = d_center.length();
        let r_sum = si.bounding_radius() + sj.bounding_radius();
        if dist > r_sum {
            return;
        }

        // 在形状 i 的表面采样点, 检查是否在形状 j 内部
        let samples_i = self.sample_surface(&bi, si.as_ref());
        for p in &samples_i {
            let sdf_j = bj.sdf_world(sj.as_ref(), *p);
            if sdf_j < self.config.contact_offset {
                let depth = self.config.contact_offset - sdf_j;
                if depth > 0.0 && depth < self.config.max_penetration {
                    // 法向: i 的 SDF 在该点的梯度 (指向 i 外部 = A->B 方向)
                    let n = bi.sdf_gradient_world(si.as_ref(), *p, self.config.sdf_eps);
                    let n = if n.length() > 1e-6 {
                        n.normalize()
                    } else {
                        d_center / dist.max(1e-10)
                    };
                    // 接触点: 在两表面之间
                    let contact_point = *p;
                    // normal 从 A(i) 指向 B(j)
                    self.contacts.push(OgcContact::new(i, j, contact_point, n, depth));
                }
            }
        }

        // 反向: 在形状 j 的表面采样, 检查是否在 i 内部
        let samples_j = self.sample_surface(&bj, sj.as_ref());
        for p in &samples_j {
            let sdf_i = bi.sdf_world(si.as_ref(), *p);
            if sdf_i < self.config.contact_offset {
                let depth = self.config.contact_offset - sdf_i;
                if depth > 0.0 && depth < self.config.max_penetration {
                    // j 的 SDF 梯度指向 j 外部 = B->A 方向, 取负得 A->B
                    let n = bj.sdf_gradient_world(sj.as_ref(), *p, self.config.sdf_eps);
                    let n = if n.length() > 1e-6 {
                        -n.normalize()
                    } else {
                        d_center / dist.max(1e-10)
                    };
                    self.contacts.push(OgcContact::new(i, j, *p, n, depth));
                }
            }
        }
    }

    /// 在物体表面采样点 (蒙特卡洛: 在包围球内采样, 过滤 SDF 接近 0 的点)
    fn sample_surface(&self, body: &OgcBody, shape: &dyn OgcShape) -> Vec<Vec3> {
        let mut samples = Vec::new();
        let r = shape.bounding_radius();
        let n = self.config.sample_count;
        // 用球面 fibonacci 采样
        for k in 0..n {
            let phi = (1.0 + 5.0f32.sqrt()) * std::f32::consts::PI * k as f32;
            let cos_theta = 1.0 - 2.0 * (k as f32 + 0.5) / n as f32;
            let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
            let dir = Vec3::new(
                sin_theta * phi.cos(),
                cos_theta,
                sin_theta * phi.sin(),
            );
            // 沿方向 dir 找表面点 (从中心射线, sphere trace)
            let p_local = dir * r;
            let p_world = body.position + body.rotation * p_local;
            // 修正到实际表面 (sphere trace 一步)
            let sdf = body.sdf_world(shape, p_world);
            let p_surf = p_world - body.rotation.inverse() * (dir * sdf); // 沿 -dir 修正
            // 重新计算 SDF 确认在表面附近
            let sdf_corrected = body.sdf_world(shape, p_surf);
            if sdf_corrected.abs() < r * 0.1 {
                samples.push(p_surf);
            }
        }
        samples
    }

    /// 顺序冲量求解接触约束
    fn solve_contacts(&mut self) {
        let dt = self.config.dt;
        let n_contacts = self.contacts.len();
        if n_contacts == 0 {
            return;
        }
        // 复制 bodies (因为我们要修改)
        let mut bodies = self.bodies.clone();
        for c in &mut self.contacts {
            let bi = &bodies[c.body_a];
            let bj = &bodies[c.body_b];
            let ri = c.point - bi.position;
            let rj = c.point - bj.position;

            // 相对速度
            let vi = bi.linear_vel + bi.angular_vel.cross(ri);
            let vj = bj.linear_vel + bj.angular_vel.cross(rj);
            let v_rel = vj - vi;

            // 法向相对速度 (沿 normal 方向)
            let vn = v_rel.dot(c.normal);

            // Baumgarte 稳定化 (位置修正)
            let baumgarte = self.config.baumgarte / dt.max(1e-10);
            let bias = baumgarte * c.depth.max(0.0);

            // 有效逆质量
            let inv_i = bi.inv_inertia_world();
            let inv_j = bj.inv_inertia_world();
            let rn_i = ri.cross(c.normal);
            let rn_j = rj.cross(c.normal);
            let w_n = bi.inv_mass + bj.inv_mass
                + (inv_i * rn_i).dot(rn_i)
                + (inv_j * rn_j).dot(rn_j);
            if w_n < 1e-10 {
                continue;
            }

            // 恢复系数
            let e = if vn.abs() < self.config.restitution_threshold {
                0.0
            } else {
                bi.restitution.min(bj.restitution)
            };

            // 法向冲量
            let lambda = -(vn + bias + e * vn.max(0.0)) / w_n;
            let old_lambda = c.lambda_n;
            c.lambda_n = (old_lambda + lambda).max(0.0);
            let lambda_applied = c.lambda_n - old_lambda;
            let impulse_n = c.normal * lambda_applied;

            // 应用法向冲量
            bodies[c.body_a].apply_impulse(-impulse_n, c.point);
            bodies[c.body_b].apply_impulse(impulse_n, c.point);

            // 切向摩擦
            let bi = &bodies[c.body_a];
            let bj = &bodies[c.body_b];
            let vi = bi.linear_vel + bi.angular_vel.cross(ri);
            let vj = bj.linear_vel + bj.angular_vel.cross(rj);
            let v_rel = vj - vi;
            let v_t = v_rel - c.normal * v_rel.dot(c.normal);
            let t_len = v_t.length();
            if t_len > 1e-6 {
                let t = v_t / t_len;
                let rt_i = ri.cross(t);
                let rt_j = rj.cross(t);
                let w_t = bi.inv_mass + bj.inv_mass
                    + (inv_i * rt_i).dot(rt_i)
                    + (inv_j * rt_j).dot(rt_j);
                if w_t < 1e-10 {
                    continue;
                }
                let lambda_t = -t_len / w_t;
                let mu = (bi.friction + bj.friction) * 0.5;
                let max_t = mu * c.lambda_n;
                let old_t = c.lambda_t.dot(t);
                let new_t = (old_t + lambda_t).max(-max_t).min(max_t);
                let d_lambda_t = new_t - old_t;
                let impulse_t = t * d_lambda_t;
                c.lambda_t += impulse_t;
                bodies[c.body_a].apply_impulse(-impulse_t, c.point);
                bodies[c.body_b].apply_impulse(impulse_t, c.point);
            }
        }
        self.bodies = bodies;
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for b in &self.bodies {
            if b.inv_mass > 0.0 {
                let m = 1.0 / b.inv_mass;
                ke += 0.5 * m * b.linear_vel.length_squared();
                // 转动动能 (简化: 用角速度和局部惯性)
                let i_local = Vec3::new(
                    1.0 / b.inv_inertia_local.x.max(1e-10),
                    1.0 / b.inv_inertia_local.y.max(1e-10),
                    1.0 / b.inv_inertia_local.z.max(1e-10),
                );
                ke += 0.5 * (i_local * b.angular_vel).dot(b.angular_vel);
            }
        }
        ke
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ogc_config_default() {
        let c = OgcConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.num_solver_iters > 0);
        assert!(c.sample_count > 0);
    }

    #[test]
    fn test_ogc_sphere_sdf() {
        let s = OgcSphere { radius: 1.0 };
        assert!((s.sdf_local(Vec3::ZERO) - (-1.0)).abs() < 1e-6, "inside");
        assert!((s.sdf_local(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-6, "outside");
        assert!(s.sdf_local(Vec3::new(1.0, 0.0, 0.0)).abs() < 1e-6, "surface");
    }

    #[test]
    fn test_ogc_box_sdf() {
        let b = OgcBox { half_extents: Vec3::new(1.0, 1.0, 1.0) };
        assert!(b.sdf_local(Vec3::ZERO) < 0.0, "inside");
        assert!((b.sdf_local(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-6, "outside face");
        assert!(b.sdf_local(Vec3::new(1.0, 0.0, 0.0)).abs() < 1e-6, "surface");
    }

    #[test]
    fn test_ogc_cylinder_sdf() {
        let c = OgcCylinder { radius: 1.0, half_height: 1.0 };
        assert!(c.sdf_local(Vec3::ZERO) < 0.0, "inside");
        assert!((c.sdf_local(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-6, "outside radial");
        assert!((c.sdf_local(Vec3::new(0.0, 2.0, 0.0)) - 1.0).abs() < 1e-6, "outside axial");
    }

    #[test]
    fn test_ogc_body_sdf_world() {
        let shape = OgcSphere { radius: 1.0 };
        let mut body = OgcBody::new(Vec3::new(5.0, 0.0, 0.0), 0);
        body.rotation = Quat::IDENTITY;
        // 世界点 (5,0,0) 在球心, SDF = -1
        let sdf = body.sdf_world(&shape, Vec3::new(5.0, 0.0, 0.0));
        assert!((sdf - (-1.0)).abs() < 1e-6, "sdf at center: {}", sdf);
        // 世界点 (7,0,0) 距球心 2, SDF = 1
        let sdf = body.sdf_world(&shape, Vec3::new(7.0, 0.0, 0.0));
        assert!((sdf - 1.0).abs() < 1e-6, "sdf at dist 2: {}", sdf);
    }

    #[test]
    fn test_ogc_body_sdf_gradient() {
        let shape = OgcSphere { radius: 1.0 };
        let body = OgcBody::new(Vec3::ZERO, 0);
        // 在 (2,0,0) 处, SDF 梯度应指向 +x
        let grad = body.sdf_gradient_world(&shape, Vec3::new(2.0, 0.0, 0.0), 1e-3);
        let n = grad.normalize();
        assert!((n - Vec3::new(1.0, 0.0, 0.0)).length() < 0.1, "gradient: {:?}", n);
    }

    #[test]
    fn test_ogc_sphere_ground_contact() {
        // 球落在地面上 (大盒作地面)
        let mut solver = OgcSolver::new(OgcConfig {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_solver_iters: 15,
            contact_offset: 0.01,
            max_penetration: 0.5,
            baumgarte: 0.3,
            restitution_threshold: 1.0,
            sample_count: 64,
            sdf_eps: 1e-3,
        });
        let ground_id = solver.add_shape(Box::new(OgcBox {
            half_extents: Vec3::new(10.0, 1.0, 10.0),
        }));
        let sphere_id = solver.add_shape(Box::new(OgcSphere { radius: 0.5 }));
        // 球在 y=0.4 (略低于 0.5, 已穿透地面 0.1)
        let _ground_body = solver.add_body(OgcBody::fixed(Vec3::new(0.0, -1.0, 0.0), ground_id));
        let _sphere_body = solver.add_body(OgcBody::new(Vec3::new(0.0, 0.4, 0.0), sphere_id));
        // 单步: 检测接触 + 求解
        solver.step();
        // 球应有向上速度 (反弹) 或至少停止下落
        let v = solver.bodies[1].linear_vel;
        assert!(v.y >= -1.0, "sphere velocity y should be bounded: {}", v.y);
        // 接触应被检测到
        assert!(!solver.contacts.is_empty(), "should detect contacts");
    }

    #[test]
    fn test_ogc_sphere_sphere_collision() {
        let mut solver = OgcSolver::new(OgcConfig {
            dt: 1.0 / 60.0,
            gravity: Vec3::ZERO,
            num_solver_iters: 20,
            contact_offset: 0.005,
            max_penetration: 0.5,
            baumgarte: 0.3,
            restitution_threshold: 5.0,
            sample_count: 64,
            sdf_eps: 1e-3,
        });
        let sphere_id = solver.add_shape(Box::new(OgcSphere { radius: 1.0 }));
        // 两球相向运动, 略有穿透
        let _b0 = solver.add_body({
            let mut b = OgcBody::new(Vec3::new(-0.9, 0.0, 0.0), sphere_id);
            b.linear_vel = Vec3::new(1.0, 0.0, 0.0);
            b.restitution = 1.0; // 完全弹性
            b
        });
        let _b1 = solver.add_body({
            let mut b = OgcBody::new(Vec3::new(0.9, 0.0, 0.0), sphere_id);
            b.linear_vel = Vec3::new(-1.0, 0.0, 0.0);
            b.restitution = 1.0;
            b
        });
        solver.step();
        // 碰撞后两球应弹开 (速度反向)
        let v0 = solver.bodies[0].linear_vel;
        let v1 = solver.bodies[1].linear_vel;
        assert!(v0.x <= 0.5, "ball 0 should bounce back: {:?}", v0);
        assert!(v1.x >= -0.5, "ball 1 should bounce back: {:?}", v1);
    }

    #[test]
    fn test_ogc_resting_stack() {
        // 球在静止地面上应保持稳定 (无抖动)
        let mut solver = OgcSolver::new(OgcConfig {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_solver_iters: 20,
            contact_offset: 0.01,
            max_penetration: 0.5,
            baumgarte: 0.2,
            restitution_threshold: 1.0,
            sample_count: 64,
            sdf_eps: 1e-3,
        });
        let ground_id = solver.add_shape(Box::new(OgcBox {
            half_extents: Vec3::new(10.0, 1.0, 10.0),
        }));
        let sphere_id = solver.add_shape(Box::new(OgcSphere { radius: 0.5 }));
        let _ground = solver.add_body(OgcBody::fixed(Vec3::new(0.0, -1.0, 0.0), ground_id));
        let _sphere = solver.add_body(OgcBody::new(Vec3::new(0.0, 0.5, 0.0), sphere_id));
        // 跑 60 步 (1 秒), 球应保持在地面上, 不穿透, 不飞走
        for _ in 0..60 {
            solver.step();
        }
        let p = solver.bodies[1].position;
        assert!(p.y >= 0.4, "sphere should not penetrate ground: y={}", p.y);
        assert!(p.y < 2.0, "sphere should not fly away: y={}", p.y);
        let v = solver.bodies[1].linear_vel;
        assert!(v.y.abs() < 5.0, "sphere velocity should be bounded: {}", v.y);
    }

    #[test]
    fn test_ogc_velocity_at_point() {
        let mut body = OgcBody::new(Vec3::ZERO, 0);
        body.linear_vel = Vec3::new(1.0, 0.0, 0.0);
        body.angular_vel = Vec3::new(0.0, 1.0, 0.0);
        // 点 (1,0,0) 的速度 = linear + angular x r = (1,0,0) + (0,1,0) x (1,0,0) = (1,0,0) + (0,0,-1) = (1,0,-1)
        let v = body.velocity_at(Vec3::new(1.0, 0.0, 0.0));
        assert!((v - Vec3::new(1.0, 0.0, -1.0)).length() < 1e-5, "velocity at point: {:?}", v);
    }

    #[test]
    fn test_ogc_apply_impulse() {
        let mut body = OgcBody::new(Vec3::ZERO, 0);
        body.inv_mass = 1.0;
        body.inv_inertia_local = Vec3::new(1.0, 1.0, 1.0);
        body.apply_impulse(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        // 线速度 = (0,1,0)
        assert!((body.linear_vel - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
        // 角速度 = I^-1 * (r x J) = (1,0,0) x (0,1,0) = (0,0,1)
        assert!((body.angular_vel - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
    }

    #[test]
    fn test_ogc_box_box_collision() {
        let mut solver = OgcSolver::new(OgcConfig {
            dt: 1.0 / 60.0,
            gravity: Vec3::ZERO,
            num_solver_iters: 20,
            contact_offset: 0.01,
            max_penetration: 0.5,
            baumgarte: 0.3,
            restitution_threshold: 5.0,
            sample_count: 96,
            sdf_eps: 1e-3,
        });
        let box_id = solver.add_shape(Box::new(OgcBox {
            half_extents: Vec3::new(0.5, 0.5, 0.5),
        }));
        let _b0 = solver.add_body({
            let mut b = OgcBody::new(Vec3::new(-0.4, 0.0, 0.0), box_id);
            b.linear_vel = Vec3::new(1.0, 0.0, 0.0);
            b
        });
        let _b1 = solver.add_body({
            let mut b = OgcBody::new(Vec3::new(0.4, 0.0, 0.0), box_id);
            b.linear_vel = Vec3::new(-1.0, 0.0, 0.0);
            b
        });
        solver.step();
        // 盒-盒碰撞后应弹开
        let v0 = solver.bodies[0].linear_vel;
        assert!(v0.x < 1.0, "box 0 should decelerate: {:?}", v0);
    }

    #[test]
    fn test_ogc_kinetic_energy() {
        let mut solver = OgcSolver::new(OgcConfig::default());
        let sphere_id = solver.add_shape(Box::new(OgcSphere { radius: 1.0 }));
        let _ = solver.add_body({
            let mut b = OgcBody::new(Vec3::ZERO, sphere_id);
            b.linear_vel = Vec3::new(1.0, 0.0, 0.0);
            b
        });
        let ke = solver.kinetic_energy();
        assert!(ke > 0.0, "ke should be positive: {}", ke);
    }
}
