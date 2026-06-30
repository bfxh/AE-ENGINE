//! Hodgkin-Huxley Neuron Model — 经典神经元动作电位模型
//!
//! Hodgkin 和 Huxley 1952 年基于乌贼巨轴突实验建立的模型,
//! 是计算神经科学的奠基性工作, 获得 1963 年诺贝尔生理学奖.
//!
//! 4 变量 ODE 系统:
//!   C·dV/dt = I_ext - g_Na·m³·h·(V-E_Na) - g_K·n⁴·(V-E_K) - g_L·(V-E_L)
//!   dm/dt = α_m(V)·(1-m) - β_m(V)·m
//!   dh/dt = α_h(V)·(1-h) - β_h(V)·h
//!   dn/dt = α_n(V)·(1-n) - β_n(V)·n
//!
//! 变量:
//!   V - 膜电位 (mV)
//!   m - Na+ 激活门 (0~1)
//!   h - Na+ 失活门 (0~1)
//!   n - K+ 激活门 (0~1)
//!
//! 速率常数 (经典 HH 公式, V 单位 mV, 速率 1/ms):
//!   α_m = 0.1·(V+40)/(1-exp(-(V+40)/10))
//!   β_m = 4·exp(-(V+65)/18)
//!   α_h = 0.07·exp(-(V+65)/20)
//!   β_h = 1/(1+exp(-(V+35)/10))
//!   α_n = 0.01·(V+55)/(1-exp(-(V+55)/10))
//!   β_n = 0.125·exp(-(V+65)/80)
//!
//! 参数 (乌贼巨轴突):
//!   C = 1 μF/cm²
//!   g_Na = 120 mS/cm², E_Na = 50 mV
//!   g_K = 36 mS/cm²,  E_K = -77 mV
//!   g_L = 0.3 mS/cm², E_L = -54.4 mV
//!
//! 数值方法: 4阶 Runge-Kutta (RK4)
//!   对混沌/刚性系统, RK4 比 Euler 精度高得多
//!
//! 动力学:
//!   - 静息电位 V ≈ -65 mV
//!   - 阈值刺激触发动作电位 (V 去极化到 ~+40 mV 后复极化)
//!   - 不应期 (h 门失活)
//!   - 持续刺激下重复发放 (周期振荡)
//!   - I_ext > I_crit 时连续发放
//!
//! 应用:
//!   - 神经元动作电位模拟
//!   - 神经网络动力学
//!   - 离子通道药理学
//!   - 心脏起搏细胞
//!
//! 基于:
//!   - Hodgkin, A.L. & Huxley, A.F. 1952. J. Physiol. 117, 500.
//!   - Dayan, P. & Abbott, L.F. "Theoretical Neuroscience." 2001.
//!   - Gerstner, W. et al. "Neuronal Dynamics." 2014.

use serde::{Deserialize, Serialize};

// ============================================================
// 默认参数 (乌贼巨轴突)
// ============================================================

pub const HH_C: f32 = 1.0;       // 膜电容 (μF/cm²)
pub const HH_G_NA: f32 = 120.0;  // Na+ 电导 (mS/cm²)
pub const HH_G_K: f32 = 36.0;    // K+ 电导 (mS/cm²)
pub const HH_G_L: f32 = 0.3;     // 漏电导 (mS/cm²)
pub const HH_E_NA: f32 = 50.0;   // Na+ 反转电位 (mV)
pub const HH_E_K: f32 = -77.0;   // K+ 反转电位 (mV)
pub const HH_E_L: f32 = -54.4;   // 漏反转电位 (mV)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HhConfig {
    pub dt: f32,
    /// 外部注入电流 (μA/cm²)
    pub i_ext: f32,
    /// 膜电容
    pub c: f32,
    pub g_na: f32,
    pub g_k: f32,
    pub g_l: f32,
    pub e_na: f32,
    pub e_k: f32,
    pub e_l: f32,
}

impl Default for HhConfig {
    fn default() -> Self {
        HhConfig {
            dt: 0.01,
            i_ext: 0.0,
            c: HH_C,
            g_na: HH_G_NA,
            g_k: HH_G_K,
            g_l: HH_G_L,
            e_na: HH_E_NA,
            e_k: HH_E_K,
            e_l: HH_E_L,
        }
    }
}

/// 神经元状态: (V, m, h, n)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NeuronState {
    pub v: f32,
    pub m: f32,
    pub h: f32,
    pub n: f32,
}

impl NeuronState {
    pub fn new(v: f32, m: f32, h: f32, n: f32) -> Self {
        NeuronState { v, m, h, n }
    }

    /// 静息状态: V=-65, m,h,n 用稳态值
    pub fn resting() -> Self {
        let v = -65.0;
        let m = alpha_m(v) / (alpha_m(v) + beta_m(v));
        let h = alpha_h(v) / (alpha_h(v) + beta_h(v));
        let n = alpha_n(v) / (alpha_n(v) + beta_n(v));
        NeuronState { v, m, h, n }
    }
}

// ============================================================
// 门控速率函数 (经典 HH 公式)
// ============================================================

#[inline]
fn alpha_m(v: f32) -> f32 {
    let x = v + 40.0;
    if x.abs() < 1e-6 {
        1.0  // 极限: x->0, 0.1x/(1-exp(-x/10)) -> 1
    } else {
        0.1 * x / (1.0 - (-x / 10.0).exp())
    }
}

#[inline]
fn beta_m(v: f32) -> f32 {
    4.0 * (-(v + 65.0) / 18.0).exp()
}

#[inline]
fn alpha_h(v: f32) -> f32 {
    0.07 * (-(v + 65.0) / 20.0).exp()
}

#[inline]
fn beta_h(v: f32) -> f32 {
    1.0 / (1.0 + (-(v + 35.0) / 10.0).exp())
}

#[inline]
fn alpha_n(v: f32) -> f32 {
    let x = v + 55.0;
    if x.abs() < 1e-6 {
        0.1
    } else {
        0.01 * x / (1.0 - (-x / 10.0).exp())
    }
}

#[inline]
fn beta_n(v: f32) -> f32 {
    0.125 * (-(v + 65.0) / 80.0).exp()
}

// ============================================================
// 导数计算
// ============================================================

#[inline]
fn i_na(v: f32, m: f32, h: f32, cfg: &HhConfig) -> f32 {
    cfg.g_na * m * m * m * h * (v - cfg.e_na)
}

#[inline]
fn i_k(v: f32, n: f32, cfg: &HhConfig) -> f32 {
    cfg.g_k * n * n * n * n * (v - cfg.e_k)
}

#[inline]
fn i_l(v: f32, cfg: &HhConfig) -> f32 {
    cfg.g_l * (v - cfg.e_l)
}

/// 计算 dState/dt
fn derivatives(s: NeuronState, cfg: &HhConfig) -> NeuronState {
    let v = s.v;
    let m = s.m;
    let h = s.h;
    let n = s.n;

    let dv = (cfg.i_ext - i_na(v, m, h, cfg) - i_k(v, n, cfg) - i_l(v, cfg)) / cfg.c;
    let dm = alpha_m(v) * (1.0 - m) - beta_m(v) * m;
    let dh = alpha_h(v) * (1.0 - h) - beta_h(v) * h;
    let dn = alpha_n(v) * (1.0 - n) - beta_n(v) * n;

    NeuronState { v: dv, m: dm, h: dh, n: dn }
}

// ============================================================
// 单神经元求解器
// ============================================================

pub struct HhSolver {
    pub config: HhConfig,
    pub state: NeuronState,
    pub time: f32,
    pub steps: usize,
    /// 动作电位发放计数 (V 上升穿过 0 mV)
    pub spike_count: usize,
    /// 上一步 V (用于检测过零)
    pub v_prev: f32,
}

impl HhSolver {
    pub fn new(config: HhConfig) -> Self {
        let state = NeuronState::resting();
        let v_prev = state.v;
        HhSolver {
            config,
            state,
            time: 0.0,
            steps: 0,
            spike_count: 0,
            v_prev,
        }
    }

    /// 设置初始状态
    pub fn set_state(&mut self, state: NeuronState) {
        self.state = state;
        self.v_prev = state.v;
        self.time = 0.0;
        self.steps = 0;
        self.spike_count = 0;
    }

    /// 设置外部电流
    pub fn set_current(&mut self, i_ext: f32) {
        self.config.i_ext = i_ext;
    }

    /// 4阶 Runge-Kutta 单步
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let s0 = self.state;

        let k1 = derivatives(s0, &self.config);
        let s1 = NeuronState::new(
            s0.v + 0.5 * dt * k1.v,
            s0.m + 0.5 * dt * k1.m,
            s0.h + 0.5 * dt * k1.h,
            s0.n + 0.5 * dt * k1.n,
        );
        let k2 = derivatives(s1, &self.config);
        let s2 = NeuronState::new(
            s0.v + 0.5 * dt * k2.v,
            s0.m + 0.5 * dt * k2.m,
            s0.h + 0.5 * dt * k2.h,
            s0.n + 0.5 * dt * k2.n,
        );
        let k3 = derivatives(s2, &self.config);
        let s3 = NeuronState::new(
            s0.v + dt * k3.v,
            s0.m + dt * k3.m,
            s0.h + dt * k3.h,
            s0.n + dt * k3.n,
        );
        let k4 = derivatives(s3, &self.config);

        self.state.v += (dt / 6.0) * (k1.v + 2.0 * k2.v + 2.0 * k3.v + k4.v);
        self.state.m += (dt / 6.0) * (k1.m + 2.0 * k2.m + 2.0 * k3.m + k4.m);
        self.state.h += (dt / 6.0) * (k1.h + 2.0 * k2.h + 2.0 * k3.h + k4.h);
        self.state.n += (dt / 6.0) * (k1.n + 2.0 * k2.n + 2.0 * k3.n + k4.n);

        // 门变量约束在 [0, 1]
        self.state.m = self.state.m.clamp(0.0, 1.0);
        self.state.h = self.state.h.clamp(0.0, 1.0);
        self.state.n = self.state.n.clamp(0.0, 1.0);

        // 检测发放: V 从负变正 (过零)
        if self.v_prev < 0.0 && self.state.v >= 0.0 {
            self.spike_count += 1;
        }
        self.v_prev = self.state.v;

        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        !self.state.v.is_finite()
            || !self.state.m.is_finite()
            || !self.state.h.is_finite()
            || !self.state.n.is_finite()
    }

    /// 当前膜电位
    pub fn v(&self) -> f32 {
        self.state.v
    }

    /// 是否处于发放状态 (V > 0)
    pub fn is_spiking(&self) -> bool {
        self.state.v > 0.0
    }
}

// ============================================================
// 神经元网络 (耦合振荡器)
// ============================================================

/// 网络类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CouplingType {
    /// 化学突触 (脉冲耦合)
    Chemical,
    /// 电突触 (间隙连接, 线性耦合)
    Electrical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HhNetworkConfig {
    pub dt: f32,
    pub i_ext: f32,
    pub n_neurons: usize,
    /// 耦合强度
    pub g_syn: f32,
    /// 突触反转电位
    pub e_syn: f32,
    pub coupling: CouplingType,
    pub c: f32,
    pub g_na: f32,
    pub g_k: f32,
    pub g_l: f32,
    pub e_na: f32,
    pub e_k: f32,
    pub e_l: f32,
}

impl Default for HhNetworkConfig {
    fn default() -> Self {
        HhNetworkConfig {
            dt: 0.01,
            i_ext: 0.0,
            n_neurons: 10,
            g_syn: 0.1,
            e_syn: 0.0,
            coupling: CouplingType::Electrical,
            c: HH_C,
            g_na: HH_G_NA,
            g_k: HH_G_K,
            g_l: HH_G_L,
            e_na: HH_E_NA,
            e_k: HH_E_K,
            e_l: HH_E_L,
        }
    }
}

pub struct HhNetworkSolver {
    pub config: HhNetworkConfig,
    pub states: Vec<NeuronState>,
    /// 邻接矩阵 (n_neurons x n_neurons), 1=连接, 0=不连接
    pub adjacency: Vec<u8>,
    pub time: f32,
    pub steps: usize,
    pub v_prev: Vec<f32>,
    pub spike_counts: Vec<usize>,
}

impl HhNetworkSolver {
    pub fn new(config: HhNetworkConfig) -> Self {
        let n = config.n_neurons;
        let state = NeuronState::resting();
        HhNetworkSolver {
            config,
            states: vec![state; n],
            adjacency: vec![0; n * n],
            time: 0.0,
            steps: 0,
            v_prev: vec![state.v; n],
            spike_counts: vec![0; n],
        }
    }

    /// 设置全连接 (除自连接)
    pub fn connect_all(&mut self) {
        let n = self.config.n_neurons;
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    self.adjacency[i * n + j] = 1;
                }
            }
        }
    }

    /// 设置环形连接 (i -> i+1, i-1)
    pub fn connect_ring(&mut self) {
        let n = self.config.n_neurons;
        for i in 0..n {
            let next = (i + 1) % n;
            let prev = if i == 0 { n - 1 } else { i - 1 };
            self.adjacency[i * n + next] = 1;
            self.adjacency[i * n + prev] = 1;
        }
    }

    /// 给第 i 个神经元注入电流
    pub fn set_current(&mut self, i: usize, _i_ext: f32) {
        // 简化: 网络统一电流, 单神经元注入通过外部耦合实现
        // 这里保留接口, 实际网络电流在 step 中处理
        let _ = i;
    }

    fn network_derivatives(
        states: &[NeuronState],
        cfg: &HhNetworkConfig,
        adjacency: &[u8],
    ) -> Vec<NeuronState> {
        let n = cfg.n_neurons;
        let mut derivs = Vec::with_capacity(n);
        for i in 0..n {
            let s = states[i];
            let v = s.v;
            let mut coupling_current = 0.0_f32;
            for j in 0..n {
                if adjacency[i * n + j] == 1 {
                    let s_j = states[j];
                    match cfg.coupling {
                        CouplingType::Electrical => {
                            // 间隙连接: I = g_syn * (V_j - V_i)
                            coupling_current += cfg.g_syn * (s_j.v - v);
                        }
                        CouplingType::Chemical => {
                            // 简化化学突触: 当突触前发放 (V_j > 0) 时激活
                            if s_j.v > 0.0 {
                                coupling_current += cfg.g_syn * (cfg.e_syn - v);
                            }
                        }
                    }
                }
            }
            let dv = (cfg.i_ext - i_na(v, s.m, s.h, &HhConfig {
                dt: cfg.dt, i_ext: cfg.i_ext, c: cfg.c,
                g_na: cfg.g_na, g_k: cfg.g_k, g_l: cfg.g_l,
                e_na: cfg.e_na, e_k: cfg.e_k, e_l: cfg.e_l,
            }) - i_k(v, s.n, &HhConfig {
                dt: cfg.dt, i_ext: cfg.i_ext, c: cfg.c,
                g_na: cfg.g_na, g_k: cfg.g_k, g_l: cfg.g_l,
                e_na: cfg.e_na, e_k: cfg.e_k, e_l: cfg.e_l,
            }) - i_l(v, &HhConfig {
                dt: cfg.dt, i_ext: cfg.i_ext, c: cfg.c,
                g_na: cfg.g_na, g_k: cfg.g_k, g_l: cfg.g_l,
                e_na: cfg.e_na, e_k: cfg.e_k, e_l: cfg.e_l,
            }) + coupling_current) / cfg.c;
            let dm = alpha_m(v) * (1.0 - s.m) - beta_m(v) * s.m;
            let dh = alpha_h(v) * (1.0 - s.h) - beta_h(v) * s.h;
            let dn = alpha_n(v) * (1.0 - s.n) - beta_n(v) * s.n;
            derivs.push(NeuronState { v: dv, m: dm, h: dh, n: dn });
        }
        derivs
    }

    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.config.n_neurons;

        let k1 = Self::network_derivatives(&self.states, &self.config, &self.adjacency);
        let mut s1 = self.states.clone();
        for i in 0..n {
            s1[i].v += 0.5 * dt * k1[i].v;
            s1[i].m += 0.5 * dt * k1[i].m;
            s1[i].h += 0.5 * dt * k1[i].h;
            s1[i].n += 0.5 * dt * k1[i].n;
        }
        let k2 = Self::network_derivatives(&s1, &self.config, &self.adjacency);
        let mut s2 = self.states.clone();
        for i in 0..n {
            s2[i].v += 0.5 * dt * k2[i].v;
            s2[i].m += 0.5 * dt * k2[i].m;
            s2[i].h += 0.5 * dt * k2[i].h;
            s2[i].n += 0.5 * dt * k2[i].n;
        }
        let k3 = Self::network_derivatives(&s2, &self.config, &self.adjacency);
        let mut s3 = self.states.clone();
        for i in 0..n {
            s3[i].v += dt * k3[i].v;
            s3[i].m += dt * k3[i].m;
            s3[i].h += dt * k3[i].h;
            s3[i].n += dt * k3[i].n;
        }
        let k4 = Self::network_derivatives(&s3, &self.config, &self.adjacency);

        for i in 0..n {
            self.states[i].v += (dt / 6.0) * (k1[i].v + 2.0 * k2[i].v + 2.0 * k3[i].v + k4[i].v);
            self.states[i].m += (dt / 6.0) * (k1[i].m + 2.0 * k2[i].m + 2.0 * k3[i].m + k4[i].m);
            self.states[i].h += (dt / 6.0) * (k1[i].h + 2.0 * k2[i].h + 2.0 * k3[i].h + k4[i].h);
            self.states[i].n += (dt / 6.0) * (k1[i].n + 2.0 * k2[i].n + 2.0 * k3[i].n + k4[i].n);
            self.states[i].m = self.states[i].m.clamp(0.0, 1.0);
            self.states[i].h = self.states[i].h.clamp(0.0, 1.0);
            self.states[i].n = self.states[i].n.clamp(0.0, 1.0);

            if self.v_prev[i] < 0.0 && self.states[i].v >= 0.0 {
                self.spike_counts[i] += 1;
            }
            self.v_prev[i] = self.states[i].v;
        }

        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.states.iter().any(|s| {
            !s.v.is_finite() || !s.m.is_finite() || !s.h.is_finite() || !s.n.is_finite()
        })
    }

    pub fn mean_v(&self) -> f32 {
        let n = self.states.len();
        if n == 0 {
            return 0.0;
        }
        self.states.iter().map(|s| s.v).sum::<f32>() / n as f32
    }

    pub fn total_spikes(&self) -> usize {
        self.spike_counts.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resting_state_potential() {
        let s = NeuronState::resting();
        // 静息电位应接近 -65 mV
        assert!((s.v - (-65.0)).abs() < 1e-3);
        // 门变量在 [0, 1]
        assert!(s.m >= 0.0 && s.m <= 1.0);
        assert!(s.h >= 0.0 && s.h <= 1.0);
        assert!(s.n >= 0.0 && s.n <= 1.0);
    }

    #[test]
    fn test_alpha_m_at_rest() {
        let v = -65.0;
        let a = alpha_m(v);
        assert!(a >= 0.0);
        assert!(a < 10.0);
    }

    #[test]
    fn test_beta_m_at_rest() {
        let v = -65.0;
        let b = beta_m(v);
        assert!(b >= 0.0);
        assert!(b < 10.0);
    }

    #[test]
    fn test_alpha_m_singular_point() {
        // V = -40 应处理 0/0 极限
        let a = alpha_m(-40.0);
        assert!(a.is_finite());
        assert!((a - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_alpha_n_singular_point() {
        // V = -55 应处理 0/0 极限
        let a = alpha_n(-55.0);
        assert!(a.is_finite());
        assert!((a - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_gating_functions_nonneg() {
        // 所有 α/β 应非负
        for v in (-100..=50).step_by(5) {
            let v = v as f32;
            assert!(alpha_m(v) >= -1e-6, "alpha_m({}) = {} < 0", v, alpha_m(v));
            assert!(beta_m(v) >= 0.0, "beta_m({}) = {} < 0", v, beta_m(v));
            assert!(alpha_h(v) >= 0.0, "alpha_h({}) = {} < 0", v, alpha_h(v));
            assert!(beta_h(v) >= 0.0, "beta_h({}) = {} < 0", v, beta_h(v));
            assert!(alpha_n(v) >= -1e-6, "alpha_n({}) = {} < 0", v, alpha_n(v));
            assert!(beta_n(v) >= 0.0, "beta_n({}) = {} < 0", v, beta_n(v));
        }
    }

    #[test]
    fn test_solver_creation() {
        let s = HhSolver::new(HhConfig::default());
        assert!((s.state.v - (-65.0)).abs() < 1e-3);
        assert_eq!(s.steps, 0);
        assert_eq!(s.spike_count, 0);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = HhSolver::new(HhConfig::default());
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_no_stimulation_stays_resting() {
        // I_ext = 0, 应保持静息
        let mut s = HhSolver::new(HhConfig::default());
        s.step_n(1000);
        assert!(!s.has_nan());
        assert!((s.state.v - (-65.0)).abs() < 1.0, "V drifted from rest: {}", s.state.v);
        assert_eq!(s.spike_count, 0);
    }

    #[test]
    fn test_below_threshold_no_spike() {
        // 小电流 (< 阈值 ~6 μA/cm²), 不应发放
        let cfg = HhConfig { i_ext: 2.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        s.step_n(5000); // 50 ms
        assert_eq!(s.spike_count, 0, "should not spike with subthreshold current");
    }

    #[test]
    fn test_above_threshold_spikes() {
        // 大电流 (> 阈值), 应发放
        let cfg = HhConfig { i_ext: 15.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        s.step_n(5000); // 50 ms
        assert!(s.spike_count > 0, "should spike with suprathreshold current");
        assert!(s.state.v > -80.0, "V should be above -80 after activity");
    }

    #[test]
    fn test_action_potential_peak() {
        // 大电流刺激, 峰值应去极化到正值
        let cfg = HhConfig { i_ext: 20.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        let mut max_v = s.state.v;
        for _ in 0..5000 {
            s.step();
            if s.state.v > max_v {
                max_v = s.state.v;
            }
        }
        assert!(max_v > 0.0, "action potential should reach positive V: max={}", max_v);
    }

    #[test]
    fn test_repetitive_firing() {
        // 持续强刺激, 应重复发放
        let cfg = HhConfig { i_ext: 20.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        s.step_n(20000); // 200 ms
        assert!(s.spike_count >= 5, "should fire repetitively: {}", s.spike_count);
    }

    #[test]
    fn test_gating_variables_bounded() {
        let cfg = HhConfig { i_ext: 20.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        s.step_n(5000);
        assert!(s.state.m >= 0.0 && s.state.m <= 1.0);
        assert!(s.state.h >= 0.0 && s.state.h <= 1.0);
        assert!(s.state.n >= 0.0 && s.state.n <= 1.0);
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = HhConfig { i_ext: 10.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        s.step_n(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_v_stays_bounded() {
        // V 应在生理范围 [-100, 60] mV
        let cfg = HhConfig { i_ext: 20.0, ..Default::default() };
        let mut s = HhSolver::new(cfg);
        let mut min_v = s.state.v;
        let mut max_v = s.state.v;
        for _ in 0..5000 {
            s.step();
            min_v = min_v.min(s.state.v);
            max_v = max_v.max(s.state.v);
        }
        assert!(min_v > -120.0, "V too low: {}", min_v);
        assert!(max_v < 80.0, "V too high: {}", max_v);
    }

    #[test]
    fn test_steady_state_gating() {
        // 静息态: m, h, n 应接近稳态 α/(α+β)
        let v = -65.0;
        let m_ss = alpha_m(v) / (alpha_m(v) + beta_m(v));
        let h_ss = alpha_h(v) / (alpha_h(v) + beta_h(v));
        let n_ss = alpha_n(v) / (alpha_n(v) + beta_n(v));
        let s = NeuronState::resting();
        assert!((s.m - m_ss).abs() < 1e-3);
        assert!((s.h - h_ss).abs() < 1e-3);
        assert!((s.n - n_ss).abs() < 1e-3);
    }

    #[test]
    fn test_current_pulse_triggers_spike() {
        // 短脉冲电流 (5 ms, 50 μA) 触发单个动作电位
        let mut s = HhSolver::new(HhConfig::default());
        // 脉冲阶段
        s.set_current(50.0);
        s.step_n(500); // 5 ms
        // 恢复阶段
        s.set_current(0.0);
        s.step_n(2000); // 20 ms
        assert!(s.spike_count >= 1, "pulse should trigger spike");
    }

    // ========================================================
    // 网络测试
    // ========================================================

    #[test]
    fn test_network_creation() {
        let net = HhNetworkSolver::new(HhNetworkConfig::default());
        assert_eq!(net.states.len(), 10);
        assert_eq!(net.config.n_neurons, 10);
    }

    #[test]
    fn test_network_connect_all() {
        let mut net = HhNetworkSolver::new(HhNetworkConfig::default());
        net.connect_all();
        let n = net.config.n_neurons;
        // 每个神经元连接 n-1 个其他神经元
        for i in 0..n {
            let degree: u32 = (0..n).map(|j| net.adjacency[i * n + j] as u32).sum();
            assert_eq!(degree, (n - 1) as u32, "neuron {} degree", i);
        }
    }

    #[test]
    fn test_network_connect_ring() {
        let mut net = HhNetworkSolver::new(HhNetworkConfig::default());
        net.connect_ring();
        let n = net.config.n_neurons;
        for i in 0..n {
            let degree: u32 = (0..n).map(|j| net.adjacency[i * n + j] as u32).sum();
            assert_eq!(degree, 2, "ring neuron {} should have degree 2", i);
        }
    }

    #[test]
    fn test_network_no_stimulation_stays_resting() {
        let mut net = HhNetworkSolver::new(HhNetworkConfig::default());
        net.connect_all();
        net.step_n(1000);
        assert!(!net.has_nan());
        assert!(net.mean_v() > -70.0 && net.mean_v() < -60.0, "mean V drifted: {}", net.mean_v());
    }

    #[test]
    fn test_network_stimulation_causes_activity() {
        let cfg = HhNetworkConfig {
            i_ext: 15.0,
            n_neurons: 5,
            g_syn: 0.05,
            ..Default::default()
        };
        let mut net = HhNetworkSolver::new(cfg);
        net.connect_all();
        net.step_n(5000);
        assert!(!net.has_nan());
        assert!(net.total_spikes() > 0, "network should have activity");
    }

    #[test]
    fn test_network_step_advances() {
        let mut net = HhNetworkSolver::new(HhNetworkConfig::default());
        net.connect_all();
        let t0 = net.time;
        net.step();
        assert!(net.time > t0);
        assert_eq!(net.steps, 1);
    }

    #[test]
    fn test_electrical_coupling_synchronizes() {
        // 电突触 (间隙连接) 促进同步
        let cfg = HhNetworkConfig {
            dt: 0.01,
            i_ext: 15.0,
            n_neurons: 4,
            g_syn: 0.5, // 强耦合
            e_syn: 0.0,
            coupling: CouplingType::Electrical,
            c: HH_C,
            g_na: HH_G_NA,
            g_k: HH_G_K,
            g_l: HH_G_L,
            e_na: HH_E_NA,
            e_k: HH_E_K,
            e_l: HH_E_L,
        };
        let mut net = HhNetworkSolver::new(cfg);
        net.connect_all();
        // 给神经元 0 一个初始扰动
        net.states[0].v = -55.0;
        net.step_n(10000); // 100 ms
        assert!(!net.has_nan());
        // 同步后所有神经元 V 应相近
        let v0 = net.states[0].v;
        let v1 = net.states[1].v;
        assert!((v0 - v1).abs() < 20.0, "electrical coupling should synchronize: v0={}, v1={}", v0, v1);
    }
}
