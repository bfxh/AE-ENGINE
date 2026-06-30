//! 三体问题 — 牛顿引力哈密顿混沌
//!
//! 三个质点在牛顿万有引力下的运动, 经典不可积系统 (Poincaré 1889),
//! 是混沌理论的诞生地. 三体问题无一般解析解, 但存在特殊周期解:
//!   - Lagrange 等边三角形解 (三体绕共同质心刚性旋转)
//!   - Euler 共线解 (三体共线保持距离比旋转)
//!   - Figure-8 解 (Chenciner & Montgomery 2000, 三体沿同一条 8 字轨道追逐)
//!
//! 方程 (质点 i, 二维):
//!   ẍ_i = Σ_{j≠i} G m_j (r_j - r_i) / |r_j - r_i|³
//!
//! 守恒量 (哈密顿系统):
//!   - 总能量 H = Σ ½m_i|v_i|² - Σ_{i<j} G m_i m_j / r_ij
//!   - 总动量 P = Σ m_i v_i
//!   - 总角动量 L = Σ m_i (r_i × v_i)
//!   - 质心位置 R = Σ m_i r_i / M
//!
//! 数值方法:
//!   - RK4 (4 阶 Runge-Kutta, 适中时间精度)
//!   - 长期演化推荐辛积分器 (leapfrog), 但 RK4 短期可用
//!
//! 特殊解参数 (G=1, 等质量 m=1):
//!   - Lagrange 等边: 边长 a, 角速度 ω = √(3Gm/a³), 周期 T = 2π/ω
//!   - Figure-8 (Chenciner-Montgomery): 等质量, 特定初值
//!     位置: r1=(-0.97000436, 0.24308753), r2=-r1, r3=(0,0)
//!     速度: v1=v2=(-0.46620369,-0.43236573)/2, v3=(0.93240737,0.86473146)

use std::f64::consts::PI;

/// 三体配置
#[derive(Clone, Debug)]
pub struct ThreeBodyConfig {
    /// 重力常数 G
    pub g: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for ThreeBodyConfig {
    fn default() -> Self {
        Self { g: 1.0, dt: 0.001 }
    }
}

/// 质点 (2D)
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub mass: f64,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

/// 三体求解器
pub struct ThreeBodySolver {
    pub config: ThreeBodyConfig,
    pub bodies: [Body; 3],
    pub step_count: u64,
    pub time: f64,
    /// 能量历史 (诊断)
    pub energy_history: Vec<f64>,
    /// 角动量历史
    pub angular_momentum_history: Vec<f64>,
}

impl ThreeBodySolver {
    pub fn new(config: ThreeBodyConfig, bodies: [Body; 3]) -> Self {
        Self {
            config,
            bodies,
            step_count: 0,
            time: 0.0,
            energy_history: Vec::new(),
            angular_momentum_history: Vec::new(),
        }
    }

    /// 总动能 T = Σ ½ m |v|²
    pub fn kinetic_energy(&self) -> f64 {
        let mut t = 0.0;
        for b in &self.bodies {
            t += 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy);
        }
        t
    }

    /// 总势能 U = -Σ_{i<j} G m_i m_j / r_ij
    pub fn potential_energy(&self) -> f64 {
        let g = self.config.g;
        let mut u = 0.0;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dx = self.bodies[j].x - self.bodies[i].x;
                let dy = self.bodies[j].y - self.bodies[i].y;
                let r = (dx * dx + dy * dy).sqrt();
                u -= g * self.bodies[i].mass * self.bodies[j].mass / r;
            }
        }
        u
    }

    /// 总能量 H = T + U (哈密顿量, 守恒)
    pub fn energy(&self) -> f64 {
        self.kinetic_energy() + self.potential_energy()
    }

    /// 总动量 (Px, Py) = Σ m v (守恒)
    pub fn total_momentum(&self) -> (f64, f64) {
        let mut px = 0.0;
        let mut py = 0.0;
        for b in &self.bodies {
            px += b.mass * b.vx;
            py += b.mass * b.vy;
        }
        (px, py)
    }

    /// 总角动量 L = Σ m (x vy - y vx) (守恒, z 分量)
    pub fn angular_momentum(&self) -> f64 {
        let mut l = 0.0;
        for b in &self.bodies {
            l += b.mass * (b.x * b.vy - b.y * b.vx);
        }
        l
    }

    /// 质心位置 (Rx, Ry) = Σ m r / M
    pub fn center_of_mass(&self) -> (f64, f64) {
        let mut m_tot = 0.0;
        let mut rx = 0.0;
        let mut ry = 0.0;
        for b in &self.bodies {
            m_tot += b.mass;
            rx += b.mass * b.x;
            ry += b.mass * b.y;
        }
        (rx / m_tot, ry / m_tot)
    }

    /// 质心速度 = Σ m v / M
    pub fn center_of_mass_velocity(&self) -> (f64, f64) {
        let mut m_tot = 0.0;
        let mut vx = 0.0;
        let mut vy = 0.0;
        for b in &self.bodies {
            m_tot += b.mass;
            vx += b.mass * b.vx;
            vy += b.mass * b.vy;
        }
        (vx / m_tot, vy / m_tot)
    }

    /// 移除质心运动 (设 COM 位置和速度为零)
    pub fn remove_com_motion(&mut self) {
        let (rx, ry) = self.center_of_mass();
        let (vx, vy) = self.center_of_mass_velocity();
        for b in &mut self.bodies {
            b.x -= rx;
            b.y -= ry;
            b.vx -= vx;
            b.vy -= vy;
        }
    }

    /// 计算每个质点的加速度 (a_x, a_y)
    fn compute_accelerations(&self) -> [(f64, f64); 3] {
        let g = self.config.g;
        let mut acc = [(0.0, 0.0); 3];
        for i in 0..3 {
            for j in 0..3 {
                if i == j {
                    continue;
                }
                let dx = self.bodies[j].x - self.bodies[i].x;
                let dy = self.bodies[j].y - self.bodies[i].y;
                let r2 = dx * dx + dy * dy;
                let r = r2.sqrt();
                let r3 = r2 * r;
                let f = g * self.bodies[j].mass / r3;
                acc[i].0 += f * dx;
                acc[i].1 += f * dy;
            }
        }
        acc
    }

    /// 单步 RK4 推进
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = 3;
        // 状态向量: [x0,y0,vx0,vy0, x1,y1,vx1,vy1, x2,y2,vx2,vy2]
        // y' = f(y), 其中 f 返回 [vx,vy,ax,ay, ...]

        let state_to_bodies = |s: &[f64]| -> [Body; 3] {
            let mut b = self.bodies.clone();
            for i in 0..n {
                b[i].x = s[4 * i];
                b[i].y = s[4 * i + 1];
                b[i].vx = s[4 * i + 2];
                b[i].vy = s[4 * i + 3];
            }
            b
        };

        let deriv = |s: &[f64]| -> Vec<f64> {
            let bodies = state_to_bodies(s);
            let g = self.config.g;
            let mut d = vec![0.0; 4 * n];
            for i in 0..n {
                d[4 * i] = bodies[i].vx;
                d[4 * i + 1] = bodies[i].vy;
                let mut ax = 0.0;
                let mut ay = 0.0;
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let dx = bodies[j].x - bodies[i].x;
                    let dy = bodies[j].y - bodies[i].y;
                    let r2 = dx * dx + dy * dy;
                    let r = r2.sqrt();
                    let r3 = r2 * r;
                    let f = g * bodies[j].mass / r3;
                    ax += f * dx;
                    ay += f * dy;
                }
                d[4 * i + 2] = ax;
                d[4 * i + 3] = ay;
            }
            d
        };

        let mut y = vec![0.0; 4 * n];
        for i in 0..n {
            y[4 * i] = self.bodies[i].x;
            y[4 * i + 1] = self.bodies[i].y;
            y[4 * i + 2] = self.bodies[i].vx;
            y[4 * i + 3] = self.bodies[i].vy;
        }

        let k1 = deriv(&y);
        let mut y2 = vec![0.0; 4 * n];
        for k in 0..4 * n {
            y2[k] = y[k] + 0.5 * dt * k1[k];
        }
        let k2 = deriv(&y2);
        let mut y3 = vec![0.0; 4 * n];
        for k in 0..4 * n {
            y3[k] = y[k] + 0.5 * dt * k2[k];
        }
        let k3 = deriv(&y3);
        let mut y4 = vec![0.0; 4 * n];
        for k in 0..4 * n {
            y4[k] = y[k] + dt * k3[k];
        }
        let k4 = deriv(&y4);

        for k in 0..4 * n {
            y[k] += dt / 6.0 * (k1[k] + 2.0 * k2[k] + 2.0 * k3[k] + k4[k]);
        }

        for i in 0..n {
            self.bodies[i].x = y[4 * i];
            self.bodies[i].y = y[4 * i + 1];
            self.bodies[i].vx = y[4 * i + 2];
            self.bodies[i].vy = y[4 * i + 3];
        }

        self.step_count += 1;
        self.time += dt;
        self.energy_history.push(self.energy());
        self.angular_momentum_history.push(self.angular_momentum());
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        self.bodies.iter().any(|b| {
            !b.x.is_finite() || !b.y.is_finite() || !b.vx.is_finite() || !b.vy.is_finite()
        })
    }

    /// 两体间距 |r_i - r_j|
    pub fn distance(&self, i: usize, j: usize) -> f64 {
        let dx = self.bodies[j].x - self.bodies[i].x;
        let dy = self.bodies[j].y - self.bodies[i].y;
        (dx * dx + dy * dy).sqrt()
    }

    /// 初始化: Lagrange 等边三角形解 (三体绕 COM 刚性旋转)
    /// 三等质量 m 在边长 a 的等边三角形顶点, 绕 COM 角速度 ω = √(3Gm/a³)
    pub fn initialize_lagrange(mass: f64, side: f64, config: ThreeBodyConfig) -> Self {
        let g = config.g;
        // 等边三角形外接圆半径 R = a/√3
        let r = side / 3.0_f64.sqrt();
        let omega = (3.0 * g * mass / (side * side * side)).sqrt();
        // 三体位于 0, 120, 240 度, 切向速度 v = ω R
        let bodies = [
            Body {
                mass,
                x: r,
                y: 0.0,
                vx: 0.0,
                vy: omega * r,
            },
            Body {
                mass,
                x: r * (-0.5),
                y: r * (0.5 * 3.0_f64.sqrt()),
                vx: -omega * r * (0.5 * 3.0_f64.sqrt()),
                vy: -omega * r * 0.5,
            },
            Body {
                mass,
                x: r * (-0.5),
                y: -r * (0.5 * 3.0_f64.sqrt()),
                vx: omega * r * (0.5 * 3.0_f64.sqrt()),
                vy: -omega * r * 0.5,
            },
        ];
        let mut s = Self::new(config, bodies);
        s.remove_com_motion();
        s.energy_history.push(s.energy());
        s.angular_momentum_history.push(s.angular_momentum());
        s
    }

    /// 初始化: Figure-8 解 (Chenciner-Montgomery 2000), 等质量 m=1, G=1
    /// 三体沿同一条 8 字曲线追逐, 周期 T ≈ 6.3259
    pub fn initialize_figure_eight(mass: f64, config: ThreeBodyConfig) -> Self {
        // Chenciner-Montgomery 初值 (G=1, m=1 标定)
        // 缩放: 对 G m 总 = G_eff, 尺度 ∝ G_eff, 时间 ∝ 1/√G_eff
        let g_eff = config.g * mass;
        let scale = 1.0 / g_eff; // 位置尺度
        let vscale = 1.0 / g_eff.sqrt(); // 速度尺度 (v ∝ √G_eff 缩反)
        // 标准初值 (G=m=1):
        // r1 = (-0.97000436, 0.24308753), r2 = -r1, r3 = (0,0)
        // v3 = (0.93240737, 0.86473146), v1 = v2 = -v3/2
        let r1x = -0.97000436 * scale;
        let r1y = 0.24308753 * scale;
        let v3x = 0.93240737 * vscale;
        let v3y = 0.86473146 * vscale;
        let bodies = [
            Body { mass, x: r1x, y: r1y, vx: -0.5 * v3x, vy: -0.5 * v3y },
            Body { mass, x: -r1x, y: -r1y, vx: -0.5 * v3x, vy: -0.5 * v3y },
            Body { mass, x: 0.0, y: 0.0, vx: v3x, vy: v3y },
        ];
        let mut s = Self::new(config, bodies);
        s.energy_history.push(s.energy());
        s.angular_momentum_history.push(s.angular_momentum());
        s
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
        let cfg = ThreeBodyConfig::default();
        assert_eq!(cfg.g, 1.0);
        assert_eq!(cfg.dt, 0.001);
    }

    #[test]
    fn test_solver_creation() {
        let bodies = [
            Body { mass: 1.0, x: 1.0, y: 0.0, vx: 0.0, vy: 0.5 },
            Body { mass: 1.0, x: -1.0, y: 0.0, vx: 0.0, vy: -0.5 },
            Body { mass: 1.0, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0 },
        ];
        let s = ThreeBodySolver::new(ThreeBodyConfig::default(), bodies);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_energy_components() {
        let s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        // 动能 + 势能 = 总能量
        let t = s.kinetic_energy();
        let u = s.potential_energy();
        let h = s.energy();
        assert!(t > 0.0, "kinetic positive");
        assert!(u < 0.0, "potential negative (bound)");
        assert!(approx_eq(t + u, h, 1e-10));
    }

    #[test]
    fn test_lagrange_initial_geometry() {
        let s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        // 等边三角形: 三边长相等
        let d12 = s.distance(0, 1);
        let d23 = s.distance(1, 2);
        let d31 = s.distance(2, 0);
        assert!(approx_eq(d12, 1.0, 1e-9));
        assert!(approx_eq(d23, 1.0, 1e-9));
        assert!(approx_eq(d31, 1.0, 1e-9));
    }

    #[test]
    fn test_lagrange_com_at_origin() {
        let s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        let (rx, ry) = s.center_of_mass();
        assert!(rx.abs() < 1e-10, "COM x: {}", rx);
        assert!(ry.abs() < 1e-10, "COM y: {}", ry);
    }

    #[test]
    fn test_lagrange_zero_total_momentum() {
        let s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        let (px, py) = s.total_momentum();
        assert!(px.abs() < 1e-10);
        assert!(py.abs() < 1e-10);
    }

    #[test]
    fn test_lagrange_zero_angular_momentum_drift() {
        // Lagrange 解角动量应非零 (有旋转), 但守恒
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig {
            dt: 0.001,
            g: 1.0,
        });
        let l0 = s.angular_momentum();
        assert!(l0.abs() > 0.01, "Lagrange has angular momentum: {}", l0);
        s.run(1000);
        let l1 = s.angular_momentum();
        let rel = (l1 - l0).abs() / l0.abs();
        assert!(rel < 1e-4, "angular momentum drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_lagrange_energy_conservation() {
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig {
            dt: 0.0005,
            g: 1.0,
        });
        let e0 = s.energy();
        s.run(2000); // t = 1.0
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-4, "energy drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_lagrange_periodic_rotation() {
        // Lagrange 解周期 T = 2π/ω, ω = √(3Gm/a³). 一周期后回到初始位置
        let a = 1.0_f64;
        let m = 1.0_f64;
        let g = 1.0_f64;
        let omega = (3.0 * g * m / (a * a * a)).sqrt();
        let period = 2.0 * PI / omega;
        let dt = 0.0005;
        let n_steps = (period / dt).round() as usize;
        let mut s = ThreeBodySolver::initialize_lagrange(m, a, ThreeBodyConfig { g, dt });
        let x0 = s.bodies[0].x;
        let y0 = s.bodies[0].y;
        s.run(n_steps);
        // 一周期后体1回到初始位置
        let dx = (s.bodies[0].x - x0).abs();
        let dy = (s.bodies[0].y - y0).abs();
        assert!(dx < 0.02, "body 0 x after one period: dx={}", dx);
        assert!(dy < 0.02, "body 0 y after one period: dy={}", dy);
    }

    #[test]
    fn test_lagrange_triangle_shape_preserved() {
        // 旋转过程中三角形边长应保持
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig {
            dt: 0.001,
            g: 1.0,
        });
        s.run(500);
        let d12 = s.distance(0, 1);
        let d23 = s.distance(1, 2);
        let d31 = s.distance(2, 0);
        // 边长仍 ≈ 1 (刚性旋转)
        assert!((d12 - 1.0).abs() < 0.01, "d12: {}", d12);
        assert!((d23 - 1.0).abs() < 0.01, "d23: {}", d23);
        assert!((d31 - 1.0).abs() < 0.01, "d31: {}", d31);
    }

    #[test]
    fn test_figure_eight_initial_com() {
        let s = ThreeBodySolver::initialize_figure_eight(1.0, ThreeBodyConfig::default());
        let (rx, ry) = s.center_of_mass();
        assert!(rx.abs() < 1e-9, "COM x: {}", rx);
        assert!(ry.abs() < 1e-9, "COM y: {}", ry);
        let (px, py) = s.total_momentum();
        assert!(px.abs() < 1e-9);
        assert!(py.abs() < 1e-9);
    }

    #[test]
    fn test_figure_eight_zero_angular_momentum() {
        // Figure-8 角动量为零 (沿同一直线运动, 对称)
        let s = ThreeBodySolver::initialize_figure_eight(1.0, ThreeBodyConfig::default());
        let l = s.angular_momentum();
        assert!(l.abs() < 1e-6, "figure-8 L ≈ 0: {}", l);
    }

    #[test]
    fn test_figure_eight_energy_conservation() {
        let mut s = ThreeBodySolver::initialize_figure_eight(1.0, ThreeBodyConfig {
            dt: 0.0005,
            g: 1.0,
        });
        let e0 = s.energy();
        s.run(2000);
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-4, "figure-8 energy drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_figure_eight_periodicity() {
        // Figure-8 周期 T ≈ 6.32591398 (G=m=1)
        let period = 6.32591398_f64;
        let dt = 0.0005_f64;
        let n_steps = (period / dt).round() as usize;
        let mut s = ThreeBodySolver::initialize_figure_eight(1.0, ThreeBodyConfig { g: 1.0, dt });
        let x0 = s.bodies[0].x;
        let y0 = s.bodies[0].y;
        s.run(n_steps);
        let dx = (s.bodies[0].x - x0).abs();
        let dy = (s.bodies[0].y - y0).abs();
        assert!(dx < 0.05, "figure-8 body0 x periodic: dx={}", dx);
        assert!(dy < 0.05, "figure-8 body0 y periodic: dy={}", dy);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        s.run(100);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long_run() {
        let mut s = ThreeBodySolver::initialize_figure_eight(1.0, ThreeBodyConfig {
            dt: 0.001,
            g: 1.0,
        });
        s.run(10000);
        assert!(!s.has_nan(), "no NaN after 10000 steps");
    }

    #[test]
    fn test_step_advances() {
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        let t0 = s.time;
        s.step();
        assert_eq!(s.step_count, 1);
        assert!((s.time - t0 - s.config.dt).abs() < 1e-12);
        assert_eq!(s.energy_history.len(), 2);
    }

    #[test]
    fn test_remove_com_motion() {
        let bodies = [
            Body { mass: 1.0, x: 10.0, y: 20.0, vx: 5.0, vy: -3.0 },
            Body { mass: 1.0, x: -5.0, y: 0.0, vx: 1.0, vy: 2.0 },
            Body { mass: 1.0, x: 0.0, y: 0.0, vx: -1.0, vy: 1.0 },
        ];
        let mut s = ThreeBodySolver::new(ThreeBodyConfig::default(), bodies);
        s.remove_com_motion();
        let (rx, ry) = s.center_of_mass();
        let (vx, vy) = s.center_of_mass_velocity();
        assert!(rx.abs() < 1e-10);
        assert!(ry.abs() < 1e-10);
        assert!(vx.abs() < 1e-10);
        assert!(vy.abs() < 1e-10);
    }

    #[test]
    fn test_angular_momentum_conservation_general() {
        // 任意初值角动量应守恒 (中心力场)
        let bodies = [
            Body { mass: 1.0, x: 1.0, y: 0.0, vx: 0.0, vy: 0.8 },
            Body { mass: 1.5, x: -0.8, y: 0.5, vx: -0.3, vy: -0.4 },
            Body { mass: 0.7, x: 0.2, y: -1.0, vx: 0.1, vy: 0.2 },
        ];
        let mut s = ThreeBodySolver::new(ThreeBodyConfig { g: 1.0, dt: 0.0005 }, bodies);
        s.remove_com_motion();
        let l0 = s.angular_momentum();
        s.run(2000);
        let l1 = s.angular_momentum();
        let rel = if l0.abs() > 1e-6 {
            (l1 - l0).abs() / l0.abs()
        } else {
            (l1 - l0).abs()
        };
        assert!(rel < 1e-4, "angular momentum drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_momentum_conservation_general() {
        let bodies = [
            Body { mass: 1.0, x: 1.0, y: 0.0, vx: 0.0, vy: 0.8 },
            Body { mass: 1.5, x: -0.8, y: 0.5, vx: -0.3, vy: -0.4 },
            Body { mass: 0.7, x: 0.2, y: -1.0, vx: 0.1, vy: 0.2 },
        ];
        let mut s = ThreeBodySolver::new(ThreeBodyConfig { g: 1.0, dt: 0.001 }, bodies);
        s.remove_com_motion();
        let (px0, py0) = s.total_momentum();
        s.run(1000);
        let (px1, py1) = s.total_momentum();
        assert!((px1 - px0).abs() < 1e-6, "px drift: {}", (px1 - px0).abs());
        assert!((py1 - py0).abs() < 1e-6, "py drift: {}", (py1 - py0).abs());
    }

    #[test]
    fn test_grid_size_flexible() {
        for dt in [0.001, 0.0005, 0.002] {
            let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig {
                g: 1.0,
                dt,
            });
            s.run(50);
            assert!(!s.has_nan());
        }
    }

    #[test]
    fn test_diagnostics_history_grows() {
        let mut s = ThreeBodySolver::initialize_lagrange(1.0, 1.0, ThreeBodyConfig::default());
        s.run(20);
        assert_eq!(s.energy_history.len(), 21);
        assert_eq!(s.angular_momentum_history.len(), 21);
    }
}
