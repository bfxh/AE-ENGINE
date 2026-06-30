//! Chaotic Dynamics Solver (Lorenz / Rossler / Double Pendulum)
//!
//! 非线性动力学与混沌理论. 经典混沌系统的 RK4 数值积分.
//!
//! Lorenz 系统 (1963, 气象对流模型):
//!   dx/dt = sigma * (y - x)
//!   dy/dt = x * (rho - z) - y
//!   dz/dt = x * y - beta * z
//!   经典参数: sigma=10, rho=28, beta=8/3
//!   奇异吸引子, 蝴蝶效应 (初值敏感性)
//!
//! Rossler 系统 (1976, 简化吸引子):
//!   dx/dt = -y - z
//!   dy/dt = x + a*y
//!   dz/dt = b + z*(x - c)
//!   经典参数: a=0.2, b=0.2, c=5.7
//!
//! 双摆 (Lagrangian 力学, 强混沌):
//!   状态 [theta1, theta2, omega1, omega2]
//!   非线性耦合方程, 能量守恒但轨迹混沌
//!
//! RK4 积分 (4 阶 Runge-Kutta, 局部截断误差 O(dt^5)):
//!   k1 = f(t, y)
//!   k2 = f(t+dt/2, y + dt/2*k1)
//!   k3 = f(t+dt/2, y + dt/2*k2)
//!   k4 = f(t+dt, y + dt*k3)
//!   y_next = y + dt/6*(k1 + 2*k2 + 2*k3 + k4)
//!
//! 最大李雅普诺夫指数 (Wolf 1985 算法):
//!   lambda_max = (1/T) * sum( ln(d(t)/d0) )
//!   lambda_max > 0 → 混沌系统
//!
//! 基于 Lorenz 1963, Rossler 1976, Wolf 1985, Strogatz 2018.

use serde::{Deserialize, Serialize};

pub const G: f32 = 9.80665;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum System {
    /// Lorenz 混沌吸引子 (3D)
    Lorenz { sigma: f32, rho: f32, beta: f32 },
    /// Rossler 简化吸引子 (3D)
    Rossler { a: f32, b: f32, c: f32 },
    /// 双摆 (4D: theta1, theta2, omega1, omega2)
    DoublePendulum { g: f32, l1: f32, l2: f32, m1: f32, m2: f32 },
}

impl System {
    pub fn lorenz() -> Self {
        System::Lorenz { sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0 }
    }
    pub fn rossler() -> Self {
        System::Rossler { a: 0.2, b: 0.2, c: 5.7 }
    }
    pub fn double_pendulum() -> Self {
        System::DoublePendulum { g: G, l1: 1.0, l2: 1.0, m1: 1.0, m2: 1.0 }
    }

    pub fn dims(&self) -> usize {
        match self {
            System::Lorenz { .. } | System::Rossler { .. } => 3,
            System::DoublePendulum { .. } => 4,
        }
    }

    pub fn default_state(&self) -> Vec<f32> {
        match self {
            System::Lorenz { .. } => vec![1.0, 1.0, 1.0],
            System::Rossler { .. } => vec![0.0, 0.0, 0.0],
            System::DoublePendulum { .. } => {
                // 初始角度 pi/2, 零初速
                vec![std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2, 0.0, 0.0]
            }
        }
    }

    /// 系统的右端导数 f(t, y)
    pub fn derivatives(&self, state: &[f32]) -> Vec<f32> {
        match self {
            System::Lorenz { sigma, rho, beta } => {
                let (x, y, z) = (state[0], state[1], state[2]);
                vec![sigma * (y - x), x * (rho - z) - y, x * y - beta * z]
            }
            System::Rossler { a, b, c } => {
                let (x, y, z) = (state[0], state[1], state[2]);
                vec![-y - z, x + a * y, b + z * (x - c)]
            }
            System::DoublePendulum { g, l1, l2, m1, m2 } => {
                let (t1, t2, w1, w2) = (state[0], state[1], state[2], state[3]);
                let delta = t1 - t2;
                let sd = delta.sin();
                let cd = delta.cos();
                let den = 2.0 * m1 + m2 - m2 * (2.0 * delta).cos();
                let num1 = -g * (2.0 * m1 + m2) * t1.sin()
                    - m2 * g * (t1 - 2.0 * t2).sin()
                    - 2.0 * sd * m2 * (w2 * w2 * l2 + w1 * w1 * l1 * cd);
                let num2 = 2.0 * sd
                    * (w1 * w1 * l1 * (m1 + m2)
                        + g * (m1 + m2) * t1.cos()
                        + w2 * w2 * l2 * m2 * cd);
                vec![w1, w2, num1 / (l1 * den), num2 / (l2 * den)]
            }
        }
    }

    /// 能量 (双摆为总机械能, Lorenz/Rossler 为 0)
    pub fn energy(&self, state: &[f32]) -> f32 {
        match self {
            System::DoublePendulum { g, l1, l2, m1, m2 } => {
                let (t1, t2, w1, w2) = (state[0], state[1], state[2], state[3]);
                let delta = t1 - t2;
                let v1sq = l1 * l1 * w1 * w1;
                let v2sq = l1 * l1 * w1 * w1 + l2 * l2 * w2 * w2
                    + 2.0 * l1 * l2 * w1 * w2 * delta.cos();
                let ke = 0.5 * m1 * v1sq + 0.5 * m2 * v2sq;
                let pe = -(m1 + m2) * g * l1 * t1.cos() - m2 * g * l2 * t2.cos();
                ke + pe
            }
            _ => 0.0,
        }
    }
}

pub struct ChaoticSolver {
    pub system: System,
    pub state: Vec<f32>,
    pub time: f32,
    pub steps: usize,
    pub dt: f32,
}

impl ChaoticSolver {
    pub fn new(system: System, dt: f32) -> Self {
        let state = system.default_state();
        ChaoticSolver { system, state, time: 0.0, steps: 0, dt }
    }

    /// RK4 单步积分
    pub fn rk4_step(&mut self) {
        let dt = self.dt;
        let n = self.state.len();
        let y = self.state.clone();

        let k1 = self.system.derivatives(&y);
        let mut y2 = vec![0.0; n];
        for i in 0..n { y2[i] = y[i] + 0.5 * dt * k1[i]; }
        let k2 = self.system.derivatives(&y2);

        let mut y3 = vec![0.0; n];
        for i in 0..n { y3[i] = y[i] + 0.5 * dt * k2[i]; }
        let k3 = self.system.derivatives(&y3);

        let mut y4 = vec![0.0; n];
        for i in 0..n { y4[i] = y[i] + dt * k3[i]; }
        let k4 = self.system.derivatives(&y4);

        for i in 0..n {
            self.state[i] = y[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        self.time += dt;
        self.steps += 1;
    }

    pub fn step(&mut self) { self.rk4_step(); }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n { self.step(); }
    }

    /// 最大李雅普诺夫指数 (Wolf 1985 算法)
    /// n_transient: 瞬态步数 (先演化到吸引子)
    /// n_steps: 采样步数
    /// epsilon: 初始扰动大小
    pub fn max_lyapunov(&mut self, n_transient: usize, n_steps: usize, epsilon: f32) -> f32 {
        for _ in 0..n_transient { self.step(); }
        let mut perturbed = self.state.clone();
        perturbed[0] += epsilon;
        let d0 = epsilon;
        let mut sum = 0.0f32;
        let dt = self.dt;
        let n = self.state.len();
        for _ in 0..n_steps {
            self.step();
            // 演化扰动轨道 (RK4, 同一 dt)
            let y = perturbed.clone();
            let k1 = self.system.derivatives(&y);
            let mut y2 = vec![0.0; n];
            for i in 0..n { y2[i] = y[i] + 0.5 * dt * k1[i]; }
            let k2 = self.system.derivatives(&y2);
            let mut y3 = vec![0.0; n];
            for i in 0..n { y3[i] = y[i] + 0.5 * dt * k2[i]; }
            let k3 = self.system.derivatives(&y3);
            let mut y4 = vec![0.0; n];
            for i in 0..n { y4[i] = y[i] + dt * k3[i]; }
            let k4 = self.system.derivatives(&y4);
            for i in 0..n {
                perturbed[i] = y[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
            }
            // 距离
            let mut d2 = 0.0f32;
            for i in 0..n {
                let diff = perturbed[i] - self.state[i];
                d2 += diff * diff;
            }
            let d = d2.sqrt();
            if d > 1e-30 {
                sum += (d / d0).ln();
                // 重置扰动到主轨道附近, 距离 d0, 方向保留
                let scale = d0 / d;
                for i in 0..n {
                    perturbed[i] = self.state[i] + (perturbed[i] - self.state[i]) * scale;
                }
            } else {
                perturbed = self.state.clone();
                perturbed[0] += d0;
            }
        }
        sum / (n_steps as f32 * dt)
    }

    pub fn energy(&self) -> f32 {
        self.system.energy(&self.state)
    }

    pub fn reset(&mut self) {
        self.state = self.system.default_state();
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
    fn test_g_constant() {
        assert!(approx_eq(G, 9.80665, 1e-5));
    }

    #[test]
    fn test_lorenz_default_params() {
        match System::lorenz() {
            System::Lorenz { sigma, rho, beta } => {
                assert!(approx_eq(sigma, 10.0, 1e-5));
                assert!(approx_eq(rho, 28.0, 1e-5));
                assert!(approx_eq(beta, 8.0 / 3.0, 1e-5));
            }
            _ => panic!("expected Lorenz"),
        }
    }

    #[test]
    fn test_rossler_default_params() {
        match System::rossler() {
            System::Rossler { a, b, c } => {
                assert!(approx_eq(a, 0.2, 1e-5));
                assert!(approx_eq(b, 0.2, 1e-5));
                assert!(approx_eq(c, 5.7, 1e-5));
            }
            _ => panic!("expected Rossler"),
        }
    }

    #[test]
    fn test_double_pendulum_default_params() {
        match System::double_pendulum() {
            System::DoublePendulum { g, l1, l2, m1, m2 } => {
                assert!(approx_eq(g, G, 1e-5));
                assert!(approx_eq(l1, 1.0, 1e-5));
                assert!(approx_eq(l2, 1.0, 1e-5));
                assert!(approx_eq(m1, 1.0, 1e-5));
                assert!(approx_eq(m2, 1.0, 1e-5));
            }
            _ => panic!("expected DoublePendulum"),
        }
    }

    #[test]
    fn test_system_dims() {
        assert_eq!(System::lorenz().dims(), 3);
        assert_eq!(System::rossler().dims(), 3);
        assert_eq!(System::double_pendulum().dims(), 4);
    }

    #[test]
    fn test_default_state_lorenz() {
        let s = System::lorenz().default_state();
        assert_eq!(s.len(), 3);
        assert!(approx_eq(s[0], 1.0, 1e-6));
        assert!(approx_eq(s[1], 1.0, 1e-6));
        assert!(approx_eq(s[2], 1.0, 1e-6));
    }

    #[test]
    fn test_default_state_double_pendulum() {
        let s = System::double_pendulum().default_state();
        assert_eq!(s.len(), 4);
        assert!(approx_eq(s[0], std::f32::consts::FRAC_PI_2, 1e-6));
        assert!(approx_eq(s[1], std::f32::consts::FRAC_PI_2, 1e-6));
        assert!(approx_eq(s[2], 0.0, 1e-6));
        assert!(approx_eq(s[3], 0.0, 1e-6));
    }

    #[test]
    fn test_derivatives_lorenz() {
        // sigma=10, rho=28, beta=8/3, state=[1,1,1]
        // dx/dt = 10*(1-1) = 0
        // dy/dt = 1*(28-1) - 1 = 26
        // dz/dt = 1*1 - 8/3 = 1/3
        let sys = System::lorenz();
        let d = sys.derivatives(&[1.0, 1.0, 1.0]);
        assert!(approx_eq(d[0], 0.0, 1e-5));
        assert!(approx_eq(d[1], 26.0, 1e-5));
        assert!(approx_eq(d[2], 1.0 - 8.0 / 3.0, 1e-5));
    }

    #[test]
    fn test_derivatives_rossler() {
        // a=0.2, b=0.2, c=5.7, state=[0,0,0]
        // dx/dt = -0 - 0 = 0
        // dy/dt = 0 + 0.2*0 = 0
        // dz/dt = 0.2 + 0*(0-5.7) = 0.2
        let sys = System::rossler();
        let d = sys.derivatives(&[0.0, 0.0, 0.0]);
        assert!(approx_eq(d[0], 0.0, 1e-5));
        assert!(approx_eq(d[1], 0.0, 1e-5));
        assert!(approx_eq(d[2], 0.2, 1e-5));
    }

    #[test]
    fn test_derivatives_double_pendulum_equilibrium() {
        // 平衡点 [0,0,0,0] (两摆垂直向下), 导数应为 0
        let sys = System::double_pendulum();
        let d = sys.derivatives(&[0.0, 0.0, 0.0, 0.0]);
        for v in d.iter() {
            assert!(v.abs() < 1e-5, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_solver_new() {
        let s = ChaoticSolver::new(System::lorenz(), 0.01);
        assert_eq!(s.state.len(), 3);
        assert_eq!(s.steps, 0);
        assert!(approx_eq(s.time, 0.0, 1e-6));
        assert!(approx_eq(s.dt, 0.01, 1e-6));
    }

    #[test]
    fn test_rk4_step_advances() {
        let mut s = ChaoticSolver::new(System::lorenz(), 0.01);
        s.rk4_step();
        assert_eq!(s.steps, 1);
        assert!(approx_eq(s.time, 0.01, 1e-6));
        // state 应该改变
        assert!((s.state[0] - 1.0).abs() > 1e-6 || (s.state[1] - 1.0).abs() > 1e-6);
    }

    #[test]
    fn test_step_n() {
        let mut s = ChaoticSolver::new(System::lorenz(), 0.01);
        s.step_n(10);
        assert_eq!(s.steps, 10);
        assert!(approx_eq(s.time, 0.1, 1e-6));
    }

    #[test]
    fn test_lorenz_bounded() {
        // 经典 Lorenz 系统长时间不发散 (有界吸引子)
        let mut s = ChaoticSolver::new(System::lorenz(), 0.005);
        s.step_n(5000);
        for v in s.state.iter() {
            assert!(v.is_finite(), "non-finite value");
            assert!(v.abs() < 100.0, "unbounded: {}", v);
        }
    }

    #[test]
    fn test_lorenz_attractor_range() {
        // Lorenz 吸引子 z 在 [0, 50] 范围内
        let mut s = ChaoticSolver::new(System::lorenz(), 0.005);
        s.step_n(3000);
        assert!(s.state[2] > -5.0 && s.state[2] < 60.0, "z = {}", s.state[2]);
    }

    #[test]
    fn test_lorenz_lyapunov_positive() {
        // Lorenz 最大李雅普诺夫指数约为 0.9 (>0 表示混沌)
        let mut s = ChaoticSolver::new(System::lorenz(), 0.005);
        let lambda = s.max_lyapunov(500, 3000, 1e-5);
        assert!(lambda > 0.3, "expected positive lyapunov, got {}", lambda);
        assert!(lambda < 3.0, "lyapunov too large: {}", lambda);
    }

    #[test]
    fn test_rossler_bounded() {
        let mut s = ChaoticSolver::new(System::rossler(), 0.01);
        s.step_n(3000);
        for v in s.state.iter() {
            assert!(v.is_finite());
            assert!(v.abs() < 50.0, "unbounded: {}", v);
        }
    }

    #[test]
    fn test_double_pendulum_energy_conservation() {
        // 双摆, 小时间步, 能量应近似守恒. 用 pi/3 初始角度使 PE != 0
        let mut s = ChaoticSolver::new(System::double_pendulum(), 0.0005);
        s.state = vec![std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_3, 0.0, 0.0];
        let e0 = s.energy();
        assert!(e0.abs() > 1.0, "e0 too small: {}", e0);
        s.step_n(2000);
        let e1 = s.energy();
        let drift = ((e1 - e0) / e0).abs();
        assert!(drift < 0.05, "energy drift = {} (e0={}, e1={})", drift, e0, e1);
    }

    #[test]
    fn test_chaotic_sensitivity() {
        // Lorenz 蝴蝶效应: 两条相近轨道指数发散
        // 先过瞬态到吸引子, 再施加小扰动, 演化 10s
        let mut s1 = ChaoticSolver::new(System::lorenz(), 0.005);
        s1.step_n(500);
        let mut s2 = ChaoticSolver::new(System::lorenz(), 0.005);
        s2.state = s1.state.clone();
        s2.state[0] += 1e-6;
        s1.step_n(2000);
        s2.step_n(2000);
        let diff = (s1.state[0] - s2.state[0]).abs();
        // Lorenz λ≈0.9, 10s 后放大 ~8000 倍, 1e-6 -> ~8e-3
        assert!(diff > 1e-4, "expected divergence, got diff = {}", diff);
    }

    #[test]
    fn test_energy_method_lorenz() {
        // Lorenz 系统能量返回 0
        let s = ChaoticSolver::new(System::lorenz(), 0.01);
        assert_eq!(s.energy(), 0.0);
    }

    #[test]
    fn test_energy_double_pendulum_finite() {
        let s = ChaoticSolver::new(System::double_pendulum(), 0.001);
        let e = s.energy();
        assert!(e.is_finite());
        // 初始角度 pi/2, PE = -(m1+m2)*g*l1*0 - m2*g*l2*0 = 0, KE = 0
        assert!(approx_eq(e, 0.0, 1e-5));
    }

    #[test]
    fn test_reset() {
        let mut s = ChaoticSolver::new(System::lorenz(), 0.01);
        s.step_n(5);
        assert!(s.steps > 0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert!(approx_eq(s.time, 0.0, 1e-6));
        assert!(approx_eq(s.state[0], 1.0, 1e-6));
    }
}
