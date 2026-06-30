//! 细胞生物力学模块 — 粘弹性、细胞骨架力学、机械转导
//!
//! 生物学背景:
//!   细胞的力学行为由细胞膜 (lipid bilayer + cortex)、细胞骨架 (微丝/微管/
//!   中间丝) 及局部粘附 (focal adhesion) 共同决定。细胞对应力表现为粘弹性
//!   体: 弹性响应来自细胞骨架与膜张力,粘性响应来自胞质流体与蛋白重组。
//!   机械转导 (mechanotransduction) 指细胞通过整合素-踝蛋白-纽蛋白复合体
//!   感受外部基质刚度,激活 YAP/TAZ、RhoA 等通路,调控基因表达。
//!
//! 论文来源:
//!   - Kelvin, Lord (Thomson W.) (1875). "Elasticity and heat." Encyclopaedia
//!     Britannica. (Kelvin-Voigt 粘弹性模型, σ = E·ε + η·dε/dt)
//!   - Discher D.E., Janmey P., Wang Y.-L. (2005). "Tissue cells feel and
//!     respond to the stiffness of their substrate." Science 310:1139-1143.
//!   - Geiger B., Spatz J.P., Bershadsky A.D. (2009). "Environmental sensing
//!     through focal adhesions." Nat. Rev. Mol. Cell Biol. 10:21-33.
//!   - Hoffman B.D., Grashoff C., Schwartz M.A. (2011). "Rethinking focal
//!     adhesion sensing." Nature Reviews 8:75-80.

use serde::{Deserialize, Serialize};

/// 粘弹性模型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViscoelasticModel {
    /// Kelvin-Voigt: 弹簧与阻尼器并联 (用于稳态蠕变)
    KelvinVoigt,
    /// Maxwell: 弹簧与阻尼器串联 (用于应力松弛)
    Maxwell,
    /// 标准线性固体 (Zener): 三元件,描述蠕变+松弛
    StandardLinearSolid,
}

/// 粘弹性体 (Kelvin-Voigt 实现)
/// σ = E·ε + η·dε/dt   (Kelvin 1875)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ViscoelasticBody {
    /// 弹性模量 E (Pa)
    pub youngs_modulus_pa: f32,
    /// 粘性系数 η (Pa·s)
    pub viscosity_pa_s: f32,
    /// 当前应变 ε (无量纲)
    pub strain: f32,
    /// 当前应变率 dε/dt (1/s)
    pub strain_rate: f32,
}

impl ViscoelasticBody {
    pub fn new(youngs_modulus_pa: f32, viscosity_pa_s: f32) -> Self {
        Self {
            youngs_modulus_pa,
            viscosity_pa_s,
            strain: 0.0,
            strain_rate: 0.0,
        }
    }

    /// Kelvin-Voigt 应力 (Pa)
    /// σ = E·ε + η·dε/dt   (Kelvin 1875, Eq.1)
    pub fn stress_pa(&self) -> f32 {
        self.youngs_modulus_pa * self.strain + self.viscosity_pa_s * self.strain_rate
    }

    /// 显式 Euler 积分: 给定应力 σ,更新应变
    /// ε(t+dt) = ε(t) + dt·(σ - E·ε)/η
    pub fn step_under_stress(&mut self, stress_pa: f32, dt: f32) {
        if self.viscosity_pa_s.abs() < 1e-9 {
            return;
        }
        let d_strain = (stress_pa - self.youngs_modulus_pa * self.strain) / self.viscosity_pa_s;
        self.strain += d_strain * dt;
        self.strain_rate = d_strain;
    }
}

impl Default for ViscoelasticBody {
    fn default() -> Self {
        // 典型细胞弹性模量 ~ 1 kPa,粘度 ~ 1 kPa·s (Discher 2005)
        Self::new(1000.0, 1000.0)
    }
}

/// 细胞膜
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellMembrane {
    /// 膜张力 (mN/m), 典型 0.03-0.3 mN/m
    pub tension_mn_m: f32,
    /// 弯曲刚度 (J = N·m), 典型 ~ 2×10⁻¹⁹ J
    pub bending_stiffness_j: f32,
    /// 表面积 (μm²)
    pub area_um2: f32,
    /// 撕裂应变阈值
    pub rupture_strain: f32,
}

impl CellMembrane {
    pub fn new() -> Self {
        Self {
            tension_mn_m: 0.1,
            bending_stiffness_j: 2.0e-19,
            area_um2: 1000.0,
            rupture_strain: 0.05,
        }
    }

    /// 是否在应变下破裂
    pub fn is_ruptured(&self, strain: f32) -> bool {
        strain.abs() > self.rupture_strain
    }

    /// 总膜张力能量 (J): E = T·A
    pub fn tension_energy_j(&self) -> f32 {
        // 转换: mN/m * μm² = 10⁻³ N/m * 10⁻¹² m² = 10⁻¹⁵ J
        self.tension_mn_m * 1e-3 * self.area_um2 * 1e-12
    }
}

impl Default for CellMembrane {
    fn default() -> Self { Self::new() }
}

/// 细胞骨架
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cytoskeleton {
    /// 肌动蛋白密度 (mg/mL), 典型 5-10
    pub actin_density_mg_ml: f32,
    /// 微管密度 (mg/mL)
    pub microtubule_density_mg_ml: f32,
    /// 中间丝密度 (mg/mL)
    pub intermediate_filament_density_mg_ml: f32,
    /// 胞质粘度 (Pa·s)
    pub cytoplasm_viscosity_pa_s: f32,
}

impl Cytoskeleton {
    pub fn new() -> Self {
        Self {
            actin_density_mg_ml: 8.0,
            microtubule_density_mg_ml: 2.0,
            intermediate_filament_density_mg_ml: 1.0,
            cytoplasm_viscosity_pa_s: 1000.0,
        }
    }

    /// 等效弹性模量 (Pa) — 主要由肌动蛋白皮层贡献
    /// E ~ E_actin * density / reference_density (线性近似)
    pub fn effective_modulus_pa(&self) -> f32 {
        let e_actin_ref = 1000.0;  // 1 kPa @ 8 mg/mL
        let density_ratio = self.actin_density_mg_ml / 8.0;
        e_actin_ref * density_ratio
    }
}

impl Default for Cytoskeleton {
    fn default() -> Self { Self::new() }
}

/// 局部粘附 (Geiger 2009)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FocalAdhesion {
    /// 面积 (μm²)
    pub area_um2: f32,
    /// 承受力 (nN)
    pub force_nn: f32,
    /// 成熟度 (0..1, 1 = 完全成熟)
    pub maturity: f32,
    /// 与基质刚度的耦合 (Pa)
    pub substrate_stiffness_pa: f32,
}

impl FocalAdhesion {
    /// 成熟阈值 (nN), 低于此力 focal adhesion 不会成熟
    pub const MATURATION_FORCE_NN: f32 = 1.0;
    /// 最大面积 (μm²)
    pub const MAX_AREA_UM2: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            area_um2: 0.1,
            force_nn: 0.0,
            maturity: 0.0,
            substrate_stiffness_pa: 1000.0,
        }
    }

    /// 成熟动力学 (显式 Euler)
    /// 成熟速率与力、刚度正相关,力低于阈值时退化
    pub fn step_maturation(&mut self, dt: f32) {
        let k_mat = 0.01;     // 1/s 成熟速率
        let k_det = 0.005;    // 1/s 解吸附速率
        if self.force_nn >= Self::MATURATION_FORCE_NN {
            // 成熟: 增加面积与成熟度
            let stiff_factor = (self.substrate_stiffness_pa / 1000.0).clamp(0.1, 10.0);
            self.maturity = (self.maturity + k_mat * stiff_factor * dt).min(1.0);
            self.area_um2 = (self.area_um2 * (1.0 + 0.1 * dt)).min(Self::MAX_AREA_UM2);
        } else {
            // 解吸附
            self.maturity = (self.maturity - k_det * dt).max(0.0);
            self.area_um2 = (self.area_um2 * (1.0 - 0.05 * dt)).max(0.01);
        }
    }
}

impl Default for FocalAdhesion {
    fn default() -> Self { Self::new() }
}

/// 机械转导 (Hoffman 2011)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MechanoTransduction {
    /// 激活阈值 (Pa)
    pub activation_threshold_pa: f32,
    /// 当前激活级别 (0..1)
    pub activation: f32,
    /// YAP/TAZ 核易位分数 (0..1)
    pub yap_taz_nuclear_fraction: f32,
    /// RhoA 活性 (0..1)
    pub rhoa_activity: f32,
}

impl MechanoTransduction {
    pub fn new() -> Self {
        Self {
            activation_threshold_pa: 500.0,
            activation: 0.0,
            yap_taz_nuclear_fraction: 0.0,
            rhoa_activity: 0.0,
        }
    }

    /// 由应力激活机械转导通路
    /// 显式 Euler,激活曲线为 Hill 型
    pub fn activate(&mut self, stress_pa: f32, dt: f32) {
        let k = 0.1; // 1/s 激活动力学速率
        let n = 2.0; // Hill 系数
        let target = if stress_pa > 0.0 {
            let s = stress_pa / self.activation_threshold_pa;
            s.powf(n) / (s.powf(n) + 1.0)
        } else {
            0.0
        };
        self.activation += (target - self.activation) * k * dt;
        self.activation = self.activation.clamp(0.0, 1.0);

        // YAP/TAZ 核易位正比于激活 (Discher 2005)
        self.yap_taz_nuclear_fraction = self.activation;

        // RhoA 由应力直接激活 (Hoffman 2011)
        let rhoa_target = (stress_pa / (self.activation_threshold_pa * 5.0)).clamp(0.0, 1.0);
        self.rhoa_activity += (rhoa_target - self.rhoa_activity) * k * dt;
        self.rhoa_activity = self.rhoa_activity.clamp(0.0, 1.0);
    }
}

impl Default for MechanoTransduction {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 默认值 ---

    #[test]
    fn test_viscoelastic_default() {
        let v = ViscoelasticBody::default();
        assert_eq!(v.youngs_modulus_pa, 1000.0);
        assert_eq!(v.viscosity_pa_s, 1000.0);
        assert_eq!(v.strain, 0.0);
        assert_eq!(v.strain_rate, 0.0);
    }

    #[test]
    fn test_cell_membrane_default() {
        let m = CellMembrane::default();
        assert!(m.tension_mn_m > 0.0);
        assert!(m.bending_stiffness_j > 0.0);
        assert!(m.area_um2 > 0.0);
        assert!(m.rupture_strain > 0.0);
    }

    #[test]
    fn test_cytoskeleton_default() {
        let c = Cytoskeleton::default();
        assert!(c.actin_density_mg_ml > 0.0);
        assert!(c.microtubule_density_mg_ml > 0.0);
        assert!(c.intermediate_filament_density_mg_ml > 0.0);
        assert!(c.cytoplasm_viscosity_pa_s > 0.0);
    }

    #[test]
    fn test_focal_adhesion_default() {
        let fa = FocalAdhesion::default();
        assert_eq!(fa.maturity, 0.0);
        assert!(fa.area_um2 > 0.0);
        assert_eq!(fa.force_nn, 0.0);
    }

    #[test]
    fn test_mechano_transduction_default() {
        let mt = MechanoTransduction::default();
        assert_eq!(mt.activation, 0.0);
        assert_eq!(mt.yap_taz_nuclear_fraction, 0.0);
        assert_eq!(mt.rhoa_activity, 0.0);
        assert!(mt.activation_threshold_pa > 0.0);
    }

    // --- Kelvin-Voigt 应力计算 (Kelvin 1875) ---

    #[test]
    fn test_kelvin_voigt_zero_strain_rate() {
        let v = ViscoelasticBody {
            strain: 0.1,
            strain_rate: 0.0,
            ..Default::default()
        };
        // σ = E·ε + η·0 = 1000 * 0.1 = 100 Pa
        let sigma = v.stress_pa();
        assert!((sigma - 100.0).abs() < 1e-3);
    }

    #[test]
    fn test_kelvin_voigt_with_strain_rate() {
        let v = ViscoelasticBody {
            strain: 0.0,
            strain_rate: 0.1,
            ..Default::default()
        };
        // σ = E·0 + η·0.1 = 1000 * 0.1 = 100 Pa
        let sigma = v.stress_pa();
        assert!((sigma - 100.0).abs() < 1e-3);
    }

    #[test]
    fn test_kelvin_voigt_combined() {
        let v = ViscoelasticBody {
            strain: 0.05,
            strain_rate: 0.02,
            youngs_modulus_pa: 1000.0,
            viscosity_pa_s: 2000.0,
        };
        // σ = 1000*0.05 + 2000*0.02 = 50 + 40 = 90 Pa
        assert!((v.stress_pa() - 90.0).abs() < 1e-3);
    }

    #[test]
    fn test_kelvin_voigt_negative_strain_compression() {
        let v = ViscoelasticBody {
            strain: -0.05,
            strain_rate: 0.0,
            ..Default::default()
        };
        // 压缩 → 负应力
        assert!(v.stress_pa() < 0.0);
    }

    // --- 蠕变动力学 (显式 Euler) ---

    #[test]
    fn test_viscoelastic_step_under_stress_increases_strain() {
        let mut v = ViscoelasticBody::default();
        v.step_under_stress(100.0, 0.1);
        // dε/dt = (σ - E·0)/η = 100/1000 = 0.1; Δε = 0.1*0.1 = 0.01
        assert!(v.strain > 0.0);
        assert!((v.strain - 0.01).abs() < 1e-4);
    }

    #[test]
    fn test_viscoelastic_step_zero_stress_no_change() {
        let mut v = ViscoelasticBody::default();
        v.strain = 0.0;
        v.step_under_stress(0.0, 0.1);
        assert_eq!(v.strain, 0.0);
    }

    // --- 细胞膜 ---

    #[test]
    fn test_membrane_rupture_above_threshold() {
        let m = CellMembrane::default();
        assert!(m.is_ruptured(0.06));
        assert!(!m.is_ruptured(0.04));
    }

    #[test]
    fn test_membrane_tension_energy_positive() {
        let m = CellMembrane::default();
        let e = m.tension_energy_j();
        assert!(e > 0.0);
    }

    // --- 细胞骨架 ---

    #[test]
    fn test_cytoskeleton_modulus_scales_with_actin() {
        let mut c = Cytoskeleton::default();
        let m1 = c.effective_modulus_pa();
        c.actin_density_mg_ml *= 2.0;
        let m2 = c.effective_modulus_pa();
        assert!(m2 > m1);
        assert!((m2 / m1 - 2.0).abs() < 1e-3);
    }

    // --- 局部粘附 (Geiger 2009) ---

    #[test]
    fn test_focal_adhesion_maturation_above_threshold() {
        let mut fa = FocalAdhesion::default();
        fa.force_nn = 2.0; // 高于阈值 1.0
        let before = fa.maturity;
        fa.step_maturation(1.0);
        assert!(fa.maturity > before);
    }

    #[test]
    fn test_focal_adhesion_no_maturation_below_threshold() {
        let mut fa = FocalAdhesion::default();
        fa.force_nn = 0.5; // 低于阈值
        fa.maturity = 0.5;
        fa.step_maturation(1.0);
        // 应当退化
        assert!(fa.maturity < 0.5);
    }

    #[test]
    fn test_focal_adhesion_area_grows_with_maturation() {
        let mut fa = FocalAdhesion::default();
        fa.force_nn = 5.0;
        fa.area_um2 = 0.5;
        let before = fa.area_um2;
        fa.step_maturation(1.0);
        assert!(fa.area_um2 > before);
    }

    #[test]
    fn test_focal_adhesion_area_caps_at_max() {
        let mut fa = FocalAdhesion::default();
        fa.force_nn = 10.0;
        fa.area_um2 = FocalAdhesion::MAX_AREA_UM2 - 0.001;
        for _ in 0..1000 {
            fa.step_maturation(0.5);
        }
        assert!(fa.area_um2 <= FocalAdhesion::MAX_AREA_UM2 + 1e-6);
    }

    // --- 机械转导 (Hoffman 2011) ---

    #[test]
    fn test_mechano_transduction_no_activation_below_threshold() {
        let mut mt = MechanoTransduction::default();
        mt.activate(100.0, 1.0); // 远低于阈值 500
        assert!(mt.activation < 0.5);
    }

    #[test]
    fn test_mechano_transduction_activation_above_threshold() {
        let mut mt = MechanoTransduction::default();
        // 高于阈值 → 激活
        mt.activate(2000.0, 10.0);
        assert!(mt.activation > 0.5);
    }

    #[test]
    fn test_mechano_transduction_caps_at_one() {
        let mut mt = MechanoTransduction::default();
        for _ in 0..100 {
            mt.activate(10000.0, 1.0);
        }
        assert!(mt.activation <= 1.0);
    }

    #[test]
    fn test_mechano_transduction_yap_taz_follows_activation() {
        let mut mt = MechanoTransduction::default();
        mt.activate(2000.0, 5.0);
        // YAP/TAZ 核易位应等于激活级别
        assert!((mt.yap_taz_nuclear_fraction - mt.activation).abs() < 1e-3);
    }

    #[test]
    fn test_mechano_transduction_rhoa_increases() {
        let mut mt = MechanoTransduction::default();
        let before = mt.rhoa_activity;
        mt.activate(5000.0, 1.0);
        assert!(mt.rhoa_activity > before);
    }

    #[test]
    fn test_viscoelastic_model_variants() {
        let _a = ViscoelasticModel::KelvinVoigt;
        let _b = ViscoelasticModel::Maxwell;
        let _c = ViscoelasticModel::StandardLinearSolid;
        assert_ne!(ViscoelasticModel::KelvinVoigt, ViscoelasticModel::Maxwell);
    }
}
