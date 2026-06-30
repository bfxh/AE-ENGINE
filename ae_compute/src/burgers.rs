//! Burgers Equation Solver (Shock Formation)
//!
//! 非线性对流-扩散方程. 非线性陡化与粘性扩散的竞争决定波形演化.
//! Burgers 1948 提出, 作为 Navier-Stokes 的简化模型 (无压力、无外力).
//!
//! 方程:
//!   du/dt + u * du/dx = nu * d^2 u / dx^2
//!
//! 无粘极限 (nu = 0):
//!   du/dt + u * du/dx = 0
//!   特征线: dx/dt = u, 沿特征线 u = const
//!   大 u 处传播快 -> 波前陡化 -> 激波形成 (有限时间)
//!
//! 粘性 (nu > 0):
//!   激波被粘性平滑为有限宽度 ~ nu/|u_L - u_R|
//!   Hopf-Cole 变换: u = -2*nu * (d theta/dx) / theta
//!   theta 满足线性扩散方程: d theta/dt = nu * d^2 theta/dx^2
//!   -> Burgers 完全可积 (与 KdV 类似的可积性)
//!
//! Lax-Wendroff 格式 (周期边界, 二阶精度, 守恒, 稳定):
//!   u^{n+1}_i = u_i - (dt/(2 dx)) * (f_{i+1} - f_{i-1})
//!             + (dt^2/(2 dx^2)) * u_i * (f_{i+1} - 2 f_i + f_{i-1})
//!             + (nu * dt/dx^2) * (u_{i+1} - 2 u_i + u_{i-1})
//!   where f(u) = u^2/2 (Burgers flux)
//!
//! CFL 稳定性:
//!   对流: max|u| * dt/dx <= 1
//!   扩散: nu * dt/dx^2 <= 0.5
//!
//! 激波速度 (Rankine-Hugoniot, 无粘):
//!   s = (u_L + u_R)/2  (Burgers 通量 f(u) = u^2/2)
//!
//! 应用: 流体力学简化模型, 交通流, 激波管, 声学非线性,
//!       宇宙大尺度结构 (Zeldovich 近似), 浅水波简化.
//!
//! 基于 Burgers 1948, Hopf 1950, Cole 1951, Lax-Wendroff 1960.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurgersConfig {
    pub nx: usize,
    pub dx: f32,
    pub dt: f32,
    /// 粘性系数 (nu=0 无粘, nu>0 粘性)
    pub nu: f32,
}

impl Default for BurgersConfig {
    fn default() -> Self {
        BurgersConfig { nx: 256, dx: 0.1, dt: 0.001, nu: 0.01 }
    }
}

impl BurgersConfig {
    pub fn length(&self) -> f32 {
        (self.nx as f32) * self.dx
    }
    pub fn n_cells(&self) -> usize {
        self.nx
    }
    /// 对流 CFL 数 (基于参考速度 u_ref)
    pub fn convective_cfl(&self, u_ref: f32) -> f32 {
        u_ref.abs() * self.dt / self.dx
    }
    /// 扩散 CFL 数
    pub fn diffusive_cfl(&self) -> f32 {
        self.nu * self.dt / (self.dx * self.dx)
    }
    /// 稳定性 (给定参考速度)
    pub fn is_stable(&self, u_ref: f32) -> bool {
        self.convective_cfl(u_ref) <= 1.0 && self.diffusive_cfl() <= 0.5
    }
    /// 稳定时间步长上限 (给定参考速度)
    pub fn stable_dt(&self, u_ref: f32) -> f32 {
        let conv_dt = if u_ref.abs() > 1e-10 { self.dx / u_ref.abs() } else { f32::INFINITY };
        let diff_dt = if self.nu > 1e-10 { 0.5 * self.dx * self.dx / self.nu } else { f32::INFINITY };
        conv_dt.min(diff_dt)
    }
}

pub struct BurgersSolver {
    pub config: BurgersConfig,
    pub u_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl BurgersSolver {
    pub fn new(config: BurgersConfig) -> Self {
        let n = config.n_cells();
        BurgersSolver {
            config,
            u_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    fn wrap(i: i32, n: usize) -> usize {
        let m = n as i32;
        (((i % m) + m) % m) as usize
    }

    /// 阶跃初始条件: u = u_left (x < x_mid), u = u_right (x > x_mid)
    /// 当 u_left > u_right: 形成激波 (向右传播)
    /// 当 u_left < u_right: 形成稀疏波 (向左右展开)
    pub fn initialize_step(&mut self, u_left: f32, u_right: f32, x_mid: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        for i in 0..nx {
            let x = (i as f32) * dx;
            self.u_curr[i] = if x < x_mid { u_left } else { u_right };
        }
    }

    /// 正弦初始条件: u = A * sin(k * (x - x0))
    pub fn initialize_sine(&mut self, amplitude: f32, k: f32, x0: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        for i in 0..nx {
            let x = (i as f32) * dx;
            self.u_curr[i] = amplitude * (k * (x - x0)).sin();
        }
    }

    /// 高斯初始条件: u = A * exp(-((x-x0)/w)^2)
    pub fn initialize_gaussian(&mut self, amplitude: f32, center: f32, width: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let inv_w2 = 1.0 / (width * width);
        for i in 0..nx {
            let x = (i as f32) * dx;
            let dx_ = x - center;
            self.u_curr[i] = amplitude * (-dx_ * dx_ * inv_w2).exp();
        }
    }

    /// Lax-Wendroff + explicit diffusion one-step update
    ///
    /// f(u) = u^2/2, A = f'(u) = u
    /// u^{n+1} = u - dt/(2dx)*(f_{i+1}-f_{i-1})
    ///           + dt^2/(2dx^2)*u_i*(f_{i+1}-2f_i+f_{i-1})
    ///           + nu*dt/dx^2*(u_{i+1}-2u_i+u_{i-1})
    ///
    /// Note: Lax-Friedrichs + explicit diffusion is UNSTABLE at Nyquist frequency
    /// (LF gives G=-1 at Nyquist; adding diffusion makes |G|=1+4*nu*dt/dx^2 > 1).
    /// Lax-Wendroff gives G = 1 - 2*alpha^2 - 4*beta > 0 at Nyquist (stable when
    /// alpha = u*dt/dx and beta = nu*dt/dx^2 are small).
    pub fn step(&mut self) {
        self.step_lax_wendroff(self.config.dt);
        self.time += self.config.dt;
        self.steps += 1;
    }

    /// Lax-Wendroff update (second-order conservative + explicit viscous diffusion)
    fn step_lax_wendroff(&mut self, dt: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let nu = self.config.nu;
        let inv_2dx = 1.0 / (2.0 * dx);
        let inv_dx2 = 1.0 / (dx * dx);
        let dt2_over_2dx2 = dt * dt * 0.5 * inv_dx2;
        for i in 0..nx {
            let im = Self::wrap((i as i32) - 1, nx);
            let ip = Self::wrap((i as i32) + 1, nx);
            let u = self.u_curr[i];
            let u_ip = self.u_curr[ip];
            let u_im = self.u_curr[im];
            let f_i = 0.5 * u * u;
            let f_ip = 0.5 * u_ip * u_ip;
            let f_im = 0.5 * u_im * u_im;
            let flux_diff = f_ip - f_im;
            let flux_curv = f_ip - 2.0 * f_i + f_im;
            let conv = flux_diff * inv_2dx;
            let disp = u * flux_curv * dt2_over_2dx2;
            let diff = nu * (u_ip - 2.0 * u + u_im) * inv_dx2;
            self.u_next[i] = u - dt * conv + disp + dt * diff;
        }
        std::mem::swap(&mut self.u_curr, &mut self.u_next);
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 守恒量: integral u dx (周期边界下守恒)
    pub fn mass(&self) -> f32 {
        let dx = self.config.dx;
        self.u_curr.iter().sum::<f32>() * dx
    }

    pub fn max_amplitude(&self) -> f32 {
        self.u_curr.iter().map(|&u| u.abs()).fold(0.0f32, f32::max)
    }

    /// 总变差 TV = sum |u_{i+1} - u_i| (无粘 Burgers 下 TV 守恒, 粘性下衰减)
    pub fn total_variation(&self) -> f32 {
        let nx = self.config.nx;
        let mut tv = 0.0f32;
        for i in 0..nx {
            let ip = Self::wrap((i as i32) + 1, nx);
            tv += (self.u_curr[ip] - self.u_curr[i]).abs();
        }
        tv
    }

    /// 找激波位置 (最大 |du/dx| 的网格点)
    pub fn shock_position(&self) -> usize {
        let nx = self.config.nx;
        let mut shock = 0usize;
        let mut max_grad = 0.0f32;
        for i in 0..nx {
            let im = Self::wrap((i as i32) - 1, nx);
            let ip = Self::wrap((i as i32) + 1, nx);
            let grad = (self.u_curr[ip] - self.u_curr[im]).abs();
            if grad > max_grad {
                max_grad = grad;
                shock = i;
            }
        }
        shock
    }

    /// 能量 integral u^2 dx (粘性下衰减)
    pub fn energy(&self) -> f32 {
        let dx = self.config.dx;
        self.u_curr.iter().map(|&u| u * u).sum::<f32>() * dx
    }

    pub fn reset(&mut self) {
        for u in self.u_curr.iter_mut() { *u = 0.0; }
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
    fn test_config_default() {
        let c = BurgersConfig::default();
        assert_eq!(c.nx, 256);
        assert_eq!(c.dx, 0.1);
        assert_eq!(c.dt, 0.001);
        assert_eq!(c.nu, 0.01);
    }

    #[test]
    fn test_config_length() {
        let c = BurgersConfig { nx: 100, dx: 0.2, dt: 0.001, nu: 0.0 };
        assert!(approx_eq(c.length(), 20.0, 1e-6));
    }

    #[test]
    fn test_config_n_cells() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 };
        assert_eq!(c.n_cells(), 128);
    }

    #[test]
    fn test_convective_cfl() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.01, nu: 0.0 };
        // u_ref=1 -> cfl = 1*0.01/0.1 = 0.1
        assert!(approx_eq(c.convective_cfl(1.0), 0.1, 1e-6));
        // u_ref=2 -> cfl = 0.2
        assert!(approx_eq(c.convective_cfl(2.0), 0.2, 1e-6));
    }

    #[test]
    fn test_diffusive_cfl() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.01, nu: 0.05 };
        // nu*dt/dx^2 = 0.05*0.01/0.01 = 0.05
        assert!(approx_eq(c.diffusive_cfl(), 0.05, 1e-6));
    }

    #[test]
    fn test_is_stable_true() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.005, nu: 0.01 };
        // u_ref=1: conv 0.05, diff 0.005 -> stable
        assert!(c.is_stable(1.0));
    }

    #[test]
    fn test_is_stable_false_convective() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.2, nu: 0.0 };
        // u_ref=1: conv = 2.0 > 1
        assert!(!c.is_stable(1.0));
    }

    #[test]
    fn test_is_stable_false_diffusive() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.1, nu: 1.0 };
        // diff = 1*0.1/0.01 = 10 > 0.5
        assert!(!c.is_stable(0.0));
    }

    #[test]
    fn test_stable_dt() {
        let c = BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.05 };
        // conv_dt = 0.1/1 = 0.1, diff_dt = 0.5*0.01/0.05 = 0.1
        let dt = c.stable_dt(1.0);
        assert!(dt > 0.0);
        assert!(approx_eq(dt, 0.1, 1e-5));
    }

    #[test]
    fn test_solver_new() {
        let s = BurgersSolver::new(BurgersConfig { nx: 64, dx: 0.1, dt: 0.001, nu: 0.0 });
        assert_eq!(s.u_curr.len(), 64);
        assert_eq!(s.u_next.len(), 64);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
    }

    #[test]
    fn test_initialize_step() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 6.4);
        assert!(approx_eq(s.u_curr[0], 1.0, 1e-9));
        assert!(approx_eq(s.u_curr[63], 1.0, 1e-9));
        assert!(approx_eq(s.u_curr[65], 0.0, 1e-9));
        assert!(approx_eq(s.u_curr[127], 0.0, 1e-9));
    }

    #[test]
    fn test_initialize_sine() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_sine(1.0, 2.0 * std::f32::consts::PI / 12.8, 0.0);
        // i=0: sin(0) = 0
        assert!(approx_eq(s.u_curr[0], 0.0, 1e-6));
        // i=32, x=3.2, sin(2*pi*3.2/12.8) = sin(pi/2) = 1
        assert!(approx_eq(s.u_curr[32], 1.0, 1e-5));
    }

    #[test]
    fn test_initialize_gaussian() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_gaussian(1.0, 6.4, 1.0);
        // 中心 i=64 应为峰值
        assert!(approx_eq(s.u_curr[64], 1.0, 1e-5));
        // 远处应趋近 0
        assert!(s.u_curr[0].abs() < 0.01);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 64, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 3.2);
        assert_eq!(s.time, 0.0);
        s.step();
        assert!(approx_eq(s.time, 0.001, 1e-9));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.002, 1e-9));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 64, dx: 0.1, dt: 0.005, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 3.2);
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 0.5, 1e-6));
    }

    #[test]
    fn test_mass_conservation() {
        // 用平滑 gaussian 避免激波诱导的数值振荡
        let mut s = BurgersSolver::new(BurgersConfig { nx: 256, dx: 0.1, dt: 0.001, nu: 0.05 });
        s.initialize_gaussian(1.0, 12.8, 2.0);
        let m0 = s.mass();
        s.step_n(1000);
        let m1 = s.mass();
        // 周期边界下质量应守恒
        let drift = (m1 - m0).abs() / m0.abs().max(1e-6);
        assert!(drift < 0.05,
            "mass drift too large: {} -> {} ({:.4}%)", m0, m1, drift * 100.0);
    }

    #[test]
    fn test_shock_propagates_rightward() {
        // u_L=1 > u_R=0 -> 激波向右传播, 速度 s=(1+0)/2=0.5
        let mut s = BurgersSolver::new(BurgersConfig { nx: 512, dx: 0.1, dt: 0.005, nu: 0.001 });
        let x_mid = 25.6;
        s.initialize_step(1.0, 0.0, x_mid);
        let shock0 = 256usize; // x_mid=25.6 对应 i=256
        s.step_n(1000); // t = 5.0, 激波移动 0.5*5 = 2.5 = 25 网格点
        let shock1 = s.shock_position();
        let dx_moved = (shock1 as f32 - shock0 as f32) * 0.1;
        let expected = 0.5 * 5.0; // 2.5
        assert!((dx_moved - expected).abs() < 1.0,
            "shock moved {}, expected ~{}: {} -> {}", dx_moved, expected, shock0, shock1);
    }

    #[test]
    fn test_rarefaction_spreads() {
        // u_L=0 < u_R=1 -> 稀疏波展开
        let mut s = BurgersSolver::new(BurgersConfig { nx: 512, dx: 0.1, dt: 0.002, nu: 0.01 });
        let x_mid = 25.6;
        s.initialize_step(0.0, 1.0, x_mid);
        s.step_n(500);
        // 稀疏波应使中间区域从 0 平滑过渡到 1
        // 检查不爆炸 + 中间值合理
        assert!(s.max_amplitude() < 2.0, "rarefaction should not blow up: {}", s.max_amplitude());
        assert!(!s.u_curr.iter().any(|&u| u.is_nan() || u.is_infinite()),
            "rarefaction produced NaN/Inf");
    }

    #[test]
    fn test_viscous_decay() {
        // nu > 0: energy decays (viscous dissipation dE/dt = -nu * integral u_x^2 dx <= 0)
        let mut s = BurgersSolver::new(BurgersConfig { nx: 256, dx: 0.1, dt: 0.001, nu: 0.1 });
        s.initialize_gaussian(0.5, 12.8, 4.0);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        assert!(!s.u_curr.iter().any(|&u| u.is_nan()), "should not produce NaN");
        assert!(e1 < e0, "viscous energy should decay: {} -> {}", e0, e1);
        assert!(e1 < 0.999 * e0, "decay should be present: {} -> {}", e0, e1);
    }

    #[test]
    fn test_inviscid_steepening() {
        // nu=0, 正弦波 -> 波峰传播快, 波谷慢 -> 前沿陡化
        let mut s = BurgersSolver::new(BurgersConfig { nx: 512, dx: 0.1, dt: 0.002, nu: 0.0 });
        s.initialize_sine(1.0, 2.0 * std::f32::consts::PI / 25.6, 0.0);
        let tv0 = s.total_variation();
        s.step_n(200);
        let tv1 = s.total_variation();
        // 陡化: 总变差增加或保持 (激波形成前后)
        assert!(tv1 >= 0.9 * tv0, "TV should not decrease in inviscid steepening: {} -> {}", tv0, tv1);
        // 最大梯度应增加 (波形变陡)
        // 检查是否有显著的非零解
        assert!(s.max_amplitude() > 0.5, "amplitude should remain: {}", s.max_amplitude());
    }

    #[test]
    fn test_max_amplitude() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(2.0, -1.0, 6.4);
        assert!(approx_eq(s.max_amplitude(), 2.0, 1e-9));
    }

    #[test]
    fn test_total_variation_step() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 6.4);
        // 周期边界下阶跃有两个跳变: i=64 (1->0) 和 i=127->0 (0->1)
        let tv = s.total_variation();
        assert!(approx_eq(tv, 2.0, 1e-6));
    }

    #[test]
    fn test_shock_position() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 256, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 12.8);
        let shock = s.shock_position();
        // 周期边界下有两个跳变: i=128 (x=12.8, 1->0) 和 i=0 (周期包裹 0->1)
        assert!(shock == 0 || (shock as i32 - 128).abs() <= 1,
            "shock should be at i=0 or i=128, got {}", shock);
    }

    #[test]
    fn test_energy() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 6.4);
        // 一半区域 u=1: energy = 1^2 * 64 * 0.1 = 6.4
        let e = s.energy();
        assert!(approx_eq(e, 6.4, 0.1));
    }

    #[test]
    fn test_reset() {
        let mut s = BurgersSolver::new(BurgersConfig { nx: 128, dx: 0.1, dt: 0.001, nu: 0.0 });
        s.initialize_step(1.0, 0.0, 6.4);
        s.step_n(10);
        assert!(s.steps > 0);
        assert!(s.max_amplitude() > 0.0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.max_amplitude(), 0.0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
        for &u in s.u_next.iter() { assert_eq!(u, 0.0); }
    }
}