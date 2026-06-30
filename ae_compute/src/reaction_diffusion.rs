//! Reaction-Diffusion Solver — 反应扩散系统 (Gray-Scott 模型)
//!
//! 基于:
//! - Turing, A.M. 1952. "The Chemical Basis of Morphogenesis."
//!   Philosophical Transactions of the Royal Society B.
//! - Gray, P., Scott, S.K. 1985. "Sustained oscillations and other
//!   exotic patterns in isothermal reactions." J. Phys. Chem.
//! - Pearson, J.E. 1993. "Complex patterns in a simple system."
//!   Science.
//! - Sanderson, A.R., et al. 2006. "Reaction-Diffusion Textures."
//!   SIGGRAPH Courses.
//!
//! 核心方程 (Gray-Scott):
//!   dA/dt = Da * laplacian(A) - A*B*B + feed*(1-A)
//!   dB/dt = Db * laplacian(B) + A*B*B - (feed+kill)*B
//!
//! 其中:
//!   A = 反应物浓度 [0, 1]
//!   B = 催化剂浓度 [0, 1]
//!   Da, Db = 扩散系数 (通常 Db < Da, B 扩散慢)
//!   feed = A 的补充速率
//!   kill = B 的移除速率
//!
//! 反应: A + 2B -> 3B  (B 自催化, 消耗 A)
//!
//! 参数空间 (feed, kill) 决定图案类型:
//!   - solitons (孤子):     feed=0.0367, kill=0.0649
//!   - mazes (迷宫):        feed=0.0545, kill=0.062
//!   - spots (斑点):        feed=0.025,  kill=0.06
//!   - stripes (条纹):      feed=0.022,  kill=0.059
//!   - pulsating (脉动):    feed=0.014,  kill=0.045
//!   - worms (蠕虫):        feed=0.078,  kill=0.061
//!
//! 应用:
//! - 程序化纹理生成 (动物皮毛, 大理石, 皮革)
//! - 生物斑纹模拟 (豹纹, 斑马纹, 珊瑚)
//! - 迷宫生成
//! - 化学反应可视化
//! - 程序化地形细节

use serde::{Deserialize, Serialize};

// ============================================================
// 索引
// ============================================================

#[inline]
fn idx(i: usize, j: usize, k: usize, nx: usize, ny: usize) -> usize {
    i + nx * (j + ny * k)
}

// ============================================================
// 图案类型
// ============================================================

/// Gray-Scott 参数空间中的图案类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    /// 孤子 (自我复制的斑点)
    Solitons,
    /// 迷宫 (连通的走廊)
    Mazes,
    /// 斑点 (离散圆点)
    Spots,
    /// 条纹 (平行带状)
    Stripes,
    /// 脉动 (振荡斑点)
    Pulsating,
    /// 蠕虫 (弯曲的线状)
    Worms,
    /// 混沌 (无序)
    Chaotic,
    /// 死亡 (B 衰减到 0)
    Dying,
    /// 未知/过渡
    Unknown,
}

/// 根据 (feed, kill) 参数推断图案类型
///
/// 基于 Pearson 1993 的参数空间相图
pub fn classify_pattern(feed: f32, kill: f32) -> PatternType {
    // B 衰减区: kill > feed + 0.02 或 kill > 0.07
    if kill > 0.07 || feed < 0.01 {
        return PatternType::Dying;
    }

    // 各图案区域的近似边界 (基于经验)
    // 注: 真实相图更复杂, 这里用简化版
    let f = feed;
    let k = kill;

    if (f - 0.0367).abs() < 0.003 && (k - 0.0649).abs() < 0.003 {
        PatternType::Solitons
    } else if (f - 0.0545).abs() < 0.004 && (k - 0.062).abs() < 0.003 {
        PatternType::Mazes
    } else if (f - 0.025).abs() < 0.002 && (k - 0.06).abs() < 0.003 {
        PatternType::Spots
    } else if (f - 0.022).abs() < 0.003 && (k - 0.059).abs() < 0.003 {
        PatternType::Stripes
    } else if (f - 0.014).abs() < 0.003 && (k - 0.045).abs() < 0.003 {
        PatternType::Pulsating
    } else if (f - 0.078).abs() < 0.005 && (k - 0.061).abs() < 0.003 {
        PatternType::Worms
    } else if f > 0.05 && k < 0.06 {
        PatternType::Chaotic
    } else {
        PatternType::Unknown
    }
}

// ============================================================
// 预设参数
// ============================================================

/// Gray-Scott 求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrayScottConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h: f32,
    /// A 的补充速率 feed (F)
    pub feed: f32,
    /// B 的移除速率 kill (k)
    pub kill: f32,
    /// A 的扩散系数
    pub diff_a: f32,
    /// B 的扩散系数 (通常 = 0.5 * diff_a)
    pub diff_b: f32,
    /// 时间步长
    pub dt: f32,
    /// 是否使用周期性边界
    pub periodic: bool,
}

impl GrayScottConfig {
    /// 孤子图案
    pub fn solitons(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.0367, kill: 0.0649,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }

    /// 迷宫图案
    pub fn mazes(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.0545, kill: 0.062,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }

    /// 斑点图案
    pub fn spots(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.025, kill: 0.06,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }

    /// 条纹图案
    pub fn stripes(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.022, kill: 0.059,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }

    /// 脉动图案
    pub fn pulsating(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.014, kill: 0.045,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }

    /// 蠕虫图案
    pub fn worms(nx: usize, ny: usize) -> Self {
        Self {
            nx, ny, nz: 1, h: 1.0,
            feed: 0.078, kill: 0.061,
            diff_a: 0.082, diff_b: 0.041,
            dt: 1.0, periodic: true,
        }
    }
}

impl Default for GrayScottConfig {
    fn default() -> Self {
        Self::solitons(64, 64)
    }
}

// ============================================================
// Gray-Scott 求解器
// ============================================================

/// Gray-Scott 反应扩散求解器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrayScottSolver {
    pub config: GrayScottConfig,
    /// 反应物 A (初始全场 = 1.0)
    pub a: Vec<f32>,
    /// 催化剂 B (初始全场 = 0.0, 在种子点 = 1.0)
    pub b: Vec<f32>,
    /// 模拟时间
    pub time: f32,
    /// 步数
    pub steps: usize,
}

impl GrayScottSolver {
    /// 创建求解器 (A=1, B=0)
    pub fn new(config: GrayScottConfig) -> Self {
        let n = config.nx * config.ny * config.nz;
        Self {
            config,
            a: vec![1.0; n],
            b: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    pub fn num_cells(&self) -> usize {
        self.config.nx * self.config.ny * self.config.nz
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        idx(i, j, k, self.config.nx, self.config.ny)
    }

    /// 周期性索引 (wrap-around)
    #[inline]
    fn wrap(&self, i: isize, j: isize, k: isize) -> (usize, usize, usize) {
        let nx = self.config.nx as isize;
        let ny = self.config.ny as isize;
        let nz = self.config.nz as isize;
        let wi = ((i % nx + nx) % nx) as usize;
        let wj = ((j % ny + ny) % ny) as usize;
        let wk = ((k % nz + nz) % nz) as usize;
        (wi, wj, wk)
    }

    /// 获取 A 的值 (带边界处理)
    pub fn a_at(&self, i: isize, j: isize, k: isize) -> f32 {
        let nx = self.config.nx as isize;
        let ny = self.config.ny as isize;
        let nz = self.config.nz as isize;
        if i < 0 || i >= nx || j < 0 || j >= ny || k < 0 || k >= nz {
            if self.config.periodic {
                let (wi, wj, wk) = self.wrap(i, j, k);
                return self.a[self.idx(wi, wj, wk)];
            }
            // 非周期: 钳制 (Dirichlet A=1, B=0)
            return 1.0;
        }
        self.a[self.idx(i as usize, j as usize, k as usize)]
    }

    /// 获取 B 的值 (带边界处理)
    pub fn b_at(&self, i: isize, j: isize, k: isize) -> f32 {
        let nx = self.config.nx as isize;
        let ny = self.config.ny as isize;
        let nz = self.config.nz as isize;
        if i < 0 || i >= nx || j < 0 || j >= ny || k < 0 || k >= nz {
            if self.config.periodic {
                let (wi, wj, wk) = self.wrap(i, j, k);
                return self.b[self.idx(wi, wj, wk)];
            }
            return 0.0;
        }
        self.b[self.idx(i as usize, j as usize, k as usize)]
    }

    /// 在 (i,j,k) 处注入 B (种子)
    pub fn seed(&mut self, i: usize, j: usize, k: usize, value: f32) {
        let idx = self.idx(i, j, k);
        self.b[idx] = value;
        self.a[idx] = 1.0 - value;
    }

    /// 在中心区域注入 B (圆形种子)
    pub fn seed_center(&mut self, radius: usize) {
        let cx = self.config.nx / 2;
        let cy = self.config.ny / 2;
        let cz = self.config.nz / 2;
        let r2 = (radius * radius) as f32;
        for k in 0..self.config.nz {
            for j in 0..self.config.ny {
                for i in 0..self.config.nx {
                    let di = (i as isize - cx as isize) as f32;
                    let dj = (j as isize - cy as isize) as f32;
                    let dk = (k as isize - cz as isize) as f32;
                    let d2 = di * di + dj * dj + dk * dk;
                    if d2 <= r2 {
                        let idx = self.idx(i, j, k);
                        self.b[idx] = 1.0;
                        self.a[idx] = 0.0;
                    }
                }
            }
        }
    }

    /// 添加随机噪声种子
    pub fn seed_random(&mut self, seed: u64, count: usize, value: f32) {
        // 简单 LCG 随机数
        let mut state = seed.max(1);
        let n = self.num_cells();
        for _ in 0..count {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (state >> 33) as usize % n;
            self.b[r] = value;
            self.a[r] = 0.0;
        }
    }

    /// 计算 A 的拉普拉斯算子 (7点)
    pub fn laplacian_a(&self, i: usize, j: usize, k: usize) -> f32 {
        let h2 = self.config.h * self.config.h;
        let ii = i as isize;
        let jj = j as isize;
        let kk = k as isize;
        let a_c = self.a_at(ii, jj, kk);
        let a_ip = self.a_at(ii + 1, jj, kk);
        let a_im = self.a_at(ii - 1, jj, kk);
        let a_jp = self.a_at(ii, jj + 1, kk);
        let a_jm = self.a_at(ii, jj - 1, kk);
        let a_kp = self.a_at(ii, jj, kk + 1);
        let a_km = self.a_at(ii, jj, kk - 1);
        (a_ip + a_im + a_jp + a_jm + a_kp + a_km - 6.0 * a_c) / h2
    }

    /// 计算 B 的拉普拉斯算子 (7点)
    pub fn laplacian_b(&self, i: usize, j: usize, k: usize) -> f32 {
        let h2 = self.config.h * self.config.h;
        let ii = i as isize;
        let jj = j as isize;
        let kk = k as isize;
        let b_c = self.b_at(ii, jj, kk);
        let b_ip = self.b_at(ii + 1, jj, kk);
        let b_im = self.b_at(ii - 1, jj, kk);
        let b_jp = self.b_at(ii, jj + 1, kk);
        let b_jm = self.b_at(ii, jj - 1, kk);
        let b_kp = self.b_at(ii, jj, kk + 1);
        let b_km = self.b_at(ii, jj, kk - 1);
        (b_ip + b_im + b_jp + b_jm + b_kp + b_km - 6.0 * b_c) / h2
    }

    /// 单步显式 Euler
    ///
    /// dA/dt = Da * lap(A) - A*B*B + feed*(1-A)
    /// dB/dt = Db * lap(B) + A*B*B - (feed+kill)*B
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let da = self.config.diff_a;
        let db = self.config.diff_b;
        let feed = self.config.feed;
        let kill = self.config.kill;

        let old_a = self.a.clone();
        let old_b = self.b.clone();

        for k in 0..self.config.nz {
            for j in 0..self.config.ny {
                for i in 0..self.config.nx {
                    let idx = self.idx(i, j, k);
                    let a = old_a[idx];
                    let b = old_b[idx];
                    let lap_a = self.laplacian_a(i, j, k);
                    let lap_b = self.laplacian_b(i, j, k);
                    let abb = a * b * b;

                    let new_a = a + dt * (da * lap_a - abb + feed * (1.0 - a));
                    let new_b = b + dt * (db * lap_b + abb - (feed + kill) * b);

                    self.a[idx] = new_a.clamp(0.0, 1.0);
                    self.b[idx] = new_b.clamp(0.0, 1.0);
                }
            }
        }

        self.time += dt;
        self.steps += 1;
    }

    /// 批量步进
    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// CFL 稳定性条件: dt < h^2 / (4 * max(Da, Db))
    pub fn cfl_dt(&self) -> f32 {
        let h2 = self.config.h * self.config.h;
        let dmax = self.config.diff_a.max(self.config.diff_b);
        if dmax < 1e-12 {
            f32::INFINITY
        } else {
            h2 / (4.0 * dmax)
        }
    }

    /// 检查稳定性
    pub fn is_stable(&self) -> bool {
        self.config.dt <= self.cfl_dt()
    }

    /// 平均 A 浓度
    pub fn average_a(&self) -> f32 {
        if self.a.is_empty() {
            return 0.0;
        }
        self.a.iter().sum::<f32>() / self.a.len() as f32
    }

    /// 平均 B 浓度
    pub fn average_b(&self) -> f32 {
        if self.b.is_empty() {
            return 0.0;
        }
        self.b.iter().sum::<f32>() / self.b.len() as f32
    }

    /// 最大 B 浓度
    pub fn max_b(&self) -> f32 {
        self.b.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    /// 最小 A 浓度
    pub fn min_a(&self) -> f32 {
        self.a.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    /// B 浓度方差 (衡量图案复杂度)
    pub fn b_variance(&self) -> f32 {
        if self.b.is_empty() {
            return 0.0;
        }
        let mean = self.average_b();
        let mut sum_sq = 0.0;
        for &b in &self.b {
            let d = b - mean;
            sum_sq += d * d;
        }
        sum_sq / self.b.len() as f32
    }

    /// 推断当前图案类型
    pub fn pattern_type(&self) -> PatternType {
        classify_pattern(self.config.feed, self.config.kill)
    }

    /// 重置 (A=1, B=0)
    pub fn reset(&mut self) {
        for a in &mut self.a {
            *a = 1.0;
        }
        for b in &mut self.b {
            *b = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 获取 B 的 2D 切片 (用于渲染/可视化)
    pub fn b_slice_2d(&self) -> &[f32] {
        &self.b[..self.config.nx * self.config.ny]
    }

    /// 获取 A 的 2D 切片
    pub fn a_slice_2d(&self) -> &[f32] {
        &self.a[..self.config.nx * self.config.ny]
    }

    /// 检测图案是否活跃 (B 浓度有显著变化)
    pub fn is_active(&self) -> bool {
        self.max_b() > 0.1 && self.b_variance() > 1e-4
    }

    /// 检测图案是否已死亡 (B 衰减到 ~0)
    pub fn is_dead(&self) -> bool {
        self.max_b() < 0.01
    }
}


// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn make_small_solver() -> GrayScottSolver {
        let cfg = GrayScottConfig {
            nx: 8,
            ny: 8,
            nz: 1,
            h: 1.0,
            feed: 0.0367,
            kill: 0.0649,
            diff_a: 0.082,
            diff_b: 0.041,
            dt: 1.0,
            periodic: true,
        };
        GrayScottSolver::new(cfg)
    }

    #[test]
    fn test_default_config() {
        let cfg = GrayScottConfig::default();
        assert_eq!(cfg.nx, 64);
        assert_eq!(cfg.ny, 64);
        assert_eq!(cfg.nz, 1);
        assert!(approx(cfg.feed, 0.0367, 1e-6));
        assert!(approx(cfg.kill, 0.0649, 1e-6));
        assert!(approx(cfg.diff_a, 0.082, 1e-6));
        assert!(approx(cfg.diff_b, 0.041, 1e-6));
    }

    #[test]
    fn test_presets() {
        let s = GrayScottConfig::solitons(16, 16);
        assert!(approx(s.feed, 0.0367, 1e-6));
        assert!(approx(s.kill, 0.0649, 1e-6));

        let m = GrayScottConfig::mazes(16, 16);
        assert!(approx(m.feed, 0.0545, 1e-6));
        assert!(approx(m.kill, 0.062, 1e-6));

        let sp = GrayScottConfig::spots(16, 16);
        assert!(approx(sp.feed, 0.025, 1e-6));
        assert!(approx(sp.kill, 0.06, 1e-6));

        let st = GrayScottConfig::stripes(16, 16);
        assert!(approx(st.feed, 0.022, 1e-6));
        assert!(approx(st.kill, 0.059, 1e-6));

        let p = GrayScottConfig::pulsating(16, 16);
        assert!(approx(p.feed, 0.014, 1e-6));
        assert!(approx(p.kill, 0.045, 1e-6));

        let w = GrayScottConfig::worms(16, 16);
        assert!(approx(w.feed, 0.078, 1e-6));
        assert!(approx(w.kill, 0.061, 1e-6));
    }

    #[test]
    fn test_solver_creation() {
        let solver = make_small_solver();
        assert_eq!(solver.num_cells(), 64);
        assert_eq!(solver.a.len(), 64);
        assert_eq!(solver.b.len(), 64);
        assert_eq!(solver.time, 0.0);
        assert_eq!(solver.steps, 0);
    }

    #[test]
    fn test_initial_values() {
        let solver = make_small_solver();
        // A 初始 = 1.0, B 初始 = 0.0
        for &a in &solver.a {
            assert!(approx(a, 1.0, 1e-6));
        }
        for &b in &solver.b {
            assert!(approx(b, 0.0, 1e-6));
        }
    }

    #[test]
    fn test_seed() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        let idx = solver.idx(4, 4, 0);
        assert!(approx(solver.b[idx], 1.0, 1e-6));
        // 其他点仍为 0
        let other = solver.idx(0, 0, 0);
        assert!(approx(solver.b[other], 0.0, 1e-6));
    }

    #[test]
    fn test_seed_center() {
        let mut solver = make_small_solver();
        solver.seed_center(2);
        // 中心 (4,4,0) 应有 B=1
        let cx = solver.idx(4, 4, 0);
        assert!(approx(solver.b[cx], 1.0, 1e-6));
        assert!(approx(solver.a[cx], 0.0, 1e-6));
        // 角落应未受影响
        let corner = solver.idx(0, 0, 0);
        assert!(approx(solver.b[corner], 0.0, 1e-6));
    }

    #[test]
    fn test_seed_random() {
        let mut solver = make_small_solver();
        solver.seed_random(42, 10, 1.0);
        // 至少有一些 B > 0
        let count = solver.b.iter().filter(|&&b| b > 0.5).count();
        assert!(count > 0, "random seed should inject some B");
    }

    #[test]
    fn test_periodic_boundary() {
        let mut solver = make_small_solver();
        solver.seed(0, 0, 0, 1.0);
        // 周期性: a_at(-1, 0, 0) 应等于 a_at(nx-1, 0, 0)
        let a_wrap = solver.a_at(-1, 0, 0);
        let a_last = solver.a[solver.idx(7, 0, 0)];
        assert!(approx(a_wrap, a_last, 1e-6));
    }

    #[test]
    fn test_non_periodic_boundary() {
        let mut solver = make_small_solver();
        solver.config.periodic = false;
        // 非周期: a_at(-1, 0, 0) 应 = 1.0 (Dirichlet)
        let a = solver.a_at(-1, 0, 0);
        assert!(approx(a, 1.0, 1e-6));
        let b = solver.b_at(-1, 0, 0);
        assert!(approx(b, 0.0, 1e-6));
    }

    #[test]
    fn test_laplacian_zero_uniform() {
        let solver = make_small_solver();
        // 均匀场, 拉普拉斯 = 0
        let lap = solver.laplacian_a(4, 4, 0);
        assert!(lap.abs() < 1e-6);
    }

    #[test]
    fn test_laplacian_b_zero_uniform() {
        let solver = make_small_solver();
        let lap = solver.laplacian_b(4, 4, 0);
        assert!(lap.abs() < 1e-6);
    }

    #[test]
    fn test_laplacian_with_seed() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        // 中心 B=1, 邻居 B=0 -> laplacian_B < 0 (凹)
        let lap = solver.laplacian_b(4, 4, 0);
        assert!(lap < 0.0, "laplacian at seed should be negative: {}", lap);
    }

    #[test]
    fn test_step_reduces_a_at_seed() {
        // A=0.5, B=1.0: reaction A*B*B = 0.5 > 0, A is consumed
        let mut solver = make_small_solver();
        let idx = solver.idx(4, 4, 0);
        solver.a[idx] = 0.5;
        solver.b[idx] = 1.0;
        let a_before = solver.a[idx];
        solver.step();
        let a_after = solver.a[idx];
        assert!(a_after < a_before, "A should decrease: before={}, after={}", a_before, a_after);
    }

    #[test]
    fn test_step_b_reacts() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        let b_before = solver.b[solver.idx(4, 4, 0)];
        solver.step();
        let b_after = solver.b[solver.idx(4, 4, 0)];
        // B 的变化取决于反应和扩散
        // 在种子点, A=0, 所以 A*B*B = 0, B 主要被 (feed+kill)*B 移除
        // 但邻居 B=0 -> 扩散带走 B
        // B 应该减少 (无 A 不能自催化)
        assert!(b_after <= b_before + 1e-6, "B should not increase without A");
    }

    #[test]
    fn test_step_with_a_and_b() {
        // A 和 B 都存在时, 反应发生
        let mut solver = make_small_solver();
        // 设置一个区域 A=0.5, B=0.5
        for i in 3..5 {
            for j in 3..5 {
                let idx = solver.idx(i, j, 0);
                solver.a[idx] = 0.5;
                solver.b[idx] = 0.5;
            }
        }
        let a_before = solver.a[solver.idx(3, 3, 0)];
        solver.step();
        let a_after = solver.a[solver.idx(3, 3, 0)];
        // A*B*B > 0, A 应减少
        assert!(a_after < a_before, "A should be consumed: before={}, after={}", a_before, a_after);
    }

    #[test]
    fn test_step_b_diffuses() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        let b_neighbor_before = solver.b[solver.idx(5, 4, 0)];
        solver.step();
        let b_neighbor_after = solver.b[solver.idx(5, 4, 0)];
        // B 应扩散到邻居
        assert!(b_neighbor_after > b_neighbor_before, "B should diffuse to neighbor");
    }

    #[test]
    fn test_step_advances_time() {
        let mut solver = make_small_solver();
        assert_eq!(solver.time, 0.0);
        assert_eq!(solver.steps, 0);
        solver.step();
        assert!(approx(solver.time, 1.0, 1e-6));
        assert_eq!(solver.steps, 1);
        solver.step();
        assert!(approx(solver.time, 2.0, 1e-6));
        assert_eq!(solver.steps, 2);
    }

    #[test]
    fn test_step_n() {
        let mut solver = make_small_solver();
        solver.seed_center(2);
        solver.step_n(10);
        assert_eq!(solver.steps, 10);
        assert!(approx(solver.time, 10.0, 1e-6));
    }

    #[test]
    fn test_cfl_dt() {
        let solver = make_small_solver();
        let dt = solver.cfl_dt();
        // h=1, Da=0.082, Db=0.041
        // CFL: h^2 / (4 * max(Da, Db)) = 1 / (4 * 0.082) = 3.048
        assert!(approx(dt, 1.0 / (4.0 * 0.082), 1e-3));
        assert!(solver.is_stable());  // dt=1.0 < 3.048
    }

    #[test]
    fn test_unstable_config() {
        let mut solver = make_small_solver();
        solver.config.dt = 100.0;  // 远超 CFL
        assert!(!solver.is_stable());
    }

    #[test]
    fn test_pattern_classification() {
        assert_eq!(classify_pattern(0.0367, 0.0649), PatternType::Solitons);
        assert_eq!(classify_pattern(0.0545, 0.062), PatternType::Mazes);
        assert_eq!(classify_pattern(0.025, 0.06), PatternType::Spots);
        assert_eq!(classify_pattern(0.022, 0.059), PatternType::Stripes);
        assert_eq!(classify_pattern(0.014, 0.045), PatternType::Pulsating);
        assert_eq!(classify_pattern(0.078, 0.061), PatternType::Worms);
        assert_eq!(classify_pattern(0.005, 0.05), PatternType::Dying);
        assert_eq!(classify_pattern(0.01, 0.08), PatternType::Dying);
    }

    #[test]
    fn test_solver_pattern_type() {
        let solver = make_small_solver();
        assert_eq!(solver.pattern_type(), PatternType::Solitons);
    }

    #[test]
    fn test_average_a_b() {
        let solver = make_small_solver();
        // 初始: A=1, B=0
        assert!(approx(solver.average_a(), 1.0, 1e-6));
        assert!(approx(solver.average_b(), 0.0, 1e-6));
    }

    #[test]
    fn test_max_b_min_a() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        assert!(approx(solver.max_b(), 1.0, 1e-6));
        assert!(approx(solver.min_a(), 0.0, 1e-6));
    }

    #[test]
    fn test_b_variance() {
        let solver = make_small_solver();
        // 均匀场, 方差 = 0
        assert!(approx(solver.b_variance(), 0.0, 1e-6));
    }

    #[test]
    fn test_b_variance_with_seed() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        // 非均匀, 方差 > 0
        assert!(solver.b_variance() > 0.0);
    }

    #[test]
    fn test_is_active_initial() {
        let solver = make_small_solver();
        // 初始 B=0, 不活跃
        assert!(!solver.is_active());
    }

    #[test]
    fn test_is_active_with_seed() {
        let mut solver = make_small_solver();
        solver.seed_center(2);
        // 有种子, 应活跃
        assert!(solver.is_active());
    }

    #[test]
    fn test_is_dead_initial() {
        let solver = make_small_solver();
        // B=0, 视为死亡
        assert!(solver.is_dead());
    }

    #[test]
    fn test_is_not_dead_with_seed() {
        let mut solver = make_small_solver();
        solver.seed(4, 4, 0, 1.0);
        assert!(!solver.is_dead());
    }

    #[test]
    fn test_reset() {
        let mut solver = make_small_solver();
        solver.seed_center(2);
        solver.step_n(5);
        solver.reset();
        assert!(approx(solver.time, 0.0, 1e-6));
        assert_eq!(solver.steps, 0);
        for &a in &solver.a {
            assert!(approx(a, 1.0, 1e-6));
        }
        for &b in &solver.b {
            assert!(approx(b, 0.0, 1e-6));
        }
    }

    #[test]
    fn test_b_slice_2d() {
        let solver = make_small_solver();
        let slice = solver.b_slice_2d();
        assert_eq!(slice.len(), 64);
        let a_slice = solver.a_slice_2d();
        assert_eq!(a_slice.len(), 64);
    }

    #[test]
    fn test_pattern_grows_over_time() {
        // 种子注入后, B 应随时间扩散和反应
        let mut solver = make_small_solver();
        solver.seed_center(1);
        let b_var_0 = solver.b_variance();
        solver.step_n(20);
        let b_var_20 = solver.b_variance();
        // 图案应发展 (方差变化)
        // 注: 方差可能增加或减少, 但应变化
        assert!((b_var_20 - b_var_0).abs() > 1e-6, "pattern should evolve");
    }

    #[test]
    fn test_clamp_values() {
        // 测试 A, B 被钳制到 [0, 1]
        let mut solver = make_small_solver();
        // 设置极端初始值
        for i in 0..solver.num_cells() {
            solver.a[i] = 0.5;
            solver.b[i] = 0.5;
        }
        // 多步后, 值应在 [0, 1]
        solver.step_n(50);
        for &a in &solver.a {
            assert!(a >= 0.0 && a <= 1.0, "A out of range: {}", a);
        }
        for &b in &solver.b {
            assert!(b >= 0.0 && b <= 1.0, "B out of range: {}", b);
        }
    }

    #[test]
    fn test_3d_support() {
        let cfg = GrayScottConfig {
            nx: 4,
            ny: 4,
            nz: 4,
            h: 1.0,
            feed: 0.0367,
            kill: 0.0649,
            diff_a: 0.082,
            diff_b: 0.041,
            dt: 1.0,
            periodic: true,
        };
        let mut solver = GrayScottSolver::new(cfg);
        assert_eq!(solver.num_cells(), 64);
        solver.seed(2, 2, 2, 1.0);
        solver.step();
        // 不崩溃即成功
        assert_eq!(solver.steps, 1);
    }

    #[test]
    fn test_no_reaction_without_b() {
        // 没有 B 时, A 应趋向 1 (feed 补充)
        let mut solver = make_small_solver();
        // 把 A 设为 0.5
        for a in &mut solver.a {
            *a = 0.5;
        }
        solver.step_n(10);
        // A 应上升 (feed*(1-A) > 0, 无消耗)
        let a_avg = solver.average_a();
        assert!(a_avg > 0.5, "A should increase without B: {}", a_avg);
    }

    #[test]
    fn test_feed_replenishes_a() {
        // feed 速率越大, A 恢复越快
        let mut solver_fast = make_small_solver();
        solver_fast.config.feed = 0.1;
        let mut solver_slow = make_small_solver();
        solver_slow.config.feed = 0.01;
        for s in [&mut solver_fast, &mut solver_slow] {
            for a in &mut s.a {
                *a = 0.5;
            }
            for b in &mut s.b {
                *b = 0.0;
            }
        }
        solver_fast.step_n(5);
        solver_slow.step_n(5);
        assert!(solver_fast.average_a() > solver_slow.average_a(),
            "faster feed should replenish A quicker");
    }

    #[test]
    fn test_kill_reduces_b() {
        // kill 速率越大, B 衰减越快
        let mut solver_fast = make_small_solver();
        solver_fast.config.kill = 0.1;
        let mut solver_slow = make_small_solver();
        solver_slow.config.kill = 0.03;
        for s in [&mut solver_fast, &mut solver_slow] {
            for a in &mut s.a {
                *a = 0.0;  // 无 A, 反应不发生
            }
            for b in &mut s.b {
                *b = 0.5;
            }
        }
        solver_fast.step_n(5);
        solver_slow.step_n(5);
        assert!(solver_fast.average_b() < solver_slow.average_b(),
            "faster kill should reduce B quicker");
    }
}

