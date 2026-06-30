//! Gross-Pitaevskii Equation — 玻色-爱因斯坦凝聚 (BEC) 平均场模型
//!
//! Eugene Gross (1961) 与 Lev Pitaevskii (1961) 独立提出的方程, 描述
//! 玻色-爱因斯坦凝聚体在平均场近似下的波函数动力学. 是非线性
//! Schrödinger 方程 (NLS) 的物理实例, 含外部势阱 (通常为磁阱/光阱
//! 谐振势) 与原子间相互作用.
//!
//! 方程 (无量纲, ħ = m = 1):
//!   i ∂ψ/∂t = [-½ ∇² + V(r) + g |ψ|²] ψ
//!
//! 其中:
//!   ψ = 复波函数 (凝聚体序参量)
//!   V(r) = 外部势阱 (谐振势 V = ½(ω_x² x² + ω_y² y²))
//!   g = 相互作用耦合 (g > 0 排斥, g < 0 吸引)
//!   |ψ|² = 凝聚体密度
//!
//! 守恒量:
//!   N = ∫|ψ|² dA  (粒子数, 严格守恒)
//!   E = ∫[½|∇ψ|² + V|ψ|² + (g/2)|ψ|⁴] dA  (能量, 严格守恒)
//!
//! 实部/虚部拆分 (ψ = u + iv):
//!   ∂u/∂t = -½ ∇²v + V v + g(u²+v²) v =: H[v]
//!   ∂v/∂t =  ½ ∇²u - V u - g(u²+v²) u = -H[u]
//!
//! 其中 H[f] = -½ ∇²f + V f + g(u²+v²) f
//!
//! 数值方法:
//!   - 5 点 Laplacian 离散 ∇²
//!   - 4 阶 Runge-Kutta (RK4) 时间推进, 长期近似保 N/E
//!   - 周期边界 (简化)
//!
//! 物理现象:
//!   - g = 0: 线性 Schrödinger 方程 (谐振势中相干态振荡)
//!   - g > 0 (排斥): 暗孤子, 涡旋 (带相位缠绕的拓扑缺陷)
//!   - g < 0 (吸引): 亮孤子 (1D 稳定), 2D/3D 坍缩 (临界粒子数)
//!   - Imaginary time propagation: 求基态 (ψ → ψ/sqrt(N), t → -iτ)
//!
//! 应用:
//!   - 超冷原子物理 (BEC 动力学)
//!   - 超流 (Landau 临界速度, 涡旋)
//!   - 量子流体 (Bogoliubov 涨落)
//!   - 非线性光学 (等效 NLS, 光孤子)
//!   - 引力波全息 (AdS/CFT 标量场)
//!
//! 基于:
//!   - Gross, E.P. 1961. "Structure of a quantized vortex in boson
//!     systems." Nuovo Cim. 20, 454.
//!   - Pitaevskii, L.P. 1961. "Vortex lines in an imperfect Bose
//!     gas." Sov. Phys. JETP 13, 451.
//!   - Pitaevskii, L. & Stringari, S. 2016. "Bose-Einstein
//!     Condensation and Superfluidity." Oxford Univ. Press.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpConfig {
    /// x 方向格点数
    pub nx: usize,
    /// y 方向格点数
    pub ny: usize,
    /// 空间步长
    pub dx: f32,
    /// 时间步长
    pub dt: f32,
    /// 相互作用耦合 g (>0 排斥, <0 吸引)
    pub g: f32,
    /// x 方向谐振势频率 ω_x
    pub omega_x: f32,
    /// y 方向谐振势频率 ω_y
    pub omega_y: f32,
}

impl Default for GpConfig {
    fn default() -> Self {
        GpConfig {
            nx: 64,
            ny: 64,
            dx: 0.2,
            dt: 0.001,
            g: 1.0,
            omega_x: 1.0,
            omega_y: 1.0,
        }
    }
}

impl GpConfig {
    /// 域大小
    pub fn domain_size(&self) -> (f32, f32) {
        ((self.nx as f32) * self.dx, (self.ny as f32) * self.dx)
    }

    /// 格点数
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    /// 在格点 (i,j) 处的外部势 V = ½(ω_x² x² + ω_y² y²)
    /// 中心在 (L_x/2, L_y/2)
    pub fn potential_at(&self, i: usize, j: usize) -> f32 {
        let (lx, ly) = self.domain_size();
        let x = (i as f32) * self.dx - lx * 0.5;
        let y = (j as f32) * self.dx - ly * 0.5;
        0.5 * (self.omega_x * self.omega_x * x * x + self.omega_y * self.omega_y * y * y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpBoundary {
    /// 周期边界
    Periodic,
    /// 零边界 (Dirichlet, ψ=0)
    Zero,
}

pub struct GpSolver {
    pub config: GpConfig,
    pub boundary: GpBoundary,
    /// 外部势 V (预计算)
    pub potential: Vec<f32>,
    /// Re(ψ) 当前
    pub u_curr: Vec<f32>,
    /// Im(ψ) 当前
    pub v_curr: Vec<f32>,
    /// Re(ψ) 下一步缓冲
    pub u_next: Vec<f32>,
    /// Im(ψ) 下一步缓冲
    pub v_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl GpSolver {
    pub fn new(config: GpConfig) -> Self {
        Self::with_boundary(config, GpBoundary::Periodic)
    }

    pub fn with_boundary(config: GpConfig, boundary: GpBoundary) -> Self {
        let n = config.n_cells();
        let mut potential = vec![0.0; n];
        let nx = config.nx;
        let ny = config.ny;
        for j in 0..ny {
            for i in 0..nx {
                potential[j * nx + i] = config.potential_at(i, j);
            }
        }
        GpSolver {
            config,
            boundary,
            potential,
            u_curr: vec![0.0; n],
            v_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            v_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    /// 初始化为 0
    pub fn initialize_zero(&mut self) {
        for v in &mut self.u_curr {
            *v = 0.0;
        }
        for v in &mut self.v_curr {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为高斯波包 (基态近似)
    /// ψ(r) = (1/πσ²)^(1/4) exp(-r²/(2σ²))
    pub fn initialize_gaussian(&mut self, sigma: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let cx = lx * 0.5;
        let cy = ly * 0.5;
        let s2 = sigma * sigma;
        let norm = (1.0 / (std::f32::consts::PI * s2)).powf(0.25);
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - cx;
                let y = (j as f32) * self.config.dx - cy;
                let r2 = x * x + y * y;
                let idx = j * nx + i;
                let psi = norm * (-r2 / (2.0 * s2)).exp();
                self.u_curr[idx] = psi;
                self.v_curr[idx] = 0.0;
            }
        }
        // 归一化使 N = 1
        let n = self.norm();
        if n > 0.0 {
            let scale = 1.0 / n.sqrt();
            for v in &mut self.u_curr {
                *v *= scale;
            }
            for v in &mut self.v_curr {
                *v *= scale;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为带涡旋的波函数 (中心相位缠绕 2π)
    /// ψ(r,θ) = ψ_gaussian(r) · exp(i·m·θ), m = 涡旋荷
    pub fn initialize_vortex(&mut self, sigma: f32, charge: i32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let cx = lx * 0.5;
        let cy = ly * 0.5;
        let s2 = sigma * sigma;
        let norm = (1.0 / (std::f32::consts::PI * s2)).powf(0.25);
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - cx;
                let y = (j as f32) * self.config.dx - cy;
                let r2 = x * x + y * y;
                let theta = y.atan2(x);
                let idx = j * nx + i;
                // 涡旋波函数: 密度在 r=0 处为零
                let amp = norm * (-r2 / (2.0 * s2)).exp() * (r2 / s2).sqrt();
                let phase = (charge as f32) * theta;
                self.u_curr[idx] = amp * phase.cos();
                self.v_curr[idx] = amp * phase.sin();
            }
        }
        let n = self.norm();
        if n > 0.0 {
            let scale = 1.0 / n.sqrt();
            for v in &mut self.u_curr {
                *v *= scale;
            }
            for v in &mut self.v_curr {
                *v *= scale;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    #[inline]
    fn wrap(&self, i: i32, n: usize) -> usize {
        let nn = n as i32;
        (((i % nn) + nn) % nn) as usize
    }

    /// 邻居索引 (按边界类型)
    #[inline]
    fn neighbor(&self, i: i32, j: i32) -> usize {
        let nx = self.config.nx as i32;
        let ny = self.config.ny as i32;
        match self.boundary {
            GpBoundary::Periodic => {
                let ii = self.wrap(i, self.config.nx);
                let jj = self.wrap(j, self.config.ny);
                (jj * self.config.nx + ii) as usize
            }
            GpBoundary::Zero => {
                let ii = i.clamp(0, nx - 1) as usize;
                let jj = j.clamp(0, ny - 1) as usize;
                (jj * self.config.nx + ii) as usize
            }
        }
    }

    /// 计算 H[f] = -½ ∇²f + V f + g(u²+v²) f (在当前 u,v 上)
    /// 输出存入 out
    fn hamiltonian_apply(&self, f: &[f32], u: &[f32], v: &[f32], out: &mut [f32]) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx2 = self.config.dx * self.config.dx;
        let g = self.config.g;
        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let ii = i as i32;
                let jj = j as i32;
                let i_e = self.neighbor(ii + 1, jj);
                let i_w = self.neighbor(ii - 1, jj);
                let i_n = self.neighbor(ii, jj + 1);
                let i_s = self.neighbor(ii, jj - 1);
                let lap = (f[i_e] + f[i_w] + f[i_n] + f[i_s] - 4.0 * f[idx]) / dx2;
                let density = u[idx] * u[idx] + v[idx] * v[idx];
                out[idx] = -0.5 * lap + self.potential[idx] * f[idx] + g * density * f[idx];
            }
        }
    }

    /// 4 阶 Runge-Kutta 单步
    /// ∂u/∂t = H[v], ∂v/∂t = -H[u]
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.config.n_cells();

        // k1
        let mut hu1 = vec![0.0; n];
        let mut hv1 = vec![0.0; n];
        self.hamiltonian_apply(&self.v_curr, &self.u_curr, &self.v_curr, &mut hv1);
        self.hamiltonian_apply(&self.u_curr, &self.u_curr, &self.v_curr, &mut hu1);
        // du/dt = H[v], dv/dt = -H[u]
        let k1_u: Vec<f32> = hv1.iter().copied().collect();
        let k1_v: Vec<f32> = hu1.iter().map(|&x| -x).collect();

        // 中间态 1: u + 0.5 dt k1_u, v + 0.5 dt k1_v
        let mut u1 = vec![0.0; n];
        let mut v1 = vec![0.0; n];
        for i in 0..n {
            u1[i] = self.u_curr[i] + 0.5 * dt * k1_u[i];
            v1[i] = self.v_curr[i] + 0.5 * dt * k1_v[i];
        }

        // k2
        let mut hu2 = vec![0.0; n];
        let mut hv2 = vec![0.0; n];
        self.hamiltonian_apply(&v1, &u1, &v1, &mut hv2);
        self.hamiltonian_apply(&u1, &u1, &v1, &mut hu2);
        let k2_u: Vec<f32> = hv2.iter().copied().collect();
        let k2_v: Vec<f32> = hu2.iter().map(|&x| -x).collect();

        // 中间态 2
        let mut u2 = vec![0.0; n];
        let mut v2 = vec![0.0; n];
        for i in 0..n {
            u2[i] = self.u_curr[i] + 0.5 * dt * k2_u[i];
            v2[i] = self.v_curr[i] + 0.5 * dt * k2_v[i];
        }

        // k3
        let mut hu3 = vec![0.0; n];
        let mut hv3 = vec![0.0; n];
        self.hamiltonian_apply(&v2, &u2, &v2, &mut hv3);
        self.hamiltonian_apply(&u2, &u2, &v2, &mut hu3);
        let k3_u: Vec<f32> = hv3.iter().copied().collect();
        let k3_v: Vec<f32> = hu3.iter().map(|&x| -x).collect();

        // 中间态 3
        let mut u3 = vec![0.0; n];
        let mut v3 = vec![0.0; n];
        for i in 0..n {
            u3[i] = self.u_curr[i] + dt * k3_u[i];
            v3[i] = self.v_curr[i] + dt * k3_v[i];
        }

        // k4
        let mut hu4 = vec![0.0; n];
        let mut hv4 = vec![0.0; n];
        self.hamiltonian_apply(&v3, &u3, &v3, &mut hv4);
        self.hamiltonian_apply(&u3, &u3, &v3, &mut hu4);
        let k4_u: Vec<f32> = hv4.iter().copied().collect();
        let k4_v: Vec<f32> = hu4.iter().map(|&x| -x).collect();

        // 合成
        for i in 0..n {
            self.u_next[i] = self.u_curr[i] + (dt / 6.0) * (k1_u[i] + 2.0 * k2_u[i] + 2.0 * k3_u[i] + k4_u[i]);
            self.v_next[i] = self.v_curr[i] + (dt / 6.0) * (k1_v[i] + 2.0 * k2_v[i] + 2.0 * k3_v[i] + k4_v[i]);
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

    /// 虚时演化一步 (归一化后), 用于求基态
    /// t -> -iτ: ψ -> ψ exp(-H τ)
    /// 简化 Euler: ψ_{n+1} = ψ_n - τ H ψ_n, 然后归一化
    pub fn imaginary_time_step(&mut self, tau: f32) {
        let n = self.config.n_cells();
        let mut hu = vec![0.0; n];
        let mut hv = vec![0.0; n];
        self.hamiltonian_apply(&self.u_curr, &self.u_curr, &self.v_curr, &mut hu);
        self.hamiltonian_apply(&self.v_curr, &self.u_curr, &self.v_curr, &mut hv);
        for i in 0..n {
            self.u_next[i] = self.u_curr[i] - tau * hu[i];
            self.v_next[i] = self.v_curr[i] - tau * hv[i];
        }
        std::mem::swap(&mut self.u_curr, &mut self.u_next);
        std::mem::swap(&mut self.v_curr, &mut self.v_next);
        // 归一化
        let nrm = self.norm();
        if nrm > 0.0 {
            let scale = 1.0 / nrm.sqrt();
            for v in &mut self.u_curr {
                *v *= scale;
            }
            for v in &mut self.v_curr {
                *v *= scale;
            }
        }
        self.time += tau; // 虚时
        self.steps += 1;
    }

    pub fn imaginary_time_relax(&mut self, tau: f32, n_steps: usize) {
        for _ in 0..n_steps {
            self.imaginary_time_step(tau);
        }
    }

    pub fn has_nan(&self) -> bool {
        self.u_curr.iter().any(|&v| !v.is_finite())
            || self.v_curr.iter().any(|&v| !v.is_finite())
    }

    /// 粒子数 N = ∫|ψ|² dA = Σ (u² + v²) dx²
    pub fn norm(&self) -> f32 {
        let dx2 = self.config.dx * self.config.dx;
        let mut s = 0.0f32;
        for i in 0..self.u_curr.len() {
            s += self.u_curr[i] * self.u_curr[i] + self.v_curr[i] * self.v_curr[i];
        }
        s * dx2
    }

    /// 能量 E = ∫[½|∇ψ|² + V|ψ|² + (g/2)|ψ|⁴] dA
    pub fn energy(&self) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx2 = self.config.dx * self.config.dx;
        let g = self.config.g;
        let mut e = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let ii = i as i32;
                let jj = j as i32;
                let i_e = self.neighbor(ii + 1, jj);
                let i_w = self.neighbor(ii - 1, jj);
                let i_n = self.neighbor(ii, jj + 1);
                let i_s = self.neighbor(ii, jj - 1);
                let u = self.u_curr[idx];
                let v = self.v_curr[idx];
                // |∇ψ|² ≈ (du/dx)² + (dv/dx)² + (du/dy)² + (dv/dy)²
                let dudx = (self.u_curr[i_e] - self.u_curr[i_w]) / (2.0 * self.config.dx);
                let dvdx = (self.v_curr[i_e] - self.v_curr[i_w]) / (2.0 * self.config.dx);
                let dudy = (self.u_curr[i_n] - self.u_curr[i_s]) / (2.0 * self.config.dx);
                let dvdy = (self.v_curr[i_n] - self.v_curr[i_s]) / (2.0 * self.config.dx);
                let grad_sq = dudx * dudx + dvdx * dvdx + dudy * dudy + dvdy * dvdy;
                let density = u * u + v * v;
                let v_pot = self.potential[idx];
                e += 0.5 * grad_sq + v_pot * density + 0.5 * g * density * density;
            }
        }
        e * dx2
    }

    /// 最大密度 max|ψ|²
    pub fn max_density(&self) -> f32 {
        let mut m = 0.0f32;
        for i in 0..self.u_curr.len() {
            let d = self.u_curr[i] * self.u_curr[i] + self.v_curr[i] * self.v_curr[i];
            if d > m {
                m = d;
            }
        }
        m
    }

    /// 平均密度
    pub fn mean_density(&self) -> f32 {
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        let mut s = 0.0f32;
        for i in 0..self.u_curr.len() {
            s += self.u_curr[i] * self.u_curr[i] + self.v_curr[i] * self.v_curr[i];
        }
        s / n as f32
    }

    /// 期望位置 <r> = ∫ ψ* r |ψ|² dA / N
    pub fn mean_position(&self) -> (f32, f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let cx = lx * 0.5;
        let cy = ly * 0.5;
        let n = self.norm();
        if n < 1e-12 {
            return (cx, cy);
        }
        let mut sx = 0.0f32;
        let mut sy = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - cx;
                let y = (j as f32) * self.config.dx - cy;
                let idx = j * nx + i;
                let d = self.u_curr[idx] * self.u_curr[idx] + self.v_curr[idx] * self.v_curr[idx];
                sx += x * d;
                sy += y * d;
            }
        }
        let dx2 = self.config.dx * self.config.dx;
        (sx * dx2 / n, sy * dx2 / n)
    }

    /// 波包宽度 <r²> - <r>² 的平方根
    pub fn width(&self) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let cx = lx * 0.5;
        let cy = ly * 0.5;
        let n = self.norm();
        if n < 1e-12 {
            return 0.0;
        }
        let (mx, my) = self.mean_position();
        let mut s = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - cx;
                let y = (j as f32) * self.config.dx - cy;
                let idx = j * nx + i;
                let d = self.u_curr[idx] * self.u_curr[idx] + self.v_curr[idx] * self.v_curr[idx];
                let dx = x - mx;
                let dy = y - my;
                s += (dx * dx + dy * dy) * d;
            }
        }
        let dx2 = self.config.dx * self.config.dx;
        (s * dx2 / n).sqrt()
    }

    pub fn wrap_idx(&self, i: i32, n: usize) -> usize {
        self.wrap(i, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GpConfig::default();
        assert_eq!(cfg.nx, 64);
        assert_eq!(cfg.ny, 64);
        assert!(cfg.dx > 0.0);
        assert!(cfg.dt > 0.0);
        assert_eq!(cfg.g, 1.0);
        assert!(cfg.omega_x > 0.0);
        assert!(cfg.omega_y > 0.0);
    }

    #[test]
    fn test_n_cells() {
        let cfg = GpConfig::default();
        assert_eq!(cfg.n_cells(), 64 * 64);
    }

    #[test]
    fn test_domain_size() {
        let cfg = GpConfig::default();
        let (lx, ly) = cfg.domain_size();
        assert!((lx - 64.0 * 0.2).abs() < 1e-5);
        assert!((ly - 64.0 * 0.2).abs() < 1e-5);
    }

    #[test]
    fn test_potential_at_center() {
        let cfg = GpConfig::default();
        // 中心 (i = nx/2, j = ny/2) 处势能应为 0 (或接近)
        let v_center = cfg.potential_at(32, 32);
        assert!(v_center.abs() < 0.1, "center V should be near 0: {}", v_center);
    }

    #[test]
    fn test_potential_at_corner() {
        let cfg = GpConfig::default();
        let v_corner = cfg.potential_at(0, 0);
        assert!(v_corner > 0.0, "corner V should be positive: {}", v_corner);
    }

    #[test]
    fn test_solver_creation() {
        let s = GpSolver::new(GpConfig::default());
        assert_eq!(s.u_curr.len(), 64 * 64);
        assert_eq!(s.v_curr.len(), 64 * 64);
        assert_eq!(s.potential.len(), 64 * 64);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_zero();
        assert!(s.norm() < 1e-10);
    }

    #[test]
    fn test_initialize_gaussian_normalized() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        let n = s.norm();
        assert!(
            (n - 1.0).abs() < 1e-3,
            "gaussian should be normalized to N=1: got {}",
            n
        );
    }

    #[test]
    fn test_initialize_vortex_normalized() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_vortex(1.0, 1);
        let n = s.norm();
        assert!(
            (n - 1.0).abs() < 1e-3,
            "vortex should be normalized: got {}",
            n
        );
    }

    #[test]
    fn test_vortex_density_zero_at_center() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_vortex(1.0, 1);
        let nx = s.config.nx;
        let ny = s.config.ny;
        let center = (ny / 2) * nx + (nx / 2);
        let d_center = s.u_curr[center] * s.u_curr[center] + s.v_curr[center] * s.v_curr[center];
        // 涡旋中心密度应为零 (或非常小)
        let max_d = s.max_density();
        assert!(
            d_center < 0.1 * max_d,
            "vortex center density should be small: {} vs max {}",
            d_center,
            max_d
        );
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_norm_conservation() {
        // RK4 + Hermitian H: 粒子数应近似守恒
        let cfg = GpConfig {
            dt: 0.0005,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        let n0 = s.norm();
        s.step_n(200);
        let n1 = s.norm();
        assert!(
            (n1 - n0).abs() < 0.01 * n0.abs(),
            "norm not conserved: {} -> {}",
            n0,
            n1
        );
    }

    #[test]
    fn test_energy_conservation() {
        // RK4 应近似保能
        let cfg = GpConfig {
            dt: 0.0005,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        assert!(
            (e1 - e0).abs() < 0.05 * e0.abs().max(0.01),
            "energy not conserved: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = GpConfig {
            nx: 32,
            ny: 32,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        s.step_n(3000);
        assert!(!s.has_nan(), "NaN after 3000 steps");
    }

    #[test]
    fn test_no_nan_with_interaction() {
        // 有相互作用 g≠0
        let cfg = GpConfig {
            g: 10.0,
            dt: 0.0005,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(0.5);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN with strong repulsion");
    }

    #[test]
    fn test_no_nan_attractive() {
        // 吸引相互作用 (g < 0)
        let cfg = GpConfig {
            g: -1.0,
            dt: 0.0005,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN with attractive interaction");
    }

    #[test]
    fn test_imaginary_time_decreases_energy() {
        // 虚时演化单调降能
        let cfg = GpConfig {
            g: 1.0,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(0.5);
        let e0 = s.energy();
        s.imaginary_time_relax(0.001, 200);
        let e1 = s.energy();
        assert!(
            e1 < e0,
            "imaginary time should decrease energy: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_imaginary_time_normalization() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(0.5);
        s.imaginary_time_relax(0.001, 100);
        let n = s.norm();
        assert!(
            (n - 1.0).abs() < 1e-3,
            "imaginary time should preserve N=1: got {}",
            n
        );
    }

    #[test]
    fn test_imaginary_time_ground_state_smooth() {
        // 基态应是光滑的 (无节点)
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(0.5);
        // 加噪声扰动
        let n = s.u_curr.len();
        let mut rng = GpRng::new(42);
        for i in 0..n {
            s.u_curr[i] += 0.05 * (2.0 * rng.next() - 1.0);
        }
        s.imaginary_time_relax(0.001, 500);
        // 基态在中心最大, 单峰
        let nx = s.config.nx;
        let ny = s.config.ny;
        let center = (ny / 2) * nx + (nx / 2);
        let d_center = s.u_curr[center] * s.u_curr[center] + s.v_curr[center] * s.v_curr[center];
        let d_max = s.max_density();
        assert!(
            (d_center - d_max).abs() < 0.05 * d_max,
            "ground state should peak at center: {} vs max {}",
            d_center,
            d_max
        );
    }

    #[test]
    fn test_linear_evolution_periodic() {
        // 无势阱 (V=0) 无相互作用 (g=0): 高斯波包自由扩散
        let cfg = GpConfig {
            omega_x: 0.0,
            omega_y: 0.0,
            g: 0.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        let w0 = s.width();
        s.step_n(500);
        let w1 = s.width();
        // 自由扩散: 宽度应增大
        assert!(w1 > w0, "free expansion should broaden: {} -> {}", w0, w1);
    }

    #[test]
    fn test_periodic_wrap() {
        let s = GpSolver::new(GpConfig::default());
        assert_eq!(s.wrap_idx(-1, 10), 9);
        assert_eq!(s.wrap_idx(0, 10), 0);
        assert_eq!(s.wrap_idx(10, 10), 0);
        assert_eq!(s.wrap_idx(11, 10), 1);
    }

    #[test]
    fn test_norm_zero_initial() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_zero();
        assert!(s.norm() < 1e-10);
    }

    #[test]
    fn test_energy_zero_initial() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_zero();
        assert!(s.energy().abs() < 1e-10);
    }

    #[test]
    fn test_mean_position_at_center() {
        // 高斯波包中心应在域中心 (mean_position 返回相对中心的坐标, 应为 0)
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        let (mx, my) = s.mean_position();
        assert!(mx.abs() < 0.2, "mean x (rel center): {}", mx);
        assert!(my.abs() < 0.2, "mean y (rel center): {}", my);
    }

    #[test]
    fn test_width_positive() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        assert!(s.width() > 0.0);
    }

    #[test]
    fn test_max_density_positive() {
        let mut s = GpSolver::new(GpConfig::default());
        s.initialize_gaussian(1.0);
        assert!(s.max_density() > 0.0);
    }

    #[test]
    fn test_harmonic_trap_bounded() {
        // 谐振势中波包应保持有界 (不发散)
        let cfg = GpConfig {
            g: 1.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = GpSolver::new(cfg);
        s.initialize_gaussian(1.0);
        let d0 = s.max_density();
        s.step_n(2000);
        let d1 = s.max_density();
        assert!(
            d1 < d0 * 5.0 + 1.0,
            "density blew up: {} -> {}",
            d0,
            d1
        );
    }

    #[test]
    fn test_dim_flexible() {
        for n in [16, 32, 64] {
            let cfg = GpConfig {
                nx: n,
                ny: n,
                ..Default::default()
            };
            let mut s = GpSolver::new(cfg);
            s.initialize_gaussian(0.5);
            s.step_n(50);
            assert!(!s.has_nan(), "NaN for n={}", n);
        }
    }

    #[test]
    fn test_vortex_charge_sign() {
        // 不同电荷的涡旋应有不同相位
        let mut s1 = GpSolver::new(GpConfig::default());
        s1.initialize_vortex(1.0, 1);
        let mut s2 = GpSolver::new(GpConfig::default());
        s2.initialize_vortex(1.0, -1);
        // 密度分布相同, 但虚部符号不同 (在 y>0 处)
        let nx = s1.config.nx;
        let ny = s1.config.ny;
        let test_idx = (ny / 2 + 2) * nx + (nx / 2); // y > 0 处
        // 相位符号相反 → 虚部符号相反
        let v1 = s1.v_curr[test_idx];
        let v2 = s2.v_curr[test_idx];
        assert!(
            v1 * v2 < 0.0 || v1.abs() < 1e-6 || v2.abs() < 1e-6,
            "vortices of opposite charge should have opposite phase: {} vs {}",
            v1,
            v2
        );
    }

    #[test]
    fn test_boundary_zero_works() {
        let mut s = GpSolver::with_boundary(GpConfig::default(), GpBoundary::Zero);
        s.initialize_gaussian(1.0);
        s.step_n(50);
        assert!(!s.has_nan(), "NaN with zero boundary");
    }

    #[test]
    fn test_hamiltonian_apply_zero_input() {
        let s = GpSolver::new(GpConfig::default());
        let n = s.config.n_cells();
        let f = vec![0.0; n];
        let u = vec![0.0; n];
        let v = vec![0.0; n];
        let mut out = vec![0.0; n];
        s.hamiltonian_apply(&f, &u, &v, &mut out);
        for x in &out {
            assert!(x.abs() < 1e-6);
        }
    }

    #[test]
    fn test_potential_precomputed() {
        let s = GpSolver::new(GpConfig::default());
        // 中心点势能应近 0
        let nx = s.config.nx;
        let ny = s.config.ny;
        let center = (ny / 2) * nx + (nx / 2);
        assert!(s.potential[center].abs() < 0.1);
        // 角点势能应正
        assert!(s.potential[0] > 0.0);
    }
}

struct GpRng {
    state: u64,
}

impl GpRng {
    fn new(seed: u64) -> Self {
        GpRng {
            state: if seed == 0 {
                0xff00ff00ff00ff00
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 11) as f32 / (1u64 << 53) as f32
    }
}
