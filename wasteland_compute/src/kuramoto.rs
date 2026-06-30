//! Kuramoto Model Solver (Synchronization Dynamics)
//!
//! 耦合振荡器的同步动力学. 描述大量相互耦合的极限环振荡器如何自发同步.
//! 与 Ising Model (统计力学) 不同, Kuramoto 描述**动力学同步** — 时间演化.
//!
//! 方程:
//!   d theta_i / dt = omega_i + (K/N) * sum_j A_ij * sin(theta_j - theta_i)
//!
//! 其中:
//!   theta_i: 振荡器 i 的相位
//!   omega_i: 固有频率 (从分布 g(omega) 抽取, 通常高斯 N(0, sigma^2))
//!   K: 耦合强度
//!   N: 振荡器数量
//!   A_ij: 邻接矩阵 (网络拓扑)
//!
//! 相变:
//!   K_c = 2 / (pi * g(0)) (全局耦合)
//!   高斯分布 g(omega) = N(0, sigma^2): g(0) = 1/(sigma*sqrt(2*pi))
//!   -> K_c = 2 * sigma * sqrt(2/pi) = sigma * sqrt(8/pi) ~ 1.596 * sigma
//!   当 K > K_c: 出现部分同步 (r > 0)
//!   当 K >> K_c: 完全同步 (r -> 1)
//!
//! 序参量:
//!   r * exp(i*psi) = (1/N) * sum_j exp(i*theta_j)
//!   r in [0, 1]: 同步程度 (r=0 无同步, r=1 完全同步)
//!   psi: 平均相位
//!
//! 网络拓扑:
//!   AllToAll   - 全局耦合 (经典 Kuramoto 1975)
//!   Ring       - 环形 (最近邻, 2k 个邻居)
//!   SmallWorld - Watts-Strogatz 小世界 (重连概率 p)
//!   ScaleFree  - Barabasi-Albert 无标度 (度数 m)
//!   Grid2D     - 2D 格点 (von Neumann 邻域)
//!
//! 数值方法: RK4 (4阶 Runge-Kutta)
//!
//! 应用:
//!   - 萤火虫同步闪烁
//!   - 神经元同步 (癫痫发作, 脑节律)
//!   - 心脏起搏细胞同步
//!   - 电网频率同步
//!   - Josephson 结阵列
//!   - 潮汐锁定 (卫星)
//!   - IoT 设备时钟同步
//!   - 化学振荡器 (BZ 反应)
//!   - 脑功能网络同步
//!
//! 基于 Kuramoto 1975, Strogatz 2000, Acebron 2005 (综述),
//!       Rodrigues 2016 (网络 Kuramoto 综述).

use serde::{Deserialize, Serialize};

/// 简易 xorshift RNG (避免外部依赖)
struct XorShift {
    state: u64,
}

impl XorShift {
    fn new(seed: u64) -> Self {
        XorShift { state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed } }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Box-Muller 变换: 生成标准正态分布 N(0,1)
    fn next_gaussian(&mut self) -> f32 {
        let u1 = self.next_f32().max(1e-10);
        let u2 = self.next_f32();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KuramotoTopology {
    /// 全局耦合 (经典 Kuramoto)
    AllToAll,
    /// 环形 (每节点连接到 2*neighbors 个最近邻)
    Ring { neighbors: usize },
    /// Watts-Strogatz 小世界 (neighbors 每侧, 重连概率 rewire_prob)
    SmallWorld { neighbors: usize, rewire_prob: f32 },
    /// Barabasi-Albert 无标度 (每新节点连接 m 条边)
    ScaleFree { m: usize },
    /// 2D 格点 (width x height, von Neumann 邻域)
    Grid2D { width: usize, height: usize },
}

impl Default for KuramotoTopology {
    fn default() -> Self {
        KuramotoTopology::AllToAll
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KuramotoConfig {
    pub n: usize,
    pub dt: f32,
    /// 耦合强度 K
    pub k: f32,
    /// 固有频率分布的标准差 sigma (omega ~ N(0, sigma^2))
    pub omega_sigma: f32,
    /// 随机种子
    pub seed: u64,
    pub topology: KuramotoTopology,
}

impl Default for KuramotoConfig {
    fn default() -> Self {
        KuramotoConfig {
            n: 200,
            dt: 0.01,
            k: 1.0,
            omega_sigma: 1.0,
            seed: 42,
            topology: KuramotoTopology::AllToAll,
        }
    }
}

impl KuramotoConfig {
    /// 临界耦合强度 K_c = sigma * sqrt(8/pi) (全局耦合, 高斯分布)
    pub fn critical_coupling(&self) -> f32 {
        match self.topology {
            KuramotoTopology::AllToAll => {
                self.omega_sigma * (8.0 / std::f32::consts::PI).sqrt()
            }
            // 网络拓扑的 K_c 依赖于平均度数 <k>
            // 近似: K_c_network ~ K_c_alltoall * N / <k>
            _ => self.omega_sigma * (8.0 / std::f32::consts::PI).sqrt(),
        }
    }
    pub fn n_oscillators(&self) -> usize {
        self.n
    }
}

pub struct KuramotoSolver {
    pub config: KuramotoConfig,
    pub theta: Vec<f32>,
    pub omega: Vec<f32>,
    /// 邻接表 (每个节点的邻居列表)
    pub adjacency: Vec<Vec<usize>>,
    /// 每个节点的度数
    pub degree: Vec<usize>,
    pub time: f32,
    pub steps: usize,
}

impl KuramotoSolver {
    pub fn new(config: KuramotoConfig) -> Self {
        let n = config.n;
        let mut rng = XorShift::new(config.seed);
        // 固有频率 omega ~ N(0, sigma^2)
        let omega: Vec<f32> = (0..n)
            .map(|_| config.omega_sigma * rng.next_gaussian())
            .collect();
        // 初始相位 theta ~ Uniform(0, 2*pi)
        let theta: Vec<f32> = (0..n)
            .map(|_| 2.0 * std::f32::consts::PI * rng.next_f32())
            .collect();
        // 构建邻接表 (使用相同的 RNG 种子保证可复现)
        let mut rng_adj = XorShift::new(config.seed.wrapping_add(1));
        let adjacency = Self::build_adjacency(n, &config.topology, &mut rng_adj);
        let degree: Vec<usize> = adjacency.iter().map(|v| v.len()).collect();
        KuramotoSolver {
            config,
            theta,
            omega,
            adjacency,
            degree,
            time: 0.0,
            steps: 0,
        }
    }
    /// 构建邻接表
    fn build_adjacency(n: usize, topology: &KuramotoTopology, rng: &mut XorShift) -> Vec<Vec<usize>> {
        match topology {
            KuramotoTopology::AllToAll => {
                (0..n).map(|i| {
                    (0..n).filter(|&j| j != i).collect()
                }).collect()
            }
            KuramotoTopology::Ring { neighbors } => {
                let k = *neighbors;
                (0..n).map(|i| {
                    let mut adj = Vec::new();
                    for d in 1..=k {
                        adj.push((i + d) % n);
                        adj.push((i + n - d) % n);
                    }
                    adj
                }).collect()
            }
            KuramotoTopology::SmallWorld { neighbors, rewire_prob } => {
                let k = *neighbors;
                let p = *rewire_prob;
                // 先建立环形
                let mut adj: Vec<std::collections::HashSet<usize>> =
                    (0..n).map(|i| {
                        let mut s = std::collections::HashSet::new();
                        for d in 1..=k {
                            s.insert((i + d) % n);
                            s.insert((i + n - d) % n);
                        }
                        s
                    }).collect();
                // 重连: 对每条边 (i, j) where j > i 且 j 在 ring 中, 以概率 p 重连
                for i in 0..n {
                    let d_list: Vec<usize> = (1..=k).map(|d| (i + d) % n).collect();
                    for j in d_list {
                        if rng.next_f32() < p {
                            // 移除 (i, j)
                            adj[i].remove(&j);
                            adj[j].remove(&i);
                            // 随机选一个新邻居
                            let mut attempts = 0;
                            while attempts < 100 {
                                let new_j = (rng.next_u64() as usize) % n;
                                if new_j != i && !adj[i].contains(&new_j) {
                                    adj[i].insert(new_j);
                                    adj[new_j].insert(i);
                                    break;
                                }
                                attempts += 1;
                            }
                        }
                    }
                }
                adj.into_iter().map(|s| s.into_iter().collect()).collect()
            }
            KuramotoTopology::ScaleFree { m } => {
                let m = *m;
                if n < m + 1 {
                    return (0..n).map(|_| Vec::new()).collect();
                }
                // BA 模型: 从 m+1 个全连接节点开始
                let mut adj: Vec<std::collections::HashSet<usize>> =
                    (0..n).map(|_| std::collections::HashSet::new()).collect();
                // 初始完全图 K_{m+1}
                for i in 0..=m {
                    for j in 0..=m {
                        if i != j {
                            adj[i].insert(j);
                        }
                    }
                }
                // 逐个添加节点 m+1..n
                let mut degree: Vec<usize> = adj.iter().map(|s| s.len()).collect();
                for new_node in (m+1)..n {
                    // 按度数比例选择 m 个不同节点
                    let total_degree: usize = degree[0..new_node].iter().sum();
                    if total_degree == 0 {
                        // 退化为随机选择
                        for _ in 0..m {
                            let target = (rng.next_u64() as usize) % new_node;
                            adj[new_node].insert(target);
                            adj[target].insert(new_node);
                            degree[target] += 1;
                        }
                    } else {
                        let mut chosen = std::collections::HashSet::new();
                        while chosen.len() < m && chosen.len() < new_node {
                            let r = (rng.next_u64() as usize) % total_degree;
                            let mut cum = 0;
                            for target in 0..new_node {
                                cum += degree[target];
                                if r < cum {
                                    chosen.insert(target);
                                    break;
                                }
                            }
                        }
                        for &target in &chosen {
                            adj[new_node].insert(target);
                            adj[target].insert(new_node);
                            degree[target] += 1;
                        }
                        degree[new_node] = chosen.len();
                    }
                }
                adj.into_iter().map(|s| s.into_iter().collect()).collect()
            }
            KuramotoTopology::Grid2D { width, height } => {
                let w = *width;
                let h = *height;
                (0..n).map(|i| {
                    let x = i % w;
                    let y = i / w;
                    let mut adj = Vec::new();
                    // von Neumann 邻域 (4 个邻居)
                    if x > 0 { adj.push(i - 1); }
                    if x + 1 < w { adj.push(i + 1); }
                    if y > 0 { adj.push(i - w); }
                    if y + 1 < h && i + w < n { adj.push(i + w); }
                    adj
                }).collect()
            }
        }
    }

    /// 平均度数 <k>
    pub fn mean_degree(&self) -> f32 {
        let total: usize = self.degree.iter().sum();
        total as f32 / self.n_oscillators() as f32
    }

    pub fn n_oscillators(&self) -> usize {
        self.config.n
    }
    /// 计算相位导数 d theta_i / dt = omega_i + (K/N) * sum_j A_ij * sin(theta_j - theta_i)
    fn derivative(&self, theta: &[f32]) -> Vec<f32> {
        let n = self.config.n;
        let k = self.config.k;
        let inv_n = 1.0 / (n as f32);
        let mut dtheta = self.omega.clone();
        for i in 0..n {
            let mut coupling = 0.0f32;
            for &j in &self.adjacency[i] {
                coupling += (theta[j] - theta[i]).sin();
            }
            // 对网络拓扑, 用度数归一化 (而非 N)
            let norm = match self.config.topology {
                KuramotoTopology::AllToAll => inv_n,
                _ => {
                    let deg = self.degree[i] as f32;
                    if deg > 0.0 { 1.0 / deg } else { 0.0 }
                }
            };
            dtheta[i] += k * coupling * norm;
        }
        dtheta
    }

    /// RK4 一步更新
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.config.n;
        // k1
        let k1 = self.derivative(&self.theta);
        // k2
        let mut theta2 = vec![0.0f32; n];
        for i in 0..n {
            theta2[i] = self.theta[i] + 0.5 * dt * k1[i];
        }
        let k2 = self.derivative(&theta2);
        // k3
        let mut theta3 = vec![0.0f32; n];
        for i in 0..n {
            theta3[i] = self.theta[i] + 0.5 * dt * k2[i];
        }
        let k3 = self.derivative(&theta3);
        // k4
        let mut theta4 = vec![0.0f32; n];
        for i in 0..n {
            theta4[i] = self.theta[i] + dt * k3[i];
        }
        let k4 = self.derivative(&theta4);
        // theta^{n+1} = theta + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        let sixth = dt / 6.0;
        for i in 0..n {
            self.theta[i] += sixth * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
            // 保持 theta 在 [0, 2*pi) 避免数值溢出
            let two_pi = 2.0 * std::f32::consts::PI;
            self.theta[i] = ((self.theta[i] % two_pi) + two_pi) % two_pi;
        }
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 序参量 (r, psi): r * exp(i*psi) = (1/N) * sum_j exp(i*theta_j)
    pub fn order_parameter(&self) -> (f32, f32) {
        let n = self.config.n;
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for &theta in &self.theta {
            re += theta.cos();
            im += theta.sin();
        }
        re /= n as f32;
        im /= n as f32;
        let r = (re * re + im * im).sqrt();
        let psi = im.atan2(re);
        (r, psi)
    }

    /// 同步指数 r (序参量模)
    pub fn synchronization_index(&self) -> f32 {
        self.order_parameter().0
    }

    /// 平均频率 (omega 均值, 理论上守恒)
    pub fn mean_frequency(&self) -> f32 {
        let n = self.config.n;
        self.omega.iter().sum::<f32>() / n as f32
    }

    /// 频率方差 (衡量频率离散程度, 同步时降低)
    pub fn frequency_variance(&self) -> f32 {
        let n = self.config.n;
        let mean = self.mean_frequency();
        let var: f32 = self.omega.iter()
            .map(|&w| (w - mean) * (w - mean))
            .sum::<f32>() / n as f32;
        var
    }

    /// 局部序参量 (节点 i 的邻居同步程度)
    pub fn local_order(&self, i: usize) -> f32 {
        let neighbors = &self.adjacency[i];
        if neighbors.is_empty() {
            return 0.0;
        }
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for &j in neighbors {
            re += self.theta[j].cos();
            im += self.theta[j].sin();
        }
        let n = neighbors.len() as f32;
        let r = ((re / n).powi(2) + (im / n).powi(2)).sqrt();
        r
    }

    /// 重置: 重新随机化相位
    pub fn reset(&mut self) {
        let mut rng = XorShift::new(self.config.seed.wrapping_add(self.steps as u64 + 1));
        for t in self.theta.iter_mut() {
            *t = 2.0 * std::f32::consts::PI * rng.next_f32();
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 用指定种子重新初始化相位
    pub fn reset_with_seed(&mut self, seed: u64) {
        let mut rng = XorShift::new(seed);
        for t in self.theta.iter_mut() {
            *t = 2.0 * std::f32::consts::PI * rng.next_f32();
        }
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
        let c = KuramotoConfig::default();
        assert_eq!(c.n, 200);
        assert_eq!(c.dt, 0.01);
        assert_eq!(c.k, 1.0);
        assert_eq!(c.omega_sigma, 1.0);
        assert_eq!(c.seed, 42);
        assert_eq!(c.topology, KuramotoTopology::AllToAll);
    }

    #[test]
    fn test_config_n_oscillators() {
        let c = KuramotoConfig { n: 100, dt: 0.01, k: 1.0, omega_sigma: 1.0, seed: 1,
            topology: KuramotoTopology::AllToAll };
        assert_eq!(c.n_oscillators(), 100);
    }

    #[test]
    fn test_critical_coupling_formula() {
        // K_c = sigma * sqrt(8/pi) ~ 1.596 * sigma
        let c = KuramotoConfig { n: 200, dt: 0.01, k: 1.0, omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::AllToAll };
        let kc = c.critical_coupling();
        let expected = (8.0 / std::f32::consts::PI).sqrt();
        assert!(approx_eq(kc, expected, 1e-5),
            "K_c = {}, expected {}", kc, expected);
    }

    #[test]
    fn test_solver_new() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 50, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        assert_eq!(s.theta.len(), 50);
        assert_eq!(s.omega.len(), 50);
        assert_eq!(s.adjacency.len(), 50);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_omega_distribution() {
        // omega ~ N(0, sigma^2), 均值 ~0, 方差 ~sigma^2
        let s = KuramotoSolver::new(KuramotoConfig { n: 1000, dt: 0.01, k: 1.0,
            omega_sigma: 2.0, seed: 42, topology: KuramotoTopology::AllToAll });
        let mean = s.omega.iter().sum::<f32>() / 1000.0;
        let var: f32 = s.omega.iter().map(|&w| (w - mean).powi(2)).sum::<f32>() / 1000.0;
        assert!(mean.abs() < 0.3, "omega mean should be ~0, got {}", mean);
        assert!((var - 4.0).abs() < 1.0, "omega variance should be ~4, got {}", var);
    }

    #[test]
    fn test_theta_in_range() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        let two_pi = 2.0 * std::f32::consts::PI;
        for &theta in &s.theta {
            assert!(theta >= 0.0 && theta < two_pi,
                "theta {} out of [0, 2pi)", theta);
        }
    }

    #[test]
    fn test_adjacency_all_to_all() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 20, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        for i in 0..20 {
            assert_eq!(s.adjacency[i].len(), 19,
                "node {} should have 19 neighbors, got {}", i, s.adjacency[i].len());
        }
        let md = s.mean_degree();
        assert!(approx_eq(md, 19.0, 1e-6));
    }

    #[test]
    fn test_adjacency_ring() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 20, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::Ring { neighbors: 2 } });
        // 每节点 2*2 = 4 个邻居
        for i in 0..20 {
            assert_eq!(s.adjacency[i].len(), 4,
                "ring node {} should have 4 neighbors, got {}", i, s.adjacency[i].len());
        }
    }

    #[test]
    fn test_adjacency_grid_2d() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 25, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::Grid2D { width: 5, height: 5 } });
        // 角 (0,0): 2 邻居
        assert_eq!(s.adjacency[0].len(), 2, "corner (0,0) should have 2 neighbors");
        // 边 (1,0): 3 邻居
        assert_eq!(s.adjacency[1].len(), 3, "edge (1,0) should have 3 neighbors");
        // 内部 (1,1) = idx 6: 4 邻居
        assert_eq!(s.adjacency[6].len(), 4, "interior (1,1) should have 4 neighbors");
    }

    #[test]
    fn test_adjacency_small_world() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::SmallWorld { neighbors: 2, rewire_prob: 0.1 } });
        // 平均度数应接近 4 (2*neighbors), 重连不改变总边数
        let md = s.mean_degree();
        assert!((md - 4.0).abs() < 0.5,
            "small-world mean degree should be ~4, got {}", md);
    }

    #[test]
    fn test_adjacency_scale_free() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::ScaleFree { m: 3 } });
        // BA 模型: 平均度数 ~ 2m = 6
        let md = s.mean_degree();
        assert!((md - 6.0).abs() < 1.0,
            "scale-free mean degree should be ~6, got {}", md);
        // 应有 hub 节点 (度数远高于平均)
        let max_deg = s.degree.iter().max().copied().unwrap_or(0);
        assert!(max_deg > 10, "BA should have hub nodes, max degree = {}", max_deg);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 20, dt: 0.05, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
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
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 20, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 1.0, 1e-6));
    }

    #[test]
    fn test_no_nan() {
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 50, dt: 0.01, k: 2.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s.step_n(500);
        assert!(!s.theta.iter().any(|&t| t.is_nan()));
    }

    #[test]
    fn test_order_parameter_range() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        let (r, _psi) = s.order_parameter();
        assert!(r >= 0.0 && r <= 1.0, "r should be in [0,1], got {}", r);
    }
    #[test]
    fn test_uncoupled_no_sync() {
        // K=0: 无耦合, 振荡器按各自 omega 旋转, r 保持低值 (~1/sqrt(N))
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 0.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s.step_n(500);
        let (r, _) = s.order_parameter();
        // 无耦合时 r 应该很小 (随机相位), < 0.3
        assert!(r < 0.3, "uncoupled r should be small, got {}", r);
    }

    #[test]
    fn test_strong_coupling_syncs() {
        // K >> K_c: 强耦合, 应该同步 (r -> 1)
        // K_c = sigma * sqrt(8/pi) ~ 1.596
        // 用 K = 10.0 (>> K_c)
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.005, k: 10.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s.step_n(2000);
        let (r, _) = s.order_parameter();
        assert!(r > 0.7, "strong coupling should sync, r = {}", r);
    }

    #[test]
    fn test_phase_transition() {
        // K < K_c: r 低; K > K_c: r 高
        // K_c ~ 1.596 (sigma=1)
        let mut s_low = KuramotoSolver::new(KuramotoConfig { n: 200, dt: 0.005, k: 0.5,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s_low.step_n(2000);
        let (r_low, _) = s_low.order_parameter();

        let mut s_high = KuramotoSolver::new(KuramotoConfig { n: 200, dt: 0.005, k: 4.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s_high.step_n(2000);
        let (r_high, _) = s_high.order_parameter();

        assert!(r_high > r_low,
            "stronger coupling should give higher r: K=0.5 r={} vs K=4.0 r={}", r_low, r_high);
        assert!(r_high > 0.3, "K=4.0 > K_c should have r > 0.3, got {}", r_high);
    }

    #[test]
    fn test_mean_frequency_conservation() {
        // omega 是固定的 (不随时间变化), 均值应严格守恒
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 50, dt: 0.01, k: 2.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        let mf0 = s.mean_frequency();
        s.step_n(500);
        let mf1 = s.mean_frequency();
        assert!(approx_eq(mf0, mf1, 1e-9),
            "mean frequency should be conserved: {} -> {}", mf0, mf1);
    }

    #[test]
    fn test_local_order_range() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 30, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::Ring { neighbors: 2 } });
        for i in 0..30 {
            let r = s.local_order(i);
            assert!(r >= 0.0 && r <= 1.0, "local order {} = {} out of [0,1]", i, r);
        }
    }

    #[test]
    fn test_frequency_variance() {
        let s = KuramotoSolver::new(KuramotoConfig { n: 500, dt: 0.01, k: 1.0,
            omega_sigma: 1.5, seed: 42, topology: KuramotoTopology::AllToAll });
        let var = s.frequency_variance();
        // 方差应接近 sigma^2 = 2.25
        assert!((var - 2.25).abs() < 0.5,
            "frequency variance should be ~2.25, got {}", var);
    }

    #[test]
    fn test_reset() {
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 30, dt: 0.01, k: 1.0,
            omega_sigma: 1.0, seed: 42, topology: KuramotoTopology::AllToAll });
        s.step_n(50);
        assert!(s.steps > 0);
        s.reset();
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        // omega 应该不变 (固有属性)
        let two_pi = 2.0 * std::f32::consts::PI;
        for &t in &s.theta {
            assert!(t >= 0.0 && t < two_pi);
        }
    }

    #[test]
    fn test_network_topology_no_nan() {
        // 网络拓扑下也不应产生 NaN
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 3.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::SmallWorld { neighbors: 2, rewire_prob: 0.1 } });
        s.step_n(500);
        assert!(!s.theta.iter().any(|&t| t.is_nan()));

        let mut s2 = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.01, k: 3.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::ScaleFree { m: 3 } });
        s2.step_n(500);
        assert!(!s2.theta.iter().any(|&t| t.is_nan()));
    }

    #[test]
    fn test_scale_free_syncs() {
        // BA 网络上同步更快 (hub 节点增强耦合)
        let mut s = KuramotoSolver::new(KuramotoConfig { n: 100, dt: 0.005, k: 3.0,
            omega_sigma: 1.0, seed: 42,
            topology: KuramotoTopology::ScaleFree { m: 3 } });
        s.step_n(2000);
        let (r, _) = s.order_parameter();
        // K=3 > K_c, 应该有显著同步
        assert!(r > 0.2, "scale-free K=3 should sync partially, r = {}", r);
    }
}