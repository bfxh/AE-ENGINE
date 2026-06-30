//! Ising Model (Metropolis Monte Carlo)
//!
//! 统计力学经典模型. 格点上自旋 s_i in {+1, -1},
//! 哈密顿量 H = -J * sum_{<i,j>} s_i s_j - h * sum_i s_i
//!
//! Metropolis 算法:
//!   1. 随机选格点 i
//!   2. dE = 2 * s_i * (J * sum_{j in nn(i)} s_j + h)
//!   3. 若 dE < 0 或 rand() < exp(-dE / (k_B T)), 翻转 s_i
//!
//! 2D Ising 相变温度 (Onsager 解析解):
//!   T_c = 2J / (k_B * ln(1 + sqrt(2)))
//!
//! 观测量:
//!   磁化 M = |sum s_i| / N
//!   能量 E = <H> / N
//!   比热 C = (<E^2> - <E>^2) / (N * k_B * T^2)
//!   磁化率 chi = (<M^2> - <M>^2) / (k_B * T)
//!
//! 应用: 相变, 临界现象, 统计物理, 晶格模型, 蒙特卡洛方法.
//!
//! 基于 Ising 1925, Onsager 1944, Metropolis 1953.

use serde::{Deserialize, Serialize};

pub const K_B: f32 = 1.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LatticeType {
    /// 2D 方格 (4 邻居)
    Square2D,
    /// 3D 简立方 (6 邻居)
    SimpleCubic3D,
}

impl LatticeType {
    pub fn dims(&self) -> usize {
        match self {
            LatticeType::Square2D => 2,
            LatticeType::SimpleCubic3D => 3,
        }
    }
    pub fn coordination(&self) -> usize {
        match self {
            LatticeType::Square2D => 4,
            LatticeType::SimpleCubic3D => 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsingConfig {
    pub lattice: LatticeType,
    /// 每边格点数
    pub size: usize,
    /// 交换耦合常数 J (>0 铁磁, <0 反铁磁)
    pub j: f32,
    /// 外磁场 h
    pub h: f32,
    pub temperature: f32,
    pub seed: u64,
}

impl Default for IsingConfig {
    fn default() -> Self {
        IsingConfig {
            lattice: LatticeType::Square2D,
            size: 16,
            j: 1.0,
            h: 0.0,
            temperature: 2.0,
            seed: 42,
        }
    }
}

impl IsingConfig {
    pub fn n_sites(&self) -> usize {
        match self.lattice {
            LatticeType::Square2D => self.size * self.size,
            LatticeType::SimpleCubic3D => self.size * self.size * self.size,
        }
    }
    /// Onsager 临界温度 (仅 2D 方格有解析解)
    pub fn critical_temperature(&self) -> f32 {
        match self.lattice {
            LatticeType::Square2D => 2.0 * self.j / (1.0 + 2.0f32.sqrt()).ln(),
            LatticeType::SimpleCubic3D => 4.5115 * self.j, // 数值结果
        }
    }
}

pub struct IsingSolver {
    pub config: IsingConfig,
    pub spins: Vec<i8>,
    pub steps: usize,
    rng_state: u64,
    pub energy_sum: f32,
    pub energy_sq_sum: f32,
    pub mag_sum: f32,
    pub mag_sq_sum: f32,
    pub n_samples: usize,
}

impl IsingSolver {
    pub fn new(config: IsingConfig) -> Self {
        let n = config.n_sites();
        let seed = if config.seed == 0 { 0x1234_5678_9ABC_DEF0 } else { config.seed };
        IsingSolver {
            config,
            spins: vec![1; n],
            steps: 0,
            rng_state: seed,
            energy_sum: 0.0,
            energy_sq_sum: 0.0,
            mag_sum: 0.0,
            mag_sq_sum: 0.0,
            n_samples: 0,
        }
    }

    fn rand(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    fn rand_f32(&mut self) -> f32 {
        (self.rand() >> 11) as f32 / (1u64 << 53) as f32
    }

    pub fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        let s = self.config.size;
        match self.config.lattice {
            LatticeType::Square2D => x + s * y,
            LatticeType::SimpleCubic3D => x + s * (y + s * z),
        }
    }

    /// 周期性包裹
    fn wrap(i: i32, s: usize) -> usize {
        let m = s as i32;
        (((i % m) + m) % m) as usize
    }

    /// 返回格点 site 的邻居索引
    pub fn neighbors(&self, site: usize) -> Vec<usize> {
        let s = self.config.size;
        match self.config.lattice {
            LatticeType::Square2D => {
                let x = (site % s) as i32;
                let y = (site / s) as i32;
                vec![
                    self.idx(Self::wrap(x + 1, s), y as usize, 0),
                    self.idx(Self::wrap(x - 1, s), y as usize, 0),
                    self.idx(x as usize, Self::wrap(y + 1, s), 0),
                    self.idx(x as usize, Self::wrap(y - 1, s), 0),
                ]
            }
            LatticeType::SimpleCubic3D => {
                let xy = site % (s * s);
                let x = (xy % s) as i32;
                let y = (xy / s) as i32;
                let z = (site / (s * s)) as i32;
                vec![
                    self.idx(Self::wrap(x + 1, s), y as usize, z as usize),
                    self.idx(Self::wrap(x - 1, s), y as usize, z as usize),
                    self.idx(x as usize, Self::wrap(y + 1, s), z as usize),
                    self.idx(x as usize, Self::wrap(y - 1, s), z as usize),
                    self.idx(x as usize, y as usize, Self::wrap(z + 1, s)),
                    self.idx(x as usize, y as usize, Self::wrap(z - 1, s)),
                ]
            }
        }
    }

    /// 随机初始化自旋
    pub fn initialize_random(&mut self) {
        let n = self.spins.len();
        for i in 0..n {
            self.spins[i] = if self.rand_f32() < 0.5 { -1 } else { 1 };
        }
    }

    /// 有序初始化 (全部向上或向下)
    pub fn initialize_ordered(&mut self, up: bool) {
        let v: i8 = if up { 1 } else { -1 };
        for s in self.spins.iter_mut() {
            *s = v;
        }
    }

    /// 单个格点的局部能量贡献 (含场)
    pub fn local_energy(&self, site: usize) -> f32 {
        let s_i = self.spins[site] as f32;
        let mut nn_sum = 0.0f32;
        for &nb in &self.neighbors(site) {
            nn_sum += self.spins[nb] as f32;
        }
        -self.config.j * s_i * nn_sum - self.config.h * s_i
    }

    /// 总能量 (键能 / 2 + 场能)
    pub fn energy(&self) -> f32 {
        let n = self.config.n_sites();
        let mut bond_e = 0.0f32;
        for i in 0..n {
            let s_i = self.spins[i] as f32;
            let mut nn_sum = 0.0f32;
            for &nb in &self.neighbors(i) {
                nn_sum += self.spins[nb] as f32;
            }
            bond_e += -self.config.j * s_i * nn_sum;
        }
        bond_e *= 0.5; // 每个键计算了两次
        // 场能 = -h * sum s_i = -h * N * M
        bond_e - self.config.h * self.magnetization() * n as f32
    }

    /// 磁化 (平均自旋)
    pub fn magnetization(&self) -> f32 {
        let n = self.config.n_sites();
        let mut m: f32 = 0.0;
        for &s in self.spins.iter() {
            m += s as f32;
        }
        m / n as f32
    }

    /// 绝对磁化 |M|
    pub fn abs_magnetization(&self) -> f32 {
        self.magnetization().abs()
    }

    /// 一次 Metropolis 扫描 (N 次尝试)
    pub fn metropolis_step(&mut self) {
        let n = self.config.n_sites();
        let t = self.config.temperature;
        let j = self.config.j;
        let h = self.config.h;
        for _ in 0..n {
            let site = (self.rand() as usize) % n;
            let s_i = self.spins[site] as f32;
            let mut nn_sum = 0.0f32;
            for &nb in self.neighbors(site).iter() {
                nn_sum += self.spins[nb] as f32;
            }
            // dE = E_after - E_before = 2 * s_i * (J * nn_sum + h)
            let dE = 2.0 * s_i * (j * nn_sum + h);
            if dE <= 0.0 || self.rand_f32() < (-dE / (K_B * t)).exp() {
                self.spins[site] = -(self.spins[site]);
            }
        }
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.metropolis_step();
        }
    }

    /// 采样当前构型的观测量 (累加到统计)
    pub fn measure(&mut self) {
        let e = self.energy();
        let m = self.magnetization();
        self.energy_sum += e;
        self.energy_sq_sum += e * e;
        self.mag_sum += m;
        self.mag_sq_sum += m * m;
        self.n_samples += 1;
    }

    /// 平均能量 <E>
    pub fn mean_energy(&self) -> f32 {
        if self.n_samples == 0 {
            0.0
        } else {
            self.energy_sum / self.n_samples as f32
        }
    }

    /// 平均磁化 <M>
    pub fn mean_magnetization(&self) -> f32 {
        if self.n_samples == 0 {
            0.0
        } else {
            self.mag_sum / self.n_samples as f32
        }
    }

    /// 比热 C = (<E^2> - <E>^2) / (N * k_B * T^2)
    pub fn specific_heat(&self) -> f32 {
        if self.n_samples == 0 {
            return 0.0;
        }
        let n = self.config.n_sites() as f32;
        let t = self.config.temperature;
        let e_avg = self.mean_energy();
        let e2_avg = self.energy_sq_sum / self.n_samples as f32;
        (e2_avg - e_avg * e_avg) / (n * K_B * t * t)
    }

    /// 磁化率 chi = (<M^2> - <M>^2) / (k_B * T)
    pub fn susceptibility(&self) -> f32 {
        if self.n_samples == 0 {
            return 0.0;
        }
        let t = self.config.temperature;
        let m_avg = self.mean_magnetization();
        let m2_avg = self.mag_sq_sum / self.n_samples as f32;
        (m2_avg - m_avg * m_avg) / (K_B * t)
    }

    /// 重置统计
    pub fn reset_stats(&mut self) {
        self.energy_sum = 0.0;
        self.energy_sq_sum = 0.0;
        self.mag_sum = 0.0;
        self.mag_sq_sum = 0.0;
        self.n_samples = 0;
    }

    pub fn reset(&mut self) {
        self.initialize_ordered(true);
        self.steps = 0;
        self.reset_stats();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_kb() {
        assert_eq!(K_B, 1.0);
    }

    #[test]
    fn test_lattice_dims() {
        assert_eq!(LatticeType::Square2D.dims(), 2);
        assert_eq!(LatticeType::SimpleCubic3D.dims(), 3);
    }

    #[test]
    fn test_lattice_coordination() {
        assert_eq!(LatticeType::Square2D.coordination(), 4);
        assert_eq!(LatticeType::SimpleCubic3D.coordination(), 6);
    }

    #[test]
    fn test_config_default() {
        let c = IsingConfig::default();
        assert_eq!(c.lattice, LatticeType::Square2D);
        assert_eq!(c.size, 16);
        assert_eq!(c.j, 1.0);
        assert_eq!(c.h, 0.0);
        assert_eq!(c.temperature, 2.0);
    }

    #[test]
    fn test_n_sites_2d() {
        let c = IsingConfig { lattice: LatticeType::Square2D, size: 8, ..Default::default() };
        assert_eq!(c.n_sites(), 64);
    }

    #[test]
    fn test_n_sites_3d() {
        let c = IsingConfig { lattice: LatticeType::SimpleCubic3D, size: 4, ..Default::default() };
        assert_eq!(c.n_sites(), 64);
    }

    #[test]
    fn test_critical_temperature_2d() {
        let c = IsingConfig { lattice: LatticeType::Square2D, j: 1.0, ..Default::default() };
        assert!(approx_eq(c.critical_temperature(), 2.269, 0.01));
    }

    #[test]
    fn test_critical_temperature_3d() {
        let c = IsingConfig { lattice: LatticeType::SimpleCubic3D, j: 1.0, ..Default::default() };
        assert!(approx_eq(c.critical_temperature(), 4.5115, 0.01));
    }

    #[test]
    fn test_solver_new() {
        let s = IsingSolver::new(IsingConfig::default());
        assert_eq!(s.spins.len(), 256);
        assert_eq!(s.steps, 0);
        for &spin in s.spins.iter() {
            assert_eq!(spin, 1);
        }
    }

    #[test]
    fn test_idx_2d() {
        let s = IsingSolver::new(IsingConfig { lattice: LatticeType::Square2D, size: 4, ..Default::default() });
        assert_eq!(s.idx(0, 0, 0), 0);
        assert_eq!(s.idx(1, 0, 0), 1);
        assert_eq!(s.idx(0, 1, 0), 4);
        assert_eq!(s.idx(3, 3, 0), 15);
    }

    #[test]
    fn test_neighbors_2d_count() {
        let s = IsingSolver::new(IsingConfig { lattice: LatticeType::Square2D, size: 4, ..Default::default() });
        assert_eq!(s.neighbors(0).len(), 4);
        assert_eq!(s.neighbors(5).len(), 4);
        assert_eq!(s.neighbors(15).len(), 4);
    }

    #[test]
    fn test_neighbors_2d_periodic() {
        let s = IsingSolver::new(IsingConfig { lattice: LatticeType::Square2D, size: 4, ..Default::default() });
        let nb = s.neighbors(0);
        // (0,0) 邻居: (1,0), (3,0), (0,1), (0,3)
        assert!(nb.contains(&s.idx(1, 0, 0)));
        assert!(nb.contains(&s.idx(3, 0, 0)));
        assert!(nb.contains(&s.idx(0, 1, 0)));
        assert!(nb.contains(&s.idx(0, 3, 0)));
    }

    #[test]
    fn test_neighbors_3d_count() {
        let s = IsingSolver::new(IsingConfig { lattice: LatticeType::SimpleCubic3D, size: 4, ..Default::default() });
        assert_eq!(s.neighbors(0).len(), 6);
    }

    #[test]
    fn test_initialize_ordered() {
        let mut s = IsingSolver::new(IsingConfig::default());
        s.initialize_ordered(false);
        for &spin in s.spins.iter() {
            assert_eq!(spin, -1);
        }
        s.initialize_ordered(true);
        for &spin in s.spins.iter() {
            assert_eq!(spin, 1);
        }
    }

    #[test]
    fn test_initialize_random() {
        let mut s = IsingSolver::new(IsingConfig::default());
        s.initialize_random();
        for &spin in s.spins.iter() {
            assert!(spin == 1 || spin == -1);
        }
    }

    #[test]
    fn test_ordered_energy_no_field() {
        let s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, j: 1.0, h: 0.0,
            ..Default::default()
        });
        // 全+1: E/N = -2J = -2.0
        let n = s.config.n_sites() as f32;
        assert!(approx_eq(s.energy() / n, -2.0, 1e-4));
    }

    #[test]
    fn test_ordered_energy_with_field() {
        let s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, j: 1.0, h: 0.5,
            ..Default::default()
        });
        // 全+1: E/N = -2J - h = -2.5
        let n = s.config.n_sites() as f32;
        assert!(approx_eq(s.energy() / n, -2.5, 1e-4));
    }

    #[test]
    fn test_magnetization_ordered() {
        let s = IsingSolver::new(IsingConfig::default());
        assert!(approx_eq(s.magnetization(), 1.0, 1e-6));
        assert!(approx_eq(s.abs_magnetization(), 1.0, 1e-6));
    }

    #[test]
    fn test_metropolis_step_progress() {
        let mut s = IsingSolver::new(IsingConfig::default());
        s.metropolis_step();
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_low_temp_stays_ordered() {
        let mut s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, j: 1.0, h: 0.0,
            temperature: 1.0, seed: 42,
        });
        s.step_n(50);
        let m = s.abs_magnetization();
        assert!(m > 0.8, "Low T should stay ordered, M = {}", m);
    }

    #[test]
    fn test_high_temp_disordered() {
        let mut s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, j: 1.0, h: 0.0,
            temperature: 5.0, seed: 42,
        });
        s.initialize_random();
        s.step_n(100);
        let m = s.abs_magnetization();
        assert!(m < 0.3, "High T should be disordered, M = {}", m);
    }

    #[test]
    fn test_measure_stats() {
        let mut s = IsingSolver::new(IsingConfig::default());
        s.measure();
        s.measure();
        assert_eq!(s.n_samples, 2);
        assert!(s.mean_energy() < 0.0);
    }

    #[test]
    fn test_specific_heat_positive() {
        let mut s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, temperature: 2.0, seed: 42,
            ..Default::default()
        });
        s.initialize_random();
        for _ in 0..50 {
            s.metropolis_step();
            s.measure();
        }
        let c = s.specific_heat();
        assert!(c.is_finite());
        assert!(c >= 0.0, "C should be >= 0, got {}", c);
    }

    #[test]
    fn test_susceptibility_positive() {
        let mut s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, temperature: 2.0, seed: 42,
            ..Default::default()
        });
        s.initialize_random();
        for _ in 0..50 {
            s.metropolis_step();
            s.measure();
        }
        let chi = s.susceptibility();
        assert!(chi.is_finite());
        assert!(chi >= 0.0, "chi should be >= 0, got {}", chi);
    }

    #[test]
    fn test_phase_transition() {
        let mut low = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, temperature: 1.5, seed: 42,
            ..Default::default()
        });
        low.step_n(100);
        let m_low = low.abs_magnetization();

        let mut high = IsingSolver::new(IsingConfig {
            lattice: LatticeType::Square2D, size: 8, temperature: 3.5, seed: 42,
            ..Default::default()
        });
        high.initialize_random();
        high.step_n(100);
        let m_high = high.abs_magnetization();

        assert!(m_low > m_high, "Low T M={} should > High T M={}", m_low, m_high);
    }

    #[test]
    fn test_reset() {
        let mut s = IsingSolver::new(IsingConfig::default());
        s.initialize_random();
        s.step_n(10);
        s.measure();
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.n_samples, 0);
        for &spin in s.spins.iter() {
            assert_eq!(spin, 1);
        }
    }

    #[test]
    fn test_3d_solver() {
        let mut s = IsingSolver::new(IsingConfig {
            lattice: LatticeType::SimpleCubic3D, size: 4, j: 1.0, h: 0.0,
            temperature: 3.0, seed: 42,
        });
        s.initialize_random();
        s.step_n(20);
        assert!(s.steps > 0);
        assert!(s.energy().is_finite());
    }
}

