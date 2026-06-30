//! 肌肉骨骼系统模块 (细胞级) — 肌节动力学与肌肉力-速度关系
//!
//! 生物学背景:
//!   骨骼肌纤维由肌节 (sarcomere) 串联构成,肌节内粗肌丝 (肌球蛋白) 与
//!   细肌丝 (肌动蛋白) 通过横桥 (cross-bridge) 相互作用产生张力与缩短。
//!   肌纤维按收缩速度与代谢特征分为 I 型 (慢氧化)、IIa (快氧化)、
//!   IIx (快糖酵解)、IIb (极快糖酵解) 四类。
//!
//! 论文来源:
//!   - Huxley A.F. (1957). "Muscle structure and theories of contraction."
//!     Prog. Biophys. Biophys. Chem. 7:255-318. (横桥状态方程 Eq.1)
//!   - Hill A.V. (1938). "The heat of shortening and the dynamic constants
//!     of muscle." Proc. R. Soc. B 126:136-195. (力-速度方程, 1922 Nobel)
//!   - Gordon A.M., Huxley A.F., Julian F.J. (1966). "The variation in
//!     isometric tension with sarcomere length in vertebrate muscle fibres."
//!     J. Physiol. 184:170-192. (长度-张力曲线)
//!   - Schiaffino S., Reggiani C. (2011). "Fiber types in mammalian skeletal
//!     muscles." Physiol. Rev. 91:1447-1531.

use serde::{Deserialize, Serialize};

/// 肌纤维类型 (Schiaffino & Reggiani 2011 分类)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiberType {
    /// I 型 — 慢氧化, 高耐力 (MHC I)
    TypeI,
    /// IIa 型 — 快氧化糖酵解 (MHC IIa)
    TypeIIa,
    /// IIx 型 — 快糖酵解 (MHC IIx, 人类)
    TypeIIx,
    /// IIb 型 — 极快糖酵解 (MHC IIb, 啮齿类)
    TypeIIb,
}

impl FiberType {
    /// 最大缩短速度 (肌节长度/秒, Schiaffino 2011 Table 1)
    pub fn v_max_per_s(&self) -> f32 {
        match self {
            Self::TypeI   => 2.0,
            Self::TypeIIa => 5.0,
            Self::TypeIIx => 8.0,
            Self::TypeIIb => 12.0,
        }
    }

    /// 比张力 (N/cm²)
    pub fn specific_tension_n_cm2(&self) -> f32 {
        match self {
            Self::TypeI   => 12.0,
            Self::TypeIIa => 18.0,
            Self::TypeIIx => 20.0,
            Self::TypeIIb => 22.0,
        }
    }

    /// ATP 酶活性相对值 (I 型 = 1.0)
    pub fn atpase_relative(&self) -> f32 {
        match self {
            Self::TypeI   => 1.0,
            Self::TypeIIa => 3.0,
            Self::TypeIIx => 5.0,
            Self::TypeIIb => 7.0,
        }
    }
}

/// 横桥状态 (Huxley 1957)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CrossBridge {
    /// 附着率 α (1/s)
    pub alpha: f32,
    /// 脱离率 β (1/s)
    pub beta: f32,
    /// 当前附着比例 n (0..1)
    pub fraction_attached: f32,
}

impl CrossBridge {
    pub fn new(alpha: f32, beta: f32) -> Self {
        Self { alpha, beta, fraction_attached: 0.0 }
    }

    /// 稳态附着比例 n* = α/(α+β) (Huxley 1957 Eq.2 稳态解)
    pub fn steady_state(&self) -> f32 {
        self.alpha / (self.alpha + self.beta)
    }

    /// 时间常数 τ = 1/(α+β)
    pub fn time_constant(&self) -> f32 {
        1.0 / (self.alpha + self.beta)
    }

    /// Huxley 状态方程迭代 (显式 Euler)
    /// dn/dt = α(1-n) - βn   (Huxley 1957 Eq.1)
    pub fn step(&mut self, dt: f32) {
        let n = self.fraction_attached;
        let dn_dt = self.alpha * (1.0 - n) - self.beta * n;
        self.fraction_attached = (n + dn_dt * dt).clamp(0.0, 1.0);
    }
}

impl Default for CrossBridge {
    fn default() -> Self {
        // 典型哺乳动物横桥速率 (Huxley 1957 Table 1)
        Self::new(50.0, 50.0)
    }
}

/// 肌节 (Gordon 1966 长度-张力曲线)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sarcomere {
    /// 当前长度 (μm), 最佳 2.2 μm
    pub length_um: f32,
    /// 钙离子浓度 (μM)
    pub calcium_um: f32,
    /// 横桥状态
    pub cross_bridge: CrossBridge,
}

impl Sarcomere {
    /// 最佳长度 (Gordon 1966 Plateau 区中心)
    pub const OPTIMAL_LENGTH_UM: f32 = 2.2;
    /// 最大激活钙浓度阈值 (μM)
    pub const CALCIUM_ACTIVATION_UM: f32 = 1.0;

    pub fn new() -> Self {
        Self {
            length_um: Self::OPTIMAL_LENGTH_UM,
            calcium_um: 0.0,
            cross_bridge: CrossBridge::default(),
        }
    }

    /// 长度-张力关系 (Gordon 1966)
    /// 返回相对张力 (0..1)
    pub fn length_tension_factor(&self) -> f32 {
        // Plateau: 2.0 - 2.25 μm → 1.0
        // 上升支: 1.27 - 2.0 μm → 线性 0..1
        // 下降支: 2.25 - 3.6 μm → 线性 1..0
        let l = self.length_um;
        if l < 1.27 || l > 3.6 {
            0.0
        } else if l < 2.0 {
            (l - 1.27) / (2.0 - 1.27)
        } else if l <= 2.25 {
            1.0
        } else {
            (3.6 - l) / (3.6 - 2.25)
        }
    }

    /// 钙激活因子 (Hill 型 cooperative activation)
    pub fn calcium_activation(&self) -> f32 {
        // Hill 系数 n=2, K_d=1.0 μM
        let n = 2.0;
        let k_d = Self::CALCIUM_ACTIVATION_UM;
        let c = self.calcium_um;
        c.powf(n) / (c.powf(n) + k_d.powf(n))
    }

    /// 单肌节产生的张力 (相对单位 0..1)
    pub fn active_tension_relative(&self) -> f32 {
        self.length_tension_factor() * self.calcium_activation() * self.cross_bridge.fraction_attached
    }

    /// 推进一个时间步 (横桥动力学)
    pub fn step(&mut self, dt: f32) {
        // 钙浓度驱动横桥附着率
        let calcium_factor = self.calcium_activation();
        let mut cb = self.cross_bridge;
        cb.alpha = 50.0 * calcium_factor;
        cb.step(dt);
        self.cross_bridge = cb;
    }
}

impl Default for Sarcomere {
    fn default() -> Self { Self::new() }
}

/// 肌纤维
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MuscleFiber {
    pub fiber_type: FiberType,
    /// 肌节数量 (串联)
    pub sarcomere_count: u32,
    /// 单肌节
    pub sarcomere: Sarcomere,
    /// 横截面积 (cm²)
    pub cross_section_cm2: f32,
}

impl MuscleFiber {
    pub fn new(fiber_type: FiberType) -> Self {
        Self {
            fiber_type,
            sarcomere_count: 1000,
            sarcomere: Sarcomere::new(),
            cross_section_cm2: 0.001,
        }
    }

    /// 最大等长张力 (N) = 比张力 × 截面积
    pub fn f_max_n(&self) -> f32 {
        self.fiber_type.specific_tension_n_cm2() * self.cross_section_cm2
    }

    /// 最大缩短速度 (μm/s) = V_max × 肌节数
    pub fn v_max_um_s(&self) -> f32 {
        self.fiber_type.v_max_per_s() * Self::SARCOMERE_LENGTH_UM * self.sarcomere_count as f32
    }

    pub const SARCOMERE_LENGTH_UM: f32 = 2.2;
}

impl Default for MuscleFiber {
    fn default() -> Self { Self::new(FiberType::TypeI) }
}

/// Hill 肌肉模型 (Hill 1938 力-速度方程)
/// F = (F_max · b + v · a) / (b + v)
/// v < 0 表示向心缩短, v > 0 表示离心拉长
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HillMuscle {
    /// 最大等长力 (N)
    pub f_max: f32,
    /// Hill 系数 a (N)
    pub a: f32,
    /// Hill 系数 b (速度单位, μm/s)
    pub b: f32,
    /// 当前收缩速度 (μm/s), 负值=缩短
    pub velocity_um_s: f32,
}

impl HillMuscle {
    pub fn new(f_max: f32) -> Self {
        // 典型 a/F_max = 0.25 (Hill 1938)
        let a = 0.25 * f_max;
        // b ≈ V_max × a/F_max = V_max × 0.25
        let v_max = 5000.0; // μm/s 典型
        let b = 0.25 * v_max;
        Self { f_max, a, b, velocity_um_s: 0.0 }
    }

    /// 当前力 (N) — Hill 1938 方程
    /// velocity_um_s < 0 表示向心收缩（缩短），> 0 表示离心收缩（拉长）
    /// 标准形式（缩短速度 v_s > 0）: F = (F_max·b - v_s·a)/(b + v_s)
    /// 代入 v_s = -v: F = (F_max·b + v·a)/(b - v)
    pub fn force_n(&self) -> f32 {
        let v = self.velocity_um_s;
        let denom = self.b - v;
        if denom.abs() < 1e-6 {
            return self.f_max;
        }
        (self.f_max * self.b + v * self.a) / denom
    }

    /// 最大缩短速度 (μm/s) — 此速度下 F = 0
    /// V_max = -F_max · b / a
    pub fn v_max_um_s(&self) -> f32 {
        -self.f_max * self.b / self.a
    }
}

impl Default for HillMuscle {
    fn default() -> Self { Self::new(10.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 默认值测试 ---

    #[test]
    fn test_sarcomere_default() {
        let s = Sarcomere::default();
        assert_eq!(s.length_um, Sarcomere::OPTIMAL_LENGTH_UM);
        assert_eq!(s.calcium_um, 0.0);
        assert_eq!(s.cross_bridge.fraction_attached, 0.0);
    }

    #[test]
    fn test_cross_bridge_default() {
        let cb = CrossBridge::default();
        assert_eq!(cb.alpha, 50.0);
        assert_eq!(cb.beta, 50.0);
        assert_eq!(cb.fraction_attached, 0.0);
    }

    #[test]
    fn test_muscle_fiber_default() {
        let mf = MuscleFiber::default();
        assert_eq!(mf.fiber_type, FiberType::TypeI);
        assert!(mf.sarcomere_count > 0);
        assert!(mf.cross_section_cm2 > 0.0);
    }

    #[test]
    fn test_hill_muscle_default() {
        let h = HillMuscle::default();
        assert_eq!(h.f_max, 10.0);
        assert_eq!(h.a, 2.5);          // 0.25 * 10
        assert!(h.b > 0.0);
        assert_eq!(h.velocity_um_s, 0.0);
    }

    // --- 横桥动力学 (Huxley 1957) ---

    #[test]
    fn test_cross_bridge_steady_state() {
        let cb = CrossBridge::new(40.0, 60.0);
        // n* = α/(α+β) = 40/100 = 0.4
        assert!((cb.steady_state() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_cross_bridge_time_constant() {
        let cb = CrossBridge::new(40.0, 60.0);
        // τ = 1/(α+β) = 1/100 = 0.01 s
        assert!((cb.time_constant() - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_cross_bridge_step_increases_attachment() {
        let mut cb = CrossBridge::new(50.0, 10.0);
        cb.fraction_attached = 0.0;
        cb.step(0.01);
        // dn/dt = 50*(1-0) - 10*0 = 50; n_new = 0 + 50*0.01 = 0.5
        assert!(cb.fraction_attached > 0.4 && cb.fraction_attached < 0.6);
    }

    #[test]
    fn test_cross_bridge_step_clamps_to_one() {
        let mut cb = CrossBridge::new(1000.0, 0.0);
        cb.fraction_attached = 0.99;
        cb.step(1.0);
        assert!(cb.fraction_attached <= 1.0);
    }

    #[test]
    fn test_cross_bridge_step_converges_to_steady_state() {
        let mut cb = CrossBridge::new(50.0, 50.0);
        let target = cb.steady_state(); // 0.5
        for _ in 0..1000 {
            cb.step(0.01);
        }
        assert!((cb.fraction_attached - target).abs() < 0.05);
    }

    // --- 长度-张力关系 (Gordon 1966) ---

    #[test]
    fn test_length_tension_at_optimal() {
        let s = Sarcomere { length_um: 2.2, calcium_um: 1.0, cross_bridge: CrossBridge::default() };
        assert!((s.length_tension_factor() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_length_tension_too_short() {
        let s = Sarcomere { length_um: 1.0, calcium_um: 1.0, cross_bridge: CrossBridge::default() };
        assert_eq!(s.length_tension_factor(), 0.0);
    }

    #[test]
    fn test_length_tension_too_long() {
        let s = Sarcomere { length_um: 4.0, calcium_um: 1.0, cross_bridge: CrossBridge::default() };
        assert_eq!(s.length_tension_factor(), 0.0);
    }

    #[test]
    fn test_length_tension_plateau() {
        let s = Sarcomere { length_um: 2.1, calcium_um: 1.0, cross_bridge: CrossBridge::default() };
        assert!((s.length_tension_factor() - 1.0).abs() < 1e-6);
    }

    // --- 钙激活 ---

    #[test]
    fn test_calcium_activation_zero() {
        let s = Sarcomere::default();
        assert_eq!(s.calcium_activation(), 0.0);
    }

    #[test]
    fn test_calcium_activation_high() {
        let mut s = Sarcomere::default();
        s.calcium_um = 10.0;
        let act = s.calcium_activation();
        assert!(act > 0.9 && act < 1.0);
    }

    // --- Hill 1938 力-速度方程 ---

    #[test]
    fn test_hill_zero_velocity_isometric() {
        let h = HillMuscle::new(10.0);
        // v=0 → F = F_max·b/b = F_max
        let f = h.force_n();
        assert!((f - 10.0).abs() < 1e-3);
    }

    #[test]
    fn test_hill_max_velocity_zero_force() {
        let mut h = HillMuscle::new(10.0);
        let v_max = h.v_max_um_s();
        h.velocity_um_s = v_max; // 最大缩短速度
        let f = h.force_n();
        assert!(f.abs() < 1e-3, "force at V_max should be ~0, got {}", f);
    }

    #[test]
    fn test_hill_concentric_force_decreases_with_speed() {
        let mut h1 = HillMuscle::new(10.0);
        h1.velocity_um_s = -100.0;
        let f1 = h1.force_n();
        let mut h2 = h1;
        h2.velocity_um_s = -500.0;
        let f2 = h2.force_n();
        // 更快缩短 → 更小力
        assert!(f2 < f1);
        assert!(f1 < 10.0); // 都小于等长力
    }

    #[test]
    fn test_hill_v_max_formula() {
        let h = HillMuscle::new(10.0);
        // V_max = -F_max·b/a = -10·1250/2.5 = -5000
        let expected = -10.0 * h.b / h.a;
        assert!((h.v_max_um_s() - expected).abs() < 1e-3);
    }

    // --- 纤维类型属性 ---

    #[test]
    fn test_fiber_type_v_max_ordering() {
        // I < IIa < IIx < IIb
        assert!(FiberType::TypeI.v_max_per_s() < FiberType::TypeIIa.v_max_per_s());
        assert!(FiberType::TypeIIa.v_max_per_s() < FiberType::TypeIIx.v_max_per_s());
        assert!(FiberType::TypeIIx.v_max_per_s() < FiberType::TypeIIb.v_max_per_s());
    }

    #[test]
    fn test_fiber_type_specific_tension_ordering() {
        // I < IIa <= IIx <= IIb
        assert!(FiberType::TypeI.specific_tension_n_cm2() < FiberType::TypeIIa.specific_tension_n_cm2());
    }

    #[test]
    fn test_muscle_fiber_f_max_calculation() {
        let mf = MuscleFiber::new(FiberType::TypeI);
        // f_max = 12 N/cm² * 0.001 cm² = 0.012 N
        let f = mf.f_max_n();
        assert!((f - 0.012).abs() < 1e-6);
    }

    #[test]
    fn test_muscle_fiber_v_max_positive() {
        let mf = MuscleFiber::new(FiberType::TypeIIb);
        let v = mf.v_max_um_s();
        assert!(v > 0.0);
    }

    // --- 肌节整合测试 ---

    #[test]
    fn test_sarcomere_step_with_calcium() {
        let mut s = Sarcomere::default();
        s.calcium_um = 5.0;
        s.cross_bridge.fraction_attached = 0.0;
        s.step(0.1);
        // 钙驱动 α 增大,横桥应开始附着
        assert!(s.cross_bridge.fraction_attached > 0.0);
        assert!(s.cross_bridge.alpha > 0.0);
    }

    #[test]
    fn test_sarcomere_active_tension_zero_at_rest() {
        let s = Sarcomere::default();
        // 钙 0,横桥 0 → 张力 0
        assert_eq!(s.active_tension_relative(), 0.0);
    }

    #[test]
    fn test_sarcomere_active_tension_maximal() {
        let mut s = Sarcomere::default();
        s.calcium_um = 100.0;
        s.cross_bridge.fraction_attached = 1.0;
        let t = s.active_tension_relative();
        assert!((t - 1.0).abs() < 0.01);
    }
}
