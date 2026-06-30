//! Fermi-Pasta-Ulam-Tsingou (FPU-β) 非线性晶格 — 哈密顿系统
//!
//! 1955 年 Los Alamos 的 Fermi, Pasta, Ulam 在 MANIAC 计算机上首次
//! 大规模数值物理模拟, 研究非线性晶格的能量弛豫. 期望看到能量从
//! 最低模式流向高模式并最终热化 (统计力学的遍历假说), 但结果令人震惊:
//! 能量只在少数低模式间周期性回流, 几乎不热化 — 这就是著名的 FPU 悖论.
//!
//! FPU 悖论直接催生了:
//!   - 孤立子理论的复兴 (Zabusky & Kruskal 1965, KdV 方程数值)
//!   - KAM 定理 (Kolmogorov-Arnold-Moser, 不变环面存活)
//!   - 遍历假说的实验检验
//!   - 现代非线性动力学
//!
//! 模型 (FPU-β, 固定边界, N 个粒子 + 两端固定):
//!   H = Σ_{i=1..N} [ ½ p_i² + ½(q_{i+1}-q_i)² + (β/4)(q_{i+1}-q_i)^4 ]
//!   边界: q_0 = q_{N+1} = 0 (固定) 或周期 q_{i+N}=q_i
//!
//! 方程:
//!   dq_i/dt = p_i
//!   dp_i/dt = (q_{i+1}-q_i) + β(q_{i+1}-q_i)^3
//!           - (q_i-q_{i-1}) - β(q_i-q_{i-1})^3
//!
//! 简正模式 (线性极限 β=0, 固定边界):
//!   q_i = Σ_k A_k sin(ikπ/(N+1)) cos(ω_k t)
//!   ω_k = 2 sin(kπ/(2(N+1))),  k = 1, 2, ..., N
//!
//! 模式能量:
//!   E_k = ½ (|Q_k|² ω_k² + |P_k|²),  Q_k = √(2/(N+1)) Σ q_i sin(ikπ/(N+1))
//!
//! 关键现象:
//!   - 小 β + 低模式激发: FPU 回归 (能量周期性回流, 不热化)
//!   - 大 β 或高模式激发: 能量向高模式级联, 趋向热化
//!   - 临界 β_c ≈ 1/(N E_0) (Chirikov 重叠判据)
//!
//! 数值方法:
//!   - Leapfrog (辛积分, 长期能量稳定)
//!   - 模式分析用 DST (离散正弦变换, 固定边界)
//!
//! 参考:
//!   - Fermi, E., Pasta, J., Ulam, S. 1955. "Studies of non linear problems."
//!     Los Alamos report LA-1940.
//!   - Zabusky, N. & Kruskal, M. 1965. "Interaction of 'solitons' in a
//!     collisionless plasma and the recurrence of initial states."
//!     Phys. Rev. Lett. 15, 240. (KdV 孤立子, 解释 FPU 回归)
//!   - Izrailev, F. & Chirikov, B. 1966. "Statistical properties of a
//!     nonlinear string." Sov. Phys. Dokl. 11, 30. (热化阈值)

/// FPU-β 模型配置
#[derive(Clone, Debug)]
pub struct FpuConfig {
    /// 粒子数 N (链长度)
    pub n: usize,
    /// 非线性强度 β (β=0 纯线性, β 大热化)
    pub beta: f64,
    /// 时间步长 dt
    pub dt: f64,
    /// 是否周期边界 (false = 固定边界)
    pub periodic: bool,
}

impl Default for FpuConfig {
    fn default() -> Self {
        // FPU 1955 经典参数: N=32, β=0.1, 全部能量放最低模式
        Self {
            n: 32,
            beta: 0.1,
            dt: 0.01,
            periodic: false,
        }
    }
}

/// FPU-β 求解器
pub struct FpuSolver {
    pub config: FpuConfig,
    /// 位置 q_i
    pub q: Vec<f64>,
    /// 动量 p_i
    pub p: Vec<f64>,
    pub step_count: u64,
    pub time: f64,
    /// 能量历史 (诊断)
    pub energy_history: Vec<f64>,
    /// 各模式能量历史 (诊断, 每步或每若干步记录)
    pub mode_energy_history: Vec<Vec<f64>>,
}

impl FpuSolver {
    pub fn new(config: FpuConfig) -> Self {
        assert!(config.n >= 2, "n must be >= 2");
        let n = config.n;
        Self {
            config,
            q: vec![0.0; n],
            p: vec![0.0; n],
            step_count: 0,
            time: 0.0,
            energy_history: Vec::new(),
            mode_energy_history: Vec::new(),
        }
    }

    /// 初始化: 在第 k 个简正模式注入能量 E
    /// 用 DST 反演: Q_k = A = √(2E/ω_k²), q_i = √(2/(N+1)) A sin(ikπ/(N+1))
    /// 保证模式能量 E_k = ½ ω_k² Q_k² = E, 实空间总能量 = E (Parseval)
    pub fn initialize_mode(&mut self, mode: usize, energy: f64) {
        assert!(mode >= 1 && mode <= self.config.n, "mode must be in [1, N]");
        let n = self.config.n;
        let omega_k = self.mode_frequency(mode);
        let a = (2.0 * energy / (omega_k * omega_k)).sqrt();
        let norm = (2.0 / (n as f64 + 1.0)).sqrt();
        for i in 0..n {
            let s = ((i + 1) as f64 * mode as f64 * std::f64::consts::PI
                / (n as f64 + 1.0))
                .sin();
            self.q[i] = norm * a * s;
            self.p[i] = 0.0;
        }
        self.step_count = 0;
        self.time = 0.0;
        self.energy_history.clear();
        self.mode_energy_history.clear();
        self.energy_history.push(self.energy());
        self.mode_energy_history.push(self.mode_energies());
    }

    /// 初始化: 全部位移为 0, 给定动量分布 (高斯热初始)
    pub fn initialize_thermal(&mut self, seed: u64, total_ke: f64) {
        let n = self.config.n;
        let mut rng = FpuRng::new(seed);
        // Box-Muller 生成高斯
        let mut gauss = Vec::with_capacity(n);
        for _ in 0..n {
            let u1 = rng.next_f64().max(1e-10);
            let u2 = rng.next_f64();
            let g = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            gauss.push(g);
        }
        // 减去均值 → 动量守恒 (周期边界有意义, 固定边界也减)
        let mean: f64 = gauss.iter().sum::<f64>() / n as f64;
        let var: f64 = gauss.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / n as f64;
        let scale = (2.0 * total_ke / (var * n as f64)).sqrt();
        for i in 0..n {
            self.q[i] = 0.0;
            self.p[i] = (gauss[i] - mean) * scale;
        }
        self.step_count = 0;
        self.time = 0.0;
        self.energy_history.clear();
        self.mode_energy_history.clear();
        self.energy_history.push(self.energy());
        self.mode_energy_history.push(self.mode_energies());
    }

    /// 第 k 个简正模式频率 (固定边界): ω_k = 2 sin(kπ/(2(N+1)))
    pub fn mode_frequency(&self, k: usize) -> f64 {
        let n = self.config.n as f64;
        2.0 * (k as f64 * std::f64::consts::PI / (2.0 * (n + 1.0))).sin()
    }

    /// 总能量 H = Σ ½p² + Σ_{bonds} [½(Δq)² + (β/4)(Δq)^4] (守恒)
    pub fn energy(&self) -> f64 {
        self.kinetic_energy() + self.potential_energy()
    }

    /// 动能 T = Σ ½ p²
    pub fn kinetic_energy(&self) -> f64 {
        self.p.iter().map(|&p| 0.5 * p * p).sum()
    }

    /// 势能 U = Σ_{bonds} ½(Δq)² + (β/4)(Δq)^4
    /// 周期边界: N 条键 (q_0→q_1, ..., q_{N-1}→q_0)
    /// 固定边界: N+1 条键 (0→q_1, q_1→q_2, ..., q_N→0)
    pub fn potential_energy(&self) -> f64 {
        let n = self.config.n;
        let beta = self.config.beta;
        let mut u = 0.0;
        // 遍历每条键一次: q_i → q_{i+1} = next_pos(i) - q[i]
        // 周期: i=0..N-1 给 N 条键 (含 q_N→q_0)
        // 固定: i=0..N-1 给 N 条键 (q_1→q_2 ... q_N→0), 需补一条 0→q_1
        if !self.config.periodic {
            // 0 → q_1
            let dq = self.q[0];
            u += 0.5 * dq * dq + 0.25 * beta * dq.powi(4);
        }
        for i in 0..n {
            let qn = self.next_pos(i);
            let dq = qn - self.q[i];
            u += 0.5 * dq * dq + 0.25 * beta * dq.powi(4);
        }
        u
    }

    /// 总动量 P = Σ p_i (周期边界守恒)
    pub fn total_momentum(&self) -> f64 {
        self.p.iter().sum()
    }

    /// 周期边界 / 固定边界的邻居索引
    #[inline]
    fn next_pos(&self, i: usize) -> f64 {
        let n = self.config.n;
        if self.config.periodic {
            self.q[(i + 1) % n]
        } else if i + 1 < n {
            self.q[i + 1]
        } else {
            0.0 // q_{N+1} = 0 固定
        }
    }

    #[inline]
    fn prev_pos(&self, i: usize) -> f64 {
        let n = self.config.n;
        if self.config.periodic {
            self.q[(i + n - 1) % n]
        } else if i > 0 {
            self.q[i - 1]
        } else {
            0.0 // q_0 = 0 固定
        }
    }

    /// 计算每个粒子受力 (链间非线性弹簧)
    fn compute_forces(&self) -> Vec<f64> {
        let n = self.config.n;
        let beta = self.config.beta;
        let mut f = vec![0.0; n];
        for i in 0..n {
            let qn = self.next_pos(i);
            let qp = self.prev_pos(i);
            let dq_f = qn - self.q[i]; // q_{i+1} - q_i
            let dq_b = self.q[i] - qp; // q_i - q_{i-1}
            // F_i = (q_{i+1}-q_i) + β(q_{i+1}-q_i)^3 - (q_i-q_{i-1}) - β(q_i-q_{i-1})^3
            f[i] = dq_f + beta * dq_f.powi(3) - dq_b - beta * dq_b.powi(3);
        }
        f
    }

    /// 单步 leapfrog (辛积分):
    ///   p_{1/2} = p_0 + 0.5 dt F(q_0)
    ///   q_1     = q_0 + dt p_{1/2}
    ///   p_1     = p_{1/2} + 0.5 dt F(q_1)
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let f0 = self.compute_forces();
        // 半步动量
        for i in 0..self.config.n {
            self.p[i] += 0.5 * dt * f0[i];
        }
        // 整步位置
        for i in 0..self.config.n {
            self.q[i] += dt * self.p[i];
        }
        // 半步动量 (用新位置)
        let f1 = self.compute_forces();
        for i in 0..self.config.n {
            self.p[i] += 0.5 * dt * f1[i];
        }
        self.step_count += 1;
        self.time += dt;
        self.energy_history.push(self.energy());
        self.mode_energy_history.push(self.mode_energies());
    }

    /// 多步推进, 每 `record_interval` 步记录一次诊断 (节省内存)
    pub fn run_with_recording(&mut self, n_steps: usize, record_interval: usize) {
        // 简化: 直接 step, 跳过中间记录
        // 注意: step() 总是记录, 这里若不想每步记录需用 step_no_record
        for k in 0..n_steps {
            if record_interval > 1 && k % record_interval != 0 {
                self.step_no_record();
            } else {
                self.step();
            }
        }
    }

    /// 多步推进 (每步都记录诊断)
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 单步推进, 不记录诊断 (用于长期演化节省内存)
    fn step_no_record(&mut self) {
        let dt = self.config.dt;
        let f0 = self.compute_forces();
        for i in 0..self.config.n {
            self.p[i] += 0.5 * dt * f0[i];
        }
        for i in 0..self.config.n {
            self.q[i] += dt * self.p[i];
        }
        let f1 = self.compute_forces();
        for i in 0..self.config.n {
            self.p[i] += 0.5 * dt * f1[i];
        }
        self.step_count += 1;
        self.time += dt;
    }

    /// 简正模式能量 (固定边界 DST-I)
    /// Q_k = √(2/(N+1)) Σ_{i=1..N} q_i sin(ikπ/(N+1))
    /// P_k = √(2/(N+1)) Σ_{i=1..N} p_i sin(ikπ/(N+1))
    /// E_k = ½ (ω_k² Q_k² + P_k²)
    pub fn mode_energies(&self) -> Vec<f64> {
        let n = self.config.n;
        let mut e = vec![0.0; n];
        let norm = (2.0 / (n as f64 + 1.0)).sqrt();
        for k in 1..=n {
            let mut qk = 0.0;
            let mut pk = 0.0;
            for i in 0..n {
                let s = ((i + 1) as f64 * k as f64 * std::f64::consts::PI
                    / (n as f64 + 1.0))
                    .sin();
                qk += self.q[i] * s;
                pk += self.p[i] * s;
            }
            qk *= norm;
            pk *= norm;
            let omega_k = self.mode_frequency(k);
            e[k - 1] = 0.5 * (omega_k * omega_k * qk * qk + pk * pk);
        }
        e
    }

    /// 简正模式坐标 Q_k (固定边界 DST)
    pub fn mode_coords(&self) -> Vec<f64> {
        let n = self.config.n;
        let mut q_modes = vec![0.0; n];
        let norm = (2.0 / (n as f64 + 1.0)).sqrt();
        for k in 1..=n {
            let mut qk = 0.0;
            for i in 0..n {
                let s = ((i + 1) as f64 * k as f64 * std::f64::consts::PI
                    / (n as f64 + 1.0))
                    .sin();
                qk += self.q[i] * s;
            }
            q_modes[k - 1] = qk * norm;
        }
        q_modes
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        self.q.iter().any(|&x| !x.is_finite()) || self.p.iter().any(|&x| !x.is_finite())
    }

    /// 第 k 模式能量份额 (诊断: 模式能量分布)
    pub fn mode_fraction(&self, k: usize) -> f64 {
        let total = self.energy();
        if total.abs() < 1e-12 {
            return 0.0;
        }
        let modes = self.mode_energies();
        if k >= 1 && k <= modes.len() {
            modes[k - 1] / total
        } else {
            0.0
        }
    }
}

/// 本地 xorshift64 伪随机数生成器 (零依赖, 可复现)
struct FpuRng {
    state: u64,
}

impl FpuRng {
    fn new(seed: u64) -> Self {
        FpuRng {
            state: if seed == 0 {
                0xdeadbeefcafebabe
            } else {
                seed
            },
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 32) as u32
    }

    /// 返回 [0, 1) 均匀分布
    fn next_f64(&mut self) -> f64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        let bits = (hi << 21) | (lo >> 11);
        (bits as f64) / ((1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_default_config() {
        let cfg = FpuConfig::default();
        assert_eq!(cfg.n, 32);
        assert_eq!(cfg.beta, 0.1);
        assert!(!cfg.periodic);
    }

    #[test]
    fn test_solver_creation() {
        let s = FpuSolver::new(FpuConfig::default());
        assert_eq!(s.q.len(), 32);
        assert_eq!(s.p.len(), 32);
        assert_eq!(s.step_count, 0);
        // 全零态能量为 0
        assert!(s.energy().abs() < 1e-12);
    }

    #[test]
    fn test_mode_frequency() {
        // ω_k = 2 sin(kπ/(2(N+1)))
        let s = FpuSolver::new(FpuConfig::default());
        let n = 32;
        for k in 1..=n {
            let expected = 2.0 * (k as f64 * std::f64::consts::PI / (2.0 * (n as f64 + 1.0))).sin();
            assert!(approx_eq(s.mode_frequency(k), expected, 1e-12));
        }
    }

    #[test]
    fn test_initialize_mode_energy() {
        // β=0 严格检验: 模式能量归一化后实空间总能量 = 注入能量
        // (β≠0 时实空间能量含非线性 β 项贡献, 不严格等于线性模式能量)
        let mut s = FpuSolver::new(FpuConfig { n: 16, beta: 0.0, dt: 0.01, periodic: false });
        let e0 = 1.0;
        s.initialize_mode(1, e0);
        let e = s.energy();
        assert!(approx_eq(e, e0, 1e-9), "mode 1 energy: {} expected {}", e, e0);
    }

    #[test]
    fn test_initialize_mode_higher() {
        let mut s = FpuSolver::new(FpuConfig { n: 16, beta: 0.0, dt: 0.01, periodic: false });
        let e0 = 0.5;
        s.initialize_mode(3, e0);
        let e = s.energy();
        assert!(approx_eq(e, e0, 1e-9), "mode 3 energy: {} expected {}", e, e0);
    }

    #[test]
    fn test_mode_energies_initial() {
        // 初始第 k 模式激发, 模式能量分布应主要集中在第 k 模式
        let mut s = FpuSolver::new(FpuConfig { n: 16, beta: 0.0, dt: 0.01, periodic: false });
        s.initialize_mode(1, 1.0);
        let modes = s.mode_energies();
        // β=0 时能量不转移, 第 1 模式应有全部能量
        let e1 = modes[0];
        let total: f64 = modes.iter().sum();
        assert!(approx_eq(e1, total, 1e-9), "mode 1 has all energy: {}/{}", e1, total);
    }

    #[test]
    fn test_energy_conservation_leapfrog() {
        // Leapfrog 辛积分, 能量应长期守恒 (振荡误差)
        let mut s = FpuSolver::new(FpuConfig {
            n: 32, beta: 0.1, dt: 0.05, periodic: false
        });
        s.initialize_mode(1, 1.0);
        let e0 = s.energy();
        s.run(10000); // t = 500
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-4, "energy drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_momentum_conservation_periodic() {
        // 周期边界总动量应守恒 (无外力, 平移不变)
        let mut s = FpuSolver::new(FpuConfig {
            n: 32, beta: 0.5, dt: 0.01, periodic: true
        });
        s.initialize_thermal(42, 1.0);
        let p0 = s.total_momentum();
        s.run(5000);
        let p1 = s.total_momentum();
        assert!((p1 - p0).abs() < 1e-6, "momentum drift: {}", (p1 - p0).abs());
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = FpuSolver::new(FpuConfig::default());
        s.initialize_mode(1, 1.0);
        s.run(100);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = FpuSolver::new(FpuConfig {
            n: 32, beta: 1.0, dt: 0.01, periodic: false
        });
        s.initialize_mode(1, 1.0);
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_step_advances() {
        let mut s = FpuSolver::new(FpuConfig::default());
        s.initialize_mode(1, 1.0);
        let t0 = s.time;
        s.step();
        assert_eq!(s.step_count, 1);
        assert!((s.time - t0 - s.config.dt).abs() < 1e-12);
        assert_eq!(s.energy_history.len(), 2);
        assert_eq!(s.mode_energy_history.len(), 2);
    }

    #[test]
    fn test_beta_zero_no_coupling() {
        // β=0 纯线性: 单模式激发不转移到其他模式
        let mut s = FpuSolver::new(FpuConfig {
            n: 16, beta: 0.0, dt: 0.01, periodic: false
        });
        s.initialize_mode(2, 1.0);
        s.run(5000);
        let modes = s.mode_energies();
        let e2 = modes[1];
        let total: f64 = modes.iter().sum();
        // 第 2 模式应保持全部能量 (微数值误差)
        assert!(e2 / total > 0.999, "mode 2 retains energy: {}/{}", e2, total);
    }

    #[test]
    fn test_beta_nonzero_couples_modes() {
        // β≠0 非线性: 能量从初始模式转移到其他模式
        let mut s = FpuSolver::new(FpuConfig {
            n: 32, beta: 1.0, dt: 0.01, periodic: false
        });
        s.initialize_mode(1, 10.0);
        let e1_init = s.mode_energies()[0];
        s.run(50000); // t = 500, 足够时间转移
        let modes = s.mode_energies();
        let e1_final = modes[0];
        // 第 1 模式能量应明显减少 (转移到其他模式)
        assert!(e1_final < 0.5 * e1_init, "mode 1 energy dropped: {} -> {}", e1_init, e1_final);
    }

    #[test]
    fn test_fpu_no_thermalization_small_beta() {
        // FPU 悖论核心: 小 β + 低模式 + 适度能量 → 长期不热化
        // (与大 β Chirikov 重叠热化形成对照)
        // 经典 FPU 1955 现象: 能量在少数低模式间周期性交换, 不趋向均分
        // Chirikov 参数 s = β E N / (8 ω_1²), s ≪ 1 时准周期不热化
        let mut s = FpuSolver::new(FpuConfig {
            n: 32, beta: 0.1, dt: 0.01, periodic: false
        });
        s.initialize_mode(1, 1.0);
        s.run(100000); // t=1000, 长期演化
        let modes = s.mode_energies();
        let total: f64 = modes.iter().sum();
        // 准周期判据: 前 3 个模式总能量应占主导 (>50%, 未均分到全部 32 模式)
        let top3: f64 = modes.iter().take(3).sum::<f64>() / total;
        assert!(top3 > 0.5, "FPU: top-3 modes should dominate (no thermalization): {}", top3);
    }

    #[test]
    fn test_fpu_recurrence_small_beta() {
        // FPU 准周期振荡: 模式 1 能量出现明显振荡 (能量回流迹象)
        // 用 N=8, β=0.5 (s ≈ 4) 处于 Chirikov 边界区, 振荡明显但未完全热化
        let mut s = FpuSolver::new(FpuConfig {
            n: 8, beta: 0.5, dt: 0.005, periodic: false
        });
        s.initialize_mode(1, 1.0);
        let e1_initial = s.mode_energies()[0];
        // 采样模式 1 能量轨迹
        let mut e1_traj = Vec::new();
        for _ in 0..400 {
            s.run(500); // 每 2.5 时间单位采样, 总 t=1000
            e1_traj.push(s.mode_energies()[0]);
        }
        // 准周期判据: 模式 1 能量轨迹有显著振荡 (极差 > 15% 初始能量)
        // FPU 准周期下能量在低模式间周期性交换, 模式 1 能量振荡
        let e1_max = e1_traj.iter().fold(0.0f64, |a, &b| a.max(b));
        let e1_min = e1_traj.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let oscillation = (e1_max - e1_min) / e1_initial;
        assert!(oscillation > 0.15, "FPU: mode 1 should oscillate, range = {} ({} - {})",
            oscillation, e1_min, e1_max);
    }

    #[test]
    fn test_thermalization_large_beta() {
        // 大 β + 大能量 → Chirikov 重叠 → 趋向热化 (能量均分)
        // Chirikov 参数 s = β E N / (8 ω_1²) ≫ 1 时强烈热化
        let mut s = FpuSolver::new(FpuConfig {
            n: 16, beta: 10.0, dt: 0.005, periodic: false
        });
        s.initialize_mode(1, 100.0);
        s.run(200000); // t=1000, 远超热化时间
        let modes = s.mode_energies();
        let total: f64 = modes.iter().sum();
        // 热化判定: 没有单一模式占主导 (max 份额 < 30%)
        // 严格"每模式均分"对有限 N 过严, 用主导性判据更鲁棒
        let max_frac = modes.iter().map(|&e| e / total).fold(0.0, f64::max);
        assert!(max_frac < 0.3, "thermalization: no mode should dominate, max frac {}", max_frac);
    }

    #[test]
    fn test_kinetic_potential_split() {
        // 动能 + 势能 = 总能量
        let mut s = FpuSolver::new(FpuConfig { n: 16, beta: 0.5, dt: 0.01, periodic: false });
        s.initialize_mode(1, 1.0);
        s.run(100);
        let t = s.kinetic_energy();
        let u = s.potential_energy();
        let h = s.energy();
        assert!(approx_eq(t + u, h, 1e-10));
    }

    #[test]
    fn test_periodic_boundary_momentum_invariance() {
        // 周期边界下, 整体平移 q_i → q_i + c 不改变能量
        let mut s = FpuSolver::new(FpuConfig {
            n: 8, beta: 0.5, dt: 0.01, periodic: true
        });
        s.initialize_thermal(42, 1.0);
        let e0 = s.energy();
        // 整体平移
        for q in &mut s.q {
            *q += 5.0;
        }
        let e1 = s.energy();
        assert!(approx_eq(e0, e1, 1e-10), "periodic: translation invariance {} vs {}", e0, e1);
    }

    #[test]
    fn test_diagnostics_history_grows() {
        let mut s = FpuSolver::new(FpuConfig { n: 8, beta: 0.1, dt: 0.01, periodic: false });
        s.initialize_mode(1, 1.0);
        s.run(20);
        assert_eq!(s.energy_history.len(), 21);
        assert_eq!(s.mode_energy_history.len(), 21);
        // 每个模式能量记录长度 = N
        assert_eq!(s.mode_energy_history[0].len(), 8);
    }

    #[test]
    fn test_grid_size_flexible() {
        for n in [8, 16, 32, 64] {
            let mut s = FpuSolver::new(FpuConfig {
                n, beta: 0.1, dt: 0.01, periodic: false
            });
            s.initialize_mode(1, 1.0);
            s.run(100);
            assert!(!s.has_nan(), "n={}: no NaN", n);
        }
    }

    #[test]
    fn test_run_with_recording() {
        // 记录间隔控制: 不应崩溃, 长期演化仍守恒能量
        let mut s = FpuSolver::new(FpuConfig {
            n: 16, beta: 0.5, dt: 0.01, periodic: false
        });
        s.initialize_mode(1, 1.0);
        let e0 = s.energy();
        s.run_with_recording(10000, 100);
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-4, "energy drift with recording: {}%", rel * 100.0);
    }

    #[test]
    fn test_mode_coords_initial() {
        // 第 k 模式激发 → DST 后第 k 坐标非零, 其余近零
        let mut s = FpuSolver::new(FpuConfig { n: 16, beta: 0.0, dt: 0.01, periodic: false });
        s.initialize_mode(3, 1.0);
        let coords = s.mode_coords();
        // 第 3 个坐标 (索引 2) 应显著, 其他应 ~0
        let c3 = coords[2].abs();
        let other_max = coords.iter().enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, &c)| c.abs())
            .fold(0.0, f64::max);
        assert!(c3 > 10.0 * other_max, "mode 3 isolated: c3={}, other_max={}", c3, other_max);
    }
}
