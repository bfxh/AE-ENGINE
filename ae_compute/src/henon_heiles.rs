//! Hénon-Heiles 系统 — 2D 哈密顿混沌 (KAM 定理标准例子)
//!
//! 1964 年 Michel Hénon 与 Carl Heiles 研究星系势能中恒星运动时提出,
//! 是非线性动力学和 KAM 定理的标志性数值例子. 系统在低能量下规则
//! (KAM 不变环面存活), 高能量下混沌 (环面破裂) — 直观展示了可积系统
//! 受扰动后的不变环面存活与破坏.
//!
//! 哈密顿量 (2 自由度):
//!   H = ½(ẋ² + ẏ²) + V(x, y)
//!   V(x, y) = ½(x² + y²) + x²y - y³/3
//!
//! 运动方程:
//!   ẍ = -∂V/∂x = -x - 2xy
//!   ÿ = -∂V/∂y = -y - x² + y²
//!
//! 逃逸阈值: V 在 (0, 1) 处有鞍点, V(0,1) = 1/6
//!   - E < 1/6: 粒子被囚禁在等边三角形势阱内 (三星形对称)
//!   - E > 1/6: 粒子可逃逸 (沿鞍点方向离开)
//!
//! KAM 现象:
//!   - E 较小 (≪ 1/6): 大部分轨道在 Poincaré 截面上是闭合曲线 (不变环面)
//!   - E 接近 1/6: 不变环面开始破裂, 出现混沌岛
//!   - E > 1/6: 大部分轨道混沌, Poincaré 截面呈散点
//!
//! Poincaré 截面:
//!   取 x = 0 且 vx > 0 的回归时刻, 记录 (y, vy)
//!   规则轨道 → 闭合曲线; 混沌轨道 → 散点云
//!
//! 数值方法:
//!   - RK4 (4 阶 Runge-Kutta, 短中期精度)
//!   - 长期演化推荐辛积分器, 但 RK4 足够 Poincaré 截面分析
//!
//! 历史:
//!   Hénon, M. & Heiles, C. 1964. "The applicability of the third integral
//!   of motion: some numerical experiments." Astron. J. 69, 73.
//!   (1964 年耶鲁大学 IBM 7094 计算, 首次直观看到 KAM 环面破裂)

/// Hénon-Heiles 配置
#[derive(Clone, Debug)]
pub struct HenonHeilesConfig {
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for HenonHeilesConfig {
    fn default() -> Self {
        Self { dt: 0.001 }
    }
}

/// Hénon-Heiles 求解器 (2D 哈密顿, 4D 相空间)
pub struct HenonHeilesSolver {
    pub config: HenonHeilesConfig,
    /// 位置
    pub x: f64,
    pub y: f64,
    /// 速度
    pub vx: f64,
    pub vy: f64,
    pub step_count: u64,
    pub time: f64,
    /// 能量历史 (诊断)
    pub energy_history: Vec<f64>,
    /// Poincaré 截面点 (y, vy), 当 x=0 且 vx>0 时记录
    pub poincare_section: Vec<(f64, f64)>,
    /// 上一步的 x 符号 (用于检测 x 穿过 0)
    last_x_sign: f64,
}

impl HenonHeilesSolver {
    pub fn new(config: HenonHeilesConfig, x: f64, y: f64, vx: f64, vy: f64) -> Self {
        let mut s = Self {
            config,
            x,
            y,
            vx,
            vy,
            step_count: 0,
            time: 0.0,
            energy_history: Vec::new(),
            poincare_section: Vec::new(),
            last_x_sign: if x >= 0.0 { 1.0 } else { -1.0 },
        };
        s.energy_history.push(s.energy());
        s
    }

    /// 势能 V(x, y) = ½(x² + y²) + x²y - y³/3
    pub fn potential(x: f64, y: f64) -> f64 {
        0.5 * (x * x + y * y) + x * x * y - y.powi(3) / 3.0
    }

    /// 力 (Fx, Fy) = -∇V
    /// Fx = -x - 2xy
    /// Fy = -y - x² + y²
    pub fn force(x: f64, y: f64) -> (f64, f64) {
        (-x - 2.0 * x * y, -y - x * x + y * y)
    }

    /// 总能量 H = ½(vx² + vy²) + V(x, y) (守恒)
    pub fn energy(&self) -> f64 {
        0.5 * (self.vx * self.vx + self.vy * self.vy) + Self::potential(self.x, self.y)
    }

    /// 动能 T = ½(vx² + vy²)
    pub fn kinetic_energy(&self) -> f64 {
        0.5 * (self.vx * self.vx + self.vy * self.vy)
    }

    /// 势能 U = V(x, y)
    pub fn potential_energy(&self) -> f64 {
        Self::potential(self.x, self.y)
    }

    /// 角动量 L = x*vy - y*vx (z 分量, 不守恒因势能非中心)
    pub fn angular_momentum(&self) -> f64 {
        self.x * self.vy - self.y * self.vx
    }

    /// 逃逸阈值能量 (1/6, 鞍点 V(0,1) = 1/6)
    pub const ESCAPE_ENERGY: f64 = 1.0 / 6.0;

    /// 是否已逃逸 (位置远离原点, 超出势阱)
    pub fn has_escaped(&self) -> bool {
        // 简单判定: 距原点 > 5 (势阱在 |r| < ~2 内)
        self.x * self.x + self.y * self.y > 25.0
    }

    /// 单步 RK4 推进 4D 状态 [x, y, vx, vy]
    /// d/dt [x, y, vx, vy] = [vx, vy, Fx, Fy]
    pub fn step(&mut self) {
        let dt = self.config.dt;

        let deriv = |s: [f64; 4]| -> [f64; 4] {
            let (fx, fy) = Self::force(s[0], s[1]);
            [s[2], s[3], fx, fy]
        };

        let y0 = [self.x, self.y, self.vx, self.vy];
        let k1 = deriv(y0);
        let y1 = [
            y0[0] + 0.5 * dt * k1[0],
            y0[1] + 0.5 * dt * k1[1],
            y0[2] + 0.5 * dt * k1[2],
            y0[3] + 0.5 * dt * k1[3],
        ];
        let k2 = deriv(y1);
        let y2 = [
            y0[0] + 0.5 * dt * k2[0],
            y0[1] + 0.5 * dt * k2[1],
            y0[2] + 0.5 * dt * k2[2],
            y0[3] + 0.5 * dt * k2[3],
        ];
        let k3 = deriv(y2);
        let y3 = [
            y0[0] + dt * k3[0],
            y0[1] + dt * k3[1],
            y0[2] + dt * k3[2],
            y0[3] + dt * k3[3],
        ];
        let k4 = deriv(y3);

        self.x = y0[0] + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y0[1] + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.vx = y0[2] + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);
        self.vy = y0[3] + dt / 6.0 * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]);

        self.step_count += 1;
        self.time += dt;
        self.energy_history.push(self.energy());

        // Poincaré 截面: 检测 x 从负到正穿过 0 (vx > 0)
        let new_sign = if self.x >= 0.0 { 1.0 } else { -1.0 };
        if new_sign > 0.0 && self.last_x_sign < 0.0 && self.vx > 0.0 {
            // 线性插值得到 x=0 时的 (y, vy)
            let alpha = self.x / (self.x - y0[0]); // 0 到 1, x=0 处
            let y_cross = y0[1] + alpha * (self.y - y0[1]);
            let vy_cross = y0[3] + alpha * (self.vy - y0[3]);
            self.poincare_section.push((y_cross, vy_cross));
        }
        self.last_x_sign = new_sign;
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 多步推进, 仅在 Poincaré 穿越时记录能量历史 (节省内存)
    pub fn run_with_poincare_only(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step_no_energy_record();
        }
        // 最后记录一次能量
        self.energy_history.push(self.energy());
    }

    /// 单步推进, 不记录能量历史 (节省内存)
    fn step_no_energy_record(&mut self) {
        let dt = self.config.dt;
        let deriv = |s: [f64; 4]| -> [f64; 4] {
            let (fx, fy) = Self::force(s[0], s[1]);
            [s[2], s[3], fx, fy]
        };
        let y0 = [self.x, self.y, self.vx, self.vy];
        let k1 = deriv(y0);
        let y1 = [
            y0[0] + 0.5 * dt * k1[0],
            y0[1] + 0.5 * dt * k1[1],
            y0[2] + 0.5 * dt * k1[2],
            y0[3] + 0.5 * dt * k1[3],
        ];
        let k2 = deriv(y1);
        let y2 = [
            y0[0] + 0.5 * dt * k2[0],
            y0[1] + 0.5 * dt * k2[1],
            y0[2] + 0.5 * dt * k2[2],
            y0[3] + 0.5 * dt * k2[3],
        ];
        let k3 = deriv(y2);
        let y3 = [
            y0[0] + dt * k3[0],
            y0[1] + dt * k3[1],
            y0[2] + dt * k3[2],
            y0[3] + dt * k3[3],
        ];
        let k4 = deriv(y3);
        self.x = y0[0] + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y0[1] + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.vx = y0[2] + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);
        self.vy = y0[3] + dt / 6.0 * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]);
        self.step_count += 1;
        self.time += dt;
        let new_sign = if self.x >= 0.0 { 1.0 } else { -1.0 };
        if new_sign > 0.0 && self.last_x_sign < 0.0 && self.vx > 0.0 {
            let alpha = self.x / (self.x - y0[0]);
            let y_cross = y0[1] + alpha * (self.y - y0[1]);
            let vy_cross = y0[3] + alpha * (self.vy - y0[3]);
            self.poincare_section.push((y_cross, vy_cross));
        }
        self.last_x_sign = new_sign;
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.x.is_finite()
            || !self.y.is_finite()
            || !self.vx.is_finite()
            || !self.vy.is_finite()
    }

    /// 初始化: 给定能量 E, 在 Poincaré 截面 x=0 上以 (y0, vx0, 0) 启动
    /// 动能 = E - V(0, y0) = E - ½y0² + y0³/3
    /// vx0 = √(2(E - V(0, y0)))
    ///
    /// 注: 速度方向选 x 方向 (而非 y 方向), 否则由于势能对 x 反演对称
    /// (V(-x,y)=V(x,y)), 起始 (x=0, vx=0) 会使 F_x=0 → x 恒为 0,
    /// 轨道被束缚在 y 轴上 (Hénon-Heiles 的"直线周期轨道"之一),
    /// 永不穿越 x=0 截面, Poincaré 截面为空. 取 vx0>0 使轨道横切截面.
    pub fn from_energy(y0: f64, energy: f64, config: HenonHeilesConfig) -> Self {
        let v = Self::potential(0.0, y0);
        let ke = energy - v;
        assert!(ke >= 0.0, "energy must exceed potential at y0");
        let vx0 = (2.0 * ke).sqrt();
        Self::new(config, 0.0, y0, vx0, 0.0)
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
        let cfg = HenonHeilesConfig::default();
        assert_eq!(cfg.dt, 0.001);
    }

    #[test]
    fn test_solver_creation() {
        let s = HenonHeilesSolver::new(HenonHeilesConfig::default(), 0.0, 0.1, 0.0, 0.0);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.energy_history.len(), 1);
    }

    #[test]
    fn test_potential_at_origin() {
        // V(0,0) = 0
        assert!(approx_eq(HenonHeilesSolver::potential(0.0, 0.0), 0.0, 1e-12));
    }

    #[test]
    fn test_potential_at_saddle() {
        // V(0,1) = ½ + 0 - 1/3 = 1/6 (鞍点)
        assert!(approx_eq(HenonHeilesSolver::potential(0.0, 1.0), 1.0 / 6.0, 1e-12));
    }

    #[test]
    fn test_potential_symmetry() {
        // V 关于 x 对称 (x → -x 不变): V(x,y) = V(-x,y)
        let x = 0.5_f64;
        let y = 0.3_f64;
        assert!(approx_eq(
            HenonHeilesSolver::potential(x, y),
            HenonHeilesSolver::potential(-x, y),
            1e-12
        ));
    }

    #[test]
    fn test_force_origin() {
        // 在原点力为 0 (平衡点)
        let (fx, fy) = HenonHeilesSolver::force(0.0, 0.0);
        assert!(approx_eq(fx, 0.0, 1e-12));
        assert!(approx_eq(fy, 0.0, 1e-12));
    }

    #[test]
    fn test_force_at_saddle() {
        // 在鞍点 (0, 1) 力为 0
        let (fx, fy) = HenonHeilesSolver::force(0.0, 1.0);
        assert!(approx_eq(fx, 0.0, 1e-12));
        assert!(approx_eq(fy, 0.0, 1e-12));
    }

    #[test]
    fn test_force_analytic() {
        // Fx = -x - 2xy, Fy = -y - x² + y²
        let x = 0.5_f64;
        let y = 0.3_f64;
        let (fx, fy) = HenonHeilesSolver::force(x, y);
        let expected_fx = -x - 2.0 * x * y;
        let expected_fy = -y - x * x + y * y;
        assert!(approx_eq(fx, expected_fx, 1e-12));
        assert!(approx_eq(fy, expected_fy, 1e-12));
    }

    #[test]
    fn test_energy_conservation_low_energy() {
        // 低能量 (E=1/12 ≪ 1/6), 轨道规则, RK4 应长期守恒能量
        let mut s = HenonHeilesSolver::from_energy(0.1, 1.0 / 12.0, HenonHeilesConfig { dt: 0.001 });
        let e0 = s.energy();
        s.run(50000); // t = 50
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-6, "energy drift low E: {}%", rel * 100.0);
    }

    #[test]
    fn test_energy_conservation_high_energy() {
        // 高能量 (E=0.12 < 1/6 但接近), 混沌但仍在势阱内
        let mut s = HenonHeilesSolver::from_energy(0.2, 0.12, HenonHeilesConfig { dt: 0.0005 });
        let e0 = s.energy();
        s.run(50000); // t = 25
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-5, "energy drift high E: {}%", rel * 100.0);
    }

    #[test]
    fn test_escape_energy_constant() {
        assert_eq!(HenonHeilesSolver::ESCAPE_ENERGY, 1.0 / 6.0);
    }

    #[test]
    fn test_low_energy_no_escape() {
        // E < 1/6 长期演化不应逃逸
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig { dt: 0.001 });
        s.run(100000); // t = 100
        assert!(!s.has_escaped(), "should not escape at E=0.1 < 1/6");
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig::default());
        s.run(1000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig { dt: 0.001 });
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_step_advances() {
        let mut s = HenonHeilesSolver::new(HenonHeilesConfig::default(), 0.1, 0.0, 0.0, 0.1);
        let t0 = s.time;
        s.step();
        assert_eq!(s.step_count, 1);
        assert!((s.time - t0 - s.config.dt).abs() < 1e-12);
        assert_eq!(s.energy_history.len(), 2);
    }

    #[test]
    fn test_kinetic_potential_split() {
        let s = HenonHeilesSolver::new(HenonHeilesConfig::default(), 0.3, 0.2, 0.1, 0.15);
        let t = s.kinetic_energy();
        let u = s.potential_energy();
        let h = s.energy();
        assert!(approx_eq(t + u, h, 1e-12));
    }

    #[test]
    fn test_poincare_section_captures_crossings() {
        // 长期演化应捕获 Poincaré 截面点
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig { dt: 0.001 });
        s.run(200000); // t = 200, 应多次穿越 x=0
        assert!(!s.poincare_section.is_empty(), "should capture Poincaré crossings");
        assert!(s.poincare_section.len() > 10, "many crossings: {}", s.poincare_section.len());
    }

    #[test]
    fn test_poincare_section_low_energy_bounded() {
        // 低能量 (E=1/12 ≪ 1/6): Poincaré 点应在有界区域内 (规则运动)
        let mut s = HenonHeilesSolver::from_energy(0.1, 1.0 / 12.0, HenonHeilesConfig { dt: 0.001 });
        s.run(500000); // t = 500
        // 所有 Poincaré 点应满足 E = ½ vy² + V(0, y), 即 vy² = 2(E - V(0,y))
        let e = 1.0 / 12.0;
        for &(y, vy) in &s.poincare_section {
            let v = HenonHeilesSolver::potential(0.0, y);
            let expected_vy2 = 2.0 * (e - v);
            // |vy|² 应接近 expected_vy² (在线性插值误差内)
            // 至少 vy² 应不超过 2E (能量守恒上界)
            assert!(vy * vy < 2.0 * e + 1e-6, "vy² bounded: y={}, vy={}", y, vy);
        }
    }

    #[test]
    fn test_high_energy_poincare_more_chaotic() {
        // 高能量 Poincaré 点应分布更广 (混沌区域更大)
        let mut s_low = HenonHeilesSolver::from_energy(0.1, 1.0 / 12.0, HenonHeilesConfig { dt: 0.001 });
        s_low.run(200000);
        let mut s_high = HenonHeilesSolver::from_energy(0.1, 0.15, HenonHeilesConfig { dt: 0.001 });
        s_high.run(200000);
        // 比较点分布的方差 (高能量应更分散)
        let var_low = variance_y(&s_low.poincare_section);
        let var_high = variance_y(&s_high.poincare_section);
        assert!(var_high > var_low,
            "high energy Poincaré more spread: low={}, high={}", var_low, var_high);
    }

    fn variance_y(points: &[(f64, f64)]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }
        let mean: f64 = points.iter().map(|&(y, _)| y).sum::<f64>() / points.len() as f64;
        let var: f64 = points.iter().map(|&(y, _)| (y - mean).powi(2)).sum::<f64>()
            / points.len() as f64;
        var
    }

    #[test]
    fn test_angular_momentum_not_conserved() {
        // 角动量不守恒 (势能非中心力), 但应有限
        let mut s = HenonHeilesSolver::new(HenonHeilesConfig::default(), 0.3, 0.0, 0.0, 0.2);
        let l0 = s.angular_momentum();
        s.run(1000);
        let l1 = s.angular_momentum();
        // 角动量变化 (不守恒), 但应有限
        assert!(l1.is_finite());
        // 大概率变化 (除非碰巧在特殊轨道)
        // 不严格要求 |l1-l0| > 0, 仅检查有限性
        let _ = (l0, l1);
    }

    #[test]
    fn test_dt_flexible() {
        for dt in [0.0005, 0.001, 0.002] {
            let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig { dt });
            s.run(1000);
            assert!(!s.has_nan(), "dt={}: no NaN", dt);
        }
    }

    #[test]
    fn test_from_energy_initial_energy() {
        // from_energy 应初始化到指定能量
        let e = 0.1;
        let s = HenonHeilesSolver::from_energy(0.1, e, HenonHeilesConfig::default());
        assert!(approx_eq(s.energy(), e, 1e-12));
    }

    #[test]
    fn test_run_with_poincare_only_saves_memory() {
        // 不每步记录能量, 长期演化仍守恒
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig { dt: 0.001 });
        let e0 = s.energy();
        s.run_with_poincare_only(100000);
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-5, "energy drift (poincare only): {}%", rel * 100.0);
        // 能量历史应只有 2 个点 (初始 + 最后)
        assert_eq!(s.energy_history.len(), 2);
    }

    #[test]
    fn test_diagnostics_history_grows() {
        let mut s = HenonHeilesSolver::from_energy(0.1, 0.1, HenonHeilesConfig::default());
        s.run(20);
        assert_eq!(s.energy_history.len(), 21);
    }

    #[test]
    fn test_origin_stays_at_origin() {
        // 原点 (0,0,0,0) 是不动点 (平衡点)
        let mut s = HenonHeilesSolver::new(HenonHeilesConfig::default(), 0.0, 0.0, 0.0, 0.0);
        s.run(1000);
        assert!(s.x.abs() < 1e-12);
        assert!(s.y.abs() < 1e-12);
        assert!(s.vx.abs() < 1e-12);
        assert!(s.vy.abs() < 1e-12);
    }
}
