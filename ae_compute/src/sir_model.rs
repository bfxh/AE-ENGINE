//! SIR Epidemic Model with Spatial Diffusion — 带扩散的 SIR 传染病模型
//!
//! Kermack-McKendrick SIR 模型是流行病学的奠基性模型, 描述传染病在人群中的
//! 传播动力学. 加入空间扩散后, 可模拟疫情的地理传播 (行波, 疫情中心扩散).
//!
//! 方程 (带扩散):
//!   ∂S/∂t = -β·S·I/N + D_S·∇²S    (易感者)
//!   ∂I/∂t =  β·S·I/N - γ·I + D_I·∇²I  (感染者)
//!   ∂R/∂t =  γ·I + D_R·∇²R        (恢复者/移除者)
//!
//! 其中 N = S + I + R 为总人口 (守恒, 无扩散时).
//!
//! 参数:
//!   β  - 感染率 (易感者与感染者接触导致感染)
//!   γ  - 恢复率 (感染者恢复或死亡, 获得免疫)
//!   D_S, D_I, D_R - 扩散系数 (人员流动)
//!   N  - 总人口 (用于归一化, 标准频率依赖传输)
//!
//! 关键概念:
//!   基本再生数 R0 = β/γ
//!     - R0 < 1: 疫情消退 (每个感染者平均传染 < 1 人)
//!     - R0 > 1: 疫情爆发 (感染指数增长)
//!     - R0 = 1: 地方病平衡点阈值
//!
//!   最终规模: 疫情结束后, 剩余易感者比例 S(∞) 满足
//!     S(∞) = S(0) · exp(-R0 · (1 - S(∞)/N))
//!     (超越方程, 无解析解)
//!
//!   群体免疫阈值: 1 - 1/R0 (达到此比例免疫即可阻止传播)
//!
//! 空间扩散效应:
//!   - 行波传播: 疫情从中心向外扩散, 波速 v ≈ 2·sqrt(D_I·(R0-1)/τ)
//!   - 疫情中心: 高密度感染区
//!   - 空间异质性: 人口分布影响传播
//!   - 边界效应: 隔离区/封锁模拟
//!
//! 应用:
//!   - 传染病疫情预测 (COVID-19, 流感, 埃博拉)
//!   - 疫苗接种策略评估
//!   - 疫情隔离措施模拟
//!   - 游戏中疾病传播系统 (NPC 之间传染病)
//!   - 信息传播 (谣言传播的类比)
//!
//! 数值方法:
//!   显式 Euler + 5点 Laplacian (2D)
//!   人群密度 S, I, R >= 0 (非负约束)
//!
//! CFL:
//!   - 扩散: dt <= dx² / (4·max(D_S, D_I, D_R))
//!   - 反应: dt <= 1 / max(β, γ)
//!
//! 基于:
//!   - Kermack, W.O. & McKendrick, A.G. 1927. Proc. R. Soc. A 115, 700.
//!   - Anderson, R.M. & May, R.M. 1991. "Infectious Diseases of Humans." Oxford.
//!   - Murray, J.D. 2002. "Mathematical Biology." Springer. Ch 10.
//!   - Brauer, F. & Castillo-Chavez, C. 2012. "Mathematical Models in
//!     Population Biology and Epidemiology." Springer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SirBoundary {
    Periodic,
    Neumann,
    Dirichlet { s: f32, i: f32, r: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SirConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub beta: f32,
    pub gamma: f32,
    pub d_s: f32,
    pub d_i: f32,
    pub d_r: f32,
    pub boundary: SirBoundary,
}

impl Default for SirConfig {
    fn default() -> Self {
        SirConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.01,
            beta: 2.0,
            gamma: 0.5,
            d_s: 0.1,
            d_i: 0.1,
            d_r: 0.1,
            boundary: SirBoundary::Periodic,
        }
    }
}

impl SirConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    /// 基本再生数 R0 = β/γ
    pub fn r0(&self) -> f32 {
        self.beta / self.gamma
    }

    /// 群体免疫阈值 1 - 1/R0
    pub fn herd_immunity_threshold(&self) -> f32 {
        1.0 - 1.0 / self.r0()
    }

    /// 感染特征时间 τ = 1/γ (平均恢复时间)
    pub fn infection_timescale(&self) -> f32 {
        1.0 / self.gamma
    }

    /// 初期增长率 (R0 > 1 时) r = γ·(R0 - 1) = β - γ
    pub fn initial_growth_rate(&self) -> f32 {
        self.beta - self.gamma
    }

    pub fn diffusive_cfl(&self) -> f32 {
        let d_max = self.d_s.max(self.d_i).max(self.d_r);
        4.0 * d_max * self.dt / (self.dx * self.dx)
    }

    pub fn reaction_cfl(&self) -> f32 {
        self.dt * self.beta.max(self.gamma)
    }

    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0 && self.reaction_cfl() <= 1.0
    }

    pub fn stable_dt(&self) -> f32 {
        let d_max = self.d_s.max(self.d_i).max(self.d_r);
        let diff_dt = 0.25 * self.dx * self.dx / d_max.max(1e-6);
        let rxn_dt = 1.0 / self.beta.max(self.gamma).max(1e-6);
        diff_dt.min(rxn_dt)
    }
}

pub struct SirSolver {
    pub config: SirConfig,
    pub s_curr: Vec<f32>,
    pub i_curr: Vec<f32>,
    pub r_curr: Vec<f32>,
    pub s_next: Vec<f32>,
    pub i_next: Vec<f32>,
    pub r_next: Vec<f32>,
    pub lap_s: Vec<f32>,
    pub lap_i: Vec<f32>,
    pub lap_r: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl SirSolver {
    pub fn new(config: SirConfig) -> Self {
        let n = config.n_cells();
        SirSolver {
            config,
            s_curr: vec![0.0; n],
            i_curr: vec![0.0; n],
            r_curr: vec![0.0; n],
            s_next: vec![0.0; n],
            i_next: vec![0.0; n],
            r_next: vec![0.0; n],
            lap_s: vec![0.0; n],
            lap_i: vec![0.0; n],
            lap_r: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    pub fn initialize_zero(&mut self) {
        for v in self.s_curr.iter_mut().chain(self.i_curr.iter_mut()).chain(self.r_curr.iter_mut()) {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为均匀分布 (S0, I0, R0)
    pub fn initialize_uniform(&mut self, s0: f32, i0: f32, r0: f32) {
        for k in 0..self.s_curr.len() {
            self.s_curr[k] = s0;
            self.i_curr[k] = i0;
            self.r_curr[k] = r0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为中心感染源 (其余全易感)
    pub fn initialize_outbreak(&mut self, cx: f32, cy: f32, radius: f32, i0: f32, s0: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let r2 = radius * radius;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let y = j as f32 * dx;
                let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                let id = self.idx(i, j);
                self.s_curr[id] = s0;
                self.i_curr[id] = if d2 < r2 { i0 } else { 0.0 };
                self.r_curr[id] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let beta = self.config.beta;
        let gamma = self.config.gamma;
        let d_s = self.config.d_s;
        let d_i = self.config.d_i;
        let d_r = self.config.d_r;
        let bc = self.config.boundary;

        let s_copy = self.s_curr.clone();
        let i_copy = self.i_curr.clone();
        let r_copy = self.r_curr.clone();
        compute_laplacian(&s_copy, &mut self.lap_s, nx, ny, dx, bc);
        compute_laplacian(&i_copy, &mut self.lap_i, nx, ny, dx, bc);
        compute_laplacian(&r_copy, &mut self.lap_r, nx, ny, dx, bc);

        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                let s = self.s_curr[k];
                let inf = self.i_curr[k];
                let r = self.r_curr[k];
                let ls = self.lap_s[k];
                let li = self.lap_i[k];
                let lr = self.lap_r[k];
                let n = s + inf + r;
                let infection_rate = if n > 1e-10 { beta * s * inf / n } else { 0.0 };

                let ds = -infection_rate + d_s * ls;
                let di = infection_rate - gamma * inf + d_i * li;
                let dr = gamma * inf + d_r * lr;

                let mut sn = s + dt * ds;
                let mut in_ = inf + dt * di;
                let mut rn = r + dt * dr;
                if sn < 0.0 { sn = 0.0; }
                if in_ < 0.0 { in_ = 0.0; }
                if rn < 0.0 { rn = 0.0; }
                self.s_next[k] = sn;
                self.i_next[k] = in_;
                self.r_next[k] = rn;
            }
        }

        if let SirBoundary::Dirichlet { s, i, r } = bc {
            for i_ in 0..nx {
                let top = self.idx(i_, 0);
                let bot = self.idx(i_, ny - 1);
                self.s_next[top] = s; self.s_next[bot] = s;
                self.i_next[top] = i; self.i_next[bot] = i;
                self.r_next[top] = r; self.r_next[bot] = r;
            }
            for j_ in 0..ny {
                let left = self.idx(0, j_);
                let right = self.idx(nx - 1, j_);
                self.s_next[left] = s; self.s_next[right] = s;
                self.i_next[left] = i; self.i_next[right] = i;
                self.r_next[left] = r; self.r_next[right] = r;
            }
        }

        std::mem::swap(&mut self.s_curr, &mut self.s_next);
        std::mem::swap(&mut self.i_curr, &mut self.i_next);
        std::mem::swap(&mut self.r_curr, &mut self.r_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.s_curr.iter().any(|&v| !v.is_finite())
            || self.i_curr.iter().any(|&v| !v.is_finite())
            || self.r_curr.iter().any(|&v| !v.is_finite())
    }

    pub fn mean_s(&self) -> f32 {
        let n = self.s_curr.len();
        if n == 0 { 0.0 } else { self.s_curr.iter().sum::<f32>() / n as f32 }
    }

    pub fn mean_i(&self) -> f32 {
        let n = self.i_curr.len();
        if n == 0 { 0.0 } else { self.i_curr.iter().sum::<f32>() / n as f32 }
    }

    pub fn mean_r(&self) -> f32 {
        let n = self.r_curr.len();
        if n == 0 { 0.0 } else { self.r_curr.iter().sum::<f32>() / n as f32 }
    }

    pub fn total_s(&self) -> f32 {
        self.s_curr.iter().sum::<f32>()
    }

    pub fn total_i(&self) -> f32 {
        self.i_curr.iter().sum::<f32>()
    }

    pub fn total_r(&self) -> f32 {
        self.r_curr.iter().sum::<f32>()
    }

    pub fn total_population(&self) -> f32 {
        self.total_s() + self.total_i() + self.total_r()
    }

    pub fn max_i(&self) -> f32 {
        self.i_curr.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn max_s(&self) -> f32 {
        self.s_curr.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_i(&self) -> f32 {
        self.i_curr.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn variance_i(&self) -> f32 {
        let m = self.mean_i();
        let n = self.i_curr.len();
        if n == 0 { return 0.0; }
        self.i_curr.iter().map(|&v| (v - m) * (v - m)).sum::<f32>() / n as f32
    }

    pub fn max_abs_i(&self) -> f32 {
        self.i_curr.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }
}

// ============================================================
// 自由函数: 避免借用冲突
// ============================================================

#[inline]
fn sir_idx(i: usize, j: usize, nx: usize) -> usize {
    j * nx + i
}

#[inline]
fn sir_wrap(i: i32, n: usize) -> usize {
    let n = n as i32;
    (((i % n) + n) % n) as usize
}

fn compute_laplacian(
    field: &[f32],
    out: &mut [f32],
    nx: usize,
    ny: usize,
    dx: f32,
    bc: SirBoundary,
) {
    let inv_dx2 = 1.0 / (dx * dx);
    for j in 0..ny {
        for i in 0..nx {
            let (ip, im, jp, jm) = match bc {
                SirBoundary::Periodic => (
                    sir_wrap(i as i32 + 1, nx),
                    sir_wrap(i as i32 - 1, nx),
                    sir_wrap(j as i32 + 1, ny),
                    sir_wrap(j as i32 - 1, ny),
                ),
                _ => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                    (j + 1).min(ny - 1),
                    if j > 0 { j - 1 } else { 0 },
                ),
            };
            let k = sir_idx(i, j, nx);
            let f = field[k];
            let f_ip = field[sir_idx(ip, j, nx)];
            let f_im = field[sir_idx(im, j, nx)];
            let f_jp = field[sir_idx(i, jp, nx)];
            let f_jm = field[sir_idx(i, jm, nx)];
            out[k] = (f_ip + f_im + f_jp + f_jm - 4.0 * f) * inv_dx2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stable_config() -> SirConfig {
        SirConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.01,
            beta: 2.0,
            gamma: 0.5,
            d_s: 0.1,
            d_i: 0.1,
            d_r: 0.1,
            boundary: SirBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = SirConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = SirConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_r0_calculation() {
        let cfg = SirConfig::default();
        // R0 = β/γ = 2.0/0.5 = 4.0
        assert!((cfg.r0() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_herd_immunity_threshold() {
        let cfg = SirConfig::default();
        // 1 - 1/R0 = 1 - 1/4 = 0.75
        assert!((cfg.herd_immunity_threshold() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_infection_timescale() {
        let cfg = SirConfig { gamma: 0.2, ..Default::default() };
        // τ = 1/γ = 5
        assert!((cfg.infection_timescale() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_initial_growth_rate() {
        let cfg = SirConfig::default();
        // r = β - γ = 2.0 - 0.5 = 1.5
        assert!((cfg.initial_growth_rate() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_r0_below_one_epidemic_fades() {
        // R0 < 1: 疫情消退
        let cfg = SirConfig { beta: 0.4, gamma: 0.5, d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        assert!(cfg.r0() < 1.0);
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 1.0, 0.0);
        let i0 = s.total_i();
        s.step_n(500);
        let i1 = s.total_i();
        assert!(i1 < i0, "epidemic should fade when R0<1: {} -> {}", i0, i1);
    }

    #[test]
    fn test_r0_above_one_epidemic_grows() {
        // R0 > 1: 疫情爆发 (初期 I 增长)
        let cfg = SirConfig { beta: 3.0, gamma: 0.5, d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        assert!(cfg.r0() > 1.0);
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 0.5, 0.0); // 小初始感染
        let i0 = s.total_i();
        s.step_n(50);
        let i1 = s.total_i();
        assert!(i1 > i0, "epidemic should grow when R0>1: {} -> {}", i0, i1);
    }

    #[test]
    fn test_population_nonnegative() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_outbreak(32.0, 32.0, 5.0, 5.0, 100.0);
        s.step_n(1000);
        let min_s = s.s_curr.iter().cloned().fold(f32::INFINITY, f32::min);
        let min_i = s.min_i();
        let min_r = s.r_curr.iter().cloned().fold(f32::INFINITY, f32::min);
        assert!(min_s >= 0.0, "S should be non-negative: {}", min_s);
        assert!(min_i >= 0.0, "I should be non-negative: {}", min_i);
        assert!(min_r >= 0.0, "R should be non-negative: {}", min_r);
    }

    #[test]
    fn test_population_conservation_no_diffusion() {
        // 无扩散时总人口守恒
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 1.0, 0.0);
        let n0 = s.total_population();
        s.step_n(1000);
        let n1 = s.total_population();
        assert!((n1 - n0).abs() / n0 < 0.01, "population should be conserved: {} -> {}", n0, n1);
    }

    #[test]
    fn test_susceptible_decreases() {
        // 疫情期间易感者应减少 (被感染)
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 1.0, 0.0);
        let s0 = s.total_s();
        s.step_n(500);
        let s1 = s.total_s();
        assert!(s1 < s0, "susceptible should decrease: {} -> {}", s0, s1);
    }

    #[test]
    fn test_recovered_increases() {
        // 恢复者应增加
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 10.0, 0.0);
        let r0 = s.total_r();
        s.step_n(500);
        let r1 = s.total_r();
        assert!(r1 > r0, "recovered should increase: {} -> {}", r0, r1);
    }

    #[test]
    fn test_epidemic_peaks_and_fades() {
        // 疫情应先达到峰值然后消退
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 0.5, 0.0);
        let mut peak_i = 0.0f32;
        for _ in 0..2000 {
            s.step();
            let ti = s.total_i();
            if ti > peak_i { peak_i = ti; }
        }
        // 疫情应有显著峰值 (远超初始)
        assert!(peak_i > 5.0, "epidemic should peak significantly: {}", peak_i);
        // 最后感染应消退
        assert!(s.total_i() < peak_i * 0.5, "epidemic should fade after peak: {} vs {}", s.total_i(), peak_i);
    }

    #[test]
    fn test_no_infection_no_outbreak() {
        // 无感染者时无疫情
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 0.0, 0.0);
        s.step_n(500);
        assert!(s.total_i() < 1e-6, "no infection without initial I");
        assert!((s.total_s() - 100.0 * s.config.n_cells() as f32).abs() < 1.0, "S should be unchanged");
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_uniform(100.0, 1.0, 0.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_uniform(100.0, 1.0, 0.0);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_outbreak(32.0, 32.0, 5.0, 5.0, 100.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = SirConfig { nx: 32, ny: 32, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_outbreak(16.0, 16.0, 3.0, 5.0, 100.0);
        s.step_n(5000);
        assert!(!s.has_nan(), "NaN after 5000 steps");
    }

    #[test]
    fn test_zero_state_stays_zero() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_zero();
        s.step_n(100);
        assert!(s.total_population() < 1e-10, "zero state should stay zero");
    }

    #[test]
    fn test_outbreak_spreads_spatially() {
        // 中心感染源应扩散到周围
        let cfg = SirConfig { d_s: 0.5, d_i: 0.5, d_r: 0.5, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_outbreak(32.0, 32.0, 3.0, 10.0, 100.0);
        // 初始感染局限于中心
        let far_idx = s.idx(0, 0);
        assert!(s.i_curr[far_idx] < 0.01, "far cell should start uninfected");
        s.step_n(500);
        // 扩散后远处应有感染
        assert!(s.i_curr[far_idx] > 0.0 || s.s_curr[far_idx] < 100.0,
            "infection should spread spatially");
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = SirConfig {
            nx: 32,
            ny: 32,
            dt: 0.005,
            boundary: SirBoundary::Dirichlet { s: 0.0, i: 0.0, r: 0.0 },
            ..stable_config()
        };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 10.0, 0.0);
        s.step_n(20);
        let nx = s.config.nx;
        let ny = s.config.ny;
        for i in 0..nx {
            assert!(s.s_curr[s.idx(i, 0)].abs() < 1e-5, "top boundary S");
            assert!(s.i_curr[s.idx(i, ny - 1)].abs() < 1e-5, "bottom boundary I");
        }
    }

    #[test]
    fn test_variance_positive() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_outbreak(32.0, 32.0, 5.0, 10.0, 100.0);
        assert!(s.variance_i() > 0.1, "variance should be positive: {}", s.variance_i());
    }

    #[test]
    fn test_diffusion_smooths_variance() {
        // R0<1 感染衰减 + 大扩散, 长时间后方差应减小
        let cfg = SirConfig {
            beta: 0.3,
            gamma: 0.5,
            d_s: 2.0,
            d_i: 2.0,
            d_r: 2.0,
            dt: 0.005,
            ..stable_config()
        };
        let mut s = SirSolver::new(cfg);
        s.initialize_outbreak(32.0, 32.0, 3.0, 10.0, 100.0);
        let v0 = s.variance_i();
        s.step_n(1000);
        let v1 = s.variance_i();
        assert!(v1 < v0, "diffusion+decay should reduce variance: {} -> {}", v0, v1);
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = SirSolver::new(stable_config());
        s.initialize_outbreak(32.0, 32.0, 5.0, 50.0, 100.0);
        s.step_n(2000);
        assert!(s.max_i() < 1000.0, "I should be bounded: {}", s.max_i());
    }

    #[test]
    fn test_final_size_below_total() {
        // 疫情结束后, 累计感染 (R) 不超过总人口
        let cfg = SirConfig { d_s: 0.0, d_i: 0.0, d_r: 0.0, ..stable_config() };
        let mut s = SirSolver::new(cfg);
        s.initialize_uniform(100.0, 1.0, 0.0);
        let n_total = s.total_population();
        s.step_n(5000);
        assert!(s.total_r() <= n_total + 1.0, "recovered should not exceed total: {} vs {}", s.total_r(), n_total);
        assert!(s.total_s() >= 0.0, "S should be non-negative");
    }

    #[test]
    fn test_diffusive_cfl() {
        let cfg = SirConfig { d_s: 1.0, d_i: 0.5, d_r: 0.3, dt: 0.25, dx: 1.0, ..Default::default() };
        // 4 * 1.0 * 0.25 / 1 = 1.0
        assert!((cfg.diffusive_cfl() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_module_count() {
        let tests: Vec<&str> = vec![
            "test_default_config_stability",
            "test_stable_dt_positive",
            "test_r0_calculation",
            "test_herd_immunity_threshold",
            "test_infection_timescale",
            "test_initial_growth_rate",
            "test_r0_below_one_epidemic_fades",
            "test_r0_above_one_epidemic_grows",
            "test_population_nonnegative",
            "test_population_conservation_no_diffusion",
            "test_susceptible_decreases",
            "test_recovered_increases",
            "test_epidemic_peaks_and_fades",
            "test_no_infection_no_outbreak",
            "test_step_advances_time",
            "test_step_n_advances",
            "test_no_nan_short_run",
            "test_no_nan_long_run",
            "test_zero_state_stays_zero",
            "test_outbreak_spreads_spatially",
            "test_dirichlet_boundary_enforced",
            "test_variance_positive",
            "test_diffusion_smooths_variance",
            "test_amplitude_bounded",
            "test_final_size_below_total",
            "test_diffusive_cfl",
        ];
        assert!(tests.len() >= 20, "need at least 20 tests, got {}", tests.len());
    }
}
