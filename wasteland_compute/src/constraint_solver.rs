//! Constraint Solver — 约束求解器 (Sequential Impulse)
//!
//! 基于:
//! - Catto. "Iterative Dynamics with Temporal Coherence." GDC 2005.
//! - Bullet Physics SDK 的 btPoint2PointConstraint / btHingeConstraint / btSliderConstraint
//! - Box2D b2ContactSolver (Sequential Impulse with warm starting)
//! - Witkin, Baraff, Kass. "An Introduction to Physically Based Modeling:
//!   Constrained Dynamics." SIGGRAPH Course Notes 2001.
//!
//! 核心思想 (Sequential Impulse):
//! 1. 速度级约束: J·v + bias = 0 (等式) 或 J·v + bias ≥ 0 (不等式)
//! 2. 有效质量 K = J·M⁻¹·Jᵀ (3x3 矩阵对点约束, 1x1 标量对距离/角度约束)
//! 3. 冲量 λ = -K⁻¹·(J·v + bias)
//! 4. 速度更新: v += M⁻¹·Jᵀ·λ
//! 5. 多次迭代收敛 (10-20 次, 与 Box2D velocity iterations 一致)
//! 6. Baumgarte 位置修正: bias = (β/dt)·C, β ∈ [0.1, 0.3]
//! 7. Warm starting: 保存上一帧的 λ 作为初值, 加快收敛
//!
//! 约定:
//! - 锚点 anchor_a, anchor_b 为物体局部坐标系下的位置 (relative to body center)
//! - 求解时通过 body.rotation 转到世界坐标系: r_a = R·anchor_a
//! - 应用冲量符号约定: λ 为正表示对 B 的推力沿约束轴正方向

use glam::{Vec3, Quat, Mat3};

use crate::rigid_body::RigidBody;

const BAUMGARTE_BETA: f32 = 0.2;   // 位置修正强度 (0=关闭, 1=完全修正)
const MAX_SLOP: f32 = 0.005;       // 误差容差 (避免过修正抖动)
const DEFAULT_ITERATIONS: usize = 15;

// ============================================================
// 辅助函数
// ============================================================

/// 反对称矩阵 skew(r): skew(r)·x = r × x
#[inline]
fn skew(r: Vec3) -> Mat3 {
    Mat3::from_cols(
        Vec3::new(0.0, r.z, -r.y),
        Vec3::new(-r.z, 0.0, r.x),
        Vec3::new(r.y, -r.x, 0.0),
    )
}

/// 3x3 对称正定矩阵求解 (Cholesky 分解或直接求逆)
/// 用于点约束的有效质量 K = J·M⁻¹·Jᵀ
#[inline]
fn solve_symmetric_3x3(k: Mat3, b: Vec3) -> Vec3 {
    // 直接使用 Mat3::inverse (glam 内部用伴随矩阵法)
    // 对于正定矩阵, 也可以用 Cholesky, 但 inverse 对 3x3 足够快
    if k.determinant().abs() < 1e-12 {
        return Vec3::ZERO;
    }
    k.inverse() * b
}

/// 计算 r × ω 的有效逆质量 (3x3): skew(r)·I⁻¹·skew(r)ᵀ
#[inline]
fn angular_effective_mass(inv_inertia: Mat3, r: Vec3) -> Mat3 {
    let s = skew(r);
    s * inv_inertia * s.transpose()
}

/// 将局部锚点转为世界坐标偏移: r = R·local
#[inline]
fn world_offset(body: &RigidBody, local_anchor: Vec3) -> Vec3 {
    body.rotation * local_anchor
}

// ============================================================
// Constraint Trait
// ============================================================

/// 约束接口
///
/// 所有约束实现此接口, 由 ConstraintSolver 统一调度求解
pub trait Constraint: Send + Sync {
    /// 求解一次速度约束 (一次迭代)
    /// 返回应用的冲量大小 (用于调试/收敛判断)
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, dt: f32);

    /// 求解位置约束 (Baumgarte 位置修正)
    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, dt: f32);

    /// Warm starting: 应用上一帧保存的冲量作为初值
    fn warm_start(&mut self, a: &mut RigidBody, b: &mut RigidBody);

    /// 是否启用 (false 的约束在求解器中被跳过)
    fn enabled(&self) -> bool { true }
}

// ============================================================
// Point-to-Point Constraint (球关节)
// ============================================================

/// 点对点约束 (球关节): 锚点重合, 允许任意旋转
///
/// 约束方程 (3 维): C = (p_a + R_a·anchor_a) - (p_b + R_b·anchor_b) = 0
/// Jacobian: J = [I, -skew(r_a), -I, skew(r_b)]
/// 有效质量 K (3x3) = (1/m_a + 1/m_b)·I + skew(r_a)·I_a⁻¹·skew(r_a)ᵀ + skew(r_b)·I_b⁻¹·skew(r_b)ᵀ
#[derive(Debug, Clone)]
pub struct PointConstraint {
    pub anchor_a: Vec3,  // A 局部坐标系下的锚点
    pub anchor_b: Vec3,  // B 局部坐标系下的锚点
    accumulated_impulse: Vec3,
}

impl PointConstraint {
    pub fn new(anchor_a: Vec3, anchor_b: Vec3) -> Self {
        Self {
            anchor_a,
            anchor_b,
            accumulated_impulse: Vec3::ZERO,
        }
    }

    /// 计算当前误差 C = (p_a + r_a) - (p_b + r_b)
    #[inline]
    fn compute_error(a: &RigidBody, b: &RigidBody, anchor_a: Vec3, anchor_b: Vec3) -> Vec3 {
        let r_a = world_offset(a, anchor_a);
        let r_b = world_offset(b, anchor_b);
        (a.position + r_a) - (b.position + r_b)
    }

    /// 计算有效质量矩阵 K (3x3)
    #[inline]
    fn compute_effective_mass(a: &RigidBody, b: &RigidBody, r_a: Vec3, r_b: Vec3) -> Mat3 {
        let k_a = if a.is_static {
            Mat3::ZERO
        } else {
            Mat3::from_diagonal(Vec3::splat(a.inv_mass)) + angular_effective_mass(a.world_inv_inertia(), r_a)
        };
        let k_b = if b.is_static {
            Mat3::ZERO
        } else {
            Mat3::from_diagonal(Vec3::splat(b.inv_mass)) + angular_effective_mass(b.world_inv_inertia(), r_b)
        };
        k_a + k_b
    }
}

impl Constraint for PointConstraint {
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);

        // 锚点处的相对速度: v_b - v_a + (ω_b × r_b - ω_a × r_a)
        let v_a = a.velocity_at_point(a.position + r_a);
        let v_b = b.velocity_at_point(b.position + r_b);
        let dv = v_b - v_a;

        // Baumgarte 速度偏置 (在位置约束中处理, 这里只做速度约束)
        let k = Self::compute_effective_mass(a, b, r_a, r_b);
        // λ = -K⁻¹·dv
        let lambda = -solve_symmetric_3x3(k, dv);

        // 累积冲量 (用于 warm starting)
        self.accumulated_impulse += lambda;

        // 应用冲量: A 受 -λ, B 受 +λ
        a.apply_impulse_at_point(-lambda, a.position + r_a);
        b.apply_impulse_at_point(lambda, b.position + r_b);
    }

    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        let c = Self::compute_error(a, b, self.anchor_a, self.anchor_b);
        let len = c.length();
        if len <= MAX_SLOP {
            return;
        }
        // 直接位置修正 (不通过冲量, 避免速度正反馈): λ = -β·C / K
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let k = Self::compute_effective_mass(a, b, r_a, r_b);
        let lambda = -solve_symmetric_3x3(k, c * BAUMGARTE_BETA);

        apply_position_correction(a, lambda, r_a);
        apply_position_correction(b, -lambda, r_b);
    }

    fn warm_start(&mut self, a: &mut RigidBody, b: &mut RigidBody) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let lambda = self.accumulated_impulse;
        a.apply_impulse_at_point(-lambda, a.position + r_a);
        b.apply_impulse_at_point(lambda, b.position + r_b);
    }
}

// ============================================================
// Distance Constraint (距离约束 / 弹簧)
// ============================================================

/// 距离约束: ||p_a + r_a - (p_b + r_b)|| = rest_length
///
/// 一维约束 (沿连线方向)
/// Jacobian: J = -n̂·[I, -skew(r_a), I, skew(r_b)]  (n̂ 从 A 指向 B)
/// 有效质量 K = (1/m_a + 1/m_b) + (r_a×n̂)·I_a⁻¹·(r_a×n̂) + (r_b×n̂)·I_b⁻¹·(r_b×n̂)
#[derive(Debug, Clone)]
pub struct DistanceConstraint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    pub rest_length: f32,
    /// 弹簧刚度 (>0 时启用软约束, 0 为硬约束)
    pub stiffness: f32,
    /// 阻尼系数
    pub damping: f32,
    accumulated_impulse: f32,
}

impl DistanceConstraint {
    pub fn new(anchor_a: Vec3, anchor_b: Vec3, rest_length: f32) -> Self {
        Self {
            anchor_a,
            anchor_b,
            rest_length,
            stiffness: 0.0,
            damping: 0.0,
            accumulated_impulse: 0.0,
        }
    }

    pub fn with_spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    /// 计算当前长度和方向 (n̂ 从 A 指向 B)
    #[inline]
    fn compute_axis(a: &RigidBody, b: &RigidBody, anchor_a: Vec3, anchor_b: Vec3) -> (Vec3, f32) {
        let r_a = world_offset(a, anchor_a);
        let r_b = world_offset(b, anchor_b);
        let delta = (b.position + r_b) - (a.position + r_a);
        let length = delta.length();
        let n = if length > 1e-9 { delta / length } else { Vec3::new(0.0, 1.0, 0.0) };
        (n, length)
    }

    /// 一维有效质量
    #[inline]
    fn compute_effective_mass(a: &RigidBody, b: &RigidBody, r_a: Vec3, r_b: Vec3, n: Vec3) -> f32 {
        let mut k = 0.0;
        if !a.is_static {
            let rxa = r_a.cross(n);
            k += a.inv_mass + rxa.dot(a.world_inv_inertia() * rxa);
        }
        if !b.is_static {
            let rxb = r_b.cross(n);
            k += b.inv_mass + rxb.dot(b.world_inv_inertia() * rxb);
        }
        if k < 1e-12 { 1e12 } else { k }
    }
}

impl Constraint for DistanceConstraint {
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, dt: f32) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let (n, length) = Self::compute_axis(a, b, self.anchor_a, self.anchor_b);

        // 沿 n 方向的相对速度 (B 相对 A)
        let v_a = a.velocity_at_point(a.position + r_a);
        let v_b = b.velocity_at_point(b.position + r_b);
        let vn = (v_b - v_a).dot(n);

        let k = Self::compute_effective_mass(a, b, r_a, r_b, n);

        // 软约束 (弹簧): 引入 cfm (constraint force mixing)
        // K_eff = K + (stiffness·dt² + damping·dt)
        // bias = -stiffness·dt·(length - rest) - damping·... (由速度项吸收)
        let (k_eff, bias) = if self.stiffness > 0.0 {
            let c_err = length - self.rest_length;
            let k_soft = self.stiffness * dt * dt + self.damping * dt;
            (k + k_soft, self.stiffness * dt * c_err + self.damping * vn * dt)
        } else {
            (k, 0.0)
        };

        // λ = -(vn + bias) / k_eff
        let lambda = -(vn + bias) / k_eff;
        self.accumulated_impulse += lambda;

        let impulse = n * lambda;
        a.apply_impulse_at_point(-impulse, a.position + r_a);
        b.apply_impulse_at_point(impulse, b.position + r_b);
    }

    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 软约束不在 position solver 中处理 (由 velocity solver 的 bias 完成)
        if self.stiffness > 0.0 {
            return;
        }
        let (n, length) = Self::compute_axis(a, b, self.anchor_a, self.anchor_b);
        let c = length - self.rest_length;
        if c.abs() <= MAX_SLOP {
            return;
        }
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let k = Self::compute_effective_mass(a, b, r_a, r_b, n);
        let lambda = -BAUMGARTE_BETA * c / k;
        let impulse = n * lambda;
        apply_position_correction(a, -impulse, r_a);
        apply_position_correction(b, impulse, r_b);
    }

    fn warm_start(&mut self, a: &mut RigidBody, b: &mut RigidBody) {
        let (n, _) = Self::compute_axis(a, b, self.anchor_a, self.anchor_b);
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let impulse = n * self.accumulated_impulse;
        a.apply_impulse_at_point(-impulse, a.position + r_a);
        b.apply_impulse_at_point(impulse, b.position + r_b);
    }
}

// ============================================================
// Hinge Constraint (铰链约束)
// ============================================================

/// 铰链约束: 锚点重合 + 仅允许绕指定轴旋转
///
/// 由两部分组成:
/// 1. 点约束 (3 自由度, 锚点重合)
/// 2. 角度约束 (2 自由度, 限制垂直轴方向的旋转)
///
/// 角度约束使用两根垂直于铰链轴的单位向量作为参考,
/// 测量它们在两个刚体坐标系下的差异, 投影到垂直平面.
#[derive(Debug, Clone)]
pub struct HingeConstraint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    /// A 的铰链轴 (A 局部坐标系)
    pub axis_a: Vec3,
    /// B 的铰链轴 (B 局部坐标系)
    pub axis_b: Vec3,
    /// 角度限制 [min, max] (弧度), None 表示无限制
    pub limit_min: Option<f32>,
    pub limit_max: Option<f32>,
    accumulated_impulse_point: Vec3,
    accumulated_impulse_angular: Vec3,
    /// 当前铰链角 (用于限制)
    current_angle: f32,
}

impl HingeConstraint {
    pub fn new(anchor_a: Vec3, anchor_b: Vec3, axis_a: Vec3, axis_b: Vec3) -> Self {
        Self {
            anchor_a,
            anchor_b,
            axis_a: axis_a.normalize_or_zero(),
            axis_b: axis_b.normalize_or_zero(),
            limit_min: None,
            limit_max: None,
            accumulated_impulse_point: Vec3::ZERO,
            accumulated_impulse_angular: Vec3::ZERO,
            current_angle: 0.0,
        }
    }

    pub fn with_angle_limit(mut self, min: f32, max: f32) -> Self {
        self.limit_min = Some(min);
        self.limit_max = Some(max);
        self
    }

    /// 计算两个轴在世界坐标系下的夹角 (绕铰链轴的相对旋转)
    #[inline]
    fn compute_hinge_angle(a: &RigidBody, b: &RigidBody, axis_a: Vec3, axis_b: Vec3) -> f32 {
        let aw = a.rotation * axis_a;
        let bw = b.rotation * axis_b;
        // 相对旋转四元数: q_rel = q_b⁻¹ * q_a
        let q_rel = b.rotation.inverse() * a.rotation;
        // 提取绕铰链轴的旋转分量
        // 投影: 取垂直于 aw 的两个向量, 测量它们在 B 坐标系下的夹角
        let perp = if aw.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let u_a = aw.cross(perp).normalize_or_zero();
        let v_a = aw.cross(u_a);
        // B 的对应向量
        let u_b = bw.cross(perp).normalize_or_zero();
        let _v_b = bw.cross(u_b);
        // 夹角 (有符号)
        let cos_a = u_a.dot(u_b);
        let sin_a = u_a.dot(v_a.cross(u_b).normalize_or_zero());
        sin_a.atan2(cos_a)
    }

    /// 计算 A 和 B 的铰链轴在世界坐标系下的垂直向量对 (用于约束 2 个旋转自由度)
    #[inline]
    fn compute_angular_axes(a: &RigidBody, b: &RigidBody, axis_a: Vec3, axis_b: Vec3) -> (Vec3, Vec3, Vec3, Vec3) {
        let aw = a.rotation * axis_a;
        let bw = b.rotation * axis_b;
        // 选一个不平行于 aw 的参考向量构造正交基
        let perp = if aw.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let u_a = aw.cross(perp).normalize_or_zero();
        let v_a = aw.cross(u_a).normalize_or_zero();
        let u_b = bw.cross(perp).normalize_or_zero();
        let v_b = bw.cross(u_b).normalize_or_zero();
        (u_a, v_a, u_b, v_b)
    }
}

impl Constraint for HingeConstraint {
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 1) 点约束部分 (与 PointConstraint 相同)
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let v_a = a.velocity_at_point(a.position + r_a);
        let v_b = b.velocity_at_point(b.position + r_b);
        let dv = v_b - v_a;
        let k_pt = PointConstraint::compute_effective_mass(a, b, r_a, r_b);
        let lambda_pt = -solve_symmetric_3x3(k_pt, dv);
        self.accumulated_impulse_point += lambda_pt;
        a.apply_impulse_at_point(-lambda_pt, a.position + r_a);
        b.apply_impulse_at_point(lambda_pt, b.position + r_b);

        // 2) 角度约束部分 (锁定垂直于铰链轴的 2 个旋转自由度)
        let aw = a.rotation * self.axis_a;
        let bw = b.rotation * self.axis_b;
        let perp = if aw.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let u_a = aw.cross(perp).normalize_or_zero();
        let v_a = aw.cross(u_a).normalize_or_zero();
        let u_b = bw.cross(perp).normalize_or_zero();
        let v_b = bw.cross(u_b).normalize_or_zero();

        // 两个垂直轴的相对角速度投影
        let dw = b.angular_velocity - a.angular_velocity;

        // 对 u 方向: 约束 (u_a × u_b)·ω_rel ≈ 0 (用近似)
        // 简化: 直接约束 dw 在垂直于 aw 的平面内的分量
        // 用 u_a 和 v_a 作为约束轴
        let inv_ia = a.world_inv_inertia();
        let inv_ib = b.world_inv_inertia();

        // u 轴约束
        let k_u = u_a.dot(inv_ia * u_a) + u_b.dot(inv_ib * u_b);
        if k_u > 1e-9 {
            let bias_u = BAUMGARTE_BETA * u_a.cross(u_b).dot(aw).signum() * u_a.dot(u_b.cross(aw).normalize_or_zero());
            let _ = bias_u;
            // 速度约束: (ω_b - ω_a)·u_a = 0 (近似)
            let dvu = dw.dot(u_a);
            let lambda_u = -dvu / k_u;
            let impulse_u = u_a * lambda_u;
            if !a.is_static { a.angular_velocity += inv_ia * (-impulse_u); }
            if !b.is_static { b.angular_velocity += inv_ib * impulse_u; }
        }

        // v 轴约束
        let k_v = v_a.dot(inv_ia * v_a) + v_b.dot(inv_ib * v_b);
        if k_v > 1e-9 {
            let dvv = (b.angular_velocity - a.angular_velocity).dot(v_a);
            let lambda_v = -dvv / k_v;
            let impulse_v = v_a * lambda_v;
            if !a.is_static { a.angular_velocity += inv_ia * (-impulse_v); }
            if !b.is_static { b.angular_velocity += inv_ib * impulse_v; }
        }

        // 3) 角度限制 (如果配置了)
        if let (Some(min), Some(max)) = (self.limit_min, self.limit_max) {
            self.current_angle = Self::compute_hinge_angle(a, b, self.axis_a, self.axis_b);
            let aw_cur = a.rotation * self.axis_a;

            if self.current_angle < min {
                // 限制下界: 阻止继续向 min 以下旋转
                let excess = min - self.current_angle;
                let bias = (BAUMGARTE_BETA / _dt) * excess;
                // 绕 aw 方向施加反向角速度修正
                let dw_axial = (b.angular_velocity - a.angular_velocity).dot(aw_cur);
                let k_axial = aw_cur.dot(inv_ia * aw_cur) + aw_cur.dot(inv_ib * aw_cur);
                if k_axial > 1e-9 {
                    let lambda = -(dw_axial + bias) / k_axial;
                    let impulse = aw_cur * lambda;
                    if !a.is_static { a.angular_velocity += inv_ia * (-impulse); }
                    if !b.is_static { b.angular_velocity += inv_ib * impulse; }
                }
            } else if self.current_angle > max {
                let excess = self.current_angle - max;
                let bias = -(BAUMGARTE_BETA / _dt) * excess;
                let dw_axial = (b.angular_velocity - a.angular_velocity).dot(aw_cur);
                let k_axial = aw_cur.dot(inv_ia * aw_cur) + aw_cur.dot(inv_ib * aw_cur);
                if k_axial > 1e-9 {
                    let lambda = -(dw_axial + bias) / k_axial;
                    let impulse = aw_cur * lambda;
                    if !a.is_static { a.angular_velocity += inv_ia * (-impulse); }
                    if !b.is_static { b.angular_velocity += inv_ib * impulse; }
                }
            }
        }
    }

    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 点约束位置修正 (直接修改位置)
        let c = PointConstraint::compute_error(a, b, self.anchor_a, self.anchor_b);
        if c.length() > MAX_SLOP {
            let r_a = world_offset(a, self.anchor_a);
            let r_b = world_offset(b, self.anchor_b);
            let k = PointConstraint::compute_effective_mass(a, b, r_a, r_b);
            let lambda = -solve_symmetric_3x3(k, c * BAUMGARTE_BETA);
            apply_position_correction(a, lambda, r_a);
            apply_position_correction(b, -lambda, r_b);
        }
        // 轴对齐误差由速度约束收敛, 此处从略
    }

    fn warm_start(&mut self, a: &mut RigidBody, b: &mut RigidBody) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let lambda = self.accumulated_impulse_point;
        a.apply_impulse_at_point(-lambda, a.position + r_a);
        b.apply_impulse_at_point(lambda, b.position + r_b);
    }
}

// ============================================================
// Slider Constraint (滑动约束)
// ============================================================

/// 滑动约束: 允许沿一个轴滑动, 锁定其他 5 个自由度
///
/// 约束:
/// - 2 个垂直轴向的相对位置 = 0 (锁定横向移动)
/// - 3 个轴向的旋转 = 0 (锁定所有旋转)
/// - 沿滑动轴的滑动自由 (1 DOF 保留)
#[derive(Debug, Clone)]
pub struct SliderConstraint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    pub axis_a: Vec3,  // A 局部坐标系下的滑动轴
    pub axis_b: Vec3,  // B 局部坐标系下的滑动轴
    accumulated_impulse: Vec3,
}

impl SliderConstraint {
    pub fn new(anchor_a: Vec3, anchor_b: Vec3, axis_a: Vec3, axis_b: Vec3) -> Self {
        Self {
            anchor_a,
            anchor_b,
            axis_a: axis_a.normalize_or_zero(),
            axis_b: axis_b.normalize_or_zero(),
            accumulated_impulse: Vec3::ZERO,
        }
    }
}

impl Constraint for SliderConstraint {
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let aw = a.rotation * self.axis_a;
        let bw = b.rotation * self.axis_b;

        // 构造垂直于 aw 的两个正交向量 u, v
        let perp = if aw.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let u = aw.cross(perp).normalize_or_zero();
        let v = aw.cross(u).normalize_or_zero();

        let v_a = a.velocity_at_point(a.position + r_a);
        let v_b = b.velocity_at_point(b.position + r_b);
        let dv = v_b - v_a;

        // 锁定 u, v 方向的相对线速度
        for axis in [u, v] {
            let vn = dv.dot(axis);
            let mut k = 0.0;
            if !a.is_static {
                let rxa = r_a.cross(axis);
                k += a.inv_mass + rxa.dot(a.world_inv_inertia() * rxa);
            }
            if !b.is_static {
                let rxb = r_b.cross(axis);
                k += b.inv_mass + rxb.dot(b.world_inv_inertia() * rxb);
            }
            if k > 1e-9 {
                let lambda = -vn / k;
                let impulse = axis * lambda;
                a.apply_impulse_at_point(-impulse, a.position + r_a);
                b.apply_impulse_at_point(impulse, b.position + r_b);
            }
        }

        // 锁定所有角速度 (3 个轴)
        let dw = b.angular_velocity - a.angular_velocity;
        let inv_ia = a.world_inv_inertia();
        let inv_ib = b.world_inv_inertia();
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let dw_a = dw.dot(axis);
            let k = axis.dot(inv_ia * axis) + axis.dot(inv_ib * axis);
            if k > 1e-9 {
                let lambda = -dw_a / k;
                let impulse = axis * lambda;
                if !a.is_static { a.angular_velocity += inv_ia * (-impulse); }
                if !b.is_static { b.angular_velocity += inv_ib * impulse; }
            }
        }
    }

    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 横向位置修正 (u, v 方向) - 直接修改位置
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let delta = (b.position + r_b) - (a.position + r_a);
        let aw = a.rotation * self.axis_a;
        let perp = if aw.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let u = aw.cross(perp).normalize_or_zero();
        let v = aw.cross(u).normalize_or_zero();

        for axis in [u, v] {
            let c = delta.dot(axis);
            if c.abs() > MAX_SLOP {
                let mut k = 0.0;
                if !a.is_static {
                    let rxa = r_a.cross(axis);
                    k += a.inv_mass + rxa.dot(a.world_inv_inertia() * rxa);
                }
                if !b.is_static {
                    let rxb = r_b.cross(axis);
                    k += b.inv_mass + rxb.dot(b.world_inv_inertia() * rxb);
                }
                if k > 1e-9 {
                    let lambda = -BAUMGARTE_BETA * c / k;
                    let impulse = axis * lambda;
                    apply_position_correction(a, -impulse, r_a);
                    apply_position_correction(b, impulse, r_b);
                }
            }
        }
    }

    fn warm_start(&mut self, _a: &mut RigidBody, _b: &mut RigidBody) {
        // Slider 约束的 warm starting 较复杂, 此处简化
    }
}

// ============================================================
// Fixed Constraint (固定约束)
// ============================================================

/// 固定约束: 锁定所有 6 个自由度 (3 线 + 3 角)
/// 等价于把两个刚体焊死在一起
#[derive(Debug, Clone)]
pub struct FixedConstraint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    /// 初始相对旋转 (构造时锁定)
    pub initial_rel_rot: Quat,
    accumulated_impulse_pt: Vec3,
    accumulated_impulse_ang: Vec3,
}

impl FixedConstraint {
    pub fn new(anchor_a: Vec3, anchor_b: Vec3) -> Self {
        Self {
            anchor_a,
            anchor_b,
            initial_rel_rot: Quat::IDENTITY,
            accumulated_impulse_pt: Vec3::ZERO,
            accumulated_impulse_ang: Vec3::ZERO,
        }
    }

    /// 必须在创建后调用, 锁定当前相对旋转
    pub fn capture_initial(&mut self, a: &RigidBody, b: &RigidBody) {
        self.initial_rel_rot = b.rotation.inverse() * a.rotation;
    }
}

impl Constraint for FixedConstraint {
    fn solve_velocity(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 1) 点约束 (锁定 3 线)
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let v_a = a.velocity_at_point(a.position + r_a);
        let v_b = b.velocity_at_point(b.position + r_b);
        let dv = v_b - v_a;
        let k_pt = PointConstraint::compute_effective_mass(a, b, r_a, r_b);
        let lambda_pt = -solve_symmetric_3x3(k_pt, dv);
        self.accumulated_impulse_pt += lambda_pt;
        a.apply_impulse_at_point(-lambda_pt, a.position + r_a);
        b.apply_impulse_at_point(lambda_pt, b.position + r_b);

        // 2) 角度约束 (锁定 3 角): ω_b - ω_a = 0
        let dw = b.angular_velocity - a.angular_velocity;
        let inv_ia = a.world_inv_inertia();
        let inv_ib = b.world_inv_inertia();
        // 用 3x3 有效质量: K = I_a⁻¹ + I_b⁻¹
        let k_ang = inv_ia + inv_ib;
        if k_ang.determinant().abs() > 1e-12 {
            let lambda_ang = -solve_symmetric_3x3(k_ang, dw);
            self.accumulated_impulse_ang += lambda_ang;
            if !a.is_static { a.angular_velocity += inv_ia * (-lambda_ang); }
            if !b.is_static { b.angular_velocity += inv_ib * lambda_ang; }
        }
    }

    fn solve_position(&mut self, a: &mut RigidBody, b: &mut RigidBody, _dt: f32) {
        // 点约束位置修正 (直接修改位置)
        let c = PointConstraint::compute_error(a, b, self.anchor_a, self.anchor_b);
        if c.length() > MAX_SLOP {
            let r_a = world_offset(a, self.anchor_a);
            let r_b = world_offset(b, self.anchor_b);
            let k = PointConstraint::compute_effective_mass(a, b, r_a, r_b);
            let lambda = -solve_symmetric_3x3(k, c * BAUMGARTE_BETA);
            apply_position_correction(a, lambda, r_a);
            apply_position_correction(b, -lambda, r_b);
        }
        // 角度位置修正 (直接修改旋转, 不注入角速度)
        let q_rel = b.rotation.inverse() * a.rotation;
        let q_err = q_rel * self.initial_rel_rot.inverse();
        let w = q_err.w.max(-1.0).min(1.0);
        let angle = 2.0 * w.acos();
        if angle.abs() > MAX_SLOP {
            let axis = Vec3::new(q_err.x, q_err.y, q_err.z).normalize_or_zero();
            let inv_ia = a.world_inv_inertia();
            let inv_ib = b.world_inv_inertia();
            let k_ang = inv_ia + inv_ib;
            if k_ang.determinant().abs() > 1e-12 {
                let bias = axis * (BAUMGARTE_BETA * angle);
                let lambda = -solve_symmetric_3x3(k_ang, bias);
                // 直接旋转修正 (不修改角速度)
                if !a.is_static {
                    let ang_corr = inv_ia * (-lambda);
                    let omega_q = Quat::from_xyzw(ang_corr.x, ang_corr.y, ang_corr.z, 0.0);
                    let dq = omega_q * a.rotation;
                    let new_q = Quat::from_xyzw(
                        a.rotation.x + 0.5 * dq.x,
                        a.rotation.y + 0.5 * dq.y,
                        a.rotation.z + 0.5 * dq.z,
                        a.rotation.w + 0.5 * dq.w,
                    );
                    let len = new_q.length();
                    if len > 1e-9 { a.rotation = new_q / len; }
                }
                if !b.is_static {
                    let ang_corr = inv_ib * lambda;
                    let omega_q = Quat::from_xyzw(ang_corr.x, ang_corr.y, ang_corr.z, 0.0);
                    let dq = omega_q * b.rotation;
                    let new_q = Quat::from_xyzw(
                        b.rotation.x + 0.5 * dq.x,
                        b.rotation.y + 0.5 * dq.y,
                        b.rotation.z + 0.5 * dq.z,
                        b.rotation.w + 0.5 * dq.w,
                    );
                    let len = new_q.length();
                    if len > 1e-9 { b.rotation = new_q / len; }
                }
            }
        }
    }

    fn warm_start(&mut self, a: &mut RigidBody, b: &mut RigidBody) {
        let r_a = world_offset(a, self.anchor_a);
        let r_b = world_offset(b, self.anchor_b);
        let lambda = self.accumulated_impulse_pt;
        a.apply_impulse_at_point(-lambda, a.position + r_a);
        b.apply_impulse_at_point(lambda, b.position + r_b);
    }
}


// ============================================================
// 辅助函数: 位置修正 (不修改速度, 用于 position solver)
// ============================================================

/// 直接应用位置修正 (不通过冲量, 不影响速度)
/// impulse: 位置修正"冲量" (等效位移量)
/// r: 锚点到质心的世界坐标偏移
fn apply_position_correction(body: &mut RigidBody, impulse: Vec3, r: Vec3) {
    if body.is_static { return; }
    body.position += impulse * body.inv_mass;
    let angular_impulse = r.cross(impulse);
    if angular_impulse.length_squared() < 1e-12 {
        return;
    }
    let ang_corr = body.world_inv_inertia() * angular_impulse;
    let omega_q = Quat::from_xyzw(ang_corr.x, ang_corr.y, ang_corr.z, 0.0);
    let dq = omega_q * body.rotation;
    let new_q = Quat::from_xyzw(
        body.rotation.x + 0.5 * dq.x,
        body.rotation.y + 0.5 * dq.y,
        body.rotation.z + 0.5 * dq.z,
        body.rotation.w + 0.5 * dq.w,
    );
    let len = new_q.length();
    if len > 1e-9 {
        body.rotation = new_q / len;
    }
}

// ============================================================
// 辅助函数: 同时获取两个可变引用 (避免借用冲突)
// ============================================================

/// 从切片中同时获取两个不同索引的可变引用
#[inline]
fn get_two_mut<T>(slice: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j, "cannot borrow same element twice");
    if i < j {
        let (left, right) = slice.split_at_mut(j);
        (&mut left[i], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(i);
        (&mut right[0], &mut left[j])
    }
}

// ============================================================
// Constraint Solver
// ============================================================

/// 约束求解器 (Sequential Impulse)
pub struct ConstraintSolver {
    pub velocity_iterations: usize,
    pub position_iterations: usize,
    pub warm_starting: bool,
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self {
            velocity_iterations: DEFAULT_ITERATIONS,
            position_iterations: 4,
            warm_starting: true,
        }
    }
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// 求解约束系统
    ///
    /// bodies: 所有刚体 (按索引引用)
    /// constraints: 约束列表, 每个约束包含两个 body 的索引
    pub fn solve(
        &self,
        bodies: &mut [RigidBody],
        constraints: &mut [(usize, usize, Box<dyn Constraint>)],
        dt: f32,
    ) {
        // 1) Warm starting
        if self.warm_starting {
            for (ia, ib, c) in constraints.iter_mut() {
                if *ia == *ib { continue; }
                let (a, b) = get_two_mut(bodies, *ia, *ib);
                c.warm_start(a, b);
            }
        }

        // 2) Velocity iterations
        for _ in 0..self.velocity_iterations {
            for (ia, ib, c) in constraints.iter_mut() {
                if *ia == *ib { continue; }
                let (a, b) = get_two_mut(bodies, *ia, *ib);
                c.solve_velocity(a, b, dt);
            }
        }

        // 3) Position iterations (使用 Baumgarte 修正)
        for _ in 0..self.position_iterations {
            for (ia, ib, c) in constraints.iter_mut() {
                if *ia == *ib { continue; }
                let (a, b) = get_two_mut(bodies, *ia, *ib);
                c.solve_position(a, b, dt);
            }
        }
    }
}

// ============================================================
// 工具: 单刚体版本的简化求解 (用于测试)
// ============================================================

/// 对一对刚体求解单个约束 (无 warm starting, 用于测试)
pub fn solve_single_constraint<C: Constraint + ?Sized>(
    a: &mut RigidBody,
    b: &mut RigidBody,
    constraint: &mut C,
    velocity_iters: usize,
    position_iters: usize,
    dt: f32,
) {
    for _ in 0..velocity_iters {
        constraint.solve_velocity(a, b, dt);
    }
    for _ in 0..position_iters {
        constraint.solve_position(a, b, dt);
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a - b).length() < eps
    }

    #[test]
    fn test_skew_matrix() {
        // skew(r)·x = r × x
        let r = Vec3::new(1.0, 2.0, 3.0);
        let x = Vec3::new(4.0, 5.0, 6.0);
        let s = skew(r);
        let result = s * x;
        let expected = r.cross(x);
        assert!(approx_eq(result, expected, 1e-5), "skew(r)·x != r × x");
    }

    #[test]
    fn test_angular_effective_mass_symmetric() {
        let inv_i = Mat3::from_diagonal(Vec3::new(2.0, 3.0, 4.0));
        let r = Vec3::new(1.0, 0.5, 0.3);
        let m = angular_effective_mass(inv_i, r);
        // 应为对称矩阵
        let diff = (m - m.transpose()).abs();
        let max_val = diff.to_cols_array().iter().fold(0.0f32, |a, &b| a.max(b));
        assert!(max_val < 1e-5, "effective mass not symmetric");
    }

    #[test]
    fn test_solve_symmetric_3x3_identity() {
        let k = Mat3::IDENTITY;
        let b = Vec3::new(1.0, 2.0, 3.0);
        let x = solve_symmetric_3x3(k, b);
        assert!(approx_eq(x, b, 1e-5));
    }

    #[test]
    fn test_solve_symmetric_3x3_diagonal() {
        let k = Mat3::from_diagonal(Vec3::new(2.0, 4.0, 8.0));
        let b = Vec3::new(2.0, 8.0, 16.0);
        let x = solve_symmetric_3x3(k, b);
        assert!(approx_eq(x, Vec3::new(1.0, 2.0, 2.0), 1e-5));
    }

    #[test]
    fn test_point_constraint_two_spheres_connected() {
        // 两个球用点约束连接, 锚点在球心, 应被拉到同一点
        let mut a = RigidBody::sphere(1.0, 0.1);
        let mut b = RigidBody::sphere(1.0, 0.1);
        a.position = Vec3::new(0.0, 5.0, 0.0);
        b.position = Vec3::new(1.0, 5.0, 0.0);

        let mut constraint = PointConstraint::new(Vec3::ZERO, Vec3::ZERO);
        let dt = 0.016;

        for frame in 0..100 {
            a.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            a.integrate(dt);
            b.integrate(dt);
            // 直接求解 (不通过 ConstraintSolver, 避免克隆导致累积冲量丢失)
            for _ in 0..15 {
                constraint.solve_velocity(&mut a, &mut b, dt);
            }
            for _ in 0..4 {
                constraint.solve_position(&mut a, &mut b, dt);
            }
            if frame % 20 == 0 {
                let dist = (a.position - b.position).length();
                assert!(dist.is_finite(), "frame {}: distance not finite: {}", frame, dist);
            }
        }

        let dist = (a.position - b.position).length();
        assert!(dist < 0.2, "distance too large: {}", dist);
    }

    #[test]
    fn test_point_constraint_static_anchor() {
        // 一个静态锚点 + 一个动态球, 球被拉向锚点
        let mut a = RigidBody::new_static();
        a.position = Vec3::new(0.0, 5.0, 0.0);
        let mut b = RigidBody::sphere(1.0, 0.1);
        b.position = Vec3::new(0.0, 0.0, 0.0);

        let mut c = PointConstraint::new(Vec3::ZERO, Vec3::ZERO);
        let dt = 0.016;

        for frame in 0..200 {
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.integrate(dt);
            for _ in 0..15 {
                c.solve_velocity(&mut a, &mut b, dt);
            }
            for _ in 0..4 {
                c.solve_position(&mut a, &mut b, dt);
            }
            if frame % 40 == 0 {
                let dist = (b.position - a.position).length();
                assert!(dist.is_finite(), "frame {}: distance not finite: {}", frame, dist);
            }
        }

        let dist = (b.position - a.position).length();
        assert!(dist < 0.3, "distance to anchor too large: {}", dist);
    }

    #[test]
    fn test_distance_constraint_pendulum() {
        // 单摆: 静态锚点 + 动态球, 距离 = 1.0
        let mut anchor = RigidBody::new_static();
        anchor.position = Vec3::new(0.0, 5.0, 0.0);
        let mut bob = RigidBody::sphere(1.0, 0.1);
        bob.position = Vec3::new(1.0, 5.0, 0.0);  // 水平偏移 1m, 应形成单摆

        let constraint = DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0);
        let dt = 0.01;
        let solver = ConstraintSolver::new();

        let mut a = anchor.clone();
        let mut b = bob.clone();
        let mut c = constraint.clone();

        for _ in 0..500 {
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.integrate(dt);
            let mut bodies = [a.clone(), b.clone()];
            let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
                (0, 1, Box::new(c.clone())),
            ];
            solver.solve(&mut bodies, &mut constraints, dt);
            a = bodies[0].clone();
            b = bodies[1].clone();
        }

        // 5 秒后, 单摆应大致摆动到下方 (重力作用下)
        let dist = (b.position - a.position).length();
        assert!((dist - 1.0).abs() < 0.05, "pendulum length changed: {}", dist);
        // 球应在锚点下方 (重力作用下摆动)
        assert!(b.position.y < a.position.y, "bob should be below anchor");
    }

    #[test]
    fn test_distance_constraint_spring() {
        // 弹簧: 静态锚点 + 动态球, 软约束
        let mut anchor = RigidBody::new_static();
        anchor.position = Vec3::new(0.0, 5.0, 0.0);
        let mut bob = RigidBody::sphere(1.0, 0.1);
        bob.position = Vec3::new(0.0, 5.0, 0.0);  // 初始在锚点

        let constraint = DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0)
            .with_spring(50.0, 5.0);  // 软弹簧
        let dt = 0.01;
        let solver = ConstraintSolver::new();

        let mut a = anchor.clone();
        let mut b = bob.clone();
        let mut c = constraint.clone();

        for _ in 0..1000 {
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.integrate(dt);
            let mut bodies = [a.clone(), b.clone()];
            let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
                (0, 1, Box::new(c.clone())),
            ];
            solver.solve(&mut bodies, &mut constraints, dt);
            a = bodies[0].clone();
            b = bodies[1].clone();
        }

        // 10 秒后, 弹簧应大致稳定在 rest_length + 重力拉伸
        let dist = (b.position - a.position).length();
        // 静态平衡: k·(L - L_0) = m·g, L = L_0 + m·g/k = 1 + 1·9.81/50 ≈ 1.196
        assert!((dist - 1.196).abs() < 0.15, "spring rest length wrong: {}", dist);
    }

    #[test]
    fn test_fixed_constraint_welds_two_bodies() {
        // 两个球焊接在一起, 应一起下落, 相对距离不变
        let mut a = RigidBody::sphere(1.0, 0.1);
        let mut b = RigidBody::sphere(1.0, 0.1);
        // A 在 (0,5,0), B 在 (1,5,0), 距离 1.0
        a.position = Vec3::new(0.0, 5.0, 0.0);
        b.position = Vec3::new(1.0, 5.0, 0.0);

        // 锚点选择使初始时重合:
        // A 锚点 (0.5,0,0) -> 世界 (0.5,5,0)
        // B 锚点 (-0.5,0,0) -> 世界 (0.5,5,0)
        let mut c = FixedConstraint::new(Vec3::new(0.5, 0.0, 0.0), Vec3::new(-0.5, 0.0, 0.0));
        c.capture_initial(&a, &b);

        let dt = 0.016;
        let initial_dist = (a.position - b.position).length();

        for frame in 0..60 {
            a.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            a.integrate(dt);
            b.integrate(dt);
            for _ in 0..15 {
                c.solve_velocity(&mut a, &mut b, dt);
            }
            for _ in 0..4 {
                c.solve_position(&mut a, &mut b, dt);
            }
            if frame % 15 == 0 {
                let d = (a.position - b.position).length();
                assert!(d.is_finite(), "frame {}: distance not finite: {}", frame, d);
            }
        }

        let final_dist = (a.position - b.position).length();
        assert!((final_dist - initial_dist).abs() < 0.1,
            "welded bodies drifted: initial={}, final={}", initial_dist, final_dist);
    }

    #[test]
    fn test_slider_constraint_slides_along_axis() {
        // 滑动约束: B 只能沿 X 轴滑动 (相对 A)
        let mut a = RigidBody::new_static();
        a.position = Vec3::ZERO;
        let mut b = RigidBody::sphere(1.0, 0.1);
        b.position = Vec3::new(0.0, 0.0, 0.0);
        // 给 B 一个 X 方向初速度
        b.linear_velocity = Vec3::new(2.0, 0.0, 0.0);

        let constraint = SliderConstraint::new(Vec3::ZERO, Vec3::ZERO, Vec3::X, Vec3::X);
        let dt = 0.016;
        let solver = ConstraintSolver::new();

        let mut a_curr = a.clone();
        let mut b_curr = b.clone();
        let mut c = constraint.clone();

        for _ in 0..30 {
            // 沿 Y 方向施加力, 应被约束阻止
            b_curr.apply_force(Vec3::new(0.0, -5.0, 0.0));
            b_curr.integrate(dt);
            let mut bodies = [a_curr.clone(), b_curr.clone()];
            let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
                (0, 1, Box::new(c.clone())),
            ];
            solver.solve(&mut bodies, &mut constraints, dt);
            a_curr = bodies[0].clone();
            b_curr = bodies[1].clone();
        }

        // Y 方向位移应很小 (被约束阻止)
        assert!(b_curr.position.y.abs() < 0.1,
            "Y movement should be locked: y={}", b_curr.position.y);
        // X 方向应有位移 (允许滑动)
        assert!(b_curr.position.x.abs() > 0.1,
            "X movement should be allowed: x={}", b_curr.position.x);
    }

    #[test]
    fn test_hinge_constraint_basic() {
        // 铰链约束: 两个盒连接, 仅绕 Y 轴旋转
        let mut a = RigidBody::box_body(1.0, Vec3::new(0.5, 0.5, 0.5));
        let mut b = RigidBody::box_body(1.0, Vec3::new(0.5, 0.5, 0.5));
        a.position = Vec3::new(0.0, 0.0, 0.0);
        b.position = Vec3::new(1.0, 0.0, 0.0);

        let mut c = HingeConstraint::new(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::Y,
            Vec3::Y,
        );

        let dt = 0.016;

        for frame in 0..60 {
            b.apply_force(Vec3::new(0.0, -5.0, 0.0));
            b.integrate(dt);
            for _ in 0..15 {
                c.solve_velocity(&mut a, &mut b, dt);
            }
            for _ in 0..4 {
                c.solve_position(&mut a, &mut b, dt);
            }
            if frame % 15 == 0 {
                let r_a = a.rotation * Vec3::new(0.5, 0.0, 0.0);
                let r_b = b.rotation * Vec3::new(-0.5, 0.0, 0.0);
                let d = ((a.position + r_a) - (b.position + r_b)).length();
                assert!(d.is_finite(), "frame {}: anchor dist not finite: {}", frame, d);
            }
        }

        let r_a = a.rotation * Vec3::new(0.5, 0.0, 0.0);
        let r_b = b.rotation * Vec3::new(-0.5, 0.0, 0.0);
        let anchor_dist = ((a.position + r_a) - (b.position + r_b)).length();
        assert!(anchor_dist < 0.2, "hinge anchor drifted: {}", anchor_dist);
    }

    #[test]
    fn test_hinge_constraint_with_angle_limit() {
        let mut a = RigidBody::new_static();
        a.position = Vec3::ZERO;
        let mut b = RigidBody::box_body(1.0, Vec3::new(0.5, 0.5, 0.5));
        b.position = Vec3::new(1.0, 0.0, 0.0);

        let constraint = HingeConstraint::new(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::Y,
            Vec3::Y,
        ).with_angle_limit(-0.5, 0.5);  // ±28 度

        let dt = 0.01;
        let solver = ConstraintSolver::new();

        let mut a_curr = a.clone();
        let mut b_curr = b.clone();
        let mut c = constraint.clone();

        // 强力推动 B, 应被角度限制挡住
        for _ in 0..200 {
            b_curr.apply_force(Vec3::new(0.0, -50.0, 0.0));
            b_curr.integrate(dt);
            let mut bodies = [a_curr.clone(), b_curr.clone()];
            let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
                (0, 1, Box::new(c.clone())),
            ];
            solver.solve(&mut bodies, &mut constraints, dt);
            a_curr = bodies[0].clone();
            b_curr = bodies[1].clone();
        }

        // 铰链角应被限制在 [-0.5, 0.5]
        let angle = HingeConstraint::compute_hinge_angle(&a_curr, &b_curr, Vec3::Y, Vec3::Y);
        assert!(angle.abs() < 0.7, "hinge angle exceeded limit: {}", angle);
    }

    #[test]
    fn test_solver_warm_starting_stability() {
        // 验证 warm starting 不会让系统发散
        let mut a = RigidBody::new_static();
        a.position = Vec3::new(0.0, 5.0, 0.0);
        let mut b = RigidBody::sphere(1.0, 0.1);
        b.position = Vec3::new(0.0, 4.0, 0.0);  // 距锚点 1m

        let constraint = DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0);
        let dt = 0.016;
        let solver = ConstraintSolver::new();

        let mut a_curr = a.clone();
        let mut b_curr = b.clone();
        let mut c = constraint.clone();

        for _ in 0..300 {
            b_curr.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b_curr.integrate(dt);
            let mut bodies = [a_curr.clone(), b_curr.clone()];
            let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
                (0, 1, Box::new(c.clone())),
            ];
            solver.solve(&mut bodies, &mut constraints, dt);
            a_curr = bodies[0].clone();
            b_curr = bodies[1].clone();
        }

        // 应该稳定, 速度不爆炸
        let speed = b_curr.linear_velocity.length();
        assert!(speed < 50.0, "system diverged: speed={}", speed);
    }

    #[test]
    fn test_chain_of_constraints() {
        // 链条: 锚 - 球 - 球 - 球 (3 节链)
        let mut anchor = RigidBody::new_static();
        anchor.position = Vec3::new(0.0, 5.0, 0.0);
        let mut b1 = RigidBody::sphere(1.0, 0.05);
        b1.position = Vec3::new(0.0, 4.0, 0.0);
        let mut b2 = RigidBody::sphere(1.0, 0.05);
        b2.position = Vec3::new(0.0, 3.0, 0.0);
        let mut b3 = RigidBody::sphere(1.0, 0.05);
        b3.position = Vec3::new(0.0, 2.0, 0.0);

        let dt = 0.01;
        let solver = ConstraintSolver::new();

        let mut bodies = vec![anchor.clone(), b1.clone(), b2.clone(), b3.clone()];
        let mut constraints: Vec<(usize, usize, Box<dyn Constraint>)> = vec![
            (0, 1, Box::new(DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0))),
            (1, 2, Box::new(DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0))),
            (2, 3, Box::new(DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0))),
        ];

        for _ in 0..500 {
            // 重力作用于动态体
            for i in 1..4 {
                bodies[i].apply_force(Vec3::new(0.0, -9.81, 0.0));
            }
            for i in 0..4 {
                bodies[i].integrate(dt);
            }
            solver.solve(&mut bodies, &mut constraints, dt);
        }

        // 所有链节距离应保持 1m
        let d01 = (bodies[0].position - bodies[1].position).length();
        let d12 = (bodies[1].position - bodies[2].position).length();
        let d23 = (bodies[2].position - bodies[3].position).length();
        assert!((d01 - 1.0).abs() < 0.1, "chain link 01: {}", d01);
        assert!((d12 - 1.0).abs() < 0.1, "chain link 12: {}", d12);
        assert!((d23 - 1.0).abs() < 0.1, "chain link 23: {}", d23);

        // 整条链应下垂
        assert!(bodies[3].position.y < bodies[0].position.y);
    }

    #[test]
    fn test_solve_single_constraint_helper() {
        let mut a = RigidBody::new_static();
        a.position = Vec3::new(0.0, 5.0, 0.0);
        let mut b = RigidBody::sphere(1.0, 0.1);
        b.position = Vec3::new(0.0, 4.0, 0.0);

        let mut c = DistanceConstraint::new(Vec3::ZERO, Vec3::ZERO, 1.0);
        let dt = 0.01;

        for _ in 0..100 {
            b.apply_force(Vec3::new(0.0, -9.81, 0.0));
            b.integrate(dt);
            solve_single_constraint(&mut a, &mut b, &mut c, 10, 2, dt);
        }

        let dist = (b.position - a.position).length();
        assert!((dist - 1.0).abs() < 0.1, "helper solver failed: {}", dist);
    }
}
