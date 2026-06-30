//! 神经系统模块 — 基于 Hodgkin-Huxley 模型
//!
//! 科学来源:
//! - Hodgkin, A. L., & Huxley, A. F. (1952). "A quantitative description of membrane
//!   current and its application to conduction and excitation in nerve." J. Physiol. 117(4): 500-544.
//! - Kandel, E. R., Schwartz, J. H., Jessell, T. M. (2012). "Principles of Neural Science",
//!   5th edition. McGraw-Hill.
//! - Dayan, P., & Abbott, L. F. (2001). "Theoretical Neuroscience." MIT Press.
//!
//! HH 模型描述了神经元膜电位的非线性动力学:
//!   C_m * dV/dt = I_ext - g_na * m^3 * h * (V - E_na)
//!                       - g_k  * n^4     * (V - E_k)
//!                       - g_l            * (V - E_l)
//!
//! 经典参数 (乌贼巨轴突, 1952):
//!   g_na = 120 mS/cm^2, g_k = 36 mS/cm^2, g_l = 0.3 mS/cm^2
//!   E_na =  50 mV,      E_k = -77 mV,     E_l = -54.4 mV
//!   C_m  =   1 uF/cm^2

use serde::{Deserialize, Serialize};

/// HH 门控变量 (m/h/n), 取值范围 [0, 1]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GatingVariables {
    /// 钠通道激活门 (m)
    pub m: f32,
    /// 钠通道失活门 (h)
    pub h: f32,
    /// 钾通道激活门 (n)
    pub n: f32,
}

impl GatingVariables {
    /// 静息态 (V = -65 mV) 时的稳态门控值
    pub fn resting() -> Self {
        Self {
            m: 0.0529, // m_inf(-65)
            h: 0.5961, // h_inf(-65)
            n: 0.3177, // n_inf(-65)
        }
    }

    /// 将所有门控变量钳制到 [0,1] 范围内
    pub fn clamp(&mut self) {
        self.m = self.m.clamp(0.0, 1.0);
        self.h = self.h.clamp(0.0, 1.0);
        self.n = self.n.clamp(0.0, 1.0);
    }
}

impl Default for GatingVariables {
    fn default() -> Self {
        Self::resting()
    }
}

/// 神经递质类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NeurotransmitterType {
    /// 谷氨酸 — 主要兴奋性递质
    Glutamate,
    /// GABA — 主要抑制性递质
    GABA,
    /// 多巴胺 — 奖励与运动
    Dopamine,
    /// 血清素 — 情绪与节律
    Serotonin,
    /// 乙酰胆碱 — 神经肌肉接头
    Acetylcholine,
    /// 去甲肾上腺素 — 觉醒
    Norepinephrine,
}

impl NeurotransmitterType {
    /// 默认突触强度 (无量纲)
    pub fn default_strength(&self) -> f32 {
        match self {
            Self::Glutamate => 1.0,
            Self::GABA => -0.8,
            Self::Dopamine => 0.5,
            Self::Serotonin => 0.3,
            Self::Acetylcholine => 0.7,
            Self::Norepinephrine => 0.6,
        }
    }

    /// 是否为兴奋性递质
    pub fn is_excitatory(&self) -> bool {
        matches!(self, Self::Glutamate | Self::Acetylcholine | Self::Dopamine)
    }
}

/// 化学突触
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Synapse {
    pub neurotransmitter: NeurotransmitterType,
    pub strength: f32,
    pub pre_synaptic_id: u64,
    pub post_synaptic_id: u64,
    /// 突触延迟 (ms)
    pub delay_ms: f32,
}

impl Synapse {
    pub fn new(
        neurotransmitter: NeurotransmitterType,
        pre: u64,
        post: u64,
    ) -> Self {
        Self {
            neurotransmitter,
            strength: neurotransmitter.default_strength(),
            pre_synaptic_id: pre,
            post_synaptic_id: post,
            delay_ms: 1.0,
        }
    }
}

/// Hodgkin-Huxley 神经元模型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuronModel {
    /// 最大钠电导 (mS/cm^2)
    pub g_na: f32,
    /// 最大钾电导 (mS/cm^2)
    pub g_k: f32,
    /// 漏电导 (mS/cm^2)
    pub g_l: f32,
    /// 钠反转电位 (mV)
    pub e_na: f32,
    /// 钾反转电位 (mV)
    pub e_k: f32,
    /// 漏反转电位 (mV)
    pub e_l: f32,
    /// 膜电容 (uF/cm^2)
    pub c_m: f32,
    /// 当前膜电位 (mV)
    pub v: f32,
    /// 外部注入电流 (uA/cm^2)
    pub i_ext: f32,
    /// 神经元唯一标识
    pub id: u64,
}

impl NeuronModel {
    /// 创建 HH 默认参数的神经元 (乌贼巨轴突, 1952)
    pub fn new(id: u64) -> Self {
        Self {
            g_na: 120.0,
            g_k: 36.0,
            g_l: 0.3,
            e_na: 50.0,
            e_k: -77.0,
            e_l: -54.4,
            c_m: 1.0,
            v: -65.0,
            i_ext: 0.0,
            id,
        }
    }

    /// 动作电位阈值 (Kandel 5th ed., ~ -55 mV)
    pub fn action_potential_threshold() -> f32 {
        -55.0
    }

    /// 静息膜电位 (~ -65 mV)
    pub fn resting_potential() -> f32 {
        -65.0
    }

    /// 是否正在放电 (膜电位超过阈值)
    pub fn is_firing(&self) -> bool {
        self.v >= Self::action_potential_threshold()
    }

    /// 强制触发一次发放: 将膜电位拉到 +30 mV
    pub fn fire(&mut self) {
        self.v = 30.0;
    }

    /// 钠通道 m 门稳态值与时间常数
    fn m_dynamics(v: f32) -> (f32, f32) {
        let x = v + 40.0;
        let alpha = if x.abs() < 1e-6 {
            1.0
        } else {
            0.1 * x / (1.0 - (-x / 10.0).exp())
        };
        let beta = 4.0 * (-(v + 65.0) / 18.0).exp();
        let tau = 1.0 / (alpha + beta);
        let inf = alpha / (alpha + beta);
        (inf, tau)
    }

    /// 钠通道 h 门动力学
    fn h_dynamics(v: f32) -> (f32, f32) {
        let alpha = 0.07 * (-(v + 65.0) / 20.0).exp();
        let beta = 1.0 / (1.0 + (-(v + 35.0) / 10.0).exp());
        let tau = 1.0 / (alpha + beta);
        let inf = alpha / (alpha + beta);
        (inf, tau)
    }

    /// 钾通道 n 门动力学
    fn n_dynamics(v: f32) -> (f32, f32) {
        let x = v + 55.0;
        let alpha = if x.abs() < 1e-6 {
            0.1
        } else {
            0.01 * x / (1.0 - (-x / 10.0).exp())
        };
        let beta = 0.125 * (-(v + 65.0) / 80.0).exp();
        let tau = 1.0 / (alpha + beta);
        let inf = alpha / (alpha + beta);
        (inf, tau)
    }

    /// 单步前向欧拉积分 (dt 单位: ms)
    /// 返回新的膜电位
    pub fn step(&mut self, gating: &mut GatingVariables, dt: f32) -> f32 {
        let v = self.v;

        // 门控变量更新
        let (m_inf, m_tau) = Self::m_dynamics(v);
        let (h_inf, h_tau) = Self::h_dynamics(v);
        let (n_inf, n_tau) = Self::n_dynamics(v);

        gating.m += dt * (m_inf - gating.m) / m_tau.max(1e-6);
        gating.h += dt * (h_inf - gating.h) / h_tau.max(1e-6);
        gating.n += dt * (n_inf - gating.n) / n_tau.max(1e-6);
        gating.clamp();

        // 离子电流
        let i_na = self.g_na * gating.m.powi(3) * gating.h * (v - self.e_na);
        let i_k = self.g_k * gating.n.powi(4) * (v - self.e_k);
        let i_l = self.g_l * (v - self.e_l);

        // C_m * dV/dt = I_ext - I_ionic
        let dv = (self.i_ext - i_na - i_k - i_l) / self.c_m;
        self.v += dt * dv;
        self.v
    }

    /// 计算给定电位下的稳态门控 (用于诊断)
    pub fn steady_state_gating(v: f32) -> GatingVariables {
        let (m_inf, _) = Self::m_dynamics(v);
        let (h_inf, _) = Self::h_dynamics(v);
        let (n_inf, _) = Self::n_dynamics(v);
        GatingVariables {
            m: m_inf,
            h: h_inf,
            n: n_inf,
        }
    }
}

impl Default for NeuronModel {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_default_conductances() {
        let n = NeuronModel::default();
        assert_eq!(n.g_na, 120.0);
        assert_eq!(n.g_k, 36.0);
        assert_eq!(n.g_l, 0.3);
    }

    #[test]
    fn test_neuron_default_reversal_potentials() {
        let n = NeuronModel::default();
        assert_eq!(n.e_na, 50.0);
        assert_eq!(n.e_k, -77.0);
        assert_eq!(n.e_l, -54.4);
    }

    #[test]
    fn test_membrane_capacitance_default() {
        assert_eq!(NeuronModel::default().c_m, 1.0);
    }

    #[test]
    fn test_resting_potential_constant() {
        assert_eq!(NeuronModel::resting_potential(), -65.0);
    }

    #[test]
    fn test_action_potential_threshold_constant() {
        assert_eq!(NeuronModel::action_potential_threshold(), -55.0);
    }

    #[test]
    fn test_default_initial_membrane_voltage() {
        let n = NeuronModel::default();
        assert_eq!(n.v, -65.0);
    }

    #[test]
    fn test_is_firing_below_threshold() {
        let n = NeuronModel::default();
        assert!(!n.is_firing());
    }

    #[test]
    fn test_is_firing_at_threshold() {
        let mut n = NeuronModel::default();
        n.v = -55.0;
        assert!(n.is_firing());
    }

    #[test]
    fn test_is_firing_above_threshold() {
        let mut n = NeuronModel::default();
        n.v = 0.0;
        assert!(n.is_firing());
    }

    #[test]
    fn test_fire_sets_potential() {
        let mut n = NeuronModel::default();
        n.fire();
        assert_eq!(n.v, 30.0);
        assert!(n.is_firing());
    }

    #[test]
    fn test_gating_resting_in_unit_range() {
        let g = GatingVariables::resting();
        assert!(g.m >= 0.0 && g.m <= 1.0);
        assert!(g.h >= 0.0 && g.h <= 1.0);
        assert!(g.n >= 0.0 && g.n <= 1.0);
    }

    #[test]
    fn test_gating_clamp_preserves_in_range() {
        let mut g = GatingVariables::resting();
        g.clamp();
        assert!(g.m <= 1.0 && g.h <= 1.0 && g.n <= 1.0);
    }

    #[test]
    fn test_gating_clamp_caps_overflow() {
        let mut g = GatingVariables {
            m: 5.0,
            h: -3.0,
            n: 0.5,
        };
        g.clamp();
        assert_eq!(g.m, 1.0);
        assert_eq!(g.h, 0.0);
        assert_eq!(g.n, 0.5);
    }

    #[test]
    fn test_steady_state_gating_at_rest() {
        let g = NeuronModel::steady_state_gating(-65.0);
        assert!(g.m > 0.0 && g.m < 0.1);
        assert!(g.h > 0.5 && g.h < 0.7);
        assert!(g.n > 0.3 && g.n < 0.4);
    }

    #[test]
    fn test_step_no_input_stays_near_rest() {
        let mut n = NeuronModel::default();
        let mut g = GatingVariables::resting();
        for _ in 0..1000 {
            n.step(&mut g, 0.01);
        }
        assert!(n.v > -67.0 && n.v < -63.0);
    }

    #[test]
    fn test_step_with_strong_input_fires() {
        let mut n = NeuronModel::default();
        n.i_ext = 20.0; // uA/cm^2, 远超阈值
        let mut g = GatingVariables::resting();
        let mut fired = false;
        for _ in 0..2000 {
            let v = n.step(&mut g, 0.01);
            if v >= 0.0 {
                fired = true;
                break;
            }
        }
        assert!(fired, "强刺激下应产生动作电位");
    }

    #[test]
    fn test_step_with_subthreshold_input_no_fire() {
        let mut n = NeuronModel::default();
        n.i_ext = 1.0; // 远低于阈值
        let mut g = GatingVariables::resting();
        let mut fired = false;
        for _ in 0..2000 {
            let v = n.step(&mut g, 0.01);
            if v >= 0.0 {
                fired = true;
                break;
            }
        }
        assert!(!fired, "弱刺激不应产生动作电位");
    }

    #[test]
    fn test_neurotransmitter_excitatory_classification() {
        assert!(NeurotransmitterType::Glutamate.is_excitatory());
        assert!(NeurotransmitterType::Acetylcholine.is_excitatory());
        assert!(!NeurotransmitterType::GABA.is_excitatory());
    }

    #[test]
    fn test_neurotransmitter_default_strength_signs() {
        assert!(NeurotransmitterType::Glutamate.default_strength() > 0.0);
        assert!(NeurotransmitterType::GABA.default_strength() < 0.0);
    }

    #[test]
    fn test_synapse_new_uses_neurotransmitter_strength() {
        let s = Synapse::new(NeurotransmitterType::Glutamate, 1, 2);
        assert_eq!(s.strength, NeurotransmitterType::Glutamate.default_strength());
        assert_eq!(s.pre_synaptic_id, 1);
        assert_eq!(s.post_synaptic_id, 2);
    }

    #[test]
    fn test_neuron_id_preserved() {
        let n = NeuronModel::new(42);
        assert_eq!(n.id, 42);
    }

    #[test]
    fn test_neuron_serialization_roundtrip() {
        let n = NeuronModel::default();
        let json = serde_json::to_string(&n).expect("serialize");
        let back: NeuronModel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(n, back);
    }

    #[test]
    fn test_gating_serialization_roundtrip() {
        let g = GatingVariables::resting();
        let json = serde_json::to_string(&g).expect("serialize");
        let back: GatingVariables = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(g, back);
    }
}
