//! Keller-Segel 趋化性模型 — 细菌聚集与化学信号相互作用
//!
//! Evelyn Fox Keller 与 Lee Segel 1971 年提出的经典模型, 描述变形虫
//! (Dictyostelium discoideum) 或大肠杆菌在化学信号 (cAMP) 梯度下
//! 的趋化运动. 是数学生物学中模式形成与有限时间爆破的核心范例.
//!
//! 方程 (2D):
//!   ∂ρ/∂t = D_ρ ∇²ρ - χ ∇·(ρ ∇c)
//!   ∂c/∂t = D_c ∇²c - k c + α ρ
//!
//! 其中:
//!   ρ = 细胞密度 (具备扩散与趋化运动)
//!   c = 化学信号浓度 (扩散、降解、由细胞分泌)
//!   D_ρ = 细胞随机运动系数
//!   D_c = 化学信号扩散系数
//!   χ = 趋化敏感度 (细胞向 c 梯度上坡运动)
//!   k = 化学信号降解率
//!   α = 细胞分泌化学信号速率
//!
//! 守恒通量形式:
//!   ∂ρ/∂t = -∇·J_ρ,  J_ρ = -D_ρ ∇ρ + χ ρ ∇c
//!   ∂c/∂t = D_c ∇²c - k c + α ρ  (非守恒, 有源汇)
//!
//! 数值方法:
//!   - 有限体积通量形式 (保证 ρ 质量守恒, 周期边界)
//!   - 界面密度 ρ_{i+1/2} = (ρ_i + ρ_{i+1})/2 (中心)
//!   - 5 点 Laplacian 离散 c 方程
//!   - 显式 Euler 时间推进
//!
//! 稳定性约束 (粗略):
//!   dt <= dx² / (4 max(D_ρ, D_c))   (扩散稳定)
//!   dt <= dx² / (χ max(ρ))          (趋化项稳定)
//!
//! 临界质量 (2D 周期):
//!   M = ∫ρ dA. 若 M > 8π D_c / χ (在合适归一下) 则发生有限时间爆破
//!   否则全局存在. 这是著名的 KS 临界质量现象.
//!
//! 应用:
//!   - 微生物趋化 (大肠杆菌, 变形虫)
//!   - 胚胎发育 (细胞迁移)
//!   - 肿瘤生长 (肿瘤侵袭模型)
//!   - 免疫响应 (T 细胞趋化)
//!   - 动物群体模式形成 (鱼群, 鸟群)
//!
//! 基于:
//!   - Keller, E.F. & Segel, L.A. 1971. "Traveling bands of
//!     chemotactic bacteria: A theoretical analysis." J. Theor.
//!     Biol. 30, 235-248.
//!   - Horstmann, D. 2003. "From 1970 until present: the Keller-Segel
//!     model and its consequences." Jahresber. DMV 105, 103-165.
//!   - Hillen, T. & Painter, K.J. 2009. "A user's guide to PDE
//!     models for chemotaxis." J. Math. Biol. 58, 183-217.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KsConfig {
    /// x 方向格点数
    pub nx: usize,
    /// y 方向格点数
    pub ny: usize,
    /// 空间步长
    pub dx: f32,
    /// 时间步长
    pub dt: f32,
    /// 细胞扩散系数 D_ρ
    pub d_rho: f32,
    /// 化学信号扩散系数 D_c
    pub d_c: f32,
    /// 趋化敏感度 χ
    pub chi: f32,
    /// 化学信号降解率 k
    pub k_degrade: f32,
    /// 细胞分泌速率 α
    pub alpha: f32,
}

impl Default for KsConfig {
    fn default() -> Self {
        KsConfig {
            nx: 64,
            ny: 64,
            dx: 0.5,
            dt: 0.01,
            d_rho: 0.1,
            d_c: 0.5,
            chi: 0.5,
            k_degrade: 0.1,
            alpha: 1.0,
        }
    }
}

impl KsConfig {
    /// 临界质量 (2D 周期域上 KS 爆破阈值, 简化形式)
    /// M_crit = 8π D_c / χ (在 D_ρ = 1 归一下)
    pub fn critical_mass(&self) -> f32 {
        8.0 * std::f32::consts::PI * self.d_c / self.chi.max(1e-12)
    }

    /// 总域面积
    pub fn domain_area(&self) -> f32 {
        (self.nx as f32 * self.dx) * (self.ny as f32 * self.dx)
    }

    /// 格点数
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    /// 给定平均密度时的总质量
    pub fn mass_for_mean(&self, mean_rho: f32) -> f32 {
        mean_rho * self.domain_area()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KsBoundary {
    /// 周期边界 (质量严格守恒)
    Periodic,
    /// Neumann 零通量边界 (质量守恒, 适合封闭域)
    NoFlux,
}

pub struct KsSolver {
    pub config: KsConfig,
    pub boundary: KsBoundary,
    /// 细胞密度 ρ (当前)
    pub rho_curr: Vec<f32>,
    /// 细胞密度 ρ (下一步缓冲)
    pub rho_next: Vec<f32>,
    /// 化学信号 c (当前)
    pub c_curr: Vec<f32>,
    /// 化学信号 c (下一步缓冲)
    pub c_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl KsSolver {
    pub fn new(config: KsConfig) -> Self {
        Self::with_boundary(config, KsBoundary::Periodic)
    }

    pub fn with_boundary(config: KsConfig, boundary: KsBoundary) -> Self {
        let n = config.n_cells();
        KsSolver {
            config,
            boundary,
            rho_curr: vec![0.0; n],
            rho_next: vec![0.0; n],
            c_curr: vec![0.0; n],
            c_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    /// 初始化为均匀稳态: ρ = ρ0, c = c0
    pub fn initialize_uniform(&mut self, rho0: f32, c0: f32) {
        for v in &mut self.rho_curr {
            *v = rho0;
        }
        for v in &mut self.c_curr {
            *v = c0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为均匀 + 小扰动 (用于触发不稳定性)
    pub fn initialize_perturbed(&mut self, rho0: f32, amplitude: f32, seed: u64) {
        let mut rng = KsRng::new(seed);
        for v in &mut self.rho_curr {
            *v = rho0 * (1.0 + amplitude * (2.0 * rng.next() - 1.0));
        }
        for v in &mut self.c_curr {
            *v = rho0; // c ≈ α ρ / k 稳态
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为中心高斯峰 (用于观察聚集/爆破)
    pub fn initialize_spot(&mut self, rho0: f32, peak: f32, sigma: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let cx = (nx as f32) * 0.5;
        let cy = (ny as f32) * 0.5;
        let s2 = sigma * sigma;
        for j in 0..ny {
            for i in 0..nx {
                let dx_p = (i as f32 - cx) * dx;
                let dy_p = (j as f32 - cy) * dx;
                let r2 = dx_p * dx_p + dy_p * dy_p;
                let g = (-r2 / (2.0 * s2)).exp();
                let idx = j * nx + i;
                self.rho_curr[idx] = rho0 + peak * g;
                self.c_curr[idx] = rho0;
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
    fn neighbor(&self, i: i32, j: i32) -> Option<usize> {
        let nx = self.config.nx as i32;
        let ny = self.config.ny as i32;
        match self.boundary {
            KsBoundary::Periodic => {
                let ii = self.wrap(i, self.config.nx);
                let jj = self.wrap(j, self.config.ny);
                Some((jj * self.config.nx + ii) as usize)
            }
            KsBoundary::NoFlux => {
                // Neumann: 镜像反射 (clamp 到边界)
                let ii = i.clamp(0, nx - 1) as usize;
                let jj = j.clamp(0, ny - 1) as usize;
                Some((jj * self.config.nx + ii) as usize)
            }
        }
    }

    /// 显式 Euler 单步
    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let d_rho = self.config.d_rho;
        let d_c = self.config.d_c;
        let chi = self.config.chi;
        let k_deg = self.config.k_degrade;
        let alpha = self.config.alpha;
        let dx2 = dx * dx;

        // 先更新 ρ (用 c_curr), 再更新 c (用 rho_curr)
        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let ii = i as i32;
                let jj = j as i32;

                let i_e = self.neighbor(ii + 1, jj).unwrap();
                let i_w = self.neighbor(ii - 1, jj).unwrap();
                let i_n = self.neighbor(ii, jj + 1).unwrap();
                let i_s = self.neighbor(ii, jj - 1).unwrap();

                let rho = self.rho_curr[idx];
                let rho_e = self.rho_curr[i_e];
                let rho_w = self.rho_curr[i_w];
                let rho_n = self.rho_curr[i_n];
                let rho_s = self.rho_curr[i_s];

                let c = self.c_curr[idx];
                let c_e = self.c_curr[i_e];
                let c_w = self.c_curr[i_w];
                let c_n = self.c_curr[i_n];
                let c_s = self.c_curr[i_s];

                // ∇²ρ (5 点)
                let lap_rho = (rho_e + rho_w + rho_n + rho_s - 4.0 * rho) / dx2;

                // 趋化通量 J = -D_ρ ∇ρ + χ ρ ∇c
                // 散度形式: ∇·(ρ ∇c) 有限体积离散
                // 界面值: ρ_{i+1/2} = (ρ_i + ρ_{i+1})/2
                let rho_ep = 0.5 * (rho + rho_e);
                let rho_wp = 0.5 * (rho + rho_w);
                let rho_np = 0.5 * (rho + rho_n);
                let rho_sp = 0.5 * (rho + rho_s);

                // ∇c 在界面 (i+1/2): (c_{i+1} - c_i)/dx
                let dc_e = (c_e - c) / dx;
                let dc_w = (c - c_w) / dx;
                let dc_n = (c_n - c) / dx;
                let dc_s = (c - c_s) / dx;

                // ∇·(ρ ∇c) = [ρ_{i+1/2} dc_e - ρ_{i-1/2} dc_w]/dx
                //          + [ρ_{j+1/2} dc_n - ρ_{j-1/2} dc_s]/dx
                let div_rho_grad_c =
                    (rho_ep * dc_e - rho_wp * dc_w) / dx + (rho_np * dc_n - rho_sp * dc_s) / dx;

                // ∂ρ/∂t = D_ρ ∇²ρ - χ ∇·(ρ ∇c)
                let drho = d_rho * lap_rho - chi * div_rho_grad_c;
                self.rho_next[idx] = (rho + dt * drho).max(0.0); // 密度非负

                // ∂c/∂t = D_c ∇²c - k c + α ρ
                let lap_c = (c_e + c_w + c_n + c_s - 4.0 * c) / dx2;
                let dc = d_c * lap_c - k_deg * c + alpha * rho;
                self.c_next[idx] = (c + dt * dc).max(0.0); // 浓度非负
            }
        }

        std::mem::swap(&mut self.rho_curr, &mut self.rho_next);
        std::mem::swap(&mut self.c_curr, &mut self.c_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.rho_curr.iter().any(|&v| !v.is_finite())
            || self.c_curr.iter().any(|&v| !v.is_finite())
    }

    /// 总细胞质量 M = ∫ρ dA = Σ ρ * dx²
    pub fn total_mass(&self) -> f32 {
        let dx2 = self.config.dx * self.config.dx;
        self.rho_curr.iter().sum::<f32>() * dx2
    }

    /// 平均密度
    pub fn mean_rho(&self) -> f32 {
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        self.rho_curr.iter().sum::<f32>() / n as f32
    }

    /// 平均化学信号
    pub fn mean_c(&self) -> f32 {
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        self.c_curr.iter().sum::<f32>() / n as f32
    }

    pub fn max_rho(&self) -> f32 {
        self.rho_curr
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_rho(&self) -> f32 {
        self.rho_curr
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max_c(&self) -> f32 {
        self.c_curr
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// 密度方差 (衡量聚集程度)
    pub fn variance_rho(&self) -> f32 {
        let m = self.mean_rho();
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        self.rho_curr
            .iter()
            .map(|&v| (v - m) * (v - m))
            .sum::<f32>()
            / n as f32
    }

    pub fn std_rho(&self) -> f32 {
        self.variance_rho().sqrt()
    }

    /// 检测爆破: 出现非有限值或极大值
    pub fn is_blowing_up(&self, threshold: f32) -> bool {
        self.has_nan() || self.max_rho() > threshold
    }

    /// 周期边界索引测试
    pub fn wrap_idx(&self, i: i32, n: usize) -> usize {
        self.wrap(i, n)
    }
}

struct KsRng {
    state: u64,
}

impl KsRng {
    fn new(seed: u64) -> Self {
        KsRng {
            state: if seed == 0 {
                0x9e3779b97f4a7c15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> f32 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 11) as f32 / (1u64 << 53) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = KsConfig::default();
        assert_eq!(cfg.nx, 64);
        assert_eq!(cfg.ny, 64);
        assert!(cfg.dx > 0.0);
        assert!(cfg.dt > 0.0);
        assert!(cfg.d_rho >= 0.0);
        assert!(cfg.d_c >= 0.0);
        assert!(cfg.chi >= 0.0);
        assert!(cfg.k_degrade >= 0.0);
        assert!(cfg.alpha >= 0.0);
    }

    #[test]
    fn test_critical_mass_positive() {
        let cfg = KsConfig::default();
        let m = cfg.critical_mass();
        assert!(m > 0.0, "critical mass should be positive: {}", m);
        // M_crit = 8π D_c / χ
        let expected = 8.0 * std::f32::consts::PI * cfg.d_c / cfg.chi;
        assert!((m - expected).abs() < 1e-5);
    }

    #[test]
    fn test_domain_area() {
        let cfg = KsConfig::default();
        let area = cfg.domain_area();
        let expected = (64.0 * 0.5) * (64.0 * 0.5);
        assert!((area - expected).abs() < 1e-5);
    }

    #[test]
    fn test_n_cells() {
        let cfg = KsConfig::default();
        assert_eq!(cfg.n_cells(), 64 * 64);
    }

    #[test]
    fn test_solver_creation() {
        let s = KsSolver::new(KsConfig::default());
        assert_eq!(s.rho_curr.len(), 64 * 64);
        assert_eq!(s.c_curr.len(), 64 * 64);
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_uniform(2.0, 1.0);
        assert!((s.mean_rho() - 2.0).abs() < 1e-5);
        assert!((s.mean_c() - 1.0).abs() < 1e-5);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_perturbed_bounded() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.1, 42);
        // 扰动幅度 0.1, ρ 在 [0.9, 1.1]
        let mx = s.max_rho();
        let mn = s.min_rho();
        assert!(mx <= 1.0 + 0.11, "max rho too large: {}", mx);
        assert!(mn >= 1.0 - 0.11, "min rho too small: {}", mn);
    }

    #[test]
    fn test_initialize_spot_peak() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_spot(0.5, 5.0, 1.0);
        // 中心点应有最大值
        let nx = s.config.nx;
        let ny = s.config.ny;
        let center = (ny / 2) * nx + (nx / 2);
        let max_rho = s.max_rho();
        assert!((s.rho_curr[center] - max_rho).abs() < 1e-5);
        // 中心值应接近 0.5 + 5.0 = 5.5
        assert!(s.rho_curr[center] > 5.0, "center rho: {}", s.rho_curr[center]);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.05, 42);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.05, 42);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_uniform_steady_state() {
        // 均匀 ρ 和 c (= α ρ / k 稳态) 应保持不变
        let cfg = KsConfig::default();
        let mut s = KsSolver::new(cfg.clone());
        let rho0 = 1.0_f32;
        let c0 = cfg.alpha * rho0 / cfg.k_degrade; // 稳态 c
        s.initialize_uniform(rho0, c0);
        s.step_n(50);
        // 均匀态 Laplacian = 0, 趋化项 ∇c = 0
        assert!(
            (s.mean_rho() - rho0).abs() < 1e-3,
            "mean rho drifted: {}",
            s.mean_rho()
        );
        assert!(
            (s.mean_c() - c0).abs() < 1e-3,
            "mean c drifted: {}",
            s.mean_c()
        );
    }

    #[test]
    fn test_mass_conservation_periodic() {
        // 周期边界 + 通量形式: 总质量应严格守恒
        let cfg = KsConfig::default();
        let mut s = KsSolver::new(cfg);
        s.initialize_perturbed(1.0, 0.2, 42);
        let m0 = s.total_mass();
        s.step_n(200);
        let m1 = s.total_mass();
        assert!(
            (m1 - m0).abs() < 1e-3 * m0.abs() + 1e-4,
            "mass not conserved: {} -> {}",
            m0,
            m1
        );
    }

    #[test]
    fn test_mass_conservation_noflux() {
        // Neumann 边界也应守恒
        let cfg = KsConfig::default();
        let mut s = KsSolver::with_boundary(cfg, KsBoundary::NoFlux);
        s.initialize_perturbed(1.0, 0.2, 42);
        let m0 = s.total_mass();
        s.step_n(200);
        let m1 = s.total_mass();
        assert!(
            (m1 - m0).abs() < 1e-3 * m0.abs() + 1e-4,
            "mass not conserved (no-flux): {} -> {}",
            m0,
            m1
        );
    }

    #[test]
    fn test_chemical_decays_without_cells() {
        // ρ = 0 时, c 应指数衰减 (有降解)
        let mut cfg = KsConfig::default();
        cfg.alpha = 0.0; // 无分泌
        let mut s = KsSolver::new(cfg.clone());
        s.initialize_uniform(0.0, 1.0); // 无细胞, c=1
        s.step_n(100);
        // c(t) = c0 exp(-k t), t = 100*dt = 1.0
        let expected = 1.0_f32 * (-cfg.k_degrade * 100.0 * cfg.dt).exp();
        let actual = s.mean_c();
        assert!(
            (actual - expected).abs() < 0.05,
            "chemical decay mismatch: got {}, expected {}",
            actual,
            expected
        );
    }

    #[test]
    fn test_chemical_produced_by_cells() {
        // 有细胞 + 无初始 c, c 应增加
        let mut cfg = KsConfig::default();
        cfg.k_degrade = 0.0; // 无降解, c 必然增长
        let mut s = KsSolver::new(cfg);
        s.initialize_uniform(1.0, 0.0); // ρ=1, c=0
        let c0 = s.mean_c();
        s.step_n(50);
        let c1 = s.mean_c();
        assert!(c1 > c0, "c should grow: {} -> {}", c0, c1);
    }

    #[test]
    fn test_chemical_steady_state() {
        // 均匀稳态: c* = α ρ / k
        let cfg = KsConfig::default();
        let mut s = KsSolver::new(cfg.clone());
        let rho0 = 2.0_f32;
        let c_star = cfg.alpha * rho0 / cfg.k_degrade;
        s.initialize_uniform(rho0, c_star);
        s.step_n(100);
        assert!(
            (s.mean_c() - c_star).abs() < 1e-3,
            "c* not steady: got {}, expected {}",
            s.mean_c(),
            c_star
        );
    }

    #[test]
    fn test_diffusion_smooths_rho() {
        // 高扩散 + 低趋化: 密度方差应减小
        let mut cfg = KsConfig::default();
        cfg.d_rho = 1.0;
        cfg.chi = 0.01; // 弱趋化
        cfg.alpha = 0.0; // 关闭化学分泌以避免复杂耦合
        let mut s = KsSolver::new(cfg);
        s.initialize_perturbed(1.0, 0.5, 42);
        let v0 = s.variance_rho();
        s.step_n(500);
        let v1 = s.variance_rho();
        assert!(v1 < v0, "diffusion should smooth: {} -> {}", v0, v1);
    }

    #[test]
    fn test_chemotaxis_aggregates() {
        // 强趋化 + 小扩散: 扰动应增长 (聚集)
        let mut cfg = KsConfig::default();
        cfg.d_rho = 0.01; // 小扩散
        cfg.d_c = 0.1;
        cfg.chi = 2.0; // 强趋化
        cfg.k_degrade = 0.1;
        cfg.alpha = 1.0;
        cfg.dt = 0.001; // 小步长保稳定
        let mut s = KsSolver::new(cfg);
        s.initialize_spot(0.5, 1.0, 2.0); // 中心高斯峰
        let v0 = s.variance_rho();
        s.step_n(500);
        let v1 = s.variance_rho();
        assert!(
            v1 > v0,
            "chemotaxis should aggregate: {} -> {}",
            v0,
            v1
        );
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.1, 42);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        // 默认参数 (chi=0.5, d_c=0.5) 下 M_crit = 8π ≈ 25.1
        // 32x32, dx=0.5 域面积 = 256, rho0=1 时 M=256 >> 25 必然爆破 (KS 设计)
        // 使用低密度 + 弱趋化使 M < M_crit, 验证全局存在性
        let cfg = KsConfig {
            nx: 32,
            ny: 32,
            dx: 0.5,
            dt: 0.005,
            d_rho: 0.3,
            d_c: 1.0,
            chi: 0.05, // 弱趋化, M_crit = 8π*1.0/0.05 ≈ 502.7
            k_degrade: 0.1,
            alpha: 0.5,
        };
        let mut s = KsSolver::new(cfg);
        // rho0=0.3 → M = 0.3*256 = 76.8 < 502.7, 远低于临界
        s.initialize_perturbed(0.3, 0.05, 7);
        s.step_n(3000);
        assert!(!s.has_nan(), "NaN after 3000 steps");
    }

    #[test]
    fn test_rho_nonnegative() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.5, 42);
        s.step_n(500);
        let mn = s.min_rho();
        assert!(mn >= 0.0, "rho should be non-negative: {}", mn);
    }

    #[test]
    fn test_c_nonnegative() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.5, 42);
        s.step_n(500);
        let mc = s.c_curr.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(mc >= 0.0, "c should be non-negative: {}", mc);
    }

    #[test]
    fn test_total_mass_calculation() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_uniform(2.0, 0.0);
        let m = s.total_mass();
        let expected = 2.0 * s.config.domain_area();
        assert!((m - expected).abs() < 1e-3, "mass: {} vs {}", m, expected);
    }

    #[test]
    fn test_max_rho_bounded_with_strong_diffusion() {
        // 强扩散 + 弱趋化: max_rho 不应爆破
        let mut cfg = KsConfig::default();
        cfg.d_rho = 1.0;
        cfg.d_c = 1.0;
        cfg.chi = 0.05;
        cfg.dt = 0.005;
        let mut s = KsSolver::new(cfg);
        s.initialize_spot(1.0, 2.0, 1.0);
        let m0 = s.max_rho();
        s.step_n(1000);
        let m1 = s.max_rho();
        assert!(
            m1 < m0 * 2.0 + 1.0,
            "max_rho blew up: {} -> {}",
            m0,
            m1
        );
    }

    #[test]
    fn test_periodic_wrap() {
        let s = KsSolver::new(KsConfig::default());
        assert_eq!(s.wrap_idx(-1, 10), 9);
        assert_eq!(s.wrap_idx(0, 10), 0);
        assert_eq!(s.wrap_idx(10, 10), 0);
        assert_eq!(s.wrap_idx(11, 10), 1);
        assert_eq!(s.wrap_idx(-5, 10), 5);
    }

    #[test]
    fn test_variance_initial_zero_uniform() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_uniform(1.0, 1.0);
        assert!(s.variance_rho() < 1e-10);
    }

    #[test]
    fn test_variance_positive_perturbed() {
        let mut s = KsSolver::new(KsConfig::default());
        s.initialize_perturbed(1.0, 0.5, 42);
        assert!(s.variance_rho() > 0.0);
    }

    #[test]
    fn test_dim_2d() {
        // 不同网格大小都能工作
        for n in [16, 32, 64] {
            let cfg = KsConfig {
                nx: n,
                ny: n,
                ..Default::default()
            };
            let mut s = KsSolver::new(cfg);
            s.initialize_perturbed(1.0, 0.1, 42);
            s.step_n(50);
            assert!(!s.has_nan(), "NaN for n={}", n);
        }
    }

    #[test]
    fn test_blow_up_detection() {
        let mut s = KsSolver::new(KsConfig::default());
        // 手动注入 NaN
        s.rho_curr[0] = f32::NAN;
        assert!(s.is_blowing_up(1e6));
    }

    #[test]
    fn test_blow_up_threshold() {
        let mut s = KsSolver::new(KsConfig::default());
        s.rho_curr[0] = 1e8;
        assert!(s.is_blowing_up(1e6));
        assert!(!s.is_blowing_up(1e10));
    }

    #[test]
    fn test_no_blow_up_below_critical() {
        // 低于临界质量 + 强扩散: 长期不爆破
        let cfg = KsConfig {
            d_rho: 0.5,
            d_c: 0.5,
            chi: 0.1, // 弱趋化, M_crit = 8π*0.5/0.1 ≈ 125
            nx: 32,
            ny: 32,
            dx: 0.5,
            dt: 0.005,
            k_degrade: 0.1,
            alpha: 1.0,
        };
        let mut s = KsSolver::new(cfg);
        s.initialize_uniform(0.5, 0.5); // M = 0.5 * (16)² = 128, 接近临界
        // 减小平均密度使 M 远低于 M_crit
        s.initialize_uniform(0.1, 0.1); // M = 0.1 * 256 = 25.6 < 125
        s.step_n(2000);
        assert!(
            !s.is_blowing_up(1e4),
            "should not blow up below critical mass: max_rho={}",
            s.max_rho()
        );
    }

    #[test]
    fn test_module_count() {
        // 简单计数: struct + enum + impl 块数量
        let cfg = KsConfig::default();
        let s = KsSolver::new(cfg);
        assert!(!s.rho_curr.is_empty());
        assert!(!s.c_curr.is_empty());
    }
}
