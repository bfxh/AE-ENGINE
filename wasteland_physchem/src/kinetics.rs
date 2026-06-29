//! kinetics.rs - 化学动力学模块
//!
//! 核心内容：
//! 1. Arrhenius 方程：k = A·T^n·exp(-Ea/RT)
//! 2. Eyring 过渡态理论：k = κ·(kB·T/h)·exp(-ΔG‡/RT)
//! 3. Evans-Polanyi 线性自由能关系
//! 4. Marcus 电子转移理论（含反转区）
//! 5. 速率定律与积分速率方程
//! 6. 平衡常数：van't Hoff / Kc-Kp 转换
//!
//! 物理化学符号允许 non_snake_case（Ea, ΔG‡ 等）。

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

/// 气体常数 J/(mol·K)，重复定义以便自包含
pub const R_GAS: f64 = 8.314462618;
/// 玻尔兹曼常数 J/K
pub const KB: f64 = 1.380649e-23;
/// 普朗克常数 J·s
pub const H_PLANCK: f64 = 6.62607015e-34;
/// 约化普朗克常数 J·s
pub const HBAR: f64 = 1.054571817e-34;
/// 电子电荷 C
pub const E_CHARGE: f64 = 1.602176634e-19;
/// 阿伏伽德罗数 /mol
pub const NA: f64 = 6.02214076e23;

// ============================================================================
// 2.1 Arrhenius 动力学
// ============================================================================

/// Arrhenius 参数
/// k = A · T^n · exp(-Ea/(RT))
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ArrheniusParams {
    /// 指前因子 (1/s 或 M⁻¹·s⁻¹)
    pub a: f64,
    /// 活化能 (kJ/mol)
    pub ea: f64,
    /// 温度指数（T^n，通常 0）
    pub n: f64,
}

impl Default for ArrheniusParams {
    fn default() -> Self {
        Self { a: 1.0e10, ea: 50.0, n: 0.0 }
    }
}

impl ArrheniusParams {
    /// 计算速率常数 k
    /// 注意：Ea 单位为 kJ/mol，内部转换为 J/mol
    pub fn rate_constant(&self, t: f64) -> f64 {
        // k = A · T^n · exp(-Ea/(RT))
        self.a * t.powf(self.n) * (-(self.ea * 1000.0) / (R_GAS * t)).exp()
    }

    /// 从两点 (T1, k1) 和 (T2, k2) 反推 Arrhenius 参数（假设 n=0）
    /// ln(k2/k1) = -Ea/R · (1/T2 - 1/T1)
    pub fn from_two_points(t1: f64, k1: f64, t2: f64, k2: f64) -> Self {
        let ea = -R_GAS * (k2 / k1).ln() / (1.0 / t2 - 1.0 / t1) / 1000.0;
        // A = k · exp(Ea/RT)
        let a = k1 * ((ea * 1000.0) / (R_GAS * t1)).exp();
        Self { a, ea, n: 0.0 }
    }

    /// 半衰期
    /// - 一级：t_1/2 = ln2/k
    /// - 二级：t_1/2 = 1/(k·[A]0)
    /// - 零级：t_1/2 = [A]0/(2k)
    pub fn half_life(&self, t: f64, order: u8, conc: f64) -> f64 {
        let k = self.rate_constant(t);
        match order {
            0 => conc / (2.0 * k),
            1 => std::f64::consts::LN_2 / k,
            2 => 1.0 / (k * conc),
            _ => f64::NAN,
        }
    }
}
// ============================================================================
// 2.2 过渡态理论 (Eyring 方程)
// ============================================================================

/// Eyring 方程：k = κ · (kB·T/h) · exp(-ΔG‡/(RT))
/// delta_g_dagger: ΔG‡ (kJ/mol)
/// t: 温度 (K)
/// kappa: 传输系数（通常 1.0）
pub fn eyring_rate_constant(delta_g_dagger: f64, t: f64, kappa: f64) -> f64 {
    // k = κ · (kB·T/h) · exp(-ΔG‡/(RT))
    let kb_t_h = KB * t / H_PLANCK;
    kb_t_h * kappa * (-delta_g_dagger * 1000.0 / (R_GAS * t)).exp()
}

/// 从 Arrhenius 活化能 Ea 推算 Eyring 速率常数
/// 关系：Ea = ΔH‡ + RT（气相）或 Ea = ΔH‡ + RT（溶液）
/// dh_dagger: ΔH‡ (kJ/mol)
/// ea: Arrhenius Ea (kJ/mol)
/// 返回 ΔG‡ (kJ/mol)（需要额外熵信息，此处用 Ea 近似）
pub fn eyring_from_ea(ea: f64, t: f64, dh_dagger: f64) -> f64 {
    // ΔG‡ = ΔH‡ - T·ΔS‡
    // Arrhenius: Ea = ΔH‡ + RT (溶液)
    // 这里用 Ea 反推 ΔH‡，再用 Eyring 算 k
    // 假设 ΔS‡ = 0（最简近似），则 ΔG‡ ≈ ΔH‡
    let _ = (ea, dh_dagger);
    // 简化：用 Ea 作为 ΔG‡ 的上界估计
    let dg_dagger = dh_dagger.max(0.0);
    eyring_rate_constant(dg_dagger, t, 1.0)
}

// ============================================================================
// 2.3 Evans-Polanyi 线性自由能关系
// ============================================================================

/// Evans-Polanyi 关系：Ea = E0 + α·ΔH_rxn
/// e0: 参考反应（热中性）活化能
/// alpha: 转移系数 (0-1)
/// dh_rxn: 反应焓变 (kJ/mol)
pub fn evans_polanyi_ea(e0: f64, alpha: f64, dh_rxn: f64) -> f64 {
    e0 + alpha * dh_rxn
}

/// Marcus 理论活化能（简化形式）
/// ΔG‡ = (λ + ΔG)² / (4λ)
/// lambda: 重组能 (kJ/mol)
/// dg: 反应自由能变 ΔG (kJ/mol)
pub fn marcus_ea(lambda: f64, dg: f64) -> f64 {
    if lambda.abs() < 1e-10 {
        return 0.0;
    }
    (lambda + dg).powi(2) / (4.0 * lambda)
}
// ============================================================================
// 2.4 Marcus 电子转移理论
// ============================================================================

/// Marcus 电子转移速率常数
/// k_et = (2π/ℏ) · |Hab|² · (1/√(4πλkBT)) · exp(-(ΔG+λ)²/(4λkBT))
///
/// 注意：这里使用 eV 为输入单位，内部转换为 SI
/// lambda: 重组能 (eV)
/// delta_g: ΔG (eV)
/// hab: 电子耦合 (eV)
/// t: 温度 (K)
pub fn marcus_electron_transfer_rate(
    lambda: f64,
    delta_g: f64,
    hab: f64,
    t: f64,
) -> f64 {
    if lambda <= 0.0 || t <= 0.0 {
        return 0.0;
    }
    // eV → J
    let lambda_j = lambda * E_CHARGE;
    let dg_j = delta_g * E_CHARGE;
    let hab_j = hab * E_CHARGE;
    let kbt = KB * t;

    // 前因子：(2π/ℏ) · |Hab|²
    let pre = 2.0 * std::f64::consts::PI / HBAR;
    let coupling = hab_j * hab_j;

    // 归一化因子：1/√(4πλkBT)
    let denom = (4.0 * std::f64::consts::PI * lambda_j * kbt).sqrt();

    // 指数因子：exp(-(ΔG+λ)²/(4λkBT))
    let exponent = -(dg_j + lambda_j).powi(2) / (4.0 * lambda_j * kbt);

    pre * coupling / denom * exponent.exp()
}

/// Marcus 反转点：当 -ΔG = λ 时活化能最小
/// 反转区：-ΔG > λ 时，速率随 -ΔG 增大而减小
pub fn marcus_inversion_point(lambda: f64) -> f64 {
    -lambda
}

/// 判断是否在 Marcus 反转区
pub fn is_in_marcus_inverted_region(lambda: f64, delta_g: f64) -> bool {
    // 反转区条件：-ΔG > λ，即 ΔG < -λ
    delta_g < -lambda
}

// ============================================================================
// 2.5 速率定律
// ============================================================================

/// 反应级数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RateOrder {
    Zero,
    First,
    Second,
    /// 混合级数 (级数1, 级数2)
    Mixed(f64, f64),
    /// n 级反应
    Nth(f64),
}

impl RateOrder {
    /// 获取对单一反应物的级数
    pub fn order_value(&self) -> f64 {
        match self {
            RateOrder::Zero => 0.0,
            RateOrder::First => 1.0,
            RateOrder::Second => 2.0,
            RateOrder::Mixed(a, _) => *a,
            RateOrder::Nth(n) => *n,
        }
    }
}

/// 速率定律
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLaw {
    /// 速率常数 k
    pub k: f64,
    /// (反应物索引, 级数)
    pub orders: Vec<(usize, f64)>,
}

impl RateLaw {
    /// 计算瞬时速率
    /// rate = k · Π [A_i]^order_i
    pub fn rate(&self, concentrations: &[f64]) -> f64 {
        let mut r = self.k;
        for &(idx, order) in &self.orders {
            if idx < concentrations.len() {
                r *= concentrations[idx].powf(order);
            }
        }
        r
    }

    /// 一级反应积分：[A] = [A]0 · exp(-kt)
    pub fn integrated_first_order(c0: f64, k: f64, t: f64) -> f64 {
        c0 * (-k * t).exp()
    }

    /// 二级反应积分：1/[A] = 1/[A]0 + k·t
    pub fn integrated_second_order(c0: f64, k: f64, t: f64) -> f64 {
        1.0 / (1.0 / c0 + k * t)
    }

    /// 零级反应积分：[A] = [A]0 - k·t
    pub fn integrated_zero_order(c0: f64, k: f64, t: f64) -> f64 {
        (c0 - k * t).max(0.0)
    }

    /// 一级反应半衰期：t_1/2 = ln2/k
    pub fn half_life_first_order(k: f64) -> f64 {
        std::f64::consts::LN_2 / k
    }

    /// 二级反应半衰期：t_1/2 = 1/(k·[A]0)
    pub fn half_life_second_order(k: f64, c0: f64) -> f64 {
        1.0 / (k * c0)
    }
}
// ============================================================================
// 2.6 平衡常数
// ============================================================================

/// 从吉布斯自由能变计算平衡常数
/// ΔG° = -RT·ln(K)
/// K = exp(-ΔG°/(RT))
/// dg: ΔG° (kJ/mol)
pub fn equilibrium_constant_from_dg(dg: f64, t: f64) -> f64 {
    (-dg * 1000.0 / (R_GAS * t)).exp()
}

/// 从平衡常数反算吉布斯自由能变 (kJ/mol)
pub fn dg_from_equilibrium_constant(k: f64, t: f64) -> f64 {
    -R_GAS * t * k.ln() / 1000.0
}

/// van't Hoff 方程
/// ln(k2/k1) = -ΔH/R · (1/T2 - 1/T1)
/// k1: T1 下的平衡常数
/// dh: 反应焓变 (J/mol) 注意单位
/// 返回 T2 下的平衡常数
pub fn van_t_hoff(k1: f64, t1: f64, t2: f64, dh: f64) -> f64 {
    let factor = -dh / R_GAS * (1.0 / t2 - 1.0 / t1);
    k1 * factor.exp()
}

/// Kp 与 Kc 转换
/// Kp = Kc·(RT)^(Δn)
/// 或 Kc = Kp·(RT)^(-Δn)
/// dn_gas: 气体计量数变化 (产物 - 反应物)
pub fn kc_from_kp(kp: f64, t: f64, dn_gas: i32) -> f64 {
    kp * (R_GAS * t).powi(-dn_gas)
}

/// Kp 从 Kc
pub fn kp_from_kc(kc: f64, t: f64, dn_gas: i32) -> f64 {
    kc * (R_GAS * t).powi(dn_gas)
}

/// 反应商 Q 与平衡常数 K 比较，判断反应方向
/// Q < K: 正向自发
/// Q > K: 逆向自发
/// Q = K: 平衡
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionDirection {
    Forward,
    Reverse,
    AtEquilibrium,
}

pub fn reaction_direction(q: f64, k: f64) -> ReactionDirection {
    let ratio = q / k;
    if ratio < 0.9999 {
        ReactionDirection::Forward
    } else if ratio > 1.0001 {
        ReactionDirection::Reverse
    } else {
        ReactionDirection::AtEquilibrium
    }
}

/// 从初始浓度和平衡常数计算平衡浓度（简化，仅一级可逆反应）
/// A ⇌ B, K = [B]/[A]
pub fn equilibrium_concentrations(c_a0: f64, c_b0: f64, k: f64) -> (f64, f64) {
    // A ⇌ B, K = [B]/[A]
    // 设变化 x: [A]=c_a0-x, [B]=c_b0+x
    // K = (c_b0+x)/(c_a0-x)
    // x = (K·c_a0 - c_b0)/(K+1)
    let x = (k * c_a0 - c_b0) / (k + 1.0);
    (c_a0 - x, c_b0 + x)
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Arrhenius 速率常数验证
    /// 已知：A=1e10, Ea=50 kJ/mol, T=300K
    /// k = 1e10 · exp(-50000/(8.314·300)) ≈ 1.96e1
    #[test]
    fn test_arrhenius_rate_constant() {
        let p = ArrheniusParams { a: 1.0e10, ea: 50.0, n: 0.0 };
        let k = p.rate_constant(300.0);
        // k ≈ 1e10 · exp(-50/2.494) = 1e10 · exp(-20.04) ≈ 1.97
        assert!(k > 10.0 && k < 100.0,
            "Arrhenius k 应在 1-10 范围: got {:.4}", k);
    }

    /// Arrhenius 两点法反推 Ea
    /// 已知 T1=300, k1=2.0; T2=310, k2=4.0
    /// Ea = -R · ln(k2/k1) / (1/T2 - 1/T1)
    #[test]
    fn test_arrhenius_from_two_points() {
        let p = ArrheniusParams::from_two_points(300.0, 2.0, 310.0, 4.0);
        // ln(2) = 0.693; (1/310 - 1/300) = -1.0753e-4
        // Ea = -8.314 · 0.693 / (-1.0753e-4) / 1000 ≈ 53.6 kJ/mol
        assert!(p.ea > 40.0 && p.ea < 70.0,
            "反推 Ea 应在 40-70 kJ/mol: got {:.2}", p.ea);
        // 验证：用反推参数重算 k1 应接近 2.0
        let k1_recalc = p.rate_constant(300.0);
        assert!((k1_recalc - 2.0).abs() / 2.0 < 0.1,
            "k1 重算应接近 2.0: got {:.4}", k1_recalc);
    }

    /// Eyring 过渡态理论验证
    /// ΔG‡=50 kJ/mol, T=300K, κ=1
    /// k = (kB·T/h) · exp(-50000/(8.314·300))
    /// kB·T/h ≈ 6.25e12 s⁻¹
    #[test]
    fn test_eyring_rate_constant() {
        let k = eyring_rate_constant(50.0, 300.0, 1.0);
        // kB·T/h ≈ 1.3806e-23 · 300 / 6.626e-34 ≈ 6.25e12
        // exp(-50000/(8.314·300)) ≈ exp(-20.04) ≈ 2.0e-9
        // k ≈ 6.25e12 · 2.0e-9 ≈ 1.25e4
        assert!(k > 1.0e3 && k < 1.0e5,
            "Eyring k 应在 1e3-1e5: got {:.4e}", k);
    }

    /// Evans-Polanyi 关系验证
    /// E0=100, α=0.5, ΔH=-40 (放热)
    /// Ea = 100 + 0.5·(-40) = 80
    #[test]
    fn test_evans_polanyi() {
        let ea = evans_polanyi_ea(100.0, 0.5, -40.0);
        assert!((ea - 80.0).abs() < 1e-9,
            "Evans-Polanyi Ea 应为 80: got {:.2}", ea);
        // 放热反应 Ea 应降低
        assert!(ea < 100.0, "放热反应 Ea 应低于 E0");
    }

    /// Marcus 反转区验证
    /// 当 -ΔG > λ 时进入反转区
    /// λ=1.0 eV, ΔG=-0.5 eV (正常区), ΔG=-1.5 eV (反转区)
    #[test]
    fn test_marcus_inverted_region() {
        let lambda = 1.0;
        // 正常区：ΔG=-0.5
        let k_normal = marcus_electron_transfer_rate(lambda, -0.5, 0.01, 300.0);
        // 最佳点：ΔG=-1.0 (=-λ)
        let k_optimal = marcus_electron_transfer_rate(lambda, -1.0, 0.01, 300.0);
        // 反转区：ΔG=-1.5
        let k_inverted = marcus_electron_transfer_rate(lambda, -1.5, 0.01, 300.0);

        assert!(k_optimal > k_normal, "最佳点应快于正常区");
        assert!(k_optimal > k_inverted, "反转区应慢于最佳点");
        assert!(is_in_marcus_inverted_region(lambda, -1.5), "ΔG=-1.5 应在反转区");
        assert!(!is_in_marcus_inverted_region(lambda, -0.5), "ΔG=-0.5 不应在反转区");
    }

    /// Marcus 活化能验证
    /// λ=100 kJ/mol, ΔG=-50 kJ/mol
    /// ΔG‡ = (100-50)²/(4·100) = 2500/400 = 6.25
    #[test]
    fn test_marcus_ea() {
        let dg_dagger = marcus_ea(100.0, -50.0);
        assert!((dg_dagger - 6.25).abs() < 1e-6,
            "Marcus ΔG‡ 应为 6.25: got {:.4}", dg_dagger);
    }

    /// 一级反应积分方程验证
    /// [A]0=1.0, k=0.1, t=10 → [A]=exp(-1)≈0.368
    #[test]
    fn test_integrated_first_order() {
        let c = RateLaw::integrated_first_order(1.0, 0.1, 10.0);
        assert!((c - std::f64::consts::E.powi(-1)).abs() < 1e-9,
            "一级积分 [A] 应为 e⁻¹: got {:.6}", c);
    }

    /// 二级反应积分方程验证
    /// [A]0=1.0, k=0.1, t=10 → 1/[A]=1+1=2, [A]=0.5
    #[test]
    fn test_integrated_second_order() {
        let c = RateLaw::integrated_second_order(1.0, 0.1, 10.0);
        assert!((c - 0.5).abs() < 1e-9,
            "二级积分 [A] 应为 0.5: got {:.6}", c);
    }

    /// 平衡常数与 ΔG 关系验证
    /// ΔG°=-5.7 kJ/mol, T=298K → K ≈ 10
    /// K = exp(5700/(8.314·298)) ≈ exp(2.30) ≈ 10.0
    #[test]
    fn test_equilibrium_constant_from_dg() {
        let k = equilibrium_constant_from_dg(-5.7, 298.15);
        assert!((k - 10.0).abs() / 10.0 < 0.1,
            "K 应接近 10: got {:.4}", k);
    }

    /// van't Hoff 方程验证
    /// 吸热反应 (ΔH>0)，温度升高 K 增大
    #[test]
    fn test_van_t_hoff_endothermic() {
        let k1 = 1.0;
        let dh = 50000.0; // J/mol, 吸热
        let k2 = van_t_hoff(k1, 300.0, 310.0, dh);
        assert!(k2 > k1, "吸热反应升温 K 应增大: k2={:.4}", k2);
    }

    /// Kc-Kp 转换验证
    /// 反应 N2O4 ⇌ 2NO2, Δn=1
    #[test]
    fn test_kc_kp_conversion() {
        let kp = 1.0;
        let t = 298.15;
        let kc = kc_from_kp(kp, t, 1);
        // Kc = Kp/(RT)
        let expected = kp / (R_GAS * t);
        assert!((kc - expected).abs() / expected < 1e-9,
            "Kc 转换错误: got {:.6}, expected {:.6}", kc, expected);
    }

    /// 半衰期验证
    /// 一级反应：t_1/2 = ln2/k
    #[test]
    fn test_half_life_first_order() {
        let k = 0.1;
        let t_half = RateLaw::half_life_first_order(k);
        assert!((t_half - std::f64::consts::LN_2 / 0.1).abs() < 1e-9,
            "一级半衰期应为 ln2/k: got {:.6}", t_half);
    }
}