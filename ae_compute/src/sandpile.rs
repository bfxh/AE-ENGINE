//! Bak-Tang-Wiesenfeld Sandpile — 自组织临界性 (SOC) 经典模型
//!
//! Per Bak, Chao Tang, Kurt Wiesenfeld 1987 年提出的元胞自动机模型,
//! 是自组织临界性 (Self-Organized Criticality) 的范例. 系统在无
//! 任何参数调节下自发演化到临界状态, 展现幂律分布的雪崩 —
//! 自然界许多复杂系统 (地震、神经活动、金融市场) 的简化模型.
//!
//! 规则 (BTW 沙堆):
//!   - N×N 格点, 每格有整数"高度" z(i,j) (无单位)
//!   - 驱动: 在随机格点加 1 粒沙
//!   - 雪崩规则: 若 z(i,j) >= z_critical (典型 4), 该格 topple:
//!       z(i,j) -= 4
//!       z(i±1, j) += 1
//!       z(i, j±1) += 1
//!   - 边界: 开放 (沙粒从边界流出消失)
//!   - 重复直到所有格点 z < z_critical (弛豫完成)
//!
//! 关键量:
//!   - 雪崩大小 s = 总 topple 数
//!   - 雪崩持续时间 t
//!   - 雪崩面积 a = 不同 topple 格点数
//!   - 幂律分布 P(s) ~ s^(-τ), τ ≈ 1.0 (2D BTW)
//!
//! 自组织临界:
//!   - 系统无需参数调节即可达到临界态
//!   - 临界态有任意长时间-空间关联
//!   - 小扰动可触发任意大雪崩 (无特征尺度)
//!
//! 数值方法:
//!   - 队列驱动的 topple 传播 (避免无限递归)
//!   - 边界外的 topple 直接丢弃 (开放边界)
//!
//! 应用:
//!   - 地震 (Gutenberg-Richter 律)
//!   - 神经元雪崩 (Beggs & Plenz)
//!   - 太阳耀斑
//!   - 金融崩溃
//!   - 森林火灾 (Drossel-Schwabl 模型)
//!   - 演化 (Bak-Sneppen 模型)
//!
//! 基于:
//!   - Bak, P., Tang, C. & Wiesenfeld, K. 1987. "Self-organized
//!     criticality: An explanation of 1/f noise." Phys. Rev. Lett.
//!     59, 381.
//!   - Bak, P., Tang, C. & Wiesenfeld, K. 1988. "Self-organized
//!     criticality." Phys. Rev. A 38, 364.
//!   - Dhar, D. 2006. "Self-organized critical state of sandpile
//!     automaton models." Phys. Rev. Lett. 64, 161317.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandpileConfig {
    /// 网格大小 N×N
    pub n: usize,
    /// 临界高度 z_critical (典型 4)
    pub z_critical: u32,
}

impl Default for SandpileConfig {
    fn default() -> Self {
        SandpileConfig {
            n: 32,
            z_critical: 4,
        }
    }
}

impl SandpileConfig {
    pub fn n_cells(&self) -> usize {
        self.n * self.n
    }
}

#[derive(Debug, Clone, Default)]
pub struct AvalancheStats {
    /// 总 topple 次数 (大小)
    pub size: u64,
    /// 雪崩持续时间 (步数)
    pub duration: u64,
    /// 不同 topple 格点数 (面积)
    pub area: u64,
    /// 是否触及边界
    pub touched_boundary: bool,
}

pub struct SandpileSolver {
    pub config: SandpileConfig,
    /// 格点高度 z(i,j)
    pub heights: Vec<u32>,
    /// 累计驱动次数
    pub n_drops: u64,
    /// 累计雪崩统计
    pub total_avalanches: u64,
    pub total_topples: u64,
    pub max_avalanche_size: u64,
    /// 最近一次雪崩统计
    pub last_avalanche: AvalancheStats,
    /// 历史雪崩大小 (用于幂律分析)
    pub avalanche_sizes: Vec<u64>,
}

impl SandpileSolver {
    pub fn new(config: SandpileConfig) -> Self {
        let n = config.n_cells();
        SandpileSolver {
            config,
            heights: vec![0; n],
            n_drops: 0,
            total_avalanches: 0,
            total_topples: 0,
            max_avalanche_size: 0,
            last_avalanche: AvalancheStats::default(),
            avalanche_sizes: Vec::new(),
        }
    }

    pub fn initialize_uniform(&mut self, z: u32) {
        for h in &mut self.heights {
            *h = z;
        }
        self.n_drops = 0;
        self.total_avalanches = 0;
        self.total_topples = 0;
        self.max_avalanche_size = 0;
        self.last_avalanche = AvalancheStats::default();
        self.avalanche_sizes.clear();
    }

    pub fn initialize_zero(&mut self) {
        self.initialize_uniform(0);
    }

    pub fn initialize_random(&mut self, max_z: u32, seed: u64) {
        let mut rng = SandRng::new(seed);
        for h in &mut self.heights {
            *h = (rng.next_u32() % (max_z + 1)) as u32;
        }
        self.n_drops = 0;
        self.total_avalanches = 0;
        self.total_topples = 0;
        self.max_avalanche_size = 0;
        self.last_avalanche = AvalancheStats::default();
        self.avalanche_sizes.clear();
    }

    #[inline]
    fn idx(&self, i: i32, j: i32) -> Option<usize> {
        let n = self.config.n as i32;
        if i < 0 || i >= n || j < 0 || j >= n {
            None // 开放边界外
        } else {
            Some((j as usize) * self.config.n + (i as usize))
        }
    }

    /// 在指定格点加一粒沙, 触发雪崩, 返回雪崩统计
    pub fn drop_at(&mut self, i: usize, j: usize) -> AvalancheStats {
        let idx = j * self.config.n + i;
        self.heights[idx] += 1;
        self.n_drops += 1;
        let stats = self.relarx();
        if stats.size > 0 {
            self.total_avalanches += 1;
            self.total_topples += stats.size;
            if stats.size > self.max_avalanche_size {
                self.max_avalanche_size = stats.size;
            }
            self.avalanche_sizes.push(stats.size);
        }
        self.last_avalanche = stats.clone();
        stats
    }

    /// 在随机格点加一粒沙
    pub fn drop_random(&mut self, seed: u64) -> AvalancheStats {
        let mut rng = SandRng::new(seed.wrapping_add(self.n_drops));
        let i = (rng.next_u32() as usize) % self.config.n;
        let j = (rng.next_u32() as usize) % self.config.n;
        self.drop_at(i, j)
    }

    /// 弛豫所有不稳定格点 (雪崩传播)
    fn relarx(&mut self) -> AvalancheStats {
        let n = self.config.n;
        let z_crit = self.config.z_critical;
        let mut stats = AvalancheStats::default();
        // topple 计数 (每格)
        let mut topple_count = vec![0u32; n * n];
        // 待 topple 队列
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
        // 标记是否在队列中 (避免重复)
        let mut in_queue = vec![false; n * n];

        // 找出初始不稳定格点
        for j in 0..n {
            for i in 0..n {
                let idx = j * n + i;
                if self.heights[idx] >= z_crit {
                    queue.push_back((i, j));
                    in_queue[idx] = true;
                }
            }
        }

        let mut step = 0u64;
        while let Some((i, j)) = queue.pop_front() {
            let idx = j * n + i;
            in_queue[idx] = false;
            if self.heights[idx] < z_crit {
                continue; // 已被邻居 topple 稳定化
            }
            // topple: 减去邻居数 (4) 而非 z_critical, 保证守恒 (4 出 4 进)
            // z_critical 仅作阈值, 需 >= 4 以避免负高度
            self.heights[idx] -= 4;
            topple_count[idx] += 1;
            stats.size += 1;

            // 检查 4 个邻居 (含边界外)
            let neighbors = [(i as i32 + 1, j as i32), (i as i32 - 1, j as i32),
                             (i as i32, j as i32 + 1), (i as i32, j as i32 - 1)];
            for (ni, nj) in neighbors {
                match self.idx(ni, nj) {
                    Some(nidx) => {
                        self.heights[nidx] += 1;
                        if self.heights[nidx] >= z_crit && !in_queue[nidx] {
                            queue.push_back((ni as usize, nj as usize));
                            in_queue[nidx] = true;
                        }
                    }
                    None => {
                        // 沙粒掉出边界, 消失
                        stats.touched_boundary = true;
                    }
                }
            }
            step += 1;
        }

        // 计算面积 (topple_count > 0 的格点数)
        for &c in &topple_count {
            if c > 0 {
                stats.area += 1;
            }
        }
        stats.duration = step;

        stats
    }

    /// 多次随机驱动
    pub fn run(&mut self, n_drops: usize, seed: u64) {
        for k in 0..n_drops {
            self.drop_random(seed.wrapping_add(k as u64));
        }
    }

    /// 检查是否所有格点都稳定
    pub fn is_stable(&self) -> bool {
        let z_crit = self.config.z_critical;
        self.heights.iter().all(|&h| h < z_crit)
    }

    pub fn max_height(&self) -> u32 {
        self.heights.iter().copied().max().unwrap_or(0)
    }

    pub fn min_height(&self) -> u32 {
        self.heights.iter().copied().min().unwrap_or(0)
    }

    pub fn mean_height(&self) -> f32 {
        if self.heights.is_empty() {
            return 0.0;
        }
        self.heights.iter().sum::<u32>() as f32 / self.heights.len() as f32
    }

    /// 雪崩大小分布的幂律指数估计 (简单 log-log 线性拟合)
    /// 返回 Some(tau) 若拟合成功, 否则 None
    pub fn avalanche_exponent(&self) -> Option<f64> {
        if self.avalanche_sizes.len() < 50 {
            return None;
        }
        // 构建直方图 (对数分箱)
        let max_size = *self.avalanche_sizes.iter().max()? as f64;
        let n_bins = 10;
        let log_min = 1.0_f64.ln();
        let log_max = max_size.ln().max(log_min + 1.0);
        let bin_width = (log_max - log_min) / n_bins as f64;
        let mut counts = vec![0u64; n_bins];
        let mut bin_centers = vec![0.0_f64; n_bins];
        for k in 0..n_bins {
            bin_centers[k] = ((log_min + (k as f64 + 0.5) * bin_width)).exp();
        }
        for &s in &self.avalanche_sizes {
            let s_log = (s as f64).ln();
            let bin = ((s_log - log_min) / bin_width).floor() as isize;
            if bin >= 0 && (bin as usize) < n_bins {
                counts[bin as usize] += 1;
            }
        }
        // 最小二乘 log-log 拟合
        let mut n_data = 0;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        for k in 0..n_bins {
            if counts[k] > 0 {
                let x = bin_centers[k].ln();
                let y = (counts[k] as f64).ln();
                sx += x;
                sy += y;
                sxx += x * x;
                sxy += x * y;
                n_data += 1;
            }
        }
        if n_data < 3 {
            return None;
        }
        let n_data = n_data as f64;
        let denom = n_data * sxx - sx * sx;
        if denom.abs() < 1e-10 {
            return None;
        }
        let slope = (n_data * sxy - sx * sy) / denom;
        Some(-slope) // tau = -slope
    }
}

struct SandRng {
    state: u64,
}

impl SandRng {
    fn new(seed: u64) -> Self {
        SandRng {
            state: if seed == 0 {
                0xdeadbeefcafebabe
            } else {
                seed
            },
        }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = SandpileConfig::default();
        assert_eq!(cfg.n, 32);
        assert_eq!(cfg.z_critical, 4);
        assert_eq!(cfg.n_cells(), 1024);
    }

    #[test]
    fn test_solver_creation() {
        let s = SandpileSolver::new(SandpileConfig::default());
        assert_eq!(s.heights.len(), 1024);
        assert_eq!(s.n_drops, 0);
        assert!(s.is_stable());
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.initialize_zero();
        assert!(s.heights.iter().all(|&h| h == 0));
        assert!(s.is_stable());
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.initialize_uniform(3);
        assert!(s.heights.iter().all(|&h| h == 3));
        assert!(s.is_stable());
    }

    #[test]
    fn test_initialize_random_bounded() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.initialize_random(3, 42);
        assert!(s.max_height() <= 3);
    }

    #[test]
    fn test_single_drop_no_avalanche() {
        // 全零态下加一粒沙, z=1 < 4, 无雪崩
        let mut s = SandpileSolver::new(SandpileConfig::default());
        let stats = s.drop_at(5, 5);
        assert_eq!(stats.size, 0);
        assert_eq!(s.heights[5 * 32 + 5], 1);
        assert!(s.is_stable());
    }

    #[test]
    fn test_drop_at_critical_triggers_avalanche() {
        // 在格点加沙到 z=3, 再加一粒触发 topple
        let mut s = SandpileSolver::new(SandpileConfig::default());
        let idx = 5 * 32 + 5;
        s.heights[idx] = 3;
        let stats = s.drop_at(5, 5);
        // 应 topple 1 次, 4 个邻居各 +1, 中心 z = 0
        assert_eq!(stats.size, 1);
        assert_eq!(s.heights[idx], 0);
        // 邻居: (6,5), (4,5), (5,6), (5,4) 应为 1
        assert_eq!(s.heights[5 * 32 + 6], 1);
        assert_eq!(s.heights[5 * 32 + 4], 1);
        assert_eq!(s.heights[6 * 32 + 5], 1);
        assert_eq!(s.heights[4 * 32 + 5], 1);
        assert!(s.is_stable());
    }

    #[test]
    fn test_avalanche_propagation() {
        // 构造连锁反应: 中心格点 z=3, 邻居 z=3, 加一粒沙应连锁 topple
        let cfg = SandpileConfig {
            n: 5,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        // 中心 (2,2) z=3, 邻居们 z=3
        s.heights[2 * 5 + 2] = 3;
        s.heights[2 * 5 + 3] = 3;
        s.heights[2 * 5 + 1] = 3;
        s.heights[3 * 5 + 2] = 3;
        s.heights[1 * 5 + 2] = 3;
        let stats = s.drop_at(2, 2);
        // 至少 5 次 topple (中心 + 4 邻居)
        assert!(stats.size >= 5, "expected cascade size >=5, got {}", stats.size);
        assert!(s.is_stable(), "should relax to stable state");
    }

    #[test]
    fn test_open_boundary() {
        // 边界格点 topple 应让沙粒消失
        let cfg = SandpileConfig {
            n: 3,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        // 角格点 (0,0) z=3, 加一粒触发 topple
        s.heights[0] = 3;
        let stats = s.drop_at(0, 0);
        // z=0 在中心, 4 个邻居中 2 个在界内 (1,0)(0,1), 2 个在界外
        assert_eq!(stats.size, 1);
        assert!(stats.touched_boundary, "should touch boundary");
        assert_eq!(s.heights[0], 0);
        assert_eq!(s.heights[1], 1); // (1,0)
        assert_eq!(s.heights[3], 1); // (0,1)
        // 总沙粒数: 1 (drop) - 2 (out) = -1 (实际损失 2, 因为加了 1 然后流出 2)
        // 实际: 中心 z 从 3+1=4 减到 0 (-4), 邻居们 +1 +1 (+2), 流出 2
        // 净变化: -4 + 2 = -2, 但因为加了一粒, 所以总损失 1
    }

    #[test]
    fn test_drop_at_corner() {
        let cfg = SandpileConfig {
            n: 4,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.heights[0] = 3; // 角
        let stats = s.drop_at(0, 0);
        assert_eq!(stats.size, 1);
        assert!(stats.touched_boundary);
    }

    #[test]
    fn test_drop_random_advances() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.drop_random(42);
        assert_eq!(s.n_drops, 1);
        s.drop_random(43);
        assert_eq!(s.n_drops, 2);
    }

    #[test]
    fn test_run_multiple_drops() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.run(100, 42);
        assert_eq!(s.n_drops, 100);
        assert!(s.is_stable(), "should be stable after run");
        assert!(!s.has_nan_heights());
    }

    #[test]
    fn test_long_run_no_nan_no_blowup() {
        let cfg = SandpileConfig {
            n: 16,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.run(2000, 123);
        assert!(s.is_stable());
        // 在 SOC 态, 最大高度应被 z_critical 限制 (但可能短暂超过)
        assert!(s.max_height() < 10, "max height too large: {}", s.max_height());
    }

    #[test]
    fn test_avalanche_stats_accumulate() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        // 触发若干雪崩
        for i in 0..10 {
            for j in 0..10 {
                s.heights[j * 32 + i] = 3;
            }
        }
        s.drop_at(5, 5);
        assert!(s.total_avalanches >= 1);
        assert!(s.total_topples >= 1);
        assert!(s.max_avalanche_size >= 1);
    }

    #[test]
    fn test_avalanche_size_distribution() {
        // 长时间运行后应有雪崩大小分布
        let cfg = SandpileConfig {
            n: 16,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.run(2000, 999);
        assert!(!s.avalanche_sizes.is_empty(), "should have avalanches");
        // 雪崩大小应分布广泛
        let max_s = s.avalanche_sizes.iter().max().copied().unwrap_or(0);
        let min_s = s.avalanche_sizes.iter().min().copied().unwrap_or(0);
        assert!(min_s == 0 || min_s == 1, "min avalanche: {}", min_s);
        assert!(max_s >= 10, "max avalanche: {}", max_s);
    }

    #[test]
    fn test_avalanche_exponent_estimate() {
        // 长时间运行后估计幂律指数 (BTW 2D tau ~1.0)
        let cfg = SandpileConfig {
            n: 16,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.run(3000, 42);
        let tau = s.avalanche_exponent();
        // 应能估计出一个正指数 (样本有限, 接受较宽范围)
        if let Some(t) = tau {
            assert!(
                t > 0.0 && t < 4.0,
                "avalanche exponent out of expected range: {}",
                t
            );
        }
        // 否则样本太少, 跳过
    }

    #[test]
    fn test_is_stable_after_relaxation() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.run(500, 42);
        assert!(s.is_stable());
    }

    #[test]
    fn test_mean_height_grows_to_critical() {
        // 长时间运行后平均高度应接近临界值
        let cfg = SandpileConfig {
            n: 16,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.run(2000, 42);
        let mean = s.mean_height();
        // SOC 态下平均高度应接近 (但不超) z_critical - 1
        // BTW 2D 渐近平均 ~ 2.1
        assert!(
            mean > 1.0 && mean < 4.0,
            "mean height unexpected: {}",
            mean
        );
    }

    #[test]
    fn test_idx_boundary() {
        let s = SandpileSolver::new(SandpileConfig {
            n: 5,
            z_critical: 4,
        });
        assert!(s.idx(-1, 0).is_none());
        assert!(s.idx(5, 0).is_none());
        assert!(s.idx(0, -1).is_none());
        assert!(s.idx(0, 5).is_none());
        assert!(s.idx(0, 0).is_some());
        assert!(s.idx(4, 4).is_some());
    }

    #[test]
    fn test_z_critical_flexible() {
        // 不同临界值都应工作 (z_critical >= 4, 因 2D von Neumann 有 4 邻居)
        for zc in [4u32, 5, 6, 8] {
            let cfg = SandpileConfig {
                n: 8,
                z_critical: zc,
            };
            let mut s = SandpileSolver::new(cfg);
            s.run(200, 42);
            assert!(s.is_stable(), "stable for zc={}", zc);
        }
    }

    #[test]
    fn test_grid_size_flexible() {
        for n in [4, 8, 16, 32] {
            let cfg = SandpileConfig {
                n,
                z_critical: 4,
            };
            let mut s = SandpileSolver::new(cfg);
            s.run(200, 42);
            assert!(s.is_stable(), "stable for n={}", n);
            assert_eq!(s.heights.len(), n * n);
        }
    }

    #[test]
    fn test_max_min_height() {
        let mut s = SandpileSolver::new(SandpileConfig::default());
        s.initialize_uniform(2);
        assert_eq!(s.max_height(), 2);
        assert_eq!(s.min_height(), 2);
    }

    #[test]
    fn test_avalanche_duration_positive() {
        let mut s = SandpileSolver::new(SandpileConfig {
            n: 8,
            z_critical: 4,
        });
        // 触发雪崩
        for i in 0..8 {
            for j in 0..8 {
                s.heights[j * 8 + i] = 3;
            }
        }
        let stats = s.drop_at(4, 4);
        if stats.size > 0 {
            assert!(stats.duration > 0);
        }
    }

    #[test]
    fn test_avalanche_area_bounded() {
        let cfg = SandpileConfig {
            n: 8,
            z_critical: 4,
        };
        let mut s = SandpileSolver::new(cfg);
        s.run(500, 42);
        // 单次雪崩面积不应超过总格点数
        if let Some(stats) = s.avalanche_sizes.last().map(|_| &s.last_avalanche) {
            assert!(stats.area <= (8 * 8) as u64);
        }
    }
}

impl SandpileSolver {
    fn has_nan_heights(&self) -> bool {
        // u32 不会 NaN, 此函数仅为 API 一致性
        false
    }
}
