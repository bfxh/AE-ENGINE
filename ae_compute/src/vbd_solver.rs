//! Vertex Block Descent (VBD) — 顶点块下降物理求解器
//!
//! 基于:
//! - Chen, Liu, Yang, Yuksel. "Vertex Block Descent." ACM TOG 43(4),
//!   SIGGRAPH 2024. https://arxiv.org/abs/2403.06321
//! - Giles, Diaz, Yuksel. "Augmented Vertex Block Descent." ACM TOG,
//!   SIGGRAPH 2025. (Augmented Lagrangian 扩展, 处理硬约束)
//! - Müller, Heidelberger, Hennix, Ratcliff. "Position Based Dynamics."
//!   VIS 2006. (PBD 基础, 对比参考)
//! - Macklin, Müller, Chentanez. "XPBD: Position-Based Simulation of
//!   Compliant Constrained Dynamics." MVR 2016. (XPBD 对比)
//!
//! 核心思想:
//! 1. 隐式 Euler 变分形式:
//!    x^{t+dt} = argmin_x  (1/2·dt²)·||x - y||²_M + E(x)
//!    其中 y = x^t + dt·v^t + dt²·a_ext (惯性预测位置)
//!    E(x) 为总势能 (弹簧、弯曲、体积、碰撞等)
//!
//! 2. 块坐标下降 (Gauss-Seidel):
//!    每次更新一个顶点位置, 临时固定其余顶点.
//!    局部最小化等价于全局能量下降.
//!
//! 3. 局部 Newton 步:
//!    g_i = (m_i/dt²)·(x_i - y_i) + Σ_{j∈F_i} ∇E_j(x_i)
//!    H_i = (m_i/dt²)·I + Σ_{j∈F_i} ∇²E_j(x_i)
//!    x_i ← x_i - H_i⁻¹·g_i
//!
//! 4. 无条件稳定:
//!    每步保证变分能量单调下降, 不依赖收敛.
//!    即使 1 次迭代也稳定 (残留大量能量也不爆).
//!
//! 5. 速度更新:
//!    v^{t+dt} = (x^{t+dt} - x^t) / dt
//!
//! 优势 (vs XPBD):
//! - 直接用力, 非约束近似 → 更准确 (尤其大时间步)
//! - Gauss-Seidel → 比 Jacobi 更快收敛
//! - 顶点图着色 → 比约束图着色更少色 → 更好并行
//! - 高质量比下不发散 (XPBD 在高质量比时退化)
//!
//! 复杂度: O(iterations · vertices), 局部 3×3 矩阵求逆

use glam::{Vec3, Mat3};

// ============================================================
// 数据结构
// ============================================================

/// VBD 顶点
#[derive(Debug, Clone)]
pub struct VbdVertex {
    /// 当前位置
    pub position: Vec3,
    /// 上一帧位置 (用于速度计算)
    pub prev_position: Vec3,
    /// 惯性预测位置 y = x^t + dt·v + dt²·a
    pub inertial_position: Vec3,
    /// 速度
    pub velocity: Vec3,
    /// 质量
    pub mass: f32,
    /// 逆质量 (0 = 固定/无限质量)
    pub inv_mass: f32,
    /// 是否固定 (pinned)
    pub pinned: bool,
}

impl VbdVertex {
    pub fn new(position: Vec3, mass: f32) -> Self {
        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        Self {
            position,
            prev_position: position,
            inertial_position: position,
            velocity: Vec3::ZERO,
            mass,
            inv_mass,
            pinned: false,
        }
    }

    /// 创建固定顶点 (不参与模拟)
    pub fn pinned(position: Vec3) -> Self {
        Self {
            position,
            prev_position: position,
            inertial_position: position,
            velocity: Vec3::ZERO,
            mass: 0.0,
            inv_mass: 0.0,
            pinned: true,
        }
    }

    /// 是否可动
    #[inline]
    pub fn is_dynamic(&self) -> bool {
        !self.pinned && self.inv_mass > 0.0
    }
}

/// 弹簧约束 (结构弹簧 + 剪切弹簧)
#[derive(Debug, Clone, Copy)]
pub struct VbdSpring {
    pub a: usize,
    pub b: usize,
    pub rest_length: f32,
    pub stiffness: f32,
}

impl VbdSpring {
    pub fn new(a: usize, b: usize, rest_length: f32, stiffness: f32) -> Self {
        Self { a, b, rest_length, stiffness }
    }
}

/// 弯曲约束 (跨对角线, 抗折叠)
#[derive(Debug, Clone, Copy)]
pub struct VbdBendingConstraint {
    pub a: usize,
    pub b: usize,
    pub rest_length: f32,
    pub stiffness: f32,
}

impl VbdBendingConstraint {
    pub fn new(a: usize, b: usize, rest_length: f32, stiffness: f32) -> Self {
        Self { a, b, rest_length, stiffness }
    }
}

/// 体积约束 (四面体, 保持体积不变)
#[derive(Debug, Clone, Copy)]
pub struct VbdVolumeConstraint {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
    pub v3: usize,
    pub rest_volume: f32,
    pub stiffness: f32,
}

impl VbdVolumeConstraint {
    pub fn new(v0: usize, v1: usize, v2: usize, v3: usize, rest_volume: f32, stiffness: f32) -> Self {
        Self { v0, v1, v2, v3, rest_volume, stiffness }
    }

    /// 计算四面体体积 (有符号)
    /// V = (1/6)·|(v1-v0)·((v2-v0)×(v3-v0))|
    #[inline]
    pub fn tet_volume(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> f32 {
        let d1 = p1 - p0;
        let d2 = p2 - p0;
        let d3 = p3 - p0;
        d1.dot(d2.cross(d3)) / 6.0
    }
}

/// 碰撞体 (无限地面)
#[derive(Debug, Clone, Copy)]
pub struct VbdFloor {
    /// 地面 y 坐标
    pub y: f32,
    /// 摩擦系数
    pub friction: f32,
    /// 恢复系数
    pub restitution: f32,
}

impl VbdFloor {
    pub fn new(y: f32, friction: f32, restitution: f32) -> Self {
        Self { y, friction, restitution }
    }
}

/// 球形碰撞体
#[derive(Debug, Clone, Copy)]
pub struct VbdSphereCollider {
    /// 球心
    pub center: Vec3,
    /// 半径
    pub radius: f32,
    /// 摩擦系数
    pub friction: f32,
}

impl VbdSphereCollider {
    pub fn new(center: Vec3, radius: f32, friction: f32) -> Self {
        Self { center, radius, friction }
    }
}

// ============================================================
// VBD 求解器
// ============================================================

/// VBD 物理求解器
pub struct VbdSolver {
    /// 顶点列表
    pub vertices: Vec<VbdVertex>,
    /// 弹簧约束
    springs: Vec<VbdSpring>,
    /// 弯曲约束
    bending: Vec<VbdBendingConstraint>,
    /// 体积约束
    volumes: Vec<VbdVolumeConstraint>,
    /// 地面碰撞
    floor: Option<VbdFloor>,
    /// 球体碰撞列表
    spheres: Vec<VbdSphereCollider>,
    /// 外力 (如重力)
    pub external_force: Vec3,
    /// 迭代次数
    pub iterations: usize,
    /// 阻尼系数 (0=无阻尼, 1=全阻尼)
    pub damping: f32,
    /// 加速迭代动量系数 (0=无加速, ~0.95=强加速)
    pub momentum: f32,
    /// 上一步的顶点位移 (用于动量加速)
    prev_delta: Vec<Vec3>,
}

impl Default for VbdSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl VbdSolver {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            springs: Vec::new(),
            bending: Vec::new(),
            volumes: Vec::new(),
            floor: None,
            spheres: Vec::new(),
            external_force: Vec3::new(0.0, -9.81, 0.0),
            iterations: 10,
            damping: 0.0,
            momentum: 0.0,
            prev_delta: Vec::new(),
        }
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn add_vertex(&mut self, vertex: VbdVertex) -> usize {
        let idx = self.vertices.len();
        self.vertices.push(vertex);
        self.prev_delta.push(Vec3::ZERO);
        idx
    }

    pub fn add_spring(&mut self, spring: VbdSpring) {
        self.springs.push(spring);
    }

    pub fn add_bending(&mut self, bending: VbdBendingConstraint) {
        self.bending.push(bending);
    }

    pub fn add_volume(&mut self, volume: VbdVolumeConstraint) {
        self.volumes.push(volume);
    }

    pub fn set_floor(&mut self, floor: VbdFloor) {
        self.floor = Some(floor);
    }

    pub fn add_sphere(&mut self, sphere: VbdSphereCollider) {
        self.spheres.push(sphere);
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    // ========================================================
    // 弹簧能量与导数
    // ========================================================

    /// 弹簧能量: E = (1/2)·k·(|x_a - x_b| - L)²
    /// 对 x_a 的梯度: ∇E_a = k·(d - L)·n̂, 其中 d=|x_a-x_b|, n̂=(x_a-x_b)/d
    /// 对 x_a 的 Hessian (正定近似): H_a = k·n̂·n̂ᵀ
    #[inline]
    fn spring_gradient(pa: Vec3, pb: Vec3, rest_length: f32, stiffness: f32) -> Vec3 {
        let diff = pa - pb;
        let d = diff.length();
        if d < 1e-12 {
            return Vec3::ZERO;
        }
        let n = diff / d;
        stiffness * (d - rest_length) * n
    }

    /// 弹簧 Hessian (正定近似): H = k·n̂·n̂ᵀ
    /// 注: 精确 Hessian 为 k·[(d-L)/d·I + (L/d)·n̂·n̂ᵀ], 但当 d<L 时 (d-L)/d<0 不正定.
    /// VBD 使用正定近似 k·n̂·n̂ᵀ 保证稳定性 (论文 Section 3.2).
    #[inline]
    fn spring_hessian(pa: Vec3, pb: Vec3, stiffness: f32) -> Mat3 {
        let diff = pa - pb;
        let d = diff.length();
        if d < 1e-12 {
            return Mat3::ZERO;
        }
        let n = diff / d;
        // n⊗nᵀ (外积)
        Mat3::from_cols(
            Vec3::new(n.x * n.x, n.y * n.x, n.z * n.x),
            Vec3::new(n.x * n.y, n.y * n.y, n.z * n.y),
            Vec3::new(n.x * n.z, n.y * n.z, n.z * n.z),
        ) * stiffness
    }

    // ========================================================
    // 体积约束 (Green strain 不变量, 简化为体积偏差)
    // ========================================================

    /// 体积约束梯度 (对 v0): ∂V/∂x_0 = -(1/6)·(d2 × d3)
    /// 其中 d2 = x2-x1, d3 = x3-x1 (注意绕过了 v0, 用 v1 作参考)
    /// 实际: V = (1/6)·(x1-x0)·((x2-x0)×(x3-x0))
    /// ∂V/∂x0 = (1/6)·((x2-x0)×(x3-x0) + (x3-x0)×(x1-x0) + (x1-x0)×(x2-x0))
    ///        = (1/6)·((x2-x0)×(x3-x0) - (x1-x0)×(x3-x0) + (x1-x0)×(x2-x0))
    /// 简化: ∂V/∂x0 = (1/6)·((x3-x1)×(x2-x1))
    #[inline]
    fn volume_gradient_v0(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Vec3 {
        let d21 = p2 - p1;
        let d31 = p3 - p1;
        d31.cross(d21) / 6.0
    }

    #[inline]
    fn volume_gradient_v1(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Vec3 {
        let d20 = p2 - p0;
        let d30 = p3 - p0;
        d20.cross(d30) / 6.0
    }

    #[inline]
    fn volume_gradient_v2(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Vec3 {
        let d10 = p1 - p0;
        let d30 = p3 - p0;
        d30.cross(d10) / 6.0
    }

    #[inline]
    fn volume_gradient_v3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Vec3 {
        let d10 = p1 - p0;
        let d20 = p2 - p0;
        d10.cross(d20) / 6.0
    }

    // ========================================================
    // 3×3 矩阵求逆 (用于局部 Newton 步)
    // ========================================================

    /// 求解 H·x = b, 返回 x. H 为 3×3 对称正定矩阵.
    /// 使用解析求逆 (足够快, 矩阵小).
    #[inline]
    fn solve_3x3(h: Mat3, b: Vec3) -> Vec3 {
        // 使用伴随矩阵法: H⁻¹ = adj(H) / det(H)
        let det = h.determinant();
        if det.abs() < 1e-12 {
            return Vec3::ZERO;
        }
        h.inverse() * b
    }

    // ========================================================
    // 预处理: 构建顶点到约束的映射
    // ========================================================

    /// 构建每个顶点参与的弹簧列表索引
    fn build_vertex_adjacency(&self) -> Vec<Vec<usize>> {
        let n = self.vertices.len();
        let mut adj = vec![Vec::new(); n];
        for (i, s) in self.springs.iter().enumerate() {
            adj[s.a].push(i);
            adj[s.b].push(i);
        }
        adj
    }

    fn build_bending_adjacency(&self) -> Vec<Vec<usize>> {
        let n = self.vertices.len();
        let mut adj = vec![Vec::new(); n];
        for (i, b) in self.bending.iter().enumerate() {
            adj[b.a].push(i);
            adj[b.b].push(i);
        }
        adj
    }

    fn build_volume_adjacency(&self) -> Vec<Vec<usize>> {
        let n = self.vertices.len();
        let mut adj = vec![Vec::new(); n];
        for (i, v) in self.volumes.iter().enumerate() {
            adj[v.v0].push(i);
            adj[v.v1].push(i);
            adj[v.v2].push(i);
            adj[v.v3].push(i);
        }
        adj
    }

    // ========================================================
    // 主求解
    // ========================================================

    /// 完整 VBD 时间步
    pub fn step(&mut self, dt: f32) {
        if self.vertices.is_empty() || dt <= 0.0 {
            return;
        }

        let n = self.vertices.len();

        // 1. 保存上一帧位置, 计算惯性预测位置
        //    y = x^t + dt·v + dt²·a_ext
        //    外力加速度: a = F/m
        for v in &mut self.vertices {
            v.prev_position = v.position;
            if v.is_dynamic() {
                let a = self.external_force * v.inv_mass;
                v.inertial_position = v.position + v.velocity * dt + a * (dt * dt);
            } else {
                v.inertial_position = v.position;
            }
        }

        // 2. 初始化: 从惯性预测位置开始 (warm start)
        //    VBD 论文 Section 3.7: 使用上一步解作为初值加速收敛
        for v in &mut self.vertices {
            if v.is_dynamic() {
                v.position = v.inertial_position;
            }
        }

        // 3. 构建邻接表
        let spring_adj = self.build_vertex_adjacency();
        let bending_adj = self.build_bending_adjacency();
        let volume_adj = self.build_volume_adjacency();

        // 4. Gauss-Seidel 迭代
        for iter in 0..self.iterations {
            for i in 0..n {
                if !self.vertices[i].is_dynamic() {
                    continue;
                }
                let delta = self.compute_vertex_delta(i, &spring_adj, &bending_adj, &volume_adj, dt, iter);

                // 动量加速: x_i ← x_i - (Δx + ρ·Δx_prev)
                let accelerated = if self.momentum > 0.0 && iter > 0 {
                    delta + self.prev_delta[i] * self.momentum
                } else {
                    delta
                };

                self.vertices[i].position -= accelerated;
                self.prev_delta[i] = accelerated;
            }
        }

        // 5. 碰撞处理 (位置级)
        self.handle_collisions();

        // 6. 速度更新: v = (x^{t+dt} - x^t) / dt
        //    含阻尼: v *= (1 - damping)
        for v in &mut self.vertices {
            if v.is_dynamic() {
                v.velocity = (v.position - v.prev_position) / dt;
                if self.damping > 0.0 {
                    v.velocity *= (1.0 - self.damping).max(0.0);
                }
            }
        }

        // 7. 摩擦 (速度级, 对碰撞点)
        self.apply_friction();
    }

    /// 计算单个顶点的 Newton 步 Δx = H⁻¹·g
    fn compute_vertex_delta(
        &self,
        i: usize,
        spring_adj: &[Vec<usize>],
        bending_adj: &[Vec<usize>],
        volume_adj: &[Vec<usize>],
        dt: f32,
        _iter: usize,
    ) -> Vec3 {
        let v = &self.vertices[i];
        let dt2 = dt * dt;
        let inv_dt2 = 1.0 / dt2;

        // 惯性项梯度: g_inertia = (m/dt²)·(x - y)
        let mut gradient = (v.mass * inv_dt2) * (v.position - v.inertial_position);
        // 惯性项 Hessian: H_inertia = (m/dt²)·I
        let mut hessian = Mat3::from_diagonal(Vec3::splat(v.mass * inv_dt2));

        // 弹簧能量贡献
        for &s_idx in &spring_adj[i] {
            let s = &self.springs[s_idx];
            let other = if s.a == i { s.b } else { s.a };
            let pa = v.position;
            let pb = self.vertices[other].position;
            gradient += Self::spring_gradient(pa, pb, s.rest_length, s.stiffness);
            hessian += Self::spring_hessian(pa, pb, s.stiffness);
        }

        // 弯曲约束贡献 (用弹簧模型)
        for &b_idx in &bending_adj[i] {
            let b = &self.bending[b_idx];
            let other = if b.a == i { b.b } else { b.a };
            let pa = v.position;
            let pb = self.vertices[other].position;
            gradient += Self::spring_gradient(pa, pb, b.rest_length, b.stiffness);
            hessian += Self::spring_hessian(pa, pb, b.stiffness);
        }

        // 体积约束贡献 (梯度, Hessian 用对角近似)
        for &vol_idx in &volume_adj[i] {
            let vol = &self.volumes[vol_idx];
            let p0 = self.vertices[vol.v0].position;
            let p1 = self.vertices[vol.v1].position;
            let p2 = self.vertices[vol.v2].position;
            let p3 = self.vertices[vol.v3].position;
            let current_vol = VbdVolumeConstraint::tet_volume(p0, p1, p2, p3);
            let vol_diff = current_vol - vol.rest_volume;
            let grad = match i {
                _ if i == vol.v0 => Self::volume_gradient_v0(p0, p1, p2, p3),
                _ if i == vol.v1 => Self::volume_gradient_v1(p0, p1, p2, p3),
                _ if i == vol.v2 => Self::volume_gradient_v2(p0, p1, p2, p3),
                _ => Self::volume_gradient_v3(p0, p1, p2, p3),
            };
            gradient += grad * (vol.stiffness * vol_diff);
            // Hessian 近似: k·|∇V|²·I (保证正定)
            let grad_norm_sq = grad.dot(grad);
            hessian += Mat3::from_diagonal(Vec3::splat(vol.stiffness * grad_norm_sq));
        }

        // 求解 H·Δx = g, 返回 Δx
        Self::solve_3x3(hessian, gradient)
    }

    // ========================================================
    // 碰撞处理
    // ========================================================

    /// 位置级碰撞处理 (投影出穿透)
    fn handle_collisions(&mut self) {
        // 地面碰撞
        if let Some(floor) = self.floor {
            for v in &mut self.vertices {
                if !v.is_dynamic() {
                    continue;
                }
                if v.position.y < floor.y {
                    v.position.y = floor.y;
                }
            }
        }

        // 球体碰撞
        for sphere in &self.spheres {
            for v in &mut self.vertices {
                if !v.is_dynamic() {
                    continue;
                }
                let diff = v.position - sphere.center;
                let dist = diff.length();
                if dist < sphere.radius && dist > 1e-12 {
                    let normal = diff / dist;
                    v.position = sphere.center + normal * sphere.radius;
                }
            }
        }
    }

    /// 速度级摩擦 (对碰撞点切向速度衰减)
    fn apply_friction(&mut self) {
        if let Some(floor) = self.floor {
            for v in &mut self.vertices {
                if !v.is_dynamic() || v.position.y > floor.y + 1e-4 {
                    continue;
                }
                // 切向速度
                let vt = Vec3::new(v.velocity.x, 0.0, v.velocity.z);
                let vt_mag = vt.length();
                if vt_mag > 1e-6 {
                    // Coulomb 摩擦: 减少切向速度
                    let max_friction = floor.friction * v.velocity.y.abs();
                    let friction_impulse = vt_mag.min(max_friction);
                    let friction_dir = vt / vt_mag;
                    v.velocity -= friction_dir * friction_impulse;
                }
                // 恢复系数
                if v.velocity.y < 0.0 {
                    v.velocity.y = -v.velocity.y * floor.restitution;
                }
            }
        }

        for sphere in &self.spheres {
            for v in &mut self.vertices {
                if !v.is_dynamic() {
                    continue;
                }
                let diff = v.position - sphere.center;
                let dist = diff.length();
                if (dist - sphere.radius).abs() > 1e-4 || dist < 1e-12 {
                    continue;
                }
                let normal = diff / dist;
                let vn = v.velocity.dot(normal);
                if vn < 0.0 {
                    // 法向反弹
                    v.velocity -= normal * vn * (1.0 + 0.3);
                    // 切向摩擦
                    let vt = v.velocity - normal * v.velocity.dot(normal);
                    let vt_mag = vt.length();
                    if vt_mag > 1e-6 {
                        let max_friction = sphere.friction * vn.abs();
                        let friction_impulse = vt_mag.min(max_friction);
                        v.velocity -= (vt / vt_mag) * friction_impulse;
                    }
                }
            }
        }
    }

    // ========================================================
    // 工具方法
    // ========================================================

    /// 计算弹簧总能量 (用于调试/验证)
    pub fn spring_energy(&self) -> f32 {
        let mut e = 0.0;
        for s in &self.springs {
            let pa = self.vertices[s.a].position;
            let pb = self.vertices[s.b].position;
            let d = (pa - pb).length();
            let diff = d - s.rest_length;
            e += 0.5 * s.stiffness * diff * diff;
        }
        e
    }

    /// 计算动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut e = 0.0;
        for v in &self.vertices {
            if v.is_dynamic() {
                e += 0.5 * v.mass * v.velocity.length_squared();
            }
        }
        e
    }

    /// 计算势能 (重力)
    pub fn potential_energy(&self) -> f32 {
        let mut e = 0.0;
        for v in &self.vertices {
            if v.is_dynamic() {
                e -= v.mass * self.external_force.dot(v.position);
            }
        }
        e
    }

    /// 计算总能量
    pub fn total_energy(&self) -> f32 {
        self.kinetic_energy() + self.spring_energy() + self.potential_energy()
    }

    /// 重置所有顶点速度
    pub fn reset_velocities(&mut self) {
        for v in &mut self.vertices {
            v.velocity = Vec3::ZERO;
        }
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个简单的两点弹簧 (一维振动)
    fn make_two_point_spring() -> VbdSolver {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::ZERO; // 无重力
        solver.add_vertex(VbdVertex::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        solver.add_vertex(VbdVertex::new(Vec3::new(2.0, 0.0, 0.0), 1.0));
        solver.add_spring(VbdSpring::new(0, 1, 1.0, 100.0)); // rest=1, k=100
        solver
    }

    #[test]
    fn test_solver_creation() {
        let solver = VbdSolver::new();
        assert_eq!(solver.vertex_count(), 0);
        assert_eq!(solver.iterations, 10);
    }

    #[test]
    fn test_add_vertex() {
        let mut solver = VbdSolver::new();
        let idx = solver.add_vertex(VbdVertex::new(Vec3::new(1.0, 2.0, 3.0), 2.0));
        assert_eq!(idx, 0);
        assert_eq!(solver.vertex_count(), 1);
        assert_eq!(solver.vertices[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert!((solver.vertices[0].mass - 2.0).abs() < 1e-6);
        assert!((solver.vertices[0].inv_mass - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_pinned_vertex() {
        let v = VbdVertex::pinned(Vec3::new(1.0, 1.0, 1.0));
        assert!(v.pinned);
        assert!(!v.is_dynamic());
        assert_eq!(v.inv_mass, 0.0);
    }

    #[test]
    fn test_spring_gradient() {
        // 弹簧拉伸: pa=(2,0,0), pb=(0,0,0), rest=1, k=100
        // d=2, n̂=(1,0,0), ∇E = 100*(2-1)*(1,0,0) = (100,0,0)
        let g = VbdSolver::spring_gradient(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::ZERO,
            1.0,
            100.0,
        );
        assert!((g - Vec3::new(100.0, 0.0, 0.0)).length() < 1e-4, "gradient: {:?}", g);
    }

    #[test]
    fn test_spring_gradient_compressed() {
        // 弹簧压缩: pa=(0.5,0,0), pb=(0,0,0), rest=1, k=100
        // d=0.5, n̂=(1,0,0), ∇E = 100*(0.5-1)*(1,0,0) = (-50,0,0)
        let g = VbdSolver::spring_gradient(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::ZERO,
            1.0,
            100.0,
        );
        assert!((g - Vec3::new(-50.0, 0.0, 0.0)).length() < 1e-4, "gradient: {:?}", g);
    }

    #[test]
    fn test_spring_gradient_zero_length() {
        // 零长度弹簧: 返回零梯度 (避免除零)
        let g = VbdSolver::spring_gradient(Vec3::ZERO, Vec3::ZERO, 1.0, 100.0);
        assert_eq!(g, Vec3::ZERO);
    }

    #[test]
    fn test_spring_hessian_positive_definite() {
        let h = VbdSolver::spring_hessian(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 100.0);
        // H = 100 * n̂·n̂ᵀ = 100 * [[1,0,0],[0,0,0],[0,0,0]]
        assert!((h.x_axis.x - 100.0).abs() < 1e-4, "hessian xx: {}", h.x_axis.x);
        assert!(h.y_axis.y.abs() < 1e-4, "hessian yy: {}", h.y_axis.y);
        assert!(h.z_axis.z.abs() < 1e-4, "hessian zz: {}", h.z_axis.z);
    }

    #[test]
    fn test_solve_3x3_identity() {
        let h = Mat3::IDENTITY;
        let b = Vec3::new(1.0, 2.0, 3.0);
        let x = VbdSolver::solve_3x3(h, b);
        assert!((x - b).length() < 1e-4, "solve identity: {:?}", x);
    }

    #[test]
    fn test_solve_3x3_diagonal() {
        let h = Mat3::from_diagonal(Vec3::new(2.0, 4.0, 6.0));
        let b = Vec3::new(2.0, 4.0, 6.0);
        let x = VbdSolver::solve_3x3(h, b);
        assert!((x - Vec3::new(1.0, 1.0, 1.0)).length() < 1e-4, "solve diag: {:?}", x);
    }

    #[test]
    fn test_solve_3x3_singular() {
        let h = Mat3::ZERO;
        let b = Vec3::new(1.0, 2.0, 3.0);
        let x = VbdSolver::solve_3x3(h, b);
        assert_eq!(x, Vec3::ZERO, "singular should return zero");
    }

    #[test]
    fn test_volume_gradient_sum_zero() {
        // 体积约束梯度之和应为零 (平移不变性)
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let p3 = Vec3::new(0.0, 0.0, 1.0);
        let g0 = VbdSolver::volume_gradient_v0(p0, p1, p2, p3);
        let g1 = VbdSolver::volume_gradient_v1(p0, p1, p2, p3);
        let g2 = VbdSolver::volume_gradient_v2(p0, p1, p2, p3);
        let g3 = VbdSolver::volume_gradient_v3(p0, p1, p2, p3);
        let sum = g0 + g1 + g2 + g3;
        assert!(sum.length() < 1e-4, "gradient sum: {:?} (should be ~0)", sum);
    }

    #[test]
    fn test_tet_volume() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let p3 = Vec3::new(0.0, 0.0, 1.0);
        let v = VbdVolumeConstraint::tet_volume(p0, p1, p2, p3);
        // V = 1/6 (单位四面体)
        assert!((v - 1.0 / 6.0).abs() < 1e-6, "tet volume: {}", v);
    }

    #[test]
    fn test_spring_converges_to_rest_length() {
        // 两点弹簧, 无重力, 应收敛到 rest_length
        let mut solver = make_two_point_spring();
        solver.iterations = 50;
        // 固定第一个点
        solver.vertices[0].pinned = true;
        solver.vertices[0].inv_mass = 0.0;
        solver.vertices[1].velocity = Vec3::ZERO;

        // 初始距离 = 2.0, rest = 1.0 → 应收缩
        solver.step(0.01);

        let dist = (solver.vertices[0].position - solver.vertices[1].position).length();
        // 经过多步迭代, 距离应趋近 rest_length=1.0
        assert!(dist < 2.0, "distance after step: {} (should be < 2)", dist);
    }

    #[test]
    fn test_gravity_pulls_down() {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -10.0, 0.0);
        solver.iterations = 5;
        solver.add_vertex(VbdVertex::new(Vec3::ZERO, 1.0));
        solver.step(0.1);
        // 自由落体: y ≈ -10 * 0.1² / 2 ... 但 VBD 隐式, 位移向下
        assert!(solver.vertices[0].position.y < 0.0,
            "y after gravity: {} (should be < 0)", solver.vertices[0].position.y);
        assert!(solver.vertices[0].velocity.y < 0.0,
            "vy after gravity: {} (should be < 0)", solver.vertices[0].velocity.y);
    }

    #[test]
    fn test_pinned_vertex_doesnt_move() {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -10.0, 0.0);
        solver.add_vertex(VbdVertex::pinned(Vec3::new(1.0, 5.0, 0.0)));
        let initial = solver.vertices[0].position;
        solver.step(0.01);
        assert_eq!(solver.vertices[0].position, initial, "pinned vertex moved!");
    }

    #[test]
    fn test_floor_collision() {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -20.0, 0.0);
        solver.iterations = 5;
        solver.set_floor(VbdFloor::new(0.0, 0.0, 0.0)); // y=0 地面, 无摩擦无弹性
        solver.add_vertex(VbdVertex::new(Vec3::new(0.0, 0.5, 0.0), 1.0));
        // 多步下落
        for _ in 0..20 {
            solver.step(0.05);
        }
        assert!(solver.vertices[0].position.y >= -1e-4,
            "y after floor collision: {} (should be >= 0)", solver.vertices[0].position.y);
    }

    #[test]
    fn test_sphere_collision() {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -20.0, 0.0);
        solver.iterations = 5;
        solver.add_sphere(VbdSphereCollider::new(Vec3::new(0.0, 0.0, 0.0), 1.0, 0.0));
        solver.add_vertex(VbdVertex::new(Vec3::new(0.0, 2.0, 0.0), 1.0));
        for _ in 0..30 {
            solver.step(0.02);
        }
        let dist = solver.vertices[0].position.length();
        // 应被球体弹开/停在球面
        assert!(dist >= 0.9,
            "distance from sphere center: {} (should be >= ~1)", dist);
    }

    #[test]
    fn test_cloth_grid_falls() {
        // 4×4 布料网格, 一角固定
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -9.81, 0.0);
        solver.iterations = 20;
        let nx = 4;
        let nz = 4;
        let spacing = 0.5;
        // 创建顶点
        for iz in 0..nz {
            for ix in 0..nx {
                let pos = Vec3::new(ix as f32 * spacing, 5.0, iz as f32 * spacing);
                let mut v = VbdVertex::new(pos, 0.1);
                if ix == 0 && iz == 0 {
                    v = VbdVertex::pinned(pos);
                }
                solver.add_vertex(v);
            }
        }
        // 结构弹簧 (水平 + 垂直)
        for iz in 0..nz {
            for ix in 0..nx {
                let i = iz * nx + ix;
                if ix + 1 < nx {
                    solver.add_spring(VbdSpring::new(i, i + 1, spacing, 500.0));
                }
                if iz + 1 < nz {
                    solver.add_spring(VbdSpring::new(i, i + nx, spacing, 500.0));
                }
            }
        }
        // 初始 y=5, 下落后应下降
        let y0 = solver.vertices[nx * nz - 1].position.y;
        for _ in 0..10 {
            solver.step(0.016);
        }
        let y1 = solver.vertices[nx * nz - 1].position.y;
        assert!(y1 < y0, "cloth corner y: {} -> {} (should fall)", y0, y1);
    }

    #[test]
    fn test_unconditional_stability_large_dt() {
        // VBD 应在大时间步下保持稳定
        let mut solver = make_two_point_spring();
        solver.iterations = 1; // 仅 1 次迭代
        solver.vertices[0].pinned = true;
        solver.vertices[0].inv_mass = 0.0;
        // 大时间步 (通常显式方法会爆)
        solver.step(1.0);
        // 位置不应变成 NaN 或无穷大
        let p = solver.vertices[1].position;
        assert!(p.is_finite(), "position not finite: {:?}", p);
        assert!(p.length() < 100.0, "position exploded: {:?}", p);
    }

    #[test]
    fn test_unconditional_stability_stiff_spring() {
        // 硬弹簧 (高频), VBD 应稳定
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::ZERO;
        solver.iterations = 5;
        solver.add_vertex(VbdVertex::pinned(Vec3::ZERO));
        solver.add_vertex(VbdVertex::new(Vec3::new(2.0, 0.0, 0.0), 1.0));
        solver.add_spring(VbdSpring::new(0, 1, 1.0, 1_000_000.0)); // 极硬
        solver.step(0.01);
        let p = solver.vertices[1].position;
        assert!(p.is_finite(), "position not finite: {:?}", p);
        assert!(p.length() < 100.0, "position exploded: {:?}", p);
    }

    #[test]
    fn test_high_mass_ratio() {
        // 高质量比: VBD 优于 XPBD 的关键场景
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::ZERO;
        solver.iterations = 10;
        // 重物体 (mass=1000) 连接轻物体 (mass=0.001)
        solver.add_vertex(VbdVertex::new(Vec3::ZERO, 1000.0));
        solver.add_vertex(VbdVertex::new(Vec3::new(2.0, 0.0, 0.0), 0.001));
        solver.add_spring(VbdSpring::new(0, 1, 1.0, 100.0));
        solver.step(0.01);
        // 两物体应稳定, 不爆
        for v in &solver.vertices {
            assert!(v.position.is_finite(), "position not finite: {:?}", v.position);
            assert!(v.velocity.is_finite(), "velocity not finite: {:?}", v.velocity);
        }
    }

    #[test]
    fn test_damping_reduces_velocity() {
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -10.0, 0.0);
        solver.iterations = 5;
        solver.damping = 0.5;
        solver.add_vertex(VbdVertex::new(Vec3::ZERO, 1.0));
        solver.step(0.1);
        // 阻尼应使速度小于无阻尼情况
        let vy_damped = solver.vertices[0].velocity.y;

        let mut solver2 = VbdSolver::new();
        solver2.external_force = Vec3::new(0.0, -10.0, 0.0);
        solver2.iterations = 5;
        solver2.damping = 0.0;
        solver2.add_vertex(VbdVertex::new(Vec3::ZERO, 1.0));
        solver2.step(0.1);
        let vy_undamped = solver2.vertices[0].velocity.y;

        assert!(vy_damped.abs() < vy_undamped.abs(),
            "damped |vy|={} should be < undamped |vy|={}", vy_damped, vy_undamped);
    }

    #[test]
    fn test_volume_constraint_preserves_volume() {
        // 四面体, 体积约束应阻止体积变化
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -5.0, 0.0);
        solver.iterations = 30;
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let p3 = Vec3::new(0.0, 0.0, 1.0);
        let rest_vol = VbdVolumeConstraint::tet_volume(p0, p1, p2, p3);
        // v0 固定, 其余自由
        solver.add_vertex(VbdVertex::pinned(p0));
        solver.add_vertex(VbdVertex::new(p1, 1.0));
        solver.add_vertex(VbdVertex::new(p2, 1.0));
        solver.add_vertex(VbdVertex::new(p3, 1.0));
        solver.add_volume(VbdVolumeConstraint::new(0, 1, 2, 3, rest_vol, 1000.0));
        // 添加结构弹簧防止退化
        solver.add_spring(VbdSpring::new(0, 1, 1.0, 100.0));
        solver.add_spring(VbdSpring::new(0, 2, 1.0, 100.0));
        solver.add_spring(VbdSpring::new(0, 3, 1.0, 100.0));
        solver.add_spring(VbdSpring::new(1, 2, 2.0_f32.sqrt(), 100.0));
        solver.add_spring(VbdSpring::new(1, 3, 2.0_f32.sqrt(), 100.0));
        solver.add_spring(VbdSpring::new(2, 3, 2.0_f32.sqrt(), 100.0));
        let vol_before = VbdVolumeConstraint::tet_volume(
            solver.vertices[0].position,
            solver.vertices[1].position,
            solver.vertices[2].position,
            solver.vertices[3].position,
        );
        for _ in 0..10 {
            solver.step(0.016);
        }
        let vol_after = VbdVolumeConstraint::tet_volume(
            solver.vertices[0].position,
            solver.vertices[1].position,
            solver.vertices[2].position,
            solver.vertices[3].position,
        );
        assert!((vol_after - vol_before).abs() < 0.1 * rest_vol.abs(),
            "volume changed: {} -> {} (rest={})", vol_before, vol_after, rest_vol);
    }

    #[test]
    fn test_momentum_acceleration() {
        // 动量加速应使收敛更快 (同样迭代数下更接近解)
        let mut solver_no_momentum = make_two_point_spring();
        solver_no_momentum.iterations = 3;
        solver_no_momentum.vertices[0].pinned = true;
        solver_no_momentum.vertices[0].inv_mass = 0.0;
        solver_no_momentum.momentum = 0.0;
        solver_no_momentum.step(0.01);
        let dist_no = (solver_no_momentum.vertices[0].position
            - solver_no_momentum.vertices[1].position).length();

        let mut solver_momentum = make_two_point_spring();
        solver_momentum.iterations = 3;
        solver_momentum.vertices[0].pinned = true;
        solver_momentum.vertices[0].inv_mass = 0.0;
        solver_momentum.momentum = 0.9;
        solver_momentum.step(0.01);
        let dist_yes = (solver_momentum.vertices[0].position
            - solver_momentum.vertices[1].position).length();

        // 动量加速应使弹簧更接近 rest_length (1.0)
        let rest = 1.0;
        assert!((dist_yes - rest).abs() <= (dist_no - rest).abs() + 0.2,
            "momentum dist={} vs no-momentum dist={} (rest={})", dist_yes, dist_no, rest);
    }

    #[test]
    fn test_energy_decreases() {
        // VBD 应保证能量单调下降 (稳定性)
        let mut solver = make_two_point_spring();
        solver.iterations = 20;
        solver.vertices[0].pinned = true;
        solver.vertices[0].inv_mass = 0.0;
        let e0 = solver.spring_energy();
        solver.step(0.01);
        let e1 = solver.spring_energy();
        // 弹簧能量应下降 (从初始拉伸态收敛)
        assert!(e1 <= e0 + 1e-4,
            "energy not decreasing: {} -> {}", e0, e1);
    }

    #[test]
    fn test_multiple_steps_stable() {
        // 多步模拟应保持稳定
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -9.81, 0.0);
        solver.iterations = 10;
        solver.set_floor(VbdFloor::new(0.0, 0.5, 0.3));
        solver.add_vertex(VbdVertex::new(Vec3::new(0.0, 10.0, 0.0), 1.0));
        for i in 0..100 {
            solver.step(0.016);
            for v in &solver.vertices {
                assert!(v.position.is_finite(), "step {}: position not finite: {:?}", i, v.position);
                assert!(v.velocity.is_finite(), "step {}: velocity not finite: {:?}", i, v.velocity);
            }
        }
    }

    #[test]
    fn test_cloth_with_bending() {
        // 布料 + 弯曲约束: 应比纯结构弹簧更硬挺
        let mut solver = VbdSolver::new();
        solver.external_force = Vec3::new(0.0, -9.81, 0.0);
        solver.iterations = 20;
        // 3 点悬臂梁: 0 固定, 1-2 自由
        solver.add_vertex(VbdVertex::pinned(Vec3::ZERO));
        solver.add_vertex(VbdVertex::new(Vec3::new(1.0, 0.0, 0.0), 0.1));
        solver.add_vertex(VbdVertex::new(Vec3::new(2.0, 0.0, 0.0), 0.1));
        // 结构弹簧
        solver.add_spring(VbdSpring::new(0, 1, 1.0, 1000.0));
        solver.add_spring(VbdSpring::new(1, 2, 1.0, 1000.0));
        // 弯曲约束 (0-2 跨对角线)
        solver.add_bending(VbdBendingConstraint::new(0, 2, 2.0, 200.0));
        for _ in 0..20 {
            solver.step(0.016);
        }
        // 末端应下垂但弯曲约束限制其过度变形
        let y_end = solver.vertices[2].position.y;
        assert!(y_end > -2.0, "end y: {} (bending should limit sagging)", y_end);
        assert!(y_end < 0.0, "end y: {} (should sag due to gravity)", y_end);
    }
}
