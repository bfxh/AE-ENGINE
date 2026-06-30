//! FitzHugh-Nagumo Neuronal Field Solver
//!
//! 反应扩散方程的可激发介质模型. 描述神经元膜电位 u 与恢复变量 v 的时空动力学.
//! FitzHugh 1961 (Hodgkin-Huxley 简化), Nagumo 1962 (电路实现).
//!
//! 方程 (cubic form):
//!   du/dt = D_u * laplacian(u) + u(1-u)(u-a) - v + I_ext
//!   dv/dt = epsilon * (u - gamma*v)
//!
//! 变量:
//!   u - 膜电位 (excitation, 0=rest, 1=excited)
//!   v - 恢复变量 (inhibition/refractory)
//!   I_ext - 外部刺激电流
//!
//! 参数:
//!   a - 阈值参数 (典型 0.1), u(1-u)(u-a)=0 的中间根
//!   epsilon - 恢复时间尺度 (典型 0.02, 慢恢复)
//!   gamma - 恢复耦合 (典型 2.0)
//!   D_u - 扩散系数 (典型 1.0, 仅 u 扩散)
//!
//! 动力学:
//!   - 静息态: u=0, v=0 (稳定, Jacobian 实部 < 0)
//!   - 兴奋态: u -> 1 (快速, 由 cubic u(1-u)(u-a) 驱动)
//!   - 不应期: v 上升, 拉低 u, 阻止再兴奋
//!   - 恢复: v 缓慢衰减回 0
//!
//! 波动现象:
//!   - 行波 (traveling pulse): 1D 兴奋波传播, 速度 c ~ sqrt(D*epsilon)
//!   - 螺旋波 (spiral wave): 2D 拓扑缺陷, 心脏纤颤模型
//!   - 双稳态波 (bistable front): a<0.5 时 u=0<->u=1 前沿传播
//!
//! 应用:
//!   - 神经元放电传播 (轴突, 皮层)
//!   - 心脏电活动 (心室纤颤 = 螺旋波失稳)
//!   - BZ 反应 (Belousov-Zhabotinsky 化学波)
//!   - 可激发介质通用模型
//!
//! 数值方法:
//!   显式 Euler + 中心差分 (5-point stencil for 2D diffusion)
//!   CFL: 4*D*dt/dx^2 <= 1 (2D), 反应: dt*max|f'(u)| <= 1
//!
//! 基于 FitzHugh 1961, Nagumo 1962, Hodgkin-Huxley 1952,
//! Keener & Sneyd "Mathematical Physiology" 2009.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FhnBoundary {
    Periodic,
    Neumann,
    Dirichlet { value: f32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FhnStimulus {
    None,
    Point { x: usize, y: usize, radius: usize, current: f32 },
    Line { axis: u8, coord: usize, thickness: usize, current: f32 },
    Rect { x0: usize, y0: usize, x1: usize, y1: usize, current: f32 },
    Cross { x: usize, y: usize, length: usize, thickness: usize, current: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhnConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub d_u: f32,
    pub a: f32,
    pub epsilon: f32,
    pub gamma: f32,
    pub boundary: FhnBoundary,
}

impl Default for FhnConfig {
    fn default() -> Self {
        FhnConfig {
            nx: 128,
            ny: 128,
            dx: 0.5,
            dt: 0.05,
            d_u: 1.0,
            a: 0.1,
            epsilon: 0.02,
            gamma: 2.0,
            boundary: FhnBoundary::Periodic,
        }
    }
}

impl FhnConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }
    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }
    pub fn diffusive_cfl(&self) -> f32 {
        4.0 * self.d_u * self.dt / (self.dx * self.dx)
    }
    pub fn reaction_cfl(&self) -> f32 {
        let m = (1.0 - self.a + self.a * self.a) / 3.0;
        self.dt * m
    }
    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0 && self.reaction_cfl() <= 1.0
    }
    pub fn stable_dt(&self) -> f32 {
        let diff_dt = self.dx * self.dx / (4.0 * self.d_u);
        let m = (1.0 - self.a + self.a * self.a) / 3.0;
        let rxn_dt = 1.0 / m.max(1e-6);
        diff_dt.min(rxn_dt)
    }
}

pub struct FhnSolver {
    pub config: FhnConfig,
    pub u_curr: Vec<f32>,
    pub v_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub v_next: Vec<f32>,
    pub stimulus: FhnStimulus,
    pub time: f32,
    pub steps: usize,
}

impl FhnSolver {
    pub fn new(config: FhnConfig) -> Self {
        let n = config.n_cells();
        FhnSolver {
            config,
            u_curr: vec![0.0; n],
            v_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            v_next: vec![0.0; n],
            stimulus: FhnStimulus::None,
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    pub fn initialize_rest(&mut self) {
        for u in self.u_curr.iter_mut() {
            *u = 0.0;
        }
        for v in self.v_curr.iter_mut() {
            *v = 0.0;
        }
        for u in self.u_next.iter_mut() {
            *u = 0.0;
        }
        for v in self.v_next.iter_mut() {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_traveling_pulse(&mut self, x_front: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * dx;
                let k = self.idx(i, j);
                if x < x_front {
                    self.u_curr[k] = 1.0;
                } else {
                    self.u_curr[k] = 0.0;
                }
                self.v_curr[k] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_spiral_seed(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                self.u_curr[k] = if j < ny / 2 { 1.0 } else { 0.0 };
                self.v_curr[k] = if i > nx / 2 { 0.5 } else { 0.0 };
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }
    pub fn apply_stimulus(&mut self, stim: &FhnStimulus) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        match *stim {
            FhnStimulus::None => {}
            FhnStimulus::Point { x, y, radius, current } => {
                for j in 0..ny {
                    for i in 0..nx {
                        let di = (i as i32 - x as i32).abs();
                        let dj = (j as i32 - y as i32).abs();
                        if di <= radius as i32 && dj <= radius as i32 {
                            let k = self.idx(i, j);
                            self.u_curr[k] += current;
                        }
                    }
                }
            }
            FhnStimulus::Line { axis, coord, thickness, current } => {
                let half = (thickness / 2) as i32;
                for j in 0..ny {
                    for i in 0..nx {
                        let dist = if axis == 0 {
                            (i as i32 - coord as i32).abs()
                        } else {
                            (j as i32 - coord as i32).abs()
                        };
                        if dist <= half {
                            let k = self.idx(i, j);
                            self.u_curr[k] += current;
                        }
                    }
                }
            }
            FhnStimulus::Rect { x0, y0, x1, y1, current } => {
                let xe = x1.min(nx - 1);
                let ye = y1.min(ny - 1);
                for j in y0..=ye {
                    for i in x0..=xe {
                        let k = self.idx(i, j);
                        self.u_curr[k] += current;
                    }
                }
            }
            FhnStimulus::Cross { x, y, length, thickness, current } => {
                let half_t = (thickness / 2) as i32;
                let half_l = length as i32;
                for j in 0..ny {
                    for i in 0..nx {
                        let di = i as i32 - x as i32;
                        let dj = j as i32 - y as i32;
                        let in_h = dj.abs() <= half_t && di.abs() <= half_l;
                        let in_v = di.abs() <= half_t && dj.abs() <= half_l;
                        if in_h || in_v {
                            let k = self.idx(i, j);
                            self.u_curr[k] += current;
                        }
                    }
                }
            }
        }
    }

    pub fn set_persistent_stimulus(&mut self, stim: FhnStimulus) {
        self.stimulus = stim;
    }

    fn stimulus_current(&self, i: usize, j: usize) -> f32 {
        match self.stimulus {
            FhnStimulus::None => 0.0,
            FhnStimulus::Point { x, y, radius, current } => {
                let di = (i as i32 - x as i32).abs();
                let dj = (j as i32 - y as i32).abs();
                if di <= radius as i32 && dj <= radius as i32 {
                    current
                } else {
                    0.0
                }
            }
            FhnStimulus::Line { axis, coord, thickness, current } => {
                let half = (thickness / 2) as i32;
                let dist = if axis == 0 {
                    (i as i32 - coord as i32).abs()
                } else {
                    (j as i32 - coord as i32).abs()
                };
                if dist <= half {
                    current
                } else {
                    0.0
                }
            }
            FhnStimulus::Rect { x0, y0, x1, y1, current } => {
                if i >= x0 && i <= x1 && j >= y0 && j <= y1 {
                    current
                } else {
                    0.0
                }
            }
            FhnStimulus::Cross { x, y, length, thickness, current } => {
                let half_t = (thickness / 2) as i32;
                let half_l = length as i32;
                let di = i as i32 - x as i32;
                let dj = j as i32 - y as i32;
                let in_h = dj.abs() <= half_t && di.abs() <= half_l;
                let in_v = di.abs() <= half_t && dj.abs() <= half_l;
                if in_h || in_v {
                    current
                } else {
                    0.0
                }
            }
        }
    }

    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let d_u = self.config.d_u;
        let a = self.config.a;
        let eps = self.config.epsilon;
        let gamma = self.config.gamma;
        let inv_dx2 = 1.0 / (dx * dx);
        let bc = self.config.boundary;

        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                let u = self.u_curr[k];
                let v = self.v_curr[k];

                let (ip, im, jp, jm) = match bc {
                    FhnBoundary::Periodic => (
                        (i + 1) % nx,
                        (i + nx - 1) % nx,
                        (j + 1) % ny,
                        (j + ny - 1) % ny,
                    ),
                    _ => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        (j + 1).min(ny - 1),
                        if j > 0 { j - 1 } else { 0 },
                    ),
                };
                let u_ip = self.u_curr[self.idx(ip, j)];
                let u_im = self.u_curr[self.idx(im, j)];
                let u_jp = self.u_curr[self.idx(i, jp)];
                let u_jm = self.u_curr[self.idx(i, jm)];
                let lap_u = (u_ip + u_im + u_jp + u_jm - 4.0 * u) * inv_dx2;

                let f = u * (1.0 - u) * (u - a);
                let g = eps * (u - gamma * v);

                let stim = self.stimulus_current(i, j);
                let du = d_u * lap_u + f - v + stim;
                let dv = g;

                self.u_next[k] = u + dt * du;
                self.v_next[k] = v + dt * dv;
            }
        }

        if let FhnBoundary::Dirichlet { value } = bc {
            for i in 0..nx {
                let top = self.idx(i, 0);
                let bot = self.idx(i, ny - 1);
                self.u_next[top] = value;
                self.u_next[bot] = value;
            }
            for j in 0..ny {
                let left = self.idx(0, j);
                let right = self.idx(nx - 1, j);
                self.u_next[left] = value;
                self.u_next[right] = value;
            }
        }

        std::mem::swap(&mut self.u_curr, &mut self.u_next);
        std::mem::swap(&mut self.v_curr, &mut self.v_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }
    pub fn mean_potential(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().sum::<f32>() / (n as f32)
    }

    pub fn mean_recovery(&self) -> f32 {
        let n = self.v_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.v_curr.iter().sum::<f32>() / (n as f32)
    }

    pub fn excitation_count(&self) -> usize {
        self.u_curr.iter().filter(|&&u| u > 0.5).count()
    }

    pub fn excited_fraction(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.excitation_count() as f32 / n as f32
    }

    pub fn max_potential(&self) -> f32 {
        self.u_curr.iter().cloned().fold(0.0f32, f32::max)
    }

    pub fn min_potential(&self) -> f32 {
        self.u_curr.iter().cloned().fold(0.0f32, f32::min)
    }

    pub fn total_activity(&self) -> f32 {
        let dx2 = self.config.dx * self.config.dx;
        self.u_curr.iter().map(|&u| u * u).sum::<f32>() * dx2
    }

    pub fn has_nan(&self) -> bool {
        self.u_curr.iter().any(|&u| !u.is_finite())
            || self.v_curr.iter().any(|&v| !v.is_finite())
    }

    pub fn spiral_tip_count(&self, threshold: f32) -> usize {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        if nx < 3 || ny < 3 {
            return 0;
        }
        let mut count = 0;
        let grad_thresh = 0.1 / dx;
        for j in 1..ny - 1 {
            for i in 1..nx - 1 {
                let k = self.idx(i, j);
                let u = self.u_curr[k];
                if (u - threshold).abs() < 0.05 {
                    let u_ip = self.u_curr[self.idx(i + 1, j)];
                    let u_im = self.u_curr[self.idx(i - 1, j)];
                    let u_jp = self.u_curr[self.idx(i, j + 1)];
                    let u_jm = self.u_curr[self.idx(i, j - 1)];
                    let gx = (u_ip - u_im) / (2.0 * dx);
                    let gy = (u_jp - u_jm) / (2.0 * dx);
                    if gx * gx + gy * gy < grad_thresh * grad_thresh {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    pub fn reset(&mut self) {
        self.initialize_rest();
        self.stimulus = FhnStimulus::None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_config_default() {
        let c = FhnConfig::default();
        assert_eq!(c.nx, 128);
        assert_eq!(c.ny, 128);
        assert_eq!(c.dx, 0.5);
        assert_eq!(c.dt, 0.05);
        assert_eq!(c.d_u, 1.0);
        assert_eq!(c.a, 0.1);
        assert_eq!(c.epsilon, 0.02);
        assert_eq!(c.gamma, 2.0);
        assert_eq!(c.boundary, FhnBoundary::Periodic);
    }

    #[test]
    fn test_n_cells() {
        let c = FhnConfig { nx: 32, ny: 16, ..Default::default() };
        assert_eq!(c.n_cells(), 512);
    }

    #[test]
    fn test_domain_area() {
        let c = FhnConfig { nx: 10, ny: 20, dx: 0.5, ..Default::default() };
        assert!(approx_eq(c.domain_area(), 10.0 * 0.5 * 20.0 * 0.5, 1e-6));
    }

    #[test]
    fn test_diffusive_cfl() {
        let c = FhnConfig { nx: 64, ny: 64, dx: 0.5, dt: 0.05, d_u: 1.0, ..Default::default() };
        assert!(approx_eq(c.diffusive_cfl(), 0.8, 1e-6));
    }

    #[test]
    fn test_reaction_cfl() {
        let c = FhnConfig::default();
        let m = (1.0 - 0.1 + 0.01) / 3.0;
        assert!(approx_eq(c.reaction_cfl(), 0.05 * m, 1e-6));
    }

    #[test]
    fn test_is_stable_default() {
        let c = FhnConfig::default();
        assert!(c.is_stable());
    }

    #[test]
    fn test_is_stable_unstable_diffusion() {
        let c = FhnConfig { nx: 32, ny: 32, dx: 0.1, dt: 0.5, d_u: 1.0, ..Default::default() };
        assert!(!c.is_stable());
    }

    #[test]
    fn test_stable_dt() {
        let c = FhnConfig::default();
        let dt = c.stable_dt();
        assert!(dt > 0.0);
        assert!(approx_eq(dt, 0.0625, 1e-5));
    }

    #[test]
    fn test_solver_new() {
        let s = FhnSolver::new(FhnConfig { nx: 16, ny: 8, ..Default::default() });
        assert_eq!(s.u_curr.len(), 128);
        assert_eq!(s.v_curr.len(), 128);
        assert_eq!(s.u_next.len(), 128);
        assert_eq!(s.v_next.len(), 128);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &u in s.u_curr.iter() {
            assert_eq!(u, 0.0);
        }
        for &v in s.v_curr.iter() {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn test_idx() {
        let s = FhnSolver::new(FhnConfig { nx: 16, ny: 8, ..Default::default() });
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(15, 0), 15);
        assert_eq!(s.idx(0, 1), 16);
        assert_eq!(s.idx(15, 7), 16 * 8 - 1);
    }

    #[test]
    fn test_initialize_rest() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.u_curr[5] = 0.5;
        s.v_curr[10] = 0.3;
        s.initialize_rest();
        for &u in s.u_curr.iter() {
            assert_eq!(u, 0.0);
        }
        for &v in s.v_curr.iter() {
            assert_eq!(v, 0.0);
        }
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = FhnSolver::new(FhnConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_rest();
        assert_eq!(s.time, 0.0);
        s.step();
        assert!(approx_eq(s.time, 0.05, 1e-9));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.1, 1e-9));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = FhnSolver::new(FhnConfig { nx: 16, ny: 16, dt: 0.01, ..Default::default() });
        s.initialize_rest();
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 1.0, 1e-6));
    }

    #[test]
    fn test_rest_stays_at_rest() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.step_n(200);
        let max_u = s.u_curr.iter().cloned().fold(0.0f32, f32::max).abs();
        let max_v = s.v_curr.iter().cloned().fold(0.0f32, f32::max).abs();
        assert!(max_u < 1e-6, "rest state u drifted: {}", max_u);
        assert!(max_v < 1e-6, "rest state v drifted: {}", max_v);
    }

    #[test]
    fn test_stimulus_triggers_excitation() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 1.0 });
        s.step_n(5);
        let center = s.idx(64, 64);
        assert!(s.u_curr[center] > 0.5, "stimulus failed to excite: u={}", s.u_curr[center]);
    }

    #[test]
    fn test_excitation_propagates() {
        let mut s = FhnSolver::new(FhnConfig { nx: 128, ny: 8, dx: 0.5, dt: 0.05, ..Default::default() });
        s.initialize_traveling_pulse(10.0);
        s.step_n(200);
        let excited = s.excitation_count();
        assert!(excited > 10, "excitation did not propagate: {} excited", excited);
    }

    #[test]
    fn test_plane_wave_propagates() {
        let mut s = FhnSolver::new(FhnConfig { nx: 128, ny: 8, dx: 0.5, dt: 0.05, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Rect { x0: 0, y0: 0, x1: 10, y1: 7, current: 1.0 });
        s.step_n(10);
        s.stimulus = FhnStimulus::None;
        s.step_n(200);
        let excited = s.excitation_count();
        assert!(excited > 10, "plane wave did not propagate: {} excited", excited);
    }

    #[test]
    fn test_recovery_returns_to_rest() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 4, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 16, y: 2, radius: 2, current: 0.5 });
        s.step_n(5000);
        let max_u = s.max_potential();
        let max_v = s.v_curr.iter().cloned().fold(0.0f32, f32::max).abs();
        assert!(max_u < 0.05, "u did not recover: max_u={}", max_u);
        assert!(max_v < 0.02, "v did not recover: max_v={}", max_v);
    }
    #[test]
    fn test_no_nan_long_run() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 0.5 });
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after long run");
    }

    #[test]
    fn test_u_bounded() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 0.5 });
        s.step_n(500);
        let max_u = s.max_potential();
        let min_u = s.min_potential();
        assert!(max_u < 1.5, "u runaway positive: {}", max_u);
        assert!(min_u > -0.5, "u runaway negative: {}", min_u);
    }

    #[test]
    fn test_traveling_pulse_propagates() {
        let mut s = FhnSolver::new(FhnConfig { nx: 128, ny: 8, dx: 0.5, dt: 0.05, ..Default::default() });
        s.initialize_traveling_pulse(10.0);
        let mut x0 = 0usize;
        for i in 0..s.config.nx {
            if s.u_curr[s.idx(i, 4)] > 0.5 {
                x0 = i;
                break;
            }
        }
        s.step_n(200);
        let mut x1 = 0usize;
        for i in 0..s.config.nx {
            if s.u_curr[s.idx(i, 4)] > 0.5 {
                x1 = i;
            }
        }
        assert!(x1 > x0 + 5, "wave did not propagate: {} -> {}", x0, x1);
    }

    #[test]
    fn test_spiral_seed_init() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_spiral_seed();
        let top = s.idx(64, 32);
        assert!(approx_eq(s.u_curr[top], 1.0, 1e-6));
        let bot = s.idx(64, 96);
        assert!(approx_eq(s.u_curr[bot], 0.0, 1e-6));
        let right = s.idx(96, 64);
        assert!(approx_eq(s.v_curr[right], 0.5, 1e-6));
        let left = s.idx(32, 64);
        assert!(approx_eq(s.v_curr[left], 0.0, 1e-6));
    }

    #[test]
    fn test_spiral_wave_dynamics() {
        let mut s = FhnSolver::new(FhnConfig { nx: 96, ny: 96, dx: 1.0, dt: 0.1, ..Default::default() });
        s.initialize_spiral_seed();
        s.step_n(300);
        assert!(!s.has_nan(), "NaN in spiral wave");
        let activity = s.total_activity();
        assert!(activity > 1.0, "spiral wave died too early: activity={}", activity);
        let excited = s.excitation_count();
        assert!(excited < s.config.n_cells(), "spiral wave saturated");
    }

    #[test]
    fn test_periodic_boundary_no_nan() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, boundary: FhnBoundary::Periodic, ..Default::default() });
        s.initialize_spiral_seed();
        s.step_n(500);
        assert!(!s.has_nan(), "NaN with periodic boundary");
    }

    #[test]
    fn test_neumann_boundary_no_nan() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, boundary: FhnBoundary::Neumann, ..Default::default() });
        s.initialize_spiral_seed();
        s.step_n(500);
        assert!(!s.has_nan(), "NaN with Neumann boundary");
    }

    #[test]
    fn test_dirichlet_boundary_damps() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, boundary: FhnBoundary::Dirichlet { value: 0.0 }, ..Default::default() });
        s.initialize_spiral_seed();
        s.step_n(500);
        let corner = s.idx(0, 0);
        assert!(approx_eq(s.u_curr[corner], 0.0, 1e-6));
        let edge = s.idx(15, 0);
        assert!(approx_eq(s.u_curr[edge], 0.0, 1e-6));
    }

    #[test]
    fn test_excitation_count_range() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        assert_eq!(s.excitation_count(), 0);
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 1.0 });
        let count = s.excitation_count();
        assert!(count > 0, "no excited cells after stimulus");
        assert!(count < s.config.n_cells());
    }

    #[test]
    fn test_mean_potential_bounded() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 0.5 });
        s.step_n(500);
        let m = s.mean_potential();
        assert!(m > -0.5 && m < 1.5, "mean potential out of bounds: {}", m);
    }

    #[test]
    fn test_excited_fraction_range() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        let f0 = s.excited_fraction();
        assert_eq!(f0, 0.0);
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 0.5 });
        s.step_n(100);
        let f1 = s.excited_fraction();
        assert!(f1 >= 0.0 && f1 <= 1.0);
    }

    #[test]
    fn test_refractory_period() {
        let mut s = FhnSolver::new(FhnConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 8, y: 8, radius: 3, current: 1.0 });
        s.step_n(20);
        let v_after = s.v_curr[s.idx(8, 8)];
        assert!(v_after > 0.005, "recovery variable did not rise: v={}", v_after);
    }

    #[test]
    fn test_stimulus_amplitude_threshold() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 2, current: 0.05 });
        s.step_n(200);
        let excited = s.excitation_count();
        assert!(excited < 50, "subthreshold stimulus caused excitation: {}", excited);
    }

    #[test]
    fn test_symmetry_preserved() {
        let mut s = FhnSolver::new(FhnConfig { nx: 64, ny: 64, boundary: FhnBoundary::Neumann, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 32, y: 32, radius: 3, current: 0.5 });
        s.step_n(50);
        let center = 32;
        for offset in 1..20 {
            let left = s.idx(center - offset, 32);
            let right = s.idx(center + offset, 32);
            let diff = (s.u_curr[left] - s.u_curr[right]).abs();
            assert!(diff < 1e-3, "left-right symmetry broken at offset {}: {}", offset, diff);
        }
    }

    #[test]
    fn test_reset() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_spiral_seed();
        s.set_persistent_stimulus(FhnStimulus::Point { x: 10, y: 10, radius: 3, current: 0.2 });
        s.step_n(100);
        s.reset();
        for &u in s.u_curr.iter() {
            assert_eq!(u, 0.0);
        }
        for &v in s.v_curr.iter() {
            assert_eq!(v, 0.0);
        }
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        assert_eq!(s.stimulus, FhnStimulus::None);
    }

    #[test]
    fn test_total_activity_nonneg() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_rest();
        assert!(s.total_activity() >= 0.0);
        s.apply_stimulus(&FhnStimulus::Point { x: 64, y: 64, radius: 5, current: 0.5 });
        s.step_n(100);
        assert!(s.total_activity() > 0.0);
    }

    #[test]
    fn test_stimulus_point() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Point { x: 16, y: 16, radius: 1, current: 0.5 });
        assert!(s.u_curr[s.idx(16, 16)] > 0.0);
        assert!(s.u_curr[s.idx(20, 20)] == 0.0);
    }

    #[test]
    fn test_stimulus_line() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Line { axis: 0, coord: 16, thickness: 3, current: 0.5 });
        assert!(s.u_curr[s.idx(16, 5)] > 0.0);
        assert!(s.u_curr[s.idx(17, 5)] > 0.0);
        assert!(s.u_curr[s.idx(20, 5)] == 0.0);
    }

    #[test]
    fn test_stimulus_rect() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Rect { x0: 5, y0: 5, x1: 10, y1: 10, current: 0.5 });
        assert!(s.u_curr[s.idx(7, 7)] > 0.0);
        assert!(s.u_curr[s.idx(20, 20)] == 0.0);
    }

    #[test]
    fn test_stimulus_cross() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, ..Default::default() });
        s.initialize_rest();
        s.apply_stimulus(&FhnStimulus::Cross { x: 16, y: 16, length: 5, thickness: 3, current: 0.5 });
        assert!(s.u_curr[s.idx(16, 16)] > 0.0);
        assert!(s.u_curr[s.idx(20, 16)] > 0.0);
        assert!(s.u_curr[s.idx(16, 20)] > 0.0);
    }

    #[test]
    fn test_persistent_stimulus() {
        let mut s = FhnSolver::new(FhnConfig { nx: 32, ny: 32, ..Default::default() });
        s.initialize_rest();
        s.set_persistent_stimulus(FhnStimulus::Point { x: 16, y: 16, radius: 2, current: 0.3 });
        s.step_n(10);
        let center = s.idx(16, 16);
        assert!(s.u_curr[center] > 0.0, "persistent stimulus had no effect");
    }

    #[test]
    fn test_spiral_tip_count_nonneg() {
        let mut s = FhnSolver::new(FhnConfig::default());
        s.initialize_spiral_seed();
        s.step_n(500);
        let tips = s.spiral_tip_count(0.5);
        assert!(tips >= 0);
    }
}