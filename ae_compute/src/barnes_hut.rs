//! Barnes-Hut N-body Gravitational Solver (Octree)
//!
//! 基于 Barnes & Hut 1986 "A hierarchical O(N log N) force-calculation
//! algorithm" (Nature, 324(4):446-449).
//!
//! 万有引力 (牛顿):
//!   F = G · m₁ · m₂ · (r₂ - r₁) / |r|³
//!
//! 软化 (避免奇点):
//!   F = G · m₁ · m₂ · (r₂ - r₁) / (|r|² + ε²)^(3/2)
//!
//! Barnes-Hut 判据 (开角 θ):
//!   若 s/d < θ  (s = 节点边长, d = 节点质心到粒子距离)
//!   则用节点质心近似该节点所有粒子的引力, 否则递归展开子节点.
//!   θ = 0  → 直接求和 O(N²);  θ = 1  → 最粗近似.
//!
//! 时间积分: leapfrog kick-drift-kick (KDK), 2 阶辛积分, 能量长期稳定.
//!
//! 应用: 星系动力学, 太空游戏, 轨道力学, 粒子团簇, 软物质自引力.

use serde::{Deserialize, Serialize};

/// 万有引力常数 G (m³ kg⁻¹ s⁻²), CODATA 2018
pub const G: f32 = 6.67430e-11;
/// 1 天文单位 (m)
pub const AU: f32 = 1.495978707e11;
/// 太阳质量 (kg)
pub const M_SUN: f32 = 1.98892e30;
/// 地球质量 (kg)
pub const M_EARTH: f32 = 5.972e24;

/// 最大递归深度 (防止 stack overflow)
const MAX_DEPTH: usize = 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Body {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub mass: f32,
}

impl Body {
    pub fn new(position: [f32; 3], velocity: [f32; 3], mass: f32) -> Self {
        Body { position, velocity, mass }
    }
    pub fn at_rest(position: [f32; 3], mass: f32) -> Self {
        Body { position, velocity: [0.0; 3], mass }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OpeningMethod {
    /// 标准 Barnes-Hut: s/d < θ
    Angle,
    /// 简化距离判据: d > s/θ
    Distance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarnesHutConfig {
    /// 开角 θ (0..1, 越小越精确)
    pub theta: f32,
    /// 软化长度 ε (m), 避免 r→0 时力发散
    pub softening: f32,
    pub opening_method: OpeningMethod,
    /// 万有引力常数 (允许缩放, 默认 SI)
    pub gravity_constant: f32,
}

impl Default for BarnesHutConfig {
    fn default() -> Self {
        BarnesHutConfig {
            theta: 0.5,
            softening: 0.01,
            opening_method: OpeningMethod::Angle,
            gravity_constant: G,
        }
    }
}

impl BarnesHutConfig {
    pub fn unitless() -> Self {
        BarnesHutConfig {
            theta: 0.5,
            softening: 0.01,
            opening_method: OpeningMethod::Angle,
            gravity_constant: 1.0,
        }
    }
    pub fn solar_system() -> Self {
        BarnesHutConfig {
            theta: 0.5,
            softening: 1.0e9,
            opening_method: OpeningMethod::Angle,
            gravity_constant: G,
        }
    }
}

/// 八叉树节点 (叶子可容纳多个 body, 防止退化时无限递归)
#[derive(Debug, Clone)]
struct OctreeNode {
    center: [f32; 3],
    half: f32,
    mass: f32,
    com: [f32; 3],
    children: [Option<Box<OctreeNode>>; 8],
    /// 叶子节点存储的 body 索引列表
    bodies: Vec<usize>,
    count: usize,
}

impl OctreeNode {
    fn new(center: [f32; 3], half: f32) -> Self {
        OctreeNode {
            center,
            half,
            mass: 0.0,
            com: center,
            children: Default::default(),
            bodies: Vec::new(),
            count: 0,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.iter().all(|c| c.is_none())
    }

    fn octant(&self, pos: &[f32; 3]) -> usize {
        let mut idx = 0;
        if pos[0] >= self.center[0] { idx |= 1; }
        if pos[1] >= self.center[1] { idx |= 2; }
        if pos[2] >= self.center[2] { idx |= 4; }
        idx
    }

    fn child_center(&self, octant: usize) -> [f32; 3] {
        let q = self.half * 0.5;
        [
            self.center[0] + if octant & 1 != 0 { q } else { -q },
            self.center[1] + if octant & 2 != 0 { q } else { -q },
            self.center[2] + if octant & 4 != 0 { q } else { -q },
        ]
    }

    /// 递归插入 body (带深度限制)
    fn insert(&mut self, body_idx: usize, bodies: &[Body], depth: usize) {
        if self.count == 0 {
            self.bodies.push(body_idx);
            self.count = 1;
            self.mass = bodies[body_idx].mass;
            self.com = bodies[body_idx].position;
            return;
        }
        if self.is_leaf() && depth < MAX_DEPTH {
            // 叶子节点: 细分, 把已有 body 和新 body 都下推
            let existing: Vec<usize> = self.bodies.drain(..).collect();
            let q = self.half * 0.5;
            for oct in 0..8 {
                self.children[oct] = Some(Box::new(OctreeNode::new(self.child_center(oct), q)));
            }
            for bi in existing {
                let oct = self.octant(&bodies[bi].position);
                self.children[oct].as_mut().unwrap().insert(bi, bodies, depth + 1);
            }
            let oct = self.octant(&bodies[body_idx].position);
            self.children[oct].as_mut().unwrap().insert(body_idx, bodies, depth + 1);
            self.count += 1;
            self.update_mass(bodies);
            return;
        }
        if self.is_leaf() {
            // 达到深度限制: 直接追加 (不再细分)
            self.bodies.push(body_idx);
            self.count += 1;
            self.update_mass(bodies);
            return;
        }
        // 内部节点: 递归插入
        let oct = self.octant(&bodies[body_idx].position);
        self.children[oct].as_mut().unwrap().insert(body_idx, bodies, depth + 1);
        self.count += 1;
        self.update_mass(bodies);
    }

    fn update_mass(&mut self, bodies: &[Body]) {
        let mut m = 0.0;
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        if self.is_leaf() {
            for &bi in &self.bodies {
                m += bodies[bi].mass;
                cx += bodies[bi].position[0] * bodies[bi].mass;
                cy += bodies[bi].position[1] * bodies[bi].mass;
                cz += bodies[bi].position[2] * bodies[bi].mass;
            }
        } else {
            for c in self.children.iter().flatten() {
                if c.count > 0 {
                    m += c.mass;
                    cx += c.com[0] * c.mass;
                    cy += c.com[1] * c.mass;
                    cz += c.com[2] * c.mass;
                }
            }
        }
        self.mass = m;
        if m > 0.0 {
            self.com = [cx / m, cy / m, cz / m];
        }
    }

    /// 计算 body_idx 受到的引力 (递归)
    fn force_on(
        &self,
        body_idx: usize,
        bodies: &[Body],
        config: &BarnesHutConfig,
    ) -> [f32; 3] {
        if self.count == 0 {
            return [0.0; 3];
        }
        let pos = bodies[body_idx].position;
        let mi = bodies[body_idx].mass;
        if self.is_leaf() {
            // 叶子: 对每个 body 直接求和
            let mut f = [0.0f32; 3];
            for &bi in &self.bodies {
                if bi == body_idx {
                    continue;
                }
                let pf = Self::pair_force(pos, bodies[bi].position, mi, bodies[bi].mass, config);
                f[0] += pf[0];
                f[1] += pf[1];
                f[2] += pf[2];
            }
            return f;
        }
        // 内部节点: 检查开角判据
        let dx = self.com[0] - pos[0];
        let dy = self.com[1] - pos[1];
        let dz = self.com[2] - pos[2];
        let d2 = dx * dx + dy * dy + dz * dz;
        let d = d2.sqrt();
        let open = match config.opening_method {
            OpeningMethod::Angle => {
                if d < 1e-30 { true } else { (2.0 * self.half) / d < config.theta }
            }
            OpeningMethod::Distance => {
                d > (2.0 * self.half) / config.theta
            }
        };
        if open {
            return Self::pair_force(pos, self.com, mi, self.mass, config);
        }
        let mut f = [0.0f32; 3];
        for c in self.children.iter().flatten() {
            if c.count > 0 {
                let cf = c.force_on(body_idx, bodies, config);
                f[0] += cf[0];
                f[1] += cf[1];
                f[2] += cf[2];
            }
        }
        f
    }

    fn pair_force(
        pi: [f32; 3],
        pj: [f32; 3],
        mi: f32,
        mj: f32,
        config: &BarnesHutConfig,
    ) -> [f32; 3] {
        let dx = pj[0] - pi[0];
        let dy = pj[1] - pi[1];
        let dz = pj[2] - pi[2];
        let r2 = dx * dx + dy * dy + dz * dz + config.softening * config.softening;
        let inv_r3 = 1.0 / (r2 * r2.sqrt());
        let s = config.gravity_constant * mi * mj * inv_r3;
        [s * dx, s * dy, s * dz]
    }
}

pub struct BarnesHutSolver {
    pub config: BarnesHutConfig,
    pub bodies: Vec<Body>,
    pub time: f32,
    pub steps: usize,
}

impl BarnesHutSolver {
    pub fn new(config: BarnesHutConfig) -> Self {
        BarnesHutSolver { config, bodies: Vec::new(), time: 0.0, steps: 0 }
    }

    pub fn add_body(&mut self, body: Body) {
        self.bodies.push(body);
    }

    fn bounding_box(bodies: &[Body]) -> ([f32; 3], f32) {
        if bodies.is_empty() {
            return ([0.0; 3], 1.0);
        }
        let mut lo = bodies[0].position;
        let mut hi = bodies[0].position;
        for b in bodies.iter().skip(1) {
            for d in 0..3 {
                if b.position[d] < lo[d] { lo[d] = b.position[d]; }
                if b.position[d] > hi[d] { hi[d] = b.position[d]; }
            }
        }
        let mut center = [0.0f32; 3];
        let mut max_extent = 1e-6_f32;
        for d in 0..3 {
            center[d] = 0.5 * (lo[d] + hi[d]);
            let ext = 0.5 * (hi[d] - lo[d]);
            if ext > max_extent { max_extent = ext; }
        }
        (center, max_extent + 1e-6)
    }

    fn build_tree(&self) -> OctreeNode {
        let (center, half) = Self::bounding_box(&self.bodies);
        let mut root = OctreeNode::new(center, half);
        for i in 0..self.bodies.len() {
            root.insert(i, &self.bodies, 0);
        }
        root
    }

    pub fn compute_forces(&self) -> Vec<[f32; 3]> {
        if self.bodies.is_empty() {
            return Vec::new();
        }
        let tree = self.build_tree();
        let mut forces = Vec::with_capacity(self.bodies.len());
        for i in 0..self.bodies.len() {
            forces.push(tree.force_on(i, &self.bodies, &self.config));
        }
        forces
    }

    pub fn force_direct_sum(&self, i: usize) -> [f32; 3] {
        let pi = self.bodies[i].position;
        let mi = self.bodies[i].mass;
        let mut f = [0.0f32; 3];
        for (j, b) in self.bodies.iter().enumerate() {
            if j == i { continue; }
            let pf = OctreeNode::pair_force(pi, b.position, mi, b.mass, &self.config);
            f[0] += pf[0];
            f[1] += pf[1];
            f[2] += pf[2];
        }
        f
    }

    pub fn compute_forces_direct(&self) -> Vec<[f32; 3]> {
        (0..self.bodies.len()).map(|i| self.force_direct_sum(i)).collect()
    }

    /// Leapfrog KDK 步进
    pub fn step(&mut self, dt: f32) {
        if self.bodies.is_empty() {
            self.time += dt;
            self.steps += 1;
            return;
        }
        let half_dt = 0.5 * dt;
        let forces = self.compute_forces();
        for (i, f) in forces.iter().enumerate() {
            let inv_m = 1.0 / self.bodies[i].mass;
            self.bodies[i].velocity[0] += f[0] * inv_m * half_dt;
            self.bodies[i].velocity[1] += f[1] * inv_m * half_dt;
            self.bodies[i].velocity[2] += f[2] * inv_m * half_dt;
        }
        for b in &mut self.bodies {
            b.position[0] += b.velocity[0] * dt;
            b.position[1] += b.velocity[1] * dt;
            b.position[2] += b.velocity[2] * dt;
        }
        let forces = self.compute_forces();
        for (i, f) in forces.iter().enumerate() {
            let inv_m = 1.0 / self.bodies[i].mass;
            self.bodies[i].velocity[0] += f[0] * inv_m * half_dt;
            self.bodies[i].velocity[1] += f[1] * inv_m * half_dt;
            self.bodies[i].velocity[2] += f[2] * inv_m * half_dt;
        }
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize, dt: f32) {
        for _ in 0..n {
            self.step(dt);
        }
    }

    pub fn total_mass(&self) -> f32 {
        self.bodies.iter().map(|b| b.mass).sum()
    }

    pub fn center_of_mass(&self) -> [f32; 3] {
        let m = self.total_mass();
        if m == 0.0 { return [0.0; 3]; }
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        for b in &self.bodies {
            cx += b.position[0] * b.mass;
            cy += b.position[1] * b.mass;
            cz += b.position[2] * b.mass;
        }
        [cx / m, cy / m, cz / m]
    }

    pub fn total_kinetic_energy(&self) -> f32 {
        let mut e = 0.0;
        for b in &self.bodies {
            let v2 = b.velocity[0] * b.velocity[0]
                + b.velocity[1] * b.velocity[1]
                + b.velocity[2] * b.velocity[2];
            e += 0.5 * b.mass * v2;
        }
        e
    }

    pub fn total_potential_energy(&self) -> f32 {
        let mut u = 0.0;
        let n = self.bodies.len();
        let g = self.config.gravity_constant;
        let eps2 = self.config.softening * self.config.softening;
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.bodies[j].position[0] - self.bodies[i].position[0];
                let dy = self.bodies[j].position[1] - self.bodies[i].position[1];
                let dz = self.bodies[j].position[2] - self.bodies[i].position[2];
                let r = (dx * dx + dy * dy + dz * dz + eps2).sqrt();
                u -= g * self.bodies[i].mass * self.bodies[j].mass / r;
            }
        }
        u
    }

    pub fn total_energy(&self) -> f32 {
        self.total_kinetic_energy() + self.total_potential_energy()
    }

    pub fn angular_momentum(&self) -> [f32; 3] {
        let mut l = [0.0f32; 3];
        for b in &self.bodies {
            let px = b.mass * b.velocity[0];
            let py = b.mass * b.velocity[1];
            let pz = b.mass * b.velocity[2];
            l[0] += b.position[1] * pz - b.position[2] * py;
            l[1] += b.position[2] * px - b.position[0] * pz;
            l[2] += b.position[0] * py - b.position[1] * px;
        }
        l
    }

    pub fn total_momentum(&self) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        for b in &self.bodies {
            p[0] += b.mass * b.velocity[0];
            p[1] += b.mass * b.velocity[1];
            p[2] += b.mass * b.velocity[2];
        }
        p
    }

    pub fn reset(&mut self) {
        self.bodies.clear();
        self.time = 0.0;
        self.steps = 0;
    }

    /// 生成圆轨道初始条件 (中心质量 + 环形分布的粒子)
    pub fn circular_orbit(center_mass: f32, radius: f32, count: usize, g: f32) -> Vec<Body> {
        let mut bodies = vec![Body::at_rest([0.0; 3], center_mass)];
        let v_orbit = (g * center_mass / radius).sqrt();
        for i in 0..count {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / count as f32;
            let pos = [radius * theta.cos(), radius * theta.sin(), 0.0];
            let vel = [-v_orbit * theta.sin(), v_orbit * theta.cos(), 0.0];
            bodies.push(Body::new(pos, vel, center_mass * 1.0e-6));
        }
        bodies
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cfg() -> BarnesHutConfig {
        BarnesHutConfig::unitless()
    }

    #[test]
    fn test_constants_present() {
        assert!(G > 0.0);
        assert!(AU > 0.0);
        assert!(M_SUN > M_EARTH);
    }

    #[test]
    fn test_body_new() {
        let b = Body::new([1.0, 2.0, 3.0], [0.1, 0.2, 0.3], 5.0);
        assert_eq!(b.position, [1.0, 2.0, 3.0]);
        assert_eq!(b.velocity, [0.1, 0.2, 0.3]);
        assert_eq!(b.mass, 5.0);
    }

    #[test]
    fn test_body_at_rest() {
        let b = Body::at_rest([0.0; 3], 1.0);
        assert_eq!(b.velocity, [0.0; 3]);
    }

    #[test]
    fn test_config_default_theta() {
        let c = BarnesHutConfig::default();
        assert!((c.theta - 0.5).abs() < 1e-6);
        assert!(c.softening > 0.0);
        assert_eq!(c.opening_method, OpeningMethod::Angle);
        assert!((c.gravity_constant - G).abs() / G < 1e-3);
    }

    #[test]
    fn test_config_unitless_g_is_one() {
        let c = BarnesHutConfig::unitless();
        assert_eq!(c.gravity_constant, 1.0);
    }

    #[test]
    fn test_config_solar_system() {
        let c = BarnesHutConfig::solar_system();
        assert!((c.gravity_constant - G).abs() / G < 1e-3);
        assert!(c.softening > 1.0e8);
    }

    #[test]
    fn test_solver_new_empty() {
        let s = BarnesHutSolver::new(unit_cfg());
        assert!(s.bodies.is_empty());
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_add_body() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        assert_eq!(s.bodies.len(), 2);
    }

    #[test]
    fn test_total_mass() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 2.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 3.0));
        assert!((s.total_mass() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_center_of_mass() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([2.0, 0.0, 0.0], 1.0));
        let com = s.center_of_mass();
        assert!((com[0] - 1.0).abs() < 1e-6);
        assert!((com[1] - 0.0).abs() < 1e-6);
        assert!((com[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_center_of_mass_mass_weighted() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([3.0, 0.0, 0.0], 3.0));
        let com = s.center_of_mass();
        assert!((com[0] - 2.25).abs() < 1e-6);
    }

    #[test]
    fn test_force_zero_for_single_body() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        let f = s.compute_forces();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0], [0.0; 3]);
    }

    #[test]
    fn test_force_two_bodies_attractive() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let f = s.compute_forces();
        assert!(f[0][0] > 0.0);
        assert!(f[1][0] < 0.0);
        assert!((f[0][0] + f[1][0]).abs() < 1e-3);
    }

    #[test]
    fn test_force_inverse_square() {
        let mut s1 = BarnesHutSolver::new(unit_cfg());
        s1.add_body(Body::at_rest([0.0; 3], 1.0));
        s1.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let f1 = s1.compute_forces();

        let mut s2 = BarnesHutSolver::new(unit_cfg());
        s2.add_body(Body::at_rest([0.0; 3], 1.0));
        s2.add_body(Body::at_rest([2.0, 0.0, 0.0], 1.0));
        let f2 = s2.compute_forces();

        let ratio = f1[0][0] / f2[0][0];
        assert!((ratio - 4.0).abs() / 4.0 < 0.01);
    }

    #[test]
    fn test_softening_reduces_force() {
        let mut cfg_close = unit_cfg();
        cfg_close.softening = 0.0;
        let mut cfg_soft = unit_cfg();
        cfg_soft.softening = 0.5;
        let mut s1 = BarnesHutSolver::new(cfg_close);
        s1.add_body(Body::at_rest([0.0; 3], 1.0));
        s1.add_body(Body::at_rest([0.1, 0.0, 0.0], 1.0));
        let f1 = s1.compute_forces();

        let mut s2 = BarnesHutSolver::new(cfg_soft);
        s2.add_body(Body::at_rest([0.0; 3], 1.0));
        s2.add_body(Body::at_rest([0.1, 0.0, 0.0], 1.0));
        let f2 = s2.compute_forces();

        assert!(f2[0][0] < f1[0][0]);
    }

    #[test]
    fn test_barnes_hut_matches_direct_sum() {
        let mut cfg = unit_cfg();
        cfg.theta = 0.0;
        let mut s = BarnesHutSolver::new(cfg);
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        s.add_body(Body::at_rest([0.5, 1.0, 0.0], 1.0));
        let f_bh = s.compute_forces();
        let f_direct = s.compute_forces_direct();
        for i in 0..3 {
            for d in 0..3 {
                assert!((f_bh[i][d] - f_direct[i][d]).abs() < 1e-3, "body {} axis {}", i, d);
            }
        }
    }

    #[test]
    fn test_barnes_hut_approximation_close() {
        let mut cfg = unit_cfg();
        cfg.theta = 0.05;
        cfg.softening = 0.05;
        let mut s = BarnesHutSolver::new(cfg);
        for i in 0..20u32 {
            let x = i as f32 * 0.5;
            let y = (i % 3) as f32;
            s.add_body(Body::at_rest([x, y, 0.0], 1.0));
        }
        let f_bh = s.compute_forces();
        let f_direct = s.compute_forces_direct();
        // 比较总力的大小 (近似应在量级正确)
        let mut sum_bh = 0.0;
        let mut sum_dir = 0.0;
        for i in 0..20 {
            sum_bh += (f_bh[i][0].powi(2) + f_bh[i][1].powi(2) + f_bh[i][2].powi(2)).sqrt();
            sum_dir += (f_direct[i][0].powi(2) + f_direct[i][1].powi(2) + f_direct[i][2].powi(2)).sqrt();
        }
        let rel = (sum_bh - sum_dir).abs() / sum_dir;
        assert!(rel < 0.5, "total force rel error {} too large", rel);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        s.step(0.01);
        assert!((s.time - 0.01).abs() < 1e-9);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_moves_bodies() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let p0_before = s.bodies[0].position;
        s.step(0.01);
        let p0_after = s.bodies[0].position;
        assert!((p0_after[0] - p0_before[0]).abs() > 0.0);
    }

    #[test]
    fn test_step_n() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        s.step_n(5, 0.01);
        assert_eq!(s.steps, 5);
        assert!((s.time - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_momentum_conservation() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::new([0.0; 3], [0.0; 3], 1.0));
        s.add_body(Body::new([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 1.0));
        let p0 = s.total_momentum();
        s.step_n(10, 0.001);
        let p1 = s.total_momentum();
        for d in 0..3 {
            assert!((p1[d] - p0[d]).abs() < 1e-3, "momentum drift axis {}", d);
        }
    }

    #[test]
    fn test_angular_momentum_conservation() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::new([0.0; 3], [0.0; 3], 1.0));
        s.add_body(Body::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0));
        let l0 = s.angular_momentum();
        s.step_n(10, 0.001);
        let l1 = s.angular_momentum();
        let mag0 = (l0[0].powi(2) + l0[1].powi(2) + l0[2].powi(2)).sqrt();
        let mag1 = (l1[0].powi(2) + l1[1].powi(2) + l1[2].powi(2)).sqrt();
        assert!((mag1 - mag0).abs() / (mag0 + 1e-9) < 0.05);
    }

    #[test]
    fn test_energy_near_conserved() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::new([0.0; 3], [0.0; 3], 1.0));
        s.add_body(Body::new([1.0, 0.0, 0.0], [0.0, 0.5, 0.0], 1.0));
        let e0 = s.total_energy();
        s.step_n(20, 0.001);
        let e1 = s.total_energy();
        let rel = (e1 - e0).abs() / (e0.abs() + 1e-9);
        assert!(rel < 0.05, "energy relative drift {} too large", rel);
    }

    #[test]
    fn test_kinetic_energy_positive() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::new([0.0; 3], [1.0, 0.0, 0.0], 2.0));
        let ke = s.total_kinetic_energy();
        assert!((ke - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_potential_energy_negative() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let pe = s.total_potential_energy();
        assert!(pe < 0.0);
    }

    #[test]
    fn test_circular_orbit_velocities() {
        let bodies = BarnesHutSolver::circular_orbit(100.0, 1.0, 8, 1.0);
        assert_eq!(bodies.len(), 9);
        assert_eq!(bodies[0].velocity, [0.0; 3]);
        let v = (bodies[1].velocity[0].powi(2) + bodies[1].velocity[1].powi(2)).sqrt();
        assert!((v - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_circular_orbit_stable() {
        let mut cfg = unit_cfg();
        cfg.softening = 0.001;
        let mut s = BarnesHutSolver::new(cfg);
        for b in BarnesHutSolver::circular_orbit(100.0, 1.0, 4, 1.0) {
            s.add_body(b);
        }
        let r0 = (s.bodies[1].position[0].powi(2) + s.bodies[1].position[1].powi(2)).sqrt();
        s.step_n(100, 0.001);
        let r1 = (s.bodies[1].position[0].powi(2) + s.bodies[1].position[1].powi(2)).sqrt();
        let rel = (r1 - r0).abs() / r0;
        assert!(rel < 0.05, "orbit radius drift {} too large", rel);
    }

    #[test]
    fn test_octree_subdivision() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        for i in 0..9u32 {
            let x = i as f32 * 0.01;
            s.add_body(Body::at_rest([x, 0.0, 0.0], 1.0));
        }
        let f = s.compute_forces();
        assert_eq!(f.len(), 9);
    }

    #[test]
    fn test_octree_coincident_bodies_no_crash() {
        // 两个 body 在同一位置: 不应无限递归
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([1.0, 1.0, 1.0], 1.0));
        s.add_body(Body::at_rest([1.0, 1.0, 1.0], 1.0));
        let f = s.compute_forces();
        assert_eq!(f.len(), 2);
        // 同位置: 软化后力非零但有限
        assert!(f[0][0].is_finite());
    }

    #[test]
    fn test_force_symmetry() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([0.0, 2.0, 0.0], 2.0));
        let f = s.compute_forces();
        for d in 0..3 {
            assert!((f[0][d] + f[1][d]).abs() < 1e-3);
        }
    }

    #[test]
    fn test_distance_opening_method() {
        let mut cfg = unit_cfg();
        cfg.opening_method = OpeningMethod::Distance;
        cfg.theta = 0.5;
        let mut s = BarnesHutSolver::new(cfg);
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([5.0, 0.0, 0.0], 1.0));
        s.add_body(Body::at_rest([10.0, 0.0, 0.0], 1.0));
        let f = s.compute_forces();
        assert_eq!(f.len(), 3);
    }

    #[test]
    fn test_direct_sum_no_self_force() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let f0 = s.force_direct_sum(0);
        assert!(f0[0] > 0.0);
    }

    #[test]
    fn test_total_momentum_initially_zero() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::new([0.0; 3], [1.0, 0.0, 0.0], 1.0));
        s.add_body(Body::new([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 1.0));
        let p = s.total_momentum();
        assert!(p[0].abs() < 1e-6);
    }

    #[test]
    fn test_reset_clears_bodies() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([0.0; 3], 1.0));
        s.step(0.01);
        s.reset();
        assert!(s.bodies.is_empty());
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_empty_step_no_panic() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.step(0.01);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_three_body_figure8_stability() {
        // Chenciner-Montgomery 2000 三体 8 字形轨道
        let mut cfg = unit_cfg();
        cfg.softening = 0.01;
        cfg.theta = 0.0;
        let mut s = BarnesHutSolver::new(cfg);
        let x1 = 0.97000436;
        let v1 = 0.93240737;
        let v2 = 0.86473146;
        s.add_body(Body::new([-x1, x1, 0.0], [v2 * 0.5, -v1 * 0.5, 0.0], 1.0));
        s.add_body(Body::new([x1, -x1, 0.0], [v2 * 0.5, -v1 * 0.5, 0.0], 1.0));
        s.add_body(Body::new([0.0, 0.0, 0.0], [-v2, v1, 0.0], 1.0));
        let e0 = s.total_energy();
        s.step_n(50, 0.0005);
        let e1 = s.total_energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 0.05, "figure-8 energy drift {} too large", rel);
    }

    #[test]
    fn test_gravity_constant_scaling() {
        let mut cfg_double = unit_cfg();
        cfg_double.gravity_constant = 2.0;
        let mut s1 = BarnesHutSolver::new(unit_cfg());
        s1.add_body(Body::at_rest([0.0; 3], 1.0));
        s1.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let f1 = s1.compute_forces();

        let mut s2 = BarnesHutSolver::new(cfg_double);
        s2.add_body(Body::at_rest([0.0; 3], 1.0));
        s2.add_body(Body::at_rest([1.0, 0.0, 0.0], 1.0));
        let f2 = s2.compute_forces();
        assert!((f2[0][0] / f1[0][0] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_bounding_box() {
        let mut s = BarnesHutSolver::new(unit_cfg());
        s.add_body(Body::at_rest([-2.0, 0.0, 0.0], 1.0));
        s.add_body(Body::at_rest([3.0, 0.0, 0.0], 1.0));
        let f = s.compute_forces();
        assert_eq!(f.len(), 2);
    }
}

