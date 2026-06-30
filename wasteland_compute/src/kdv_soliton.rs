//! KdV Soliton Solver (Korteweg-de Vries Equation)
//!
//! 非线性波动方程. 非线性陡化与三阶色散的平衡产生孤立子——
//! 局域化的波形以恒定速度传播并保持形状, 碰撞后恢复.
//!
//! 方程 (KdV 1895):
//!   du/dt + 6*u*du/dx + d^3 u/dx^3 = 0
//!
//! 单孤立子解:
//!   u(x,t) = A * sech^2( sqrt(A/2) * (x - 2*A*t - x0) )
//!   振幅 A, 速度 c = 2*A (越高越快越窄)
//!
//! Zabusky-Kruskal 1965 leapfrog 格式 (周期边界):
//!   (u^{n+1} - u^{n-1})/(2 dt) = -6 * u_avg * (u_{i+1} - u_{i-1})/(2 dx)
//!                                - (u_{i+2} - 2 u_{i+1} + 2 u_{i-1} - u_{i-2})/(2 dx^3)
//!   u_avg = (u_{i+1} + u_i + u_{i-1})/3   (Zabusky 平均, 抑制非线性锯齿)
//!
//! 启动: 第一步前向 Euler (leapfrog 需要 u^{n-1}).
//!
//! 守恒律 (KdV 是无穷维可积系统 — KdV 方程有无穷多守恒律):
//!   I1 = integral u dx          (质量)
//!   I2 = integral u^2 dx        (动量)
//!   I3 = integral (2 u^3 - 3 u_x^2) dx  (Hamiltonian)
//!
//! 孤立子碰撞 (Zabusky-Kruskal 1965 首次数值观察):
//!   - 快孤立子追上慢孤立子
//!   - 碰撞时振幅下降 (非线性叠加原理失效)
//!   - 碰撞后两孤立子恢复原形状, 仅相位偏移
//!   - 这是 "孤立子" 名称的由来 (soliton, 类粒子行为)
//!
//! 应用: 浅水波, 等离子体离子声波, 非线性晶格 (FPU 问题),
//!       光纤孤子通信, 大气 Rossby 波, 血管脉搏波.
//!
//! 基于 Korteweg & de Vries 1895, Zabusky & Kruskal 1965,
//!     Gardner-Greene-Kruskal-Miura 1967 (逆散射变换).

use serde::{Deserialize, Serialize};

/// KdV 非线性系数 (标准 KdV: u_t + 6 u u_x + u_xxx = 0)
pub const KDV_NONLINEAR: f32 = 6.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdvConfig {
    pub nx: usize,
    pub dx: f32,
    pub dt: f32,
}

impl Default for KdvConfig {
    fn default() -> Self {
        KdvConfig { nx: 256, dx: 0.1, dt: 0.0001 }
    }
}

impl KdvConfig {
    pub fn length(&self) -> f32 {
        (self.nx as f32) * self.dx
    }
    pub fn n_cells(&self) -> usize {
        self.nx
    }
    /// 线性色散稳定性 (u_t + u_xxx = 0): dt <= dx^3 / (2 * (pi)^3 * something)
    /// 经验值: dt <= dx^3 / 4 (粗略上界)
    pub fn linear_dt_limit(&self) -> f32 {
        self.dx * self.dx * self.dx / 4.0
    }
    pub fn is_stable(&self) -> bool {
        self.dt <= self.linear_dt_limit()
    }
}

pub struct KdvSolver {
    pub config: KdvConfig,
    pub u_curr: Vec<f32>,
    pub u_prev: Vec<f32>,
    pub u_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl KdvSolver {
    pub fn new(config: KdvConfig) -> Self {
        let n = config.n_cells();
        KdvSolver {
            config,
            u_curr: vec![0.0; n],
            u_prev: vec![0.0; n],
            u_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize) -> usize {
        i
    }

    fn wrap(i: i32, n: usize) -> usize {
        let m = n as i32;
        (((i % m) + m) % m) as usize
    }

    /// 初始化单孤立子: u(x, 0) = A * sech^2( sqrt(A/2) * (x - x0) )
    /// 速度 c = 2*A (向右传播)
    pub fn initialize_soliton(&mut self, amplitude: f32, center: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let kappa = (amplitude / 2.0).sqrt();
        for i in 0..nx {
            let x = (i as f32) * dx;
            let s = kappa * (x - center);
            let sech = 1.0 / s.cosh();
            self.u_curr[i] = amplitude * sech * sech;
            self.u_prev[i] = self.u_curr[i];
        }
    }

    /// 初始化两个孤立子 (不同振幅, 高的会追上低的并碰撞)
    pub fn initialize_two_solitons(&mut self, a1: f32, c1: f32, a2: f32, c2: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let k1 = (a1 / 2.0).sqrt();
        let k2 = (a2 / 2.0).sqrt();
        for i in 0..nx {
            let x = (i as f32) * dx;
            let s1 = k1 * (x - c1);
            let s2 = k2 * (x - c2);
            let sech1 = 1.0 / s1.cosh();
            let sech2 = 1.0 / s2.cosh();
            self.u_curr[i] = a1 * sech1 * sech1 + a2 * sech2 * sech2;
            self.u_prev[i] = self.u_curr[i];
        }
    }

    /// 前向 Euler 启动步 (leapfrog 需要 u^{n-1})
    fn euler_step(&mut self) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let dx3 = dx * dx * dx;
        let inv_2dx = 1.0 / (2.0 * dx);
        let inv_2dx3 = 1.0 / (2.0 * dx3);
        for i in 0..nx {
            let im = Self::wrap((i as i32) - 1, nx);
            let ip = Self::wrap((i as i32) + 1, nx);
            let imm = Self::wrap((i as i32) - 2, nx);
            let ipp = Self::wrap((i as i32) + 2, nx);
            let u = self.u_curr[i];
            let ux = (self.u_curr[ip] - self.u_curr[im]) * inv_2dx;
            let uxxx = (self.u_curr[ipp] - 2.0 * self.u_curr[ip]
                + 2.0 * self.u_curr[im] - self.u_curr[imm]) * inv_2dx3;
            self.u_next[i] = u - dt * (KDV_NONLINEAR * u * ux + uxxx);
        }
        std::mem::swap(&mut self.u_prev, &mut self.u_curr);
        std::mem::swap(&mut self.u_curr, &mut self.u_next);
    }

    /// Zabusky-Kruskal leapfrog 步
    fn leapfrog_step(&mut self) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let dx3 = dx * dx * dx;
        let inv_3 = 1.0 / 3.0;
        let inv_2dx = 1.0 / (2.0 * dx);
        let inv_2dx3 = 1.0 / (2.0 * dx3);
        let two_dt = 2.0 * dt;
        for i in 0..nx {
            let im = Self::wrap((i as i32) - 1, nx);
            let ip = Self::wrap((i as i32) + 1, nx);
            let imm = Self::wrap((i as i32) - 2, nx);
            let ipp = Self::wrap((i as i32) + 2, nx);
            let u = self.u_curr[i];
            let u_ip = self.u_curr[ip];
            let u_im = self.u_curr[im];
            let u_avg = (u_ip + u + u_im) * inv_3;
            let ux = (u_ip - u_im) * inv_2dx;
            let uxxx = (self.u_curr[ipp] - 2.0 * u_ip + 2.0 * u_im - self.u_curr[imm]) * inv_2dx3;
            self.u_next[i] = self.u_prev[i] - two_dt * (KDV_NONLINEAR * u_avg * ux + uxxx);
        }
        // Asselin 滤波 (抑制 leapfrog 寄生模, α=0.1)
        let asselin = 0.1;
        for i in 0..nx {
            self.u_curr[i] += asselin * (self.u_next[i] - 2.0 * self.u_curr[i] + self.u_prev[i]);
        }
        std::mem::swap(&mut self.u_prev, &mut self.u_curr);
        std::mem::swap(&mut self.u_curr, &mut self.u_next);
    }

    pub fn step(&mut self) {
        if self.steps == 0 {
            self.euler_step();
        } else {
            self.leapfrog_step();
        }
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 守恒量 I1 = integral u dx
    pub fn mass(&self) -> f32 {
        let dx = self.config.dx;
        self.u_curr.iter().map(|&u| u).sum::<f32>() * dx
    }

    /// 守恒量 I2 = integral u^2 dx
    pub fn momentum(&self) -> f32 {
        let dx = self.config.dx;
        self.u_curr.iter().map(|&u| u * u).sum::<f32>() * dx
    }

    /// 守恒量 I3 = integral (2 u^3 - 3 u_x^2) dx
    pub fn hamiltonian(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let mut h = 0.0f32;
        for i in 0..nx {
            let im = Self::wrap((i as i32) - 1, nx);
            let ip = Self::wrap((i as i32) + 1, nx);
            let u = self.u_curr[i];
            let ux = (self.u_curr[ip] - self.u_curr[im]) / (2.0 * dx);
            h += 2.0 * u * u * u - 3.0 * ux * ux;
        }
        h * dx
    }

    pub fn max_amplitude(&self) -> f32 {
        self.u_curr.iter().map(|&u| u.abs()).fold(0.0f32, f32::max)
    }

    /// 找最大值位置 (孤立子峰值, 用于追踪传播)
    pub fn find_peak(&self) -> usize {
        let mut peak = 0usize;
        let mut max_val = -1.0f32;
        for (i, &u) in self.u_curr.iter().enumerate() {
            if u > max_val {
                max_val = u;
                peak = i;
            }
        }
        peak
    }

    pub fn reset(&mut self) {
        for u in self.u_curr.iter_mut() { *u = 0.0; }
        for u in self.u_prev.iter_mut() { *u = 0.0; }
        for u in self.u_next.iter_mut() { *u = 0.0; }
        self.time = 0.0;
        self.steps = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_kdv_nonlinear_constant() {
        assert_eq!(KDV_NONLINEAR, 6.0);
    }

    #[test]
    fn test_config_default() {
        let c = KdvConfig::default();
        assert_eq!(c.nx, 256);
        assert_eq!(c.dx, 0.1);
        assert_eq!(c.dt, 0.0001);
    }

    #[test]
    fn test_config_length() {
        let c = KdvConfig { nx: 100, dx: 0.2, dt: 0.001 };
        assert!(approx_eq(c.length(), 20.0, 1e-6));
    }

    #[test]
    fn test_config_n_cells() {
        let c = KdvConfig { nx: 128, dx: 0.1, dt: 0.001 };
        assert_eq!(c.n_cells(), 128);
    }

    #[test]
    fn test_config_linear_dt_limit() {
        let c = KdvConfig { nx: 128, dx: 0.1, dt: 0.001 };
        // dx^3 / 4 = 0.001 / 4 = 0.00025
        assert!(approx_eq(c.linear_dt_limit(), 0.00025, 1e-9));
    }

    #[test]
    fn test_config_is_stable_true() {
        let c = KdvConfig { nx: 128, dx: 0.1, dt: 0.0001 };
        assert!(c.is_stable());
    }

    #[test]
    fn test_config_is_stable_false() {
        let c = KdvConfig { nx: 128, dx: 0.1, dt: 0.01 };
        assert!(!c.is_stable());
    }

    #[test]
    fn test_solver_new() {
        let s = KdvSolver::new(KdvConfig { nx: 64, dx: 0.1, dt: 0.001 });
        assert_eq!(s.u_curr.len(), 64);
        assert_eq!(s.u_prev.len(), 64);
        assert_eq!(s.u_next.len(), 64);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
    }

    #[test]
    fn test_initialize_soliton_peak() {
        let mut s = KdvSolver::new(KdvConfig { nx: 256, dx: 0.1, dt: 0.001 });
        let amplitude = 1.0;
        let center = 12.8; // 中点
        s.initialize_soliton(amplitude, center);
        // 中心网格点 i=128 (x=12.8) 应为峰值
        let peak_val = s.u_curr[128];
        assert!(approx_eq(peak_val, amplitude, 1e-5),
            "peak should be amplitude: {}", peak_val);
        // 远处应趋近 0
        let far_val = s.u_curr[0];
        assert!(far_val.abs() < 0.01);
    }

    #[test]
    fn test_initialize_soliton_symmetric() {
        let mut s = KdvSolver::new(KdvConfig { nx: 256, dx: 0.1, dt: 0.001 });
        let center = 12.8;
        s.initialize_soliton(1.0, center);
        // sech^2 关于中心对称
        let left = s.u_curr[120];
        let right = s.u_curr[136];
        assert!((left - right).abs() < 1e-5, "should be symmetric: {} vs {}", left, right);
    }

    #[test]
    fn test_initialize_two_solitons() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.001 });
        // 两个不同振幅的孤立子
        s.initialize_two_solitons(2.0, 5.0, 1.0, 30.0);
        // 应该有两个峰值
        let peak1 = s.u_curr[50]; // 第一个孤立子附近 (x=5)
        let peak2 = s.u_curr[300]; // 第二个孤立子附近 (x=30)
        assert!(peak1 > 1.5, "peak1 should be high: {}", peak1);
        assert!(peak2 > 0.5, "peak2 should be present: {}", peak2);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = KdvSolver::new(KdvConfig { nx: 64, dx: 0.1, dt: 0.001 });
        s.initialize_soliton(1.0, 3.2);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        s.step();
        assert!(approx_eq(s.time, 0.001, 1e-9));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.002, 1e-9));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = KdvSolver::new(KdvConfig { nx: 64, dx: 0.1, dt: 0.001 });
        s.initialize_soliton(1.0, 3.2);
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 0.1, 1e-6));
    }

    #[test]
    fn test_mass_conservation() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.0001 });
        s.initialize_soliton(1.0, 25.6);
        let m0 = s.mass();
        s.step_n(5000);
        let m1 = s.mass();
        let drift = ((m1 - m0) / m0).abs();
        assert!(drift < 0.05, "mass drift too large: {} -> {} ({:.4}%)", m0, m1, drift * 100.0);
    }

    #[test]
    fn test_momentum_conservation() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.0001 });
        s.initialize_soliton(1.0, 25.6);
        let p0 = s.momentum();
        s.step_n(5000);
        let p1 = s.momentum();
        let drift = ((p1 - p0) / p0).abs();
        assert!(drift < 0.05, "momentum drift too large: {} -> {} ({:.4}%)", p0, p1, drift * 100.0);
    }

    #[test]
    fn test_hamiltonian_conservation() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.0001 });
        s.initialize_soliton(1.0, 25.6);
        let h0 = s.hamiltonian();
        s.step_n(5000);
        let h1 = s.hamiltonian();
        let drift = ((h1 - h0) / h0.abs()).abs();
        assert!(drift < 0.10, "hamiltonian drift too large: {} -> {} ({:.4}%)", h0, h1, drift * 100.0);
    }

    #[test]
    fn test_soliton_propagates_rightward() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.0001 });
        s.initialize_soliton(1.0, 12.8);
        let peak0 = s.find_peak();
        s.step_n(10000); // t = 1.0, 孤立子应移动 ~2.0 (c=2A=2)
        let peak1 = s.find_peak();
        // 应向右移动
        assert!(peak1 > peak0, "soliton should move right: {} -> {}", peak0, peak1);
        let dx_moves = (peak1 as i32 - peak0 as i32) as f32;
        let expected_moves = 2.0 / 0.1; // c*t/dx = 2*1.0/0.1 = 20
        assert!((dx_moves - expected_moves).abs() < 5.0,
            "soliton moved {} cells, expected ~{}: peak {} -> {}",
            dx_moves, expected_moves, peak0, peak1);
    }

    #[test]
    fn test_soliton_amplitude_preserved() {
        let mut s = KdvSolver::new(KdvConfig { nx: 512, dx: 0.1, dt: 0.0001 });
        s.initialize_soliton(1.0, 25.6);
        let a0 = s.max_amplitude();
        s.step_n(5000);
        let a1 = s.max_amplitude();
        let drift = ((a1 - a0) / a0).abs();
        assert!(drift < 0.15, "amplitude drift too large: {} -> {} ({:.4}%)", a0, a1, drift * 100.0);
    }

    #[test]
    fn test_soliton_higher_amplitude_faster() {
        // 高孤立子 (A=2) 应比低孤立子 (A=1) 移动更快
        let cfg = KdvConfig { nx: 512, dx: 0.1, dt: 0.0002 };
        let mut s_low = KdvSolver::new(cfg.clone());
        let mut s_high = KdvSolver::new(cfg.clone());
        s_low.initialize_soliton(1.0, 12.8);
        s_high.initialize_soliton(2.0, 12.8);
        s_low.step_n(2000);
        s_high.step_n(2000);
        let peak_low = s_low.find_peak();
        let peak_high = s_high.find_peak();
        assert!(peak_high > peak_low,
            "high soliton should be faster: low={}, high={}", peak_low, peak_high);
    }

    #[test]
    fn test_two_soliton_collision_preserves_peaks() {
        // 两个孤立子碰撞后应恢复 (Zabusky-Kruskal 现象)
        let mut s = KdvSolver::new(KdvConfig { nx: 1024, dx: 0.1, dt: 0.0001 });
        // 高孤立子 (A=2, c=4) 在左, 低孤立子 (A=0.5, c=1) 在右
        s.initialize_two_solitons(2.0, 25.6, 0.5, 76.8);
        let m0 = s.mass();
        let p0 = s.momentum();
        // 演化足够长时间让碰撞发生
        s.step_n(10000);
        let m1 = s.mass();
        let p1 = s.momentum();
        // 碰撞后质量/动量应守恒 (孤立子的关键性质)
        let m_drift = ((m1 - m0) / m0).abs();
        let p_drift = ((p1 - p0) / p0).abs();
        assert!(m_drift < 0.10, "mass not conserved in collision: {:.4}%", m_drift * 100.0);
        assert!(p_drift < 0.10, "momentum not conserved in collision: {:.4}%", p_drift * 100.0);
    }

    #[test]
    fn test_max_amplitude() {
        let mut s = KdvSolver::new(KdvConfig { nx: 128, dx: 0.1, dt: 0.001 });
        s.initialize_soliton(2.0, 6.4);
        assert!(approx_eq(s.max_amplitude(), 2.0, 1e-5));
    }

    #[test]
    fn test_find_peak() {
        let mut s = KdvSolver::new(KdvConfig { nx: 256, dx: 0.1, dt: 0.001 });
        s.initialize_soliton(1.0, 12.8);
        let peak = s.find_peak();
        // 峰值应在 i=128 附近 (x=12.8)
        assert!((peak as i32 - 128).abs() <= 1, "peak should be at i=128, got {}", peak);
    }

    #[test]
    fn test_reset() {
        let mut s = KdvSolver::new(KdvConfig { nx: 128, dx: 0.1, dt: 0.001 });
        s.initialize_soliton(1.0, 6.4);
        s.step_n(10);
        assert!(s.steps > 0);
        assert!(s.max_amplitude() > 0.0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.max_amplitude(), 0.0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
        for &u in s.u_prev.iter() { assert_eq!(u, 0.0); }
        for &u in s.u_next.iter() { assert_eq!(u, 0.0); }
    }
}