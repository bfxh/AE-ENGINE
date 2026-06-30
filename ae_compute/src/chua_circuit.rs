//! Chua's Circuit — 蔡氏电路 (双卷混沌吸引子)
//!
//! Chua's Circuit 是 Leon O. Chua 于 1983 年发明的最简单电子混沌电路,
//! 由两个电容、一个电感、一个电阻和一个非线性电阻 (Chua 二极管) 组成.
//! 它是第一个在物理上实现混沌的电子电路, 也是混沌理论中与 Lorenz、
//! Rössler 齐名的三大经典混沌系统之一.
//!
//! 无量纲状态方程 (Matsumoto 1984 形式):
//!   dx/dt = α (y - x - f(x))
//!   dy/dt = x - y + z
//!   dz/dt = -β y - γ z
//!
//! 其中 f(x) 为 Chua 二极管的分段线性伏安特性:
//!   f(x) = m1 x + 0.5 (m0 - m1) (|x + 1| - |x - 1|)
//! 等价于分段:
//!   x > 1:   f(x) = m1 x + (m0 - m1)      (右段斜率 m1)
//!   |x| < 1: f(x) = m0 x                   (中段斜率 m0)
//!   x < -1:  f(x) = m1 x - (m0 - m1)      (左段斜率 m1)
//!
//! 经典参数 (双卷吸引子):
//!   α = 15.6, β = 28.0, γ = 0, m0 = -1.143, m1 = -0.714
//!   或 α = 10.0, β = 14.87, m0 = -1.27, m1 = -0.68
//!
//! 性质:
//!   - 3 个平衡点: 原点 (0,0,0) 与两个对称非零平衡点
//!     非零平衡点: x* = ±(m0 - m1)/(m1 + 1) = ±c, y* = ∓c, z* = 0 (近似)
//!   - 双卷吸引子: 轨道在两个平衡点附近交替旋转, 形成双叶结构
//!   - Lyapunov 指数: λ1 > 0 (混沌), λ2 = 0, λ3 < 0
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ1/|λ3|
//!   - 奇对称: (x, y, z) → (-x, -y, -z) 系统不变
//!
//! 应用:
//!   - 混沌通信 (CDMA-like 调制)
//!   - 真随机数发生器
//!   - 神经网络激励 (BVP 模型推广)
//!   - 混沌同步 (Pecora-Carroll 1990)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta). 注意分段线性 f(x) 在 x = ±1 处不光滑,
//!   RK4 仍有 O(dt^4) 精度但跨过断点时局部误差略增.
//!
//! 历史:
//!   Matsumoto, T. 1984. "A chaotic attractor from Chua's circuit."
//!   IEEE Trans. Circuits Syst. 31, 1055.
//!   Chua, L. O., Komuro, M. & Matsumoto, T. 1986. "The double scroll
//!   family." IEEE Trans. Circuits Syst. 33, 1072.
//!   (蔡氏电路的完整动力学分析, 双卷族)

/// Chua's Circuit 配置
#[derive(Clone, Copy, Debug)]
pub struct ChuaConfig {
    /// 参数 α (电容比)
    pub alpha: f64,
    /// 参数 β (电感时间尺度)
    pub beta: f64,
    /// 参数 γ (损耗, 经典 0)
    pub gamma: f64,
    /// 中段斜率 m0 (经典 -1.143)
    pub m0: f64,
    /// 外段斜率 m1 (经典 -0.714)
    pub m1: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for ChuaConfig {
    fn default() -> Self {
        Self {
            alpha: 15.6,
            beta: 28.0,
            gamma: 0.0,
            m0: -1.143,
            m1: -0.714,
            dt: 0.005,
        }
    }
}

/// Chua's Circuit 求解器
pub struct ChuaSolver {
    pub config: ChuaConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积 (前向欧拉变分方程)
    pub lyap_sum: f64,
    /// 切向量
    pub v: [f64; 3],
}

impl ChuaSolver {
    pub fn new(config: ChuaConfig, x0: f64, y0: f64, z0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            z: z0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, z0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: ChuaConfig) -> Self {
        // 经典初值 (在右叶吸引子内)
        Self::new(config, 0.1, 0.0, 0.0)
    }

    /// Chua 二极管分段线性伏安特性
    /// f(x) = m1 x + 0.5 (m0 - m1) (|x + 1| - |x - 1|)
    pub fn chua_diode(cfg: &ChuaConfig, x: f64) -> f64 {
        cfg.m1 * x + 0.5 * (cfg.m0 - cfg.m1) * ((x + 1.0).abs() - (x - 1.0).abs())
    }

    /// f(x) 对 x 的导数 (分段常数)
    /// |x| < 1: m0; |x| > 1: m1; |x| = 1: 不定义 (此处用中点 m0)
    pub fn chua_diode_derivative(cfg: &ChuaConfig, x: f64) -> f64 {
        if x.abs() < 1.0 {
            cfg.m0
        } else {
            cfg.m1
        }
    }

    /// 右端导数 F = [α(y - x - f(x)), x - y + z, -βy - γz]
    pub fn derivatives(cfg: &ChuaConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        let fx = Self::chua_diode(cfg, x);
        [
            cfg.alpha * (y - x - fx),
            x - y + z,
            -cfg.beta * y - cfg.gamma * z,
        ]
    }

    /// Jacobian: J = [[-α(1+f'), α, 0], [1, -1, 1], [0, -β, -γ]]
    /// 其中 f' = df/dx (分段常数)
    pub fn jacobian(cfg: &ChuaConfig, x: f64, _y: f64, _z: f64) -> [[f64; 3]; 3] {
        let fp = Self::chua_diode_derivative(cfg, x);
        [
            [-cfg.alpha * (1.0 + fp), cfg.alpha, 0.0],
            [1.0, -1.0, 1.0],
            [0.0, -cfg.beta, -cfg.gamma],
        ]
    }

    /// 散度 ∇·F = -α(1 + f') - 1 - γ
    /// 分段常数: 中段 |x|<1 散度 = -α(1+m0) - 1 - γ; 外段 = -α(1+m1) - 1 - γ
    pub fn divergence(cfg: &ChuaConfig, x: f64, _y: f64, _z: f64) -> f64 {
        let fp = Self::chua_diode_derivative(cfg, x);
        -cfg.alpha * (1.0 + fp) - 1.0 - cfg.gamma
    }

    /// 平衡点 (3 个). 返回 [(x*, y*, z*), ...]
    /// 原点 (0,0,0) 总是平衡点 (因 f(0)=0).
    /// 非零平衡点: 由 dy/dt=0 → x + z = y; dz/dt=0 → z = -βy/γ (γ≠0) 或 y=0 (γ=0)
    /// 简化 (γ=0 经典): dy/dt=0 → z = y - x; dz/dt=0 → y = 0; → z = -x
    /// dx/dt=0 → y = x + f(x) = x + m1 x ± (m0 - m1) = ... (在外段)
    ///   x* = ±(m0 - m1)/(m1 + 1) (近似, 假设 |x*| > 1)
    pub fn equilibria(cfg: &ChuaConfig) -> Vec<(f64, f64, f64)> {
        let mut pts = vec![(0.0, 0.0, 0.0)];
        // γ = 0 情形, 非零平衡点 (近似解, 假设 |x*| > 1 在外段)
        if cfg.gamma.abs() < 1e-12 {
            let denom = cfg.m1 + 1.0;
            if denom.abs() > 1e-12 {
                let c = (cfg.m0 - cfg.m1) / denom;
                // 验证 |c| > 1 (确保在外段)
                if c.abs() > 1.0 {
                    pts.push((c, 0.0, -c));
                    pts.push((-c, 0.0, c));
                }
            }
        }
        pts
    }

    /// 单步 RK4 推进
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (x, y, z) = (self.x, self.y, self.z);

        let k1 = Self::derivatives(&cfg, x, y, z);
        let k2 = Self::derivatives(&cfg, x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1], z + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(&cfg, x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1], z + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(&cfg, x + dt * k3[0], y + dt * k3[1], z + dt * k3[2]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.z = z + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.z));

        // Lyapunov: 变分方程前向欧拉 (I + dt J) v
        // 注: f' 分段常数, 跨过 x=±1 时 J 不连续, 但前向欧拉仍可用
        let j = Self::jacobian(&cfg, self.x, self.y, self.z);
        let new_v = [
            self.v[0] + dt * (j[0][0] * self.v[0] + j[0][1] * self.v[1] + j[0][2] * self.v[2]),
            self.v[1] + dt * (j[1][0] * self.v[0] + j[1][1] * self.v[1] + j[1][2] * self.v[2]),
            self.v[2] + dt * (j[2][0] * self.v[0] + j[2][1] * self.v[1] + j[2][2] * self.v[2]),
        ];
        let mag = (new_v[0] * new_v[0] + new_v[1] * new_v[1] + new_v[2] * new_v[2]).sqrt();
        if mag > 0.0 {
            self.lyap_sum += mag.ln();
            self.v[0] = new_v[0] / mag;
            self.v[1] = new_v[1] / mag;
            self.v[2] = new_v[2] / mag;
        }
    }

    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 100.0 || self.y.abs() > 100.0 || self.z.abs() > 100.0 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        let mut zmin = f64::INFINITY;
        let mut zmax = f64::NEG_INFINITY;
        for &(x, y, z) in &self.trajectory {
            if x < xmin { xmin = x; }
            if x > xmax { xmax = x; }
            if y < ymin { ymin = y; }
            if y > ymax { ymax = y; }
            if z < zmin { zmin = z; }
            if z > zmax { zmax = z; }
        }
        (xmin, xmax, ymin, ymax, zmin, zmax)
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
        let cfg = ChuaConfig::default();
        assert!(approx_eq(cfg.alpha, 15.6, 1e-12));
        assert!(approx_eq(cfg.beta, 28.0, 1e-12));
        assert!(approx_eq(cfg.gamma, 0.0, 1e-12));
        assert!(approx_eq(cfg.m0, -1.143, 1e-12));
        assert!(approx_eq(cfg.m1, -0.714, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = ChuaSolver::classic(ChuaConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_chua_diode_middle_segment() {
        // |x| < 1: f(x) = m0 x
        let cfg = ChuaConfig::default();
        let x = 0.5_f64;
        let f = ChuaSolver::chua_diode(&cfg, x);
        assert!(approx_eq(f, cfg.m0 * x, 1e-12));
    }

    #[test]
    fn test_chua_diode_right_segment() {
        // x > 1: f(x) = m1 x + (m0 - m1)
        let cfg = ChuaConfig::default();
        let x = 2.0_f64;
        let f = ChuaSolver::chua_diode(&cfg, x);
        assert!(approx_eq(f, cfg.m1 * x + (cfg.m0 - cfg.m1), 1e-12));
    }

    #[test]
    fn test_chua_diode_left_segment() {
        // x < -1: f(x) = m1 x - (m0 - m1)
        let cfg = ChuaConfig::default();
        let x = -2.0_f64;
        let f = ChuaSolver::chua_diode(&cfg, x);
        assert!(approx_eq(f, cfg.m1 * x - (cfg.m0 - cfg.m1), 1e-12));
    }

    #[test]
    fn test_chua_diode_continuity_at_breakpoints() {
        // x = 1 处左右极限应相等 (分段线性函数连续)
        let cfg = ChuaConfig::default();
        let f_left = ChuaSolver::chua_diode(&cfg, 1.0 - 1e-9);
        let f_at = ChuaSolver::chua_diode(&cfg, 1.0);
        let f_right = ChuaSolver::chua_diode(&cfg, 1.0 + 1e-9);
        // 中段 f(1) = m0·1 = m0; 外段 f(1) = m1·1 + (m0 - m1) = m0 → 连续
        // 注: f_left = m0·(1-1e-9) = m0 - m0·1e-9, |f_left - m0| = |m0|·1e-9 ≈ 1.143e-9
        assert!(approx_eq(f_left, cfg.m0, 1e-7));
        assert!(approx_eq(f_at, cfg.m0, 1e-12));
        assert!(approx_eq(f_right, cfg.m0, 1e-7));
    }

    #[test]
    fn test_chua_diode_odd_symmetry() {
        // f(-x) = -f(x) (奇函数, 因 |x| 偶)
        let cfg = ChuaConfig::default();
        for &x in &[0.3_f64, 0.8, 1.5, 2.7, 5.0] {
            let f_pos = ChuaSolver::chua_diode(&cfg, x);
            let f_neg = ChuaSolver::chua_diode(&cfg, -x);
            assert!(approx_eq(f_pos, -f_neg, 1e-12), "f({})={}, f({})={}", x, f_pos, -x, f_neg);
        }
    }

    #[test]
    fn test_chua_diode_derivative_segments() {
        let cfg = ChuaConfig::default();
        assert!(approx_eq(ChuaSolver::chua_diode_derivative(&cfg, 0.5), cfg.m0, 1e-12));
        assert!(approx_eq(ChuaSolver::chua_diode_derivative(&cfg, -0.5), cfg.m0, 1e-12));
        assert!(approx_eq(ChuaSolver::chua_diode_derivative(&cfg, 2.0), cfg.m1, 1e-12));
        assert!(approx_eq(ChuaSolver::chua_diode_derivative(&cfg, -2.0), cfg.m1, 1e-12));
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = ChuaConfig::default();
        let (x, y, z) = (0.5_f64, 0.3_f64, 0.2_f64);
        let d = ChuaSolver::derivatives(&cfg, x, y, z);
        let fx = ChuaSolver::chua_diode(&cfg, x);
        assert!(approx_eq(d[0], cfg.alpha * (y - x - fx), 1e-12));
        assert!(approx_eq(d[1], x - y + z, 1e-12));
        assert!(approx_eq(d[2], -cfg.beta * y - cfg.gamma * z, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // 中段: J = [[-α(1+m0), α, 0], [1, -1, 1], [0, -β, -γ]]
        let cfg = ChuaConfig::default();
        let j = ChuaSolver::jacobian(&cfg, 0.5, 0.3, 0.2);
        assert!(approx_eq(j[0][0], -cfg.alpha * (1.0 + cfg.m0), 1e-12));
        assert!(approx_eq(j[0][1], cfg.alpha, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 1.0, 1e-12));
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], -cfg.beta, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.gamma, 1e-12));
    }

    #[test]
    fn test_jacobian_outer_segment() {
        // 外段: f' = m1
        let cfg = ChuaConfig::default();
        let j = ChuaSolver::jacobian(&cfg, 2.0, 0.3, 0.2);
        assert!(approx_eq(j[0][0], -cfg.alpha * (1.0 + cfg.m1), 1e-12));
    }

    #[test]
    fn test_divergence_middle_segment() {
        // |x|<1: 散度 = -α(1+m0) - 1 - γ
        let cfg = ChuaConfig::default();
        let div = ChuaSolver::divergence(&cfg, 0.5, 0.0, 0.0);
        let expected = -cfg.alpha * (1.0 + cfg.m0) - 1.0 - cfg.gamma;
        assert!(approx_eq(div, expected, 1e-12));
    }

    #[test]
    fn test_divergence_outer_segment() {
        // |x|>1: 散度 = -α(1+m1) - 1 - γ
        let cfg = ChuaConfig::default();
        let div = ChuaSolver::divergence(&cfg, 2.0, 0.0, 0.0);
        let expected = -cfg.alpha * (1.0 + cfg.m1) - 1.0 - cfg.gamma;
        assert!(approx_eq(div, expected, 1e-12));
    }

    #[test]
    fn test_divergence_mixed_signs() {
        // Chua 电路的关键特征: 中段局部不稳定 + 外段耗散
        // 中段 (m0 < -1): 1 + m0 < 0, 散度 = -α(1+m0) - 1 - γ > 0 (局部扩张)
        // 外段 (|m1| < 1): 1 + m1 > 0, 散度 < 0 (耗散)
        let cfg = ChuaConfig::default();
        let div_mid = ChuaSolver::divergence(&cfg, 0.5, 0.0, 0.0);
        let div_out = ChuaSolver::divergence(&cfg, 2.0, 0.0, 0.0);
        // 中段 (m0=-1.143): div = -15.6·(-0.143) - 1 = +1.23 > 0
        assert!(div_mid > 0.0, "middle segment should be locally expansive: {}", div_mid);
        // 外段 (m1=-0.714): div = -15.6·(0.286) - 1 = -5.46 < 0
        assert!(div_out < 0.0, "outer segment should be dissipative: {}", div_out);
    }

    #[test]
    fn test_equilibria_include_origin() {
        let cfg = ChuaConfig::default();
        let eqs = ChuaSolver::equilibria(&cfg);
        assert!(eqs.iter().any(|&(x, y, z)| x.abs() < 1e-12 && y.abs() < 1e-12 && z.abs() < 1e-12));
    }

    #[test]
    fn test_equilibria_nonzero_pair() {
        // 经典参数下应有 2 个非零对称平衡点
        let cfg = ChuaConfig::default();
        let eqs = ChuaSolver::equilibria(&cfg);
        assert_eq!(eqs.len(), 3, "should have 3 equilibria");
        let (xp, yp, zp) = eqs[1];
        let (xn, yn, zn) = eqs[2];
        // 对称: (x, y, z) → (-x, -y, -z)
        assert!(approx_eq(xp, -xn, 1e-12));
        assert!(approx_eq(yp, -yn, 1e-12));
        assert!(approx_eq(zp, -zn, 1e-12));
    }

    #[test]
    fn test_equilibria_value() {
        // 非零平衡点: x* = (m0 - m1)/(m1 + 1)
        let cfg = ChuaConfig::default();
        let c_expected = (cfg.m0 - cfg.m1) / (cfg.m1 + 1.0);
        let eqs = ChuaSolver::equilibria(&cfg);
        let (xp, _, _) = eqs[1];
        assert!(approx_eq(xp, c_expected, 1e-12));
        assert!(c_expected.abs() > 1.0, "should be in outer segment");
    }

    #[test]
    fn test_origin_is_equilibrium() {
        // 原点处 dx/dt = α(0 - 0 - f(0)) = 0 (因 f(0)=0)
        let cfg = ChuaConfig::default();
        let d = ChuaSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], 0.0, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // 双卷吸引子有界
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_attractor_range_known() {
        // 双卷吸引子大致范围: |x|<5, |y|<3, |z|<15
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(50000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -10.0 && xmax < 10.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -10.0 && ymax < 10.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -30.0 && zmax < 30.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典双卷应混沌, λ_max > 0
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应分离
        let cfg = ChuaConfig::default();
        let d0 = 1e-8_f64;
        let mut s1 = ChuaSolver::new(cfg, 0.1, 0.0, 0.0);
        let mut s2 = ChuaSolver::new(cfg, 0.1 + d0, 0.0, 0.0);
        for _ in 0..50000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=250, λ~0.5, 放大 e^125 → 饱和
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_classic_initial_in_attractor() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(50000);
        assert!(!s.has_escaped());
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_odd_symmetry_of_system() {
        // 系统 (x,y,z) → (-x,-y,-z) 不变 (因 f 奇)
        // 验证: F(-x,-y,-z) = -F(x,y,z)
        let cfg = ChuaConfig::default();
        let (x, y, z) = (0.5_f64, 0.3_f64, 0.2_f64);
        let d1 = ChuaSolver::derivatives(&cfg, x, y, z);
        let d2 = ChuaSolver::derivatives(&cfg, -x, -y, -z);
        assert!(approx_eq(d1[0], -d2[0], 1e-12));
        assert!(approx_eq(d1[1], -d2[1], 1e-12));
        assert!(approx_eq(d1[2], -d2[2], 1e-12));
    }

    #[test]
    fn test_double_scroll_signature() {
        // 双卷特征: 轨道访问 x>0 和 x<0 两个区域
        let mut s = ChuaSolver::classic(ChuaConfig::default());
        s.run(50000);
        let mut pos = 0;
        let mut neg = 0;
        for &(x, _, _) in &s.trajectory {
            if x > 1.0 {
                pos += 1;
            } else if x < -1.0 {
                neg += 1;
            }
        }
        // 应该同时访问两个叶
        assert!(pos > 100, "should visit right scroll: {}", pos);
        assert!(neg > 100, "should visit left scroll: {}", neg);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = ChuaConfig::default();
        for &x in &[0.5_f64, 2.0, -2.0] {
            let j = ChuaSolver::jacobian(&cfg, x, 0.0, 0.0);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = ChuaSolver::divergence(&cfg, x, 0.0, 0.0);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }
}
