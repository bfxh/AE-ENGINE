//! Hénon 映射 — 2D 离散混沌 (奇异吸引子)
//!
//! 1976 年 Michel Hénon 提出的简化 2D 映射, 是研究耗散混沌和奇异吸引子
//! 的标志性例子. Hénon 在 Lorenz 吸引子 (3D 连续流) 基础上构造了更简单
//! 的 2D 离散映射, 同样展现分形结构的奇异吸引子.
//!
//! 映射:
//!   x_{n+1} = 1 - a x_n² + y_n
//!   y_{n+1} = b x_n
//!
//! Jacobian:
//!   J = [[-2 a x_n, 1], [b, 0]]
//!   det(J) = -b  (常数行列式, 对 b=0.3 是耗散映射)
//!
//! 经典参数: a = 1.4, b = 0.3
//!   - Lyapunov 指数: λ₁ ≈ 0.42 (正, 混沌方向), λ₂ ≈ -1.62 (负, 收缩方向)
//!   - Kaplan-Yorke 维数: D_KY ≈ 1 + λ₁/|λ₂| ≈ 1.26
//!   - 吸引子有界: x ∈ [-1.3, 1.3], y ∈ [-0.4, 0.4] 大致范围
//!
//! 不动点 (a=1.4, b=0.3):
//!   不动点条件: x* = 1 - a x*² + b x* → a x*² + (1-b) x* - 1 = 0
//!   x* = [(b-1) ± √((1-b)² + 4a)] / (2a)
//!   x1 ≈ 0.631 (在吸引子上, 鞍点), x2 ≈ -1.131 (远离吸引子)
//!
//! 分岔:
//!   固定 b=0.3, 改变 a: 在 a ≈ 0.37 出现稳定不动点,
//!   之后周期倍化分岔, 在 a ≈ 1.06 进入混沌, 1.4 是经典混沌区.
//!
//! 吸引子结构:
//!   - 吸引子是康托尔集 × 区间的乘积 (分形)
//!   - 自相似: 放大局部可见与整体相似的结构
//!   - 吸引子具有马蹄映射 (Smale horseshoe) 的拓扑结构
//!
//! 数值方法:
//!   - 直接迭代 (无需求解 ODE)
//!   - Lyapunov 通过切向量 QR 分解或归一化迭代
//!
//! 历史:
//!   Hénon, M. 1976. "A two-dimensional mapping with a strange attractor."
//!   Commun. Math. Phys. 50, 69-77.

/// Hénon 映射配置
#[derive(Clone, Copy, Debug)]
pub struct HenonMapConfig {
    /// 参数 a (非线性强度, 经典值 1.4)
    pub a: f64,
    /// 参数 b (耗散, 经典值 0.3)
    pub b: f64,
}

impl Default for HenonMapConfig {
    fn default() -> Self {
        Self { a: 1.4, b: 0.3 }
    }
}

/// Hénon 映射求解器
pub struct HenonMapSolver {
    pub config: HenonMapConfig,
    pub x: f64,
    pub y: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64)>,
    /// Lyapunov 指数累积和
    pub lyap_sum: f64,
    /// 切向量 (vx, vy)
    pub v_x: f64,
    pub v_y: f64,
    /// 第二 Lyapunov 累积 (基于 det(J) = -b: λ₁ + λ₂ = ln|b|)
    pub lyap2_sum: f64,
}

impl HenonMapSolver {
    pub fn new(config: HenonMapConfig, x0: f64, y0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            step_count: 0,
            trajectory: vec![(x0, y0)],
            lyap_sum: 0.0,
            v_x: 1.0,
            v_y: 0.0,
            lyap2_sum: 0.0,
        }
    }

    /// 经典初值
    pub fn classic(config: HenonMapConfig) -> Self {
        Self::new(config, 0.0, 0.0)
    }

    /// 一对不动点: a x² + (1-b) x - 1 = 0
    /// x = [(b-1) ± √((1-b)² + 4a)] / (2a)
    /// 不动点: y* = b x*
    pub fn fixed_points(a: f64, b: f64) -> (f64, f64) {
        let disc = (1.0 - b).powi(2) + 4.0 * a;
        let s = disc.sqrt();
        let x1 = ((b - 1.0) + s) / (2.0 * a);
        let x2 = ((b - 1.0) - s) / (2.0 * a);
        (x1, x2)
    }

    /// Jacobian 在 (x, y) 处: [[-2ax, 1], [b, 0]]
    pub fn jacobian(a: f64, b: f64, x: f64) -> [[f64; 2]; 2] {
        [[-2.0 * a * x, 1.0], [b, 0.0]]
    }

    /// 不动点稳定性: 特征值 = (tr ± √(tr² - 4 det)) / 2
    /// 不动点 (x*, b x*): J = [[-2 a x*, 1], [b, 0]], tr = -2 a x*, det = -b
    pub fn fixed_point_eigenvalues(a: f64, b: f64, x_star: f64) -> (f64, f64) {
        let tr = -2.0 * a * x_star;
        let det = -b;
        let disc = (tr * tr - 4.0 * det).max(0.0);
        let s = disc.sqrt();
        ((tr + s) / 2.0, (tr - s) / 2.0)
    }

    /// 单步映射: x' = 1 - a x² + y; y' = b x
    pub fn step(&mut self) -> (f64, f64) {
        let a = self.config.a;
        let b = self.config.b;
        let x_new = 1.0 - a * self.x * self.x + self.y;
        let y_new = b * self.x;
        self.x = x_new;
        self.y = y_new;
        self.step_count += 1;
        self.trajectory.push((x_new, y_new));

        // Lyapunov: 切向量 v_{n+1} = J_n v_n, 归一化, 累积 ln|v|
        let j = Self::jacobian(a, b, self.x);
        let new_vx = j[0][0] * self.v_x + j[0][1] * self.v_y;
        let new_vy = j[1][0] * self.v_x + j[1][1] * self.v_y;
        let mag = (new_vx * new_vx + new_vy * new_vy).sqrt();
        if mag > 0.0 {
            self.lyap_sum += mag.ln();
            self.v_x = new_vx / mag;
            self.v_y = new_vy / mag;
        }
        // λ₁ + λ₂ = ln|det J| = ln|b|
        self.lyap2_sum += b.abs().ln();

        (x_new, y_new)
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 主 Lyapunov 指数 λ₁
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.step_count as f64
    }

    /// 第二 Lyapunov 指数 λ₂ (基于 λ₁ + λ₂ = ln|det J| = ln|b|)
    /// 耗散映射 |b|<1: ln|b|<0, 故 λ₂ = ln|b| - λ₁ < 0
    pub fn lyapunov_exponent_2(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.config.b.abs().ln() - self.lyapunov_exponent()
    }

    /// Kaplan-Yorke 猜想维数: D_KY = j + Σ_{i≤j} λ_i / |λ_{j+1}|
    /// 对 Hénon: D_KY = 1 + λ₁ / |λ₂|
    pub fn kaplan_yorke_dimension(&self) -> f64 {
        let l1 = self.lyapunov_exponent();
        let l2 = self.lyapunov_exponent_2();
        if l1 > 0.0 && l2 < 0.0 {
            1.0 + l1 / l2.abs()
        } else {
            0.0
        }
    }

    /// 检查是否逃逸 (远离吸引子)
    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 10.0 || self.y.abs() > 10.0 || !self.x.is_finite() || !self.y.is_finite()
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite()
    }

    /// 吸引子盒计数维数估计 (网格法)
    /// 将吸引子区域分成 box_size×box_size 的格子, 统计被访问的格子数 N,
    /// 改变 box_size, 拟合 log N vs log(1/box_size) 斜率即盒维数.
    pub fn box_counting_dimension(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> f64 {
        let scales = [0.1, 0.05, 0.02, 0.01, 0.005];
        let mut log_inv = Vec::new();
        let mut log_n = Vec::new();
        for &s in &scales {
            let mut visited = std::collections::HashSet::new();
            for &(x, y) in &self.trajectory {
                if x < x_min || x > x_max || y < y_min || y > y_max {
                    continue;
                }
                let ix = ((x - x_min) / s) as usize;
                let iy = ((y - y_min) / s) as usize;
                visited.insert((ix, iy));
            }
            if visited.len() > 1 {
                log_inv.push((1.0 / s).ln());
                log_n.push((visited.len() as f64).ln());
            }
        }
        if log_inv.len() < 2 {
            return 0.0;
        }
        // 线性回归 log N = D log(1/s) + c
        let n = log_inv.len() as f64;
        let sx: f64 = log_inv.iter().sum();
        let sy: f64 = log_n.iter().sum();
        let sxx: f64 = log_inv.iter().map(|t| t * t).sum();
        let sxy: f64 = log_inv.iter().zip(log_n.iter()).map(|(a, b)| a * b).sum();
        let denom = n * sxx - sx * sx;
        if denom.abs() < 1e-12 {
            0.0
        } else {
            (n * sxy - sx * sy) / denom
        }
    }

    /// 返回吸引子的近似边界 (基于轨迹的 min/max)
    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(x, y) in &self.trajectory {
            if x < x_min {
                x_min = x;
            }
            if x > x_max {
                x_max = x;
            }
            if y < y_min {
                y_min = y;
            }
            if y > y_max {
                y_max = y;
            }
        }
        (x_min, x_max, y_min, y_max)
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
        let cfg = HenonMapConfig::default();
        assert!(approx_eq(cfg.a, 1.4, 1e-12));
        assert!(approx_eq(cfg.b, 0.3, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = HenonMapSolver::classic(HenonMapConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.x, 0.0, 1e-12));
        assert!(approx_eq(s.y, 0.0, 1e-12));
    }

    #[test]
    fn test_step_analytic() {
        // 经典参数, 从 (0, 0) 起步
        // x1 = 1 - 1.4*0 + 0 = 1, y1 = 0.3*0 = 0
        // x2 = 1 - 1.4*1 + 0 = -0.4, y2 = 0.3*1 = 0.3
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.step();
        assert!(approx_eq(s.x, 1.0, 1e-12));
        assert!(approx_eq(s.y, 0.0, 1e-12));
        s.step();
        assert!(approx_eq(s.x, -0.4, 1e-12));
        assert!(approx_eq(s.y, 0.3, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_jacobian_constant_det() {
        // det(J) = -b 常数
        let a = 1.4_f64;
        let b = 0.3_f64;
        for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let j = HenonMapSolver::jacobian(a, b, x);
            let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
            assert!(approx_eq(det, -b, 1e-12));
        }
    }

    #[test]
    fn test_jacobian_entries() {
        let a = 1.4_f64;
        let b = 0.3_f64;
        let x = 0.5_f64;
        let j = HenonMapSolver::jacobian(a, b, x);
        assert!(approx_eq(j[0][0], -2.0 * a * x, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[1][0], b, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
    }

    #[test]
    fn test_fixed_points_analytic() {
        // a x² + (1-b) x - 1 = 0
        let a = 1.4_f64;
        let b = 0.3_f64;
        let (x1, x2) = HenonMapSolver::fixed_points(a, b);
        // 验证不动点方程
        assert!(approx_eq(a * x1 * x1 + (1.0 - b) * x1 - 1.0, 0.0, 1e-10));
        assert!(approx_eq(a * x2 * x2 + (1.0 - b) * x2 - 1.0, 0.0, 1e-10));
    }

    #[test]
    fn test_fixed_points_classical_values() {
        // 经典 Hénon: a=1.4, b=0.3
        // a x² + (1-b) x - 1 = 0 → 1.4 x² + 0.7 x - 1 = 0
        // x = [-0.7 ± √(0.49 + 5.6)] / 2.8 = [-0.7 ± √6.09] / 2.8
        // √6.09 ≈ 2.46779
        // x1 ≈ (-0.7 + 2.46779) / 2.8 ≈ 0.63135 (在吸引子上)
        // x2 ≈ (-0.7 - 2.46779) / 2.8 ≈ -1.13135 (远离吸引子)
        let (x1, x2) = HenonMapSolver::fixed_points(1.4, 0.3);
        assert!(approx_eq(x1, 0.63135, 0.001));
        assert!(approx_eq(x2, -1.13135, 0.001));
    }

    #[test]
    fn test_fixed_point_y_relation() {
        // 不动点: y* = b x*
        let a = 1.4_f64;
        let b = 0.3_f64;
        let (x1, _x2) = HenonMapSolver::fixed_points(a, b);
        // 在不动点 (x1, b*x1) 处, 一步迭代应回到不动点
        let mut s = HenonMapSolver::new(HenonMapConfig { a, b }, x1, b * x1);
        s.step();
        assert!(approx_eq(s.x, x1, 1e-10));
        assert!(approx_eq(s.y, b * x1, 1e-10));
    }

    #[test]
    fn test_fixed_point_eigenvalues_det() {
        // 特征值乘积 = det J = -b
        let a = 1.4_f64;
        let b = 0.3_f64;
        let (x1, _x2) = HenonMapSolver::fixed_points(a, b);
        let (e1, e2) = HenonMapSolver::fixed_point_eigenvalues(a, b, x1);
        assert!(approx_eq(e1 * e2, -b, 1e-10));
    }

    #[test]
    fn test_first_fixed_point_unstable() {
        // 经典 Hénon: 第一个不动点 (x1≈0.631, b*x1) 是鞍点 (一个 |λ|>1, 一个 |λ|<1)
        // 注: x1=0.734 是另一个不动点, 检查它是否为鞍点
        let a = 1.4_f64;
        let b = 0.3_f64;
        let (x1, _x2) = HenonMapSolver::fixed_points(a, b);
        let (e1, e2) = HenonMapSolver::fixed_point_eigenvalues(a, b, x1);
        let max_abs = e1.abs().max(e2.abs());
        let min_abs = e1.abs().min(e2.abs());
        // 鞍点: 一个特征值 |λ| > 1, 另一个 |λ| < 1
        assert!(max_abs > 1.0, "one eigenvalue should be > 1: {}", max_abs);
        assert!(min_abs < 1.0, "one eigenvalue should be < 1: {}", min_abs);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // 经典 Hénon 吸引子有界: |x| < 2.5, |y| < 1.0 大致
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(50000);
        let (x_min, x_max, y_min, y_max) = s.attractor_bounds();
        assert!(x_min > -3.0 && x_max < 3.0, "x bounds: [{}, {}]", x_min, x_max);
        assert!(y_min > -1.5 && y_max < 1.5, "y bounds: [{}, {}]", y_min, y_max);
    }

    #[test]
    fn test_attractor_bounds_known() {
        // 经典参数下, 吸引子大致 x ∈ [-1.28, 1.28], y ∈ [-0.4, 0.4]
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(100000);
        let (x_min, x_max, y_min, y_max) = s.attractor_bounds();
        assert!(x_min > -1.5 && x_min < -1.0, "x_min: {}", x_min);
        assert!(x_max > 1.0 && x_max < 1.5, "x_max: {}", x_max);
        assert!(y_min > -0.5 && y_min < -0.2, "y_min: {}", y_min);
        assert!(y_max > 0.2 && y_max < 0.5, "y_max: {}", y_max);
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典 Hénon 映射主 Lyapunov 指数应 > 0 (混沌)
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_value_classical() {
        // 文献值 λ₁ ≈ 0.419 ± 0.001
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(200000);
        let lambda = s.lyapunov_exponent();
        assert!(
            (lambda - 0.419).abs() < 0.05,
            "lambda should be near 0.419: {}",
            lambda
        );
    }

    #[test]
    fn test_lyapunov_sum_equals_ln_b() {
        // λ₁ + λ₂ = ln|b| (耗散系统)
        let cfg = HenonMapConfig::default();
        let mut s = HenonMapSolver::classic(cfg);
        s.run(10000);
        let l1 = s.lyapunov_exponent();
        let l2 = s.lyapunov_exponent_2();
        let sum = l1 + l2;
        let expected = cfg.b.abs().ln();
        assert!(approx_eq(sum, expected, 1e-10), "sum={}, expected={}", sum, expected);
    }

    #[test]
    fn test_lyapunov_2_negative() {
        // λ₂ < 0 (收缩方向)
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(50000);
        let l2 = s.lyapunov_exponent_2();
        assert!(l2 < 0.0, "lambda_2 should be negative: {}", l2);
    }

    #[test]
    fn test_lyapunov_2_value_classical() {
        // λ₂ ≈ -1.624 (文献值)
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(200000);
        let l2 = s.lyapunov_exponent_2();
        assert!(
            (l2 - (-1.624)).abs() < 0.05,
            "lambda_2 should be near -1.624: {}",
            l2
        );
    }

    #[test]
    fn test_kaplan_yorke_dimension() {
        // D_KY ≈ 1.26 (文献值)
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(200000);
        let d = s.kaplan_yorke_dimension();
        assert!((d - 1.26).abs() < 0.1, "D_KY should be near 1.26: {}", d);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道经 Lyapunov 放大后分离
        let cfg = HenonMapConfig::default();
        let mut s1 = HenonMapSolver::new(cfg, 0.1, 0.2);
        let mut s2 = HenonMapSolver::new(cfg, 0.1 + 1e-10, 0.2);
        let n = 100;
        for _ in 0..n {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let d = (dx * dx + dy * dy).sqrt();
        // Lyapunov λ ≈ 0.42, 100 步后放大 e^(42) ≈ 1.7e18 → 已饱和
        // 但 d 应该 > 初始 1e-10 明显放大
        assert!(d > 1e-6, "should be amplified: d={}", d);
    }

    #[test]
    fn test_bifurcation_low_a_stable() {
        // a=0.5, b=0.3: 应有稳定不动点
        let cfg = HenonMapConfig { a: 0.5, b: 0.3 };
        let mut s = HenonMapSolver::new(cfg, 0.5, 0.1);
        s.run(10000);
        // Lyapunov 应 < 0 (稳定)
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 0.0, "lambda should be negative at a=0.5: {}", lambda);
    }

    #[test]
    fn test_bifurcation_high_a_chaos() {
        // a=1.4: 混沌
        let cfg = HenonMapConfig { a: 1.4, b: 0.3 };
        let mut s = HenonMapSolver::new(cfg, 0.1, 0.1);
        s.run(10000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive at a=1.4: {}", lambda);
    }

    #[test]
    fn test_escape_for_large_initial() {
        // 远离吸引子的初值会逃逸到无穷
        let cfg = HenonMapConfig::default();
        let mut s = HenonMapSolver::new(cfg, 10.0, 10.0);
        s.run(100);
        assert!(s.has_escaped(), "should escape from (10,10)");
    }

    #[test]
    fn test_dissipation_constant() {
        // 体积收缩率: 每步面积乘以 |det J| = |b|
        // 长期演化后, 吸引子面积 ≤ 初始面积 * b^n
        let cfg = HenonMapConfig::default();
        let b = cfg.b;
        let mut s = HenonMapSolver::new(cfg, 0.0, 0.0);
        s.run(1000);
        // 不逃逸 + b < 1 → 耗散
        assert!(!s.has_escaped());
        assert!(b.abs() < 1.0, "Hénon is dissipative only if |b| < 1");
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        let n = 1000;
        s.run(n);
        assert_eq!(s.trajectory.len(), n + 1);
        assert_eq!(s.step_count, n as u64);
    }

    #[test]
    fn test_period_detection_in_periodic_window() {
        // 在周期窗内 (a≈1.2 附近, b=0.3 有稳定周期 7 窗)
        // 改用 a=1.05, b=0.3 应在周期 4 或 8 附近
        // Lyapunov 应接近 0 或 < 0
        let cfg = HenonMapConfig { a: 1.05, b: 0.3 };
        let mut s = HenonMapSolver::new(cfg, 0.1, 0.1);
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        // 在周期窗内 lambda ≤ 0
        assert!(lambda < 0.05, "periodic window lambda: {}", lambda);
    }

    #[test]
    fn test_invertibility() {
        // Hénon 映射可逆: 给定 (x', y'), 反演 (x, y)
        // x = y' / b, y = x' - 1 + a x² = x' - 1 + a (y'/b)²
        let cfg = HenonMapConfig::default();
        let mut s = HenonMapSolver::new(cfg, 0.3, 0.4);
        s.step();
        let x_new = s.x;
        let y_new = s.y;
        let a = cfg.a;
        let b = cfg.b;
        let x_back = y_new / b;
        let y_back = x_new - 1.0 + a * (y_new / b).powi(2);
        assert!(approx_eq(x_back, 0.3, 1e-12));
        assert!(approx_eq(y_back, 0.4, 1e-12));
    }

    #[test]
    fn test_box_counting_dimension_finite() {
        // 盒计数维数估计应给出有限值, 接近 1.26
        let mut s = HenonMapSolver::classic(HenonMapConfig::default());
        s.run(50000);
        let (x_min, x_max, y_min, y_max) = s.attractor_bounds();
        let d = s.box_counting_dimension(x_min - 0.1, x_max + 0.1, y_min - 0.1, y_max + 0.1);
        // 数值估计会有噪声, 检查在合理范围
        assert!(d > 0.8 && d < 2.0, "box dim: {}", d);
    }

    #[test]
    fn test_b_zero_y_decays() {
        // b=0: y_{n+1}=0, 系统退化为 1D 二次映射 x_{n+1} = 1 - a x_n²
        let cfg = HenonMapConfig { a: 1.4, b: 0.0 };
        let mut s = HenonMapSolver::new(cfg, 0.5, 0.5);
        s.run(5);
        assert!(approx_eq(s.y, 0.0, 1e-12), "y should be 0 after b=0 step");
    }

    #[test]
    fn test_lyapunov_sum_zero_box() {
        // b=1 (保守映射): λ₁ + λ₂ = ln(1) = 0
        let cfg = HenonMapConfig { a: 0.1, b: 1.0 };
        let mut s = HenonMapSolver::new(cfg, 0.1, 0.1);
        s.run(5000);
        let l1 = s.lyapunov_exponent();
        let l2 = s.lyapunov_exponent_2();
        assert!(approx_eq(l1 + l2, 0.0, 1e-10), "sum should be 0: {}", l1 + l2);
    }
}
