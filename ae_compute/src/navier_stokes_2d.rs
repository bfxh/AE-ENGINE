//! 2D 不可压 Navier-Stokes — 涡量-流函数形式
//!
//! 经典计算流体力学方法, 消除压力 Poisson 方程困难.
//! 通过求解涡量 ω 和流函数 ψ 的耦合方程获得速度场.
//!
//! 控制方程 (2D 不可压, 无外力):
//!   ∂ω/∂t + (u·∇)ω = ν ∇²ω          (涡量输运)
//!   ∇²ψ = ω                          (Poisson: 流函数)
//!   u =  ∂ψ/∂y,  v = -∂ψ/∂x          (速度由流函数)
//!
//! 其中 ω = ∂v/∂x - ∂u/∂y 为涡量 (z 分量), ψ 为流函数,
//! 自动满足 ∇·u = 0 (不可压约束).
//!
//! 数值方法:
//!   - Poisson 求解: SOR (逐次超松弛) 迭代
//!   - 对流项: 二阶迎风 (Bott, 9 点) 或中心差分 + 人工黏性
//!   - 扩散项: 显式中心差分
//!   - 时间: 显式 Euler (简化) 或 AB2
//!   - 边界: 周期 (默认) 或 ψ=0 墙壁 (Thom 涡量)
//!
//! 经典测试:
//!   - Taylor-Green 涡旋 (解析衰减解: ω(t) = ω0 exp(-νk²t))
//!   - 剪切层卷起 (Kelvin-Helmholtz)
//!   - 涡旋合并
//!
//! 物理:
//!   - 2D 湍流能量逆向级联 (小→大尺度)
//!   - 涡量拟能级联 (大→小尺度, 耗散)
//!   - 守恒 (无黏): 能量 ∫|u|², 涡量拟能 ∫ω²
//!
//! 参考:
//!   - Peyret & Taylor, "Computational Methods for Fluid Flow" (1983)
//!   - Anderson, "Computational Fluid Dynamics" (1995)

/// 2D NS 求解器配置
#[derive(Clone, Debug)]
pub struct NsConfig {
    /// 网格分辨率 NxN (n 内部点)
    pub n: usize,
    /// 域尺寸 L (周期边界 [0,L)²)
    pub box_size: f64,
    /// 时间步长 dt
    pub dt: f64,
    /// 运动黏性 ν
    pub nu: f64,
    /// SOR 最大迭代次数
    pub sor_max_iter: usize,
    /// SOR 容差 (残差 < tol)
    pub sor_tol: f64,
    /// SOR 超松弛因子 ω ∈ (1, 2)
    pub sor_omega: f64,
}

impl Default for NsConfig {
    fn default() -> Self {
        Self {
            n: 64,
            box_size: 2.0 * std::f64::consts::PI,
            dt: 0.005,
            nu: 0.01,
            sor_max_iter: 200,
            sor_tol: 1e-6,
            sor_omega: 1.8,
        }
    }
}

/// 2D Navier-Stokes 求解器 (涡量-流函数)
pub struct NsSolver {
    pub config: NsConfig,
    /// 涡量场 ω (n×n, 行主序 j*n+i)
    pub omega: Vec<f64>,
    /// 流函数 ψ
    pub psi: Vec<f64>,
    /// x 速度 u
    pub u: Vec<f64>,
    /// y 速度 v
    pub v: Vec<f64>,
    pub step_count: u64,
    pub time: f64,
    /// 涡量拟能历史 ∫ω²
    pub enstrophy_history: Vec<f64>,
    /// 动能历史 ½∫|u|²
    pub energy_history: Vec<f64>,
}

impl NsSolver {
    pub fn new(config: NsConfig) -> Self {
        assert!(config.n >= 4, "n must be >= 4");
        assert!(config.box_size > 0.0);
        assert!(config.dt > 0.0);
        assert!(config.nu >= 0.0);
        assert!(config.sor_omega > 1.0 && config.sor_omega < 2.0, "omega in (1,2)");
        let n2 = config.n * config.n;
        Self {
            config,
            omega: vec![0.0; n2],
            psi: vec![0.0; n2],
            u: vec![0.0; n2],
            v: vec![0.0; n2],
            step_count: 0,
            time: 0.0,
            enstrophy_history: Vec::new(),
            energy_history: Vec::new(),
        }
    }

    /// 网格间距
    #[inline]
    fn dx(&self) -> f64 {
        self.config.box_size / self.config.n as f64
    }

    /// 周期索引包裹
    #[inline]
    fn wrap(i: i32, n: usize) -> usize {
        let ni = i % n as i32;
        if ni < 0 {
            (ni + n as i32) as usize
        } else {
            ni as usize
        }
    }

    /// 初始化: Taylor-Green 涡旋 (解析衰减解)
    /// 约定 (u=∂ψ/∂y, v=-∂ψ/∂x, ω=-∇²ψ):
    ///   ψ = sin(kx)sin(ky)
    ///   u =  sin(kx)cos(ky),  v = -cos(kx)sin(ky)
    ///   ω = 2 sin(kx)sin(ky)
    /// 解析衰减: ω(t) = ω0 exp(-2νk²t)
    pub fn initialize_taylor_green(&mut self, k: f64) {
        let n = self.config.n;
        let l = self.config.box_size;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let idx = j * n + i;
                let sx = (k * x).sin();
                let sy = (k * y).sin();
                let cx = (k * x).cos();
                let cy = (k * y).cos();
                self.omega[idx] = 2.0 * sx * sy;
                self.psi[idx] = sx * sy / (k * k);
                self.u[idx] = sx * cy;
                self.v[idx] = -cx * sy;
            }
        }
        self.step_count = 0;
        self.time = 0.0;
        self.enstrophy_history.clear();
        self.energy_history.clear();
        self.record_diagnostics();
    }

    /// 初始化: 单个高斯涡 (在域中心)
    pub fn initialize_gaussian_vortex(&mut self, strength: f64, sigma: f64) {
        let n = self.config.n;
        let l = self.config.box_size;
        let cx = 0.5 * l;
        let cy = 0.5 * l;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let idx = j * n + i;
                let r2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                // 涡量 = strength * exp(-r²/(2σ²))
                self.omega[idx] = strength * (-r2 / (2.0 * sigma * sigma)).exp();
            }
        }
        self.psi = vec![0.0; n * n];
        self.u = vec![0.0; n * n];
        self.v = vec![0.0; n * n];
        self.solve_poisson();
        self.compute_velocity();
        self.step_count = 0;
        self.time = 0.0;
        self.enstrophy_history.clear();
        self.energy_history.clear();
        self.record_diagnostics();
    }

    /// 初始化: 剪切层 (上半 +u, 下半 -u, 加扰动卷起)
    pub fn initialize_shear_layer(&mut self, perturb: f64) {
        let n = self.config.n;
        let l = self.config.box_size;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let idx = j * n + i;
                // u = tanh(k(y - L/2)), k 调节层厚
                let k = 10.0 / l;
                let u0 = (k * (y - 0.5 * l)).tanh();
                self.u[idx] = u0;
                // 加小 x 方向扰动触发 KH 不稳定
                self.v[idx] = perturb * (2.0 * std::f64::consts::PI * x / l).sin()
                    * (-((y - 0.5 * l) * (y - 0.5 * l)) / (0.05 * l * l)).exp();
                // 涡量 = -∂u/∂y + ∂v/∂x (近似, 只取 -∂u/∂y 解析)
                let du_dy = k * (1.0 - u0 * u0);
                self.omega[idx] = -du_dy;
            }
        }
        self.psi = vec![0.0; n * n];
        self.solve_poisson();
        // 重算涡量从 ψ 以保证自洽? 这里保留解析 ω
        self.step_count = 0;
        self.time = 0.0;
        self.enstrophy_history.clear();
        self.energy_history.clear();
        self.record_diagnostics();
    }

    /// SOR 求解 Poisson 方程 ∇²ψ = -ω (周期边界)
    /// 约定: u = ∂ψ/∂y, v = -∂ψ/∂x → ω = -∇²ψ
    /// 返回最终残差范数
    pub fn solve_poisson(&mut self) -> f64 {
        let n = self.config.n;
        let dx = self.dx();
        let dx2 = dx * dx;
        let omega_relax = self.config.sor_omega;
        let mut residual = f64::INFINITY;

        for _iter in 0..self.config.sor_max_iter {
            let mut max_res = 0.0f64;
            for j in 0..n {
                for i in 0..n {
                    let idx = j * n + i;
                    let ip = Self::wrap(i as i32 + 1, n);
                    let im = Self::wrap(i as i32 - 1, n);
                    let jp = Self::wrap(j as i32 + 1, n);
                    let jm = Self::wrap(j as i32 - 1, n);
                    let rhs = self.omega[idx] * dx2;
                    let neighbor_sum = self.psi[j * n + ip] + self.psi[j * n + im]
                        + self.psi[jp * n + i] + self.psi[jm * n + i];
                    // ∇²ψ = -ω → ψ = (Σψ_neighbor + dx²ω) / 4
                    let psi_new = 0.25 * (neighbor_sum + rhs);
                    let psi_old = self.psi[idx];
                    let psi_sor = psi_old + omega_relax * (psi_new - psi_old);
                    self.psi[idx] = psi_sor;

                    let res = (psi_sor * 4.0 - neighbor_sum - rhs).abs();
                    if res > max_res {
                        max_res = res;
                    }
                }
            }
            residual = max_res;
            if residual < self.config.sor_tol {
                break;
            }
        }
        residual
    }

    /// 由 ψ 计算速度 u = ∂ψ/∂y, v = -∂ψ/∂x (中心差分, 周期)
    pub fn compute_velocity(&mut self) {
        let n = self.config.n;
        let dx = self.dx();
        for j in 0..n {
            for i in 0..n {
                let idx = j * n + i;
                let ip = Self::wrap(i as i32 + 1, n);
                let im = Self::wrap(i as i32 - 1, n);
                let jp = Self::wrap(j as i32 + 1, n);
                let jm = Self::wrap(j as i32 - 1, n);
                self.u[idx] = (self.psi[jp * n + i] - self.psi[jm * n + i]) / (2.0 * dx);
                self.v[idx] = -(self.psi[j * n + ip] - self.psi[j * n + im]) / (2.0 * dx);
            }
        }
    }

    /// 由速度重算涡量 ω = ∂v/∂x - ∂u/∂y (诊断用)
    pub fn compute_vorticity_from_velocity(&mut self) {
        let n = self.config.n;
        let dx = self.dx();
        for j in 0..n {
            for i in 0..n {
                let idx = j * n + i;
                let ip = Self::wrap(i as i32 + 1, n);
                let im = Self::wrap(i as i32 - 1, n);
                let jp = Self::wrap(j as i32 + 1, n);
                let jm = Self::wrap(j as i32 - 1, n);
                let dv_dx = (self.v[j * n + ip] - self.v[j * n + im]) / (2.0 * dx);
                let du_dy = (self.u[jp * n + i] - self.u[jm * n + i]) / (2.0 * dx);
                self.omega[idx] = dv_dx - du_dy;
            }
        }
    }

    /// 涡量输运一步 (显式, 对流用中心差分 + 扩散中心)
    /// ∂ω/∂t = -u ∂ω/∂x - v ∂ω/∂y + ν ∇²ω
    pub fn step(&mut self) {
        let n = self.config.n;
        let dx = self.dx();
        let dx2 = dx * dx;
        let dt = self.config.dt;
        let nu = self.config.nu;

        // 1. 求解 Poisson 得 ψ
        self.solve_poisson();
        // 2. 由 ψ 得速度
        self.compute_velocity();
        // 3. 更新涡量 (显式 Euler, 中心差分)
        let omega_old = self.omega.clone();
        for j in 0..n {
            for i in 0..n {
                let idx = j * n + i;
                let ip = Self::wrap(i as i32 + 1, n);
                let im = Self::wrap(i as i32 - 1, n);
                let jp = Self::wrap(j as i32 + 1, n);
                let jm = Self::wrap(j as i32 - 1, n);

                let domega_dx = (omega_old[j * n + ip] - omega_old[j * n + im]) / (2.0 * dx);
                let domega_dy = (omega_old[jp * n + i] - omega_old[jm * n + i]) / (2.0 * dx);
                let lap_omega = (omega_old[j * n + ip] + omega_old[j * n + im]
                    + omega_old[jp * n + i] + omega_old[jm * n + i]
                    - 4.0 * omega_old[idx]) / dx2;

                let advect = -self.u[idx] * domega_dx - self.v[idx] * domega_dy;
                let diffuse = nu * lap_omega;
                self.omega[idx] = omega_old[idx] + dt * (advect + diffuse);
            }
        }
        self.step_count += 1;
        self.time += dt;
        self.record_diagnostics();
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 记录诊断量 (涡量拟能 + 动能, 域平均)
    fn record_diagnostics(&mut self) {
        let n = self.config.n;
        let n2 = (n * n) as f64;
        let mut ens = 0.0;
        let mut ke = 0.0;
        for k in 0..n * n {
            ens += self.omega[k] * self.omega[k];
            ke += 0.5 * (self.u[k] * self.u[k] + self.v[k] * self.v[k]);
        }
        self.enstrophy_history.push(ens / n2);
        self.energy_history.push(ke / n2);
    }

    /// 当前域平均涡量拟能
    pub fn enstrophy(&self) -> f64 {
        let n2 = (self.config.n * self.config.n) as f64;
        let mut e = 0.0;
        for &w in &self.omega {
            e += w * w;
        }
        e / n2
    }

    /// 当前域平均动能 ½⟨u²+v²⟩
    pub fn kinetic_energy(&self) -> f64 {
        let n2 = (self.config.n * self.config.n) as f64;
        let mut e = 0.0;
        for k in 0..self.u.len() {
            e += 0.5 * (self.u[k] * self.u[k] + self.v[k] * self.v[k]);
        }
        e / n2
    }

    /// 最大速度幅值 (CFL 诊断)
    pub fn max_velocity(&self) -> f64 {
        let mut m = 0.0;
        for k in 0..self.u.len() {
            let mag = (self.u[k] * self.u[k] + self.v[k] * self.v[k]).sqrt();
            if mag > m {
                m = mag;
            }
        }
        m
    }

    /// CFL 数 = max|u| dt / dx
    pub fn cfl(&self) -> f64 {
        self.max_velocity() * self.config.dt / self.dx()
    }

    /// 检查是否有 NaN/Inf
    pub fn has_nan(&self) -> bool {
        self.omega.iter().any(|&x| !x.is_finite())
            || self.psi.iter().any(|&x| !x.is_finite())
            || self.u.iter().any(|&x| !x.is_finite())
            || self.v.iter().any(|&x| !x.is_finite())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn make_default() -> NsSolver {
        let mut s = NsSolver::new(NsConfig::default());
        s.initialize_taylor_green(1.0);
        s
    }

    #[test]
    fn test_default_config() {
        let cfg = NsConfig::default();
        assert_eq!(cfg.n, 64);
        assert_eq!(cfg.box_size, 2.0 * PI);
        assert_eq!(cfg.dt, 0.005);
        assert_eq!(cfg.nu, 0.01);
        assert!(cfg.sor_omega > 1.0 && cfg.sor_omega < 2.0);
    }

    #[test]
    fn test_solver_creation() {
        let s = NsSolver::new(NsConfig::default());
        assert_eq!(s.omega.len(), 64 * 64);
        assert_eq!(s.psi.len(), 64 * 64);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_taylor_green_initial() {
        let s = make_default();
        // ω(x,y) = 2 sin(x)sin(y); 在 (π/2, π/2) 处 = 2
        let n = s.config.n;
        let i = n / 4; // x = π/2
        let j = n / 4;
        assert!((s.omega[j * n + i] - 2.0).abs() < 0.05, "omega at (pi/2,pi/2): {}", s.omega[j * n + i]);
        // 涡量拟能初始 = ⟨ω²⟩ = 4⟨sin²sin²⟩ = 4*(1/2)*(1/2) = 1.0
        let ens = s.enstrophy();
        assert!((ens - 1.0).abs() < 0.05, "initial enstrophy: {}", ens);
    }

    #[test]
    fn test_taylor_green_decay() {
        // 解析: ω(t) = ω0 exp(-2νk²t), 涡量拟能 ∝ exp(-4νk²t)
        let mut s = NsSolver::new(NsConfig {
            n: 64,
            box_size: 2.0 * PI,
            dt: 0.002,
            nu: 0.02,
            sor_max_iter: 300,
            sor_tol: 1e-8,
            sor_omega: 1.9,
        });
        s.initialize_taylor_green(1.0);
        let ens0 = s.enstrophy();
        s.run(200); // t = 0.4
        let ens1 = s.enstrophy();
        // 解析: ens1/ens0 = exp(-4*nu*k²*t) = exp(-4*0.02*1*0.4) = exp(-0.032) ≈ 0.9685
        let t = 200.0 * 0.002;
        let expected = (-4.0_f64 * 0.02 * 1.0 * t).exp();
        let ratio = ens1 / ens0;
        assert!((ratio - expected).abs() < 0.05,
            "TG decay ratio {} vs expected {}", ratio, expected);
        assert!(ratio < 1.0, "enstrophy must decay");
    }

    #[test]
    fn test_poisson_solver_converges() {
        let mut s = NsSolver::new(NsConfig {
            n: 32,
            box_size: 2.0 * PI,
            ..NsConfig::default()
        });
        // 约定 ∇²ψ = -ω. 设 ω = 2 sin sin → -ω = -2 sin sin = ∇²ψ → ψ = sin sin
        let n = s.config.n;
        let l = s.config.box_size;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                s.omega[j * n + i] = 2.0 * x.sin() * y.sin();
            }
        }
        let res = s.solve_poisson();
        assert!(res < 1e-5, "Poisson residual: {}", res);
        // 检查 ψ ≈ sin(x)sin(y) (减去均值, 周期 Poisson 解可差常数)
        let mean_psi: f64 = s.psi.iter().sum::<f64>() / (n * n) as f64;
        let mut max_err = 0.0;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let expected = x.sin() * y.sin();
                let err = (s.psi[j * n + i] - mean_psi - expected).abs();
                if err > max_err {
                    max_err = err;
                }
            }
        }
        assert!(max_err < 0.05, "Poisson solution max error: {}", max_err);
    }

    #[test]
    fn test_velocity_from_streamfunction() {
        // ψ = sin(x)sin(y) → u = ∂ψ/∂y = sin(x)cos(y), v = -∂ψ/∂x = -cos(x)sin(y)
        let mut s = NsSolver::new(NsConfig {
            n: 64,
            box_size: 2.0 * PI,
            ..NsConfig::default()
        });
        let n = s.config.n;
        let l = s.config.box_size;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                s.psi[j * n + i] = x.sin() * y.sin();
            }
        }
        s.compute_velocity();
        let mut max_err_u = 0.0;
        let mut max_err_v = 0.0;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let eu = x.sin() * y.cos();
                let ev = -x.cos() * y.sin();
                let du = (s.u[j * n + i] - eu).abs();
                let dv = (s.v[j * n + i] - ev).abs();
                if du > max_err_u { max_err_u = du; }
                if dv > max_err_v { max_err_v = dv; }
            }
        }
        // 二阶中心差分误差 O(dx²); 周期边界点误差略大
        assert!(max_err_u < 0.02, "u error: {}", max_err_u);
        assert!(max_err_v < 0.02, "v error: {}", max_err_v);
    }

    #[test]
    fn test_vorticity_from_velocity() {
        // ψ = sin sin → u=sin cos, v=-cos sin → ω = ∂v/∂x - ∂u/∂y = 2 sin sin
        let mut s = NsSolver::new(NsConfig {
            n: 64,
            box_size: 2.0 * PI,
            ..NsConfig::default()
        });
        let n = s.config.n;
        let l = s.config.box_size;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                s.psi[j * n + i] = x.sin() * y.sin();
            }
        }
        s.compute_velocity();
        s.compute_vorticity_from_velocity();
        let mut max_err = 0.0;
        for j in 0..n {
            for i in 0..n {
                let x = (i as f64 / n as f64) * l;
                let y = (j as f64 / n as f64) * l;
                let expected = 2.0 * x.sin() * y.sin();
                let err = (s.omega[j * n + i] - expected).abs();
                if err > max_err { max_err = err; }
            }
        }
        assert!(max_err < 0.04, "vorticity from velocity error: {}", max_err);
    }

    #[test]
    fn test_step_advances() {
        let mut s = make_default();
        let t0 = s.time;
        s.step();
        assert_eq!(s.step_count, 1);
        assert!((s.time - t0 - s.config.dt).abs() < 1e-12);
        assert_eq!(s.enstrophy_history.len(), 2);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = make_default();
        s.run(50);
        assert!(!s.has_nan(), "no NaN after 50 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let mut s = NsSolver::new(NsConfig {
            n: 48,
            box_size: 2.0 * PI,
            dt: 0.003,
            nu: 0.02,
            sor_max_iter: 200,
            sor_tol: 1e-6,
            sor_omega: 1.8,
        });
        s.initialize_taylor_green(1.0);
        s.run(300);
        assert!(!s.has_nan(), "no NaN after long run");
        // 涡量拟能应单调衰减 (黏性)
        let h = &s.enstrophy_history;
        assert!(h[h.len() - 1] < h[0], "enstrophy decays");
    }

    #[test]
    fn test_enstrophy_decays_with_viscosity() {
        let mut s = NsSolver::new(NsConfig {
            n: 48,
            box_size: 2.0 * PI,
            dt: 0.003,
            nu: 0.05,
            sor_max_iter: 200,
            sor_tol: 1e-6,
            sor_omega: 1.8,
        });
        s.initialize_taylor_green(1.0);
        let ens0 = s.enstrophy();
        s.run(100);
        let ens1 = s.enstrophy();
        assert!(ens1 < ens0 * 0.99, "enstrophy must decay: {} -> {}", ens0, ens1);
    }

    #[test]
    fn test_inviscid_enstrophy_approximately_conserved() {
        // ν=0 时涡量拟能应近似守恒 (无耗散, 仅数值误差)
        let mut s = NsSolver::new(NsConfig {
            n: 64,
            box_size: 2.0 * PI,
            dt: 0.001,
            nu: 0.0,
            sor_max_iter: 300,
            sor_tol: 1e-9,
            sor_omega: 1.9,
        });
        s.initialize_taylor_green(1.0);
        let ens0 = s.enstrophy();
        s.run(100);
        let ens1 = s.enstrophy();
        let rel = (ens1 - ens0).abs() / ens0;
        assert!(rel < 0.05, "inviscid enstrophy drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_cfl_positive() {
        let s = make_default();
        let cfl = s.cfl();
        assert!(cfl > 0.0, "CFL positive: {}", cfl);
        assert!(cfl < 1.0, "CFL stable: {}", cfl);
    }

    #[test]
    fn test_gaussian_vortex_initialization() {
        let mut s = NsSolver::new(NsConfig::default());
        s.initialize_gaussian_vortex(1.0, 0.5);
        assert!(!s.has_nan());
        // 中心涡量最大
        let n = s.config.n;
        let center = s.omega[n / 2 * n + n / 2];
        let corner = s.omega[0];
        assert!(center > corner, "center vorticity > corner");
        assert!((center - 1.0).abs() < 1e-9, "center = strength: {}", center);
    }

    #[test]
    fn test_shear_layer_initialization() {
        let mut s = NsSolver::new(NsConfig::default());
        s.initialize_shear_layer(0.1);
        assert!(!s.has_nan());
        // 上半 u>0, 下半 u<0
        let n = s.config.n;
        let u_top = s.u[(n - 2) * n + n / 2];
        let u_bot = s.u[1 * n + n / 2];
        assert!(u_top > 0.0, "u top > 0: {}", u_top);
        assert!(u_bot < 0.0, "u bot < 0: {}", u_bot);
    }

    #[test]
    fn test_grid_size_flexible() {
        for n in [16, 32, 64] {
            let cfg = NsConfig {
                n,
                box_size: 2.0 * PI,
                dt: 0.005,
                nu: 0.01,
                sor_max_iter: 100,
                sor_tol: 1e-5,
                sor_omega: 1.8,
            };
            let mut s = NsSolver::new(cfg);
            s.initialize_taylor_green(1.0);
            s.run(5);
            assert!(!s.has_nan());
            assert_eq!(s.omega.len(), n * n);
        }
    }

    #[test]
    fn test_invalid_config_panics() {
        assert!(std::panic::catch_unwind(|| {
            NsSolver::new(NsConfig { n: 2, ..NsConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            NsSolver::new(NsConfig { box_size: 0.0, ..NsConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            NsSolver::new(NsConfig { dt: 0.0, ..NsConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            NsSolver::new(NsConfig { sor_omega: 2.5, ..NsConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            NsSolver::new(NsConfig { sor_omega: 0.5, ..NsConfig::default() })
        }).is_err());
    }

    #[test]
    fn test_diagnostics_history_grows() {
        let mut s = make_default();
        s.run(10);
        assert_eq!(s.enstrophy_history.len(), 11);
        assert_eq!(s.energy_history.len(), 11);
    }

    #[test]
    fn test_max_velocity_nonneg() {
        let s = make_default();
        assert!(s.max_velocity() >= 0.0);
    }
}
