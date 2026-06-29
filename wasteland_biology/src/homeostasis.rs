//! 稳态整合模块 — 血糖 / Na+ / pH / 渗透压 综合负反馈
//!
//! 生物学背景:
//!   稳态 (homeostasis) 指机体通过负反馈维持内环境理化性质相对稳定的能力。
//!   本模块整合四大稳态子系统:
//!     1. 血糖调节 (胰岛素-胰高血糖素, Bergman 1979 最小模型)
//!     2. 钠离子调节 (肾素-血管紧张素-醛固酮系统 RAAS)
//!     3. pH 调节 (碳酸氢盐缓冲, Henderson-Hasselbalch 方程)
//!     4. 渗透压调节 (抗利尿激素 ADH / 血管加压素)
//!
//! 论文来源:
//!   - Bergman R.N., Ider Y.Z., Bowden C.R., Cobelli C. (1979).
//!     "Quantitative estimation of insulin sensitivity." Am. J. Physiol.
//!     236:E667-E677. (Bergman 最小模型)
//!   - Henderson L.J. (1908). "The theory of neutrality regulation in the
//!     animal organism." Am. J. Physiol. 21:427-448. (缓冲方程)
//!   - Hasselbalch K.A. (1917). "Die Berechnung der Wasserstoffzahl des
//!     Blutes..." Biochem. Z. 78:112-144. (Henderson-Hasselbalch 方程)
//!   - Verbalis J.G. (2003). "Disorders of body water homeostasis."
//!     Best Pract. Res. Clin. Endocrinol. Metab. 17:471-503. (ADH 调节)
//!   - Hall J.E. (2020). "Guyton and Hall Textbook of Medical Physiology"
//!     14th ed. Elsevier. (综合参考)

use serde::{Deserialize, Serialize};

// ============================================================================
// 血糖调节 (Bergman 1979 最小模型)
// ============================================================================

/// 血糖调节子系统 (Bergman minimal model)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GlucoseRegulation {
    /// 血糖浓度 (mmol/L), 空腹正常 4.0-6.0, 设定点 5.0
    pub glucose_mmol_l: f32,
    /// 胰岛素浓度 (mU/L), 空腹 5-15
    pub insulin_mu_l: f32,
    /// 胰高血糖素浓度 (pg/mL), 空腹 50-100
    pub glucagon_pg_ml: f32,
    /// 胰岛素敏感性指数 (Bergman S_I), 1e-4/(mU·L⁻¹·min⁻¹)
    pub insulin_sensitivity: f32,
    /// 基础血糖 (mmol/L)
    pub basal_glucose: f32,
    /// 基础胰岛素 (mU/L)
    pub basal_insulin: f32,
}

impl GlucoseRegulation {
    pub fn new() -> Self {
        Self {
            glucose_mmol_l: 5.0,    // 正常空腹血糖
            insulin_mu_l: 10.0,     // 基础胰岛素
            glucagon_pg_ml: 75.0,
            insulin_sensitivity: 5.0e-4,
            basal_glucose: 5.0,
            basal_insulin: 10.0,
        }
    }

    /// Bergman 最小模型: dG/dt = -p1*(G-Gb) - X*G
    /// X 为远程胰岛素作用,这里简化为正比于胰岛素增量
    /// 显式 Euler 积分
    pub fn step(&mut self, dt: f32) {
        let p1 = 0.03;   // 1/min 葡萄糖自身利用率
        let g = self.glucose_mmol_l;
        let g_b = self.basal_glucose;
        // 胰岛素敏感性 × (胰岛素 - 基础胰岛素)
        let x = self.insulin_sensitivity * (self.insulin_mu_l - self.basal_insulin);
        // 葡萄糖动力学: 胰岛素促进摄取, 胰高血糖素促进生成
        let d_g = -p1 * (g - g_b) - x * g + (self.glucagon_pg_ml - 75.0) * 0.0001;
        self.glucose_mmol_l = (g + d_g * dt).max(0.5);

        // 胰岛素分泌: 高血糖刺激 β 细胞分泌
        let i_target = if g > self.basal_glucose {
            self.basal_insulin + (g - self.basal_glucose) * 10.0
        } else {
            self.basal_insulin * (g / self.basal_glucose).max(0.5)
        };
        let k_i = 0.5; // 1/min
        self.insulin_mu_l += (i_target - self.insulin_mu_l) * k_i * dt;
        self.insulin_mu_l = self.insulin_mu_l.max(0.0);

        // 胰高血糖素分泌: 低血糖刺激 α 细胞分泌
        let gluc_target = if g < self.basal_glucose {
            75.0 + (self.basal_glucose - g) * 20.0
        } else {
            75.0 - (g - self.basal_glucose) * 5.0
        };
        self.glucagon_pg_ml += (gluc_target - self.glucagon_pg_ml) * 0.3 * dt;
        self.glucagon_pg_ml = self.glucagon_pg_ml.max(0.0);
    }

    /// 模拟摄入葡萄糖 (meal bolus, mmol/L 直接增加)
    pub fn ingest_glucose(&mut self, amount_mmol_l: f32) {
        self.glucose_mmol_l += amount_mmol_l;
    }
}

impl Default for GlucoseRegulation {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// pH 调节 (Henderson-Hasselbalch 方程)
// ============================================================================

/// pH 调节子系统 (碳酸氢盐缓冲)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PHRegulation {
    /// 血液 pH (正常 7.35-7.45, 设定点 7.40)
    pub ph: f32,
    /// 碳酸氢盐浓度 (mmol/L, 正常 22-26, 设定 24)
    pub bicarbonate_mmol_l: f32,
    /// 二氧化碳分压 (mmHg, 正常 35-45, 设定 40)
    pub pco2_mmhg: f32,
}

impl PHRegulation {
    pub const PH_SETPOINT: f32 = 7.40;
    pub const CO2_SOLUBILITY: f32 = 0.03;  // mmol/(L·mmHg)

    pub fn new() -> Self {
        Self {
            ph: 7.40,
            bicarbonate_mmol_l: 24.0,
            pco2_mmhg: 40.0,
        }
    }

    /// Henderson-Hasselbalch 方程 (Hasselbalch 1917)
    /// pH = 6.1 + log([HCO3-] / (0.03 × PCO2))
    pub fn calculate_ph(bicarbonate_mmol_l: f32, pco2_mmhg: f32) -> f32 {
        let denom = Self::CO2_SOLUBILITY * pco2_mmhg;
        if denom <= 0.0 || bicarbonate_mmol_l <= 0.0 {
            return 7.40; // 退化情况返回正常值
        }
        // Henderson-Hasselbalch: pH = pKa + log10([HCO3-]/(0.03·PCO2))
        6.1 + (bicarbonate_mmol_l / denom).log10()
    }

    /// 由当前 HCO3- 与 PCO2 重算 pH
    pub fn update_ph(&mut self) {
        self.ph = Self::calculate_ph(self.bicarbonate_mmol_l, self.pco2_mmhg);
    }

    /// 肾代偿: 慢性酸中毒时肾脏排泄 H+,保留 HCO3-
    /// 显式 Euler
    pub fn step(&mut self, dt: f32) {
        let err = self.ph - Self::PH_SETPOINT;
        let k_renal = 0.001; // 肾代偿缓慢
        let k_resp = 0.01;   // 呼吸代偿较快
        // 酸中毒 (pH 低) → 肾保留 HCO3-,呼吸降低 PCO2
        self.bicarbonate_mmol_l += -err * 100.0 * k_renal * dt;
        self.pco2_mmhg += err * 50.0 * k_resp * dt;
        self.bicarbonate_mmol_l = self.bicarbonate_mmol_l.max(5.0);
        self.pco2_mmhg = self.pco2_mmhg.max(5.0);
        self.update_ph();
    }
}

impl Default for PHRegulation {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// 钠离子调节 (RAAS)
// ============================================================================

/// 钠离子调节子系统 (RAAS)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SodiumRegulation {
    /// 血钠浓度 (mmol/L, 正常 135-145, 设定 140)
    pub sodium_mmol_l: f32,
    /// 醛固酮浓度 (pg/mL, 正常 50-250)
    pub aldosterone_pg_ml: f32,
    /// 肾素活性 (ng/(mL·h), 正常 1-3)
    pub renin_activity: f32,
}

impl SodiumRegulation {
    pub const NA_SETPOINT: f32 = 140.0;

    pub fn new() -> Self {
        Self {
            sodium_mmol_l: 140.0,
            aldosterone_pg_ml: 100.0,
            renin_activity: 2.0,
        }
    }

    /// RAAS 负反馈: 低血钠 → 肾素↑ → 醛固酮↑ → 肾保钠
    /// 显式 Euler
    pub fn step(&mut self, dt: f32) {
        let err = self.sodium_mmol_l - Self::NA_SETPOINT;
        // 低钠 → 肾素升高
        let renin_target = (2.0 - err * 0.5).max(0.0).min(10.0);
        self.renin_activity += (renin_target - self.renin_activity) * 0.1 * dt;
        // 肾素 → 醛固酮
        let aldo_target = self.renin_activity * 50.0;
        self.aldosterone_pg_ml += (aldo_target - self.aldosterone_pg_ml) * 0.05 * dt;
        // 醛固酮 → 肾保钠 → 血钠回升
        let na_change = (self.aldosterone_pg_ml - 100.0) * 0.001;
        self.sodium_mmol_l += na_change * dt;
        // 向设定点缓慢回归
        self.sodium_mmol_l += -err * 0.01 * dt;
    }
}

impl Default for SodiumRegulation {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// 渗透压调节 (ADH)
// ============================================================================

/// 渗透压调节子系统 (ADH / 血管加压素)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OsmolarityRegulation {
    /// 血浆渗透压 (mOsm/L, 正常 280-300, 设定 300)
    pub osmolarity_mosm_l: f32,
    /// ADH 浓度 (pg/mL, 正常 1-5)
    pub adh_pg_ml: f32,
}

impl OsmolarityRegulation {
    pub const OSM_SETPOINT: f32 = 300.0;

    pub fn new() -> Self {
        Self {
            osmolarity_mosm_l: 300.0,
            adh_pg_ml: 2.0,
        }
    }

    /// ADH 调节: 高渗透压 → ADH↑ → 肾重吸收水 → 渗透压下降
    /// 显式 Euler
    pub fn step(&mut self, dt: f32) {
        let err = self.osmolarity_mosm_l - Self::OSM_SETPOINT;
        let adh_target = (2.0 + err * 0.1).max(0.5).min(10.0);
        self.adh_pg_ml += (adh_target - self.adh_pg_ml) * 0.2 * dt;
        // ADH 增加水重吸收 → 稀释血液 → 渗透压下降
        let dilution = (self.adh_pg_ml - 2.0) * 0.5;
        self.osmolarity_mosm_l += -dilution * dt;
        // 向设定点缓慢回归
        self.osmolarity_mosm_l += -err * 0.05 * dt;
    }
}

impl Default for OsmolarityRegulation {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// 综合稳态系统
// ============================================================================

/// 综合稳态系统
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HomeostaticSystem {
    pub glucose: GlucoseRegulation,
    pub sodium: SodiumRegulation,
    pub ph: PHRegulation,
    pub osmolarity: OsmolarityRegulation,
    /// 时间步数计数
    pub tick: u32,
}

impl HomeostaticSystem {
    pub fn new() -> Self {
        Self {
            glucose: GlucoseRegulation::new(),
            sodium: SodiumRegulation::new(),
            ph: PHRegulation::new(),
            osmolarity: OsmolarityRegulation::new(),
            tick: 0,
        }
    }

    /// 推进所有稳态子系统 (显式 Euler)
    pub fn step(&mut self, dt: f32) {
        self.glucose.step(dt);
        self.sodium.step(dt);
        self.ph.step(dt);
        self.osmolarity.step(dt);
        self.tick += 1;
    }
}

impl Default for HomeostaticSystem {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 血糖调节 (Bergman 1979) ---

    #[test]
    fn test_glucose_default_5_mmol() {
        let g = GlucoseRegulation::default();
        assert!((g.glucose_mmol_l - 5.0).abs() < 1e-3);
        assert!(g.insulin_mu_l > 0.0);
        assert!(g.glucagon_pg_ml > 0.0);
    }

    #[test]
    fn test_glucose_insulin_secretion_high_glucose() {
        let mut g = GlucoseRegulation::default();
        g.glucose_mmol_l = 10.0; // 高血糖
        g.step(1.0);
        // 胰岛素应升高
        assert!(g.insulin_mu_l > 10.0);
    }

    #[test]
    fn test_glucose_glucagon_secretion_low_glucose() {
        let mut g = GlucoseRegulation::default();
        g.glucose_mmol_l = 3.0; // 低血糖
        let before = g.glucagon_pg_ml;
        g.step(1.0);
        assert!(g.glucagon_pg_ml > before);
    }

    #[test]
    fn test_glucose_returns_to_setpoint() {
        let mut g = GlucoseRegulation::default();
        g.ingest_glucose(5.0); // 摄入 5 mmol/L
        // 模拟 200 分钟恢复
        for _ in 0..2000 {
            g.step(0.1);
        }
        // 应回接近设定点 (允许 ±2 mmol/L)
        assert!((g.glucose_mmol_l - 5.0).abs() < 2.0, "glucose = {}", g.glucose_mmol_l);
    }

    #[test]
    fn test_glucose_ingest_increases_concentration() {
        let mut g = GlucoseRegulation::default();
        let before = g.glucose_mmol_l;
        g.ingest_glucose(3.0);
        assert!(g.glucose_mmol_l > before);
    }

    #[test]
    fn test_glucose_step_does_not_go_negative() {
        let mut g = GlucoseRegulation::default();
        g.glucose_mmol_l = 0.6;
        for _ in 0..100 {
            g.step(1.0);
        }
        assert!(g.glucose_mmol_l > 0.0);
    }

    // --- pH 调节 (Henderson-Hasselbalch 1917) ---

    #[test]
    fn test_ph_default_7_4() {
        let p = PHRegulation::default();
        assert!((p.ph - 7.40).abs() < 1e-2);
        assert!((p.bicarbonate_mmol_l - 24.0).abs() < 1e-3);
        assert!((p.pco2_mmhg - 40.0).abs() < 1e-3);
    }

    #[test]
    fn test_henderson_hasselbalch_equation_normal() {
        // 正常: HCO3=24, PCO2=40 → pH = 6.1 + log(24/(0.03*40)) = 6.1 + log(20) = 7.4017
        let ph = PHRegulation::calculate_ph(24.0, 40.0);
        assert!((ph - 7.40).abs() < 0.01, "pH = {}", ph);
    }

    #[test]
    fn test_henderson_hasselbalch_acidosis() {
        // 酸中毒: HCO3 降低
        let ph = PHRegulation::calculate_ph(12.0, 40.0);
        assert!(ph < 7.30);
    }

    #[test]
    fn test_henderson_hasselbalch_alkalosis() {
        // 碱中毒: HCO3 升高
        let ph = PHRegulation::calculate_ph(48.0, 40.0);
        assert!(ph > 7.50);
    }

    #[test]
    fn test_ph_regulation_buffer_recovery() {
        let mut p = PHRegulation::default();
        p.bicarbonate_mmol_l = 18.0; // 酸中毒
        p.update_ph();
        let ph_initial = p.ph;
        // 长时间模拟,应恢复
        for _ in 0..10000 {
            p.step(0.1);
        }
        assert!(p.ph > ph_initial);
        assert!((p.ph - 7.40).abs() < 0.1, "ph = {}", p.ph);
    }

    #[test]
    fn test_ph_calculate_zero_pco2_fallback() {
        // 退化情况: PCO2 = 0,返回默认值 7.40
        let ph = PHRegulation::calculate_ph(24.0, 0.0);
        assert!((ph - 7.40).abs() < 1e-3);
    }

    // --- 钠离子调节 (RAAS) ---

    #[test]
    fn test_sodium_default_140_mmol() {
        let s = SodiumRegulation::default();
        assert!((s.sodium_mmol_l - 140.0).abs() < 1e-3);
        assert!(s.aldosterone_pg_ml > 0.0);
        assert!(s.renin_activity > 0.0);
    }

    #[test]
    fn test_sodium_low_stimulates_renin() {
        let mut s = SodiumRegulation::default();
        s.sodium_mmol_l = 130.0; // 低钠
        let before = s.renin_activity;
        s.step(1.0);
        assert!(s.renin_activity > before);
    }

    #[test]
    fn test_sodium_high_suppresses_renin() {
        let mut s = SodiumRegulation::default();
        s.sodium_mmol_l = 150.0; // 高钠
        let before = s.renin_activity;
        s.step(1.0);
        assert!(s.renin_activity < before);
    }

    #[test]
    fn test_sodium_step_does_not_go_negative() {
        let mut s = SodiumRegulation::default();
        s.sodium_mmol_l = 130.0;
        for _ in 0..100 {
            s.step(0.1);
        }
        assert!(s.sodium_mmol_l > 0.0);
    }

    // --- 渗透压调节 (ADH) ---

    #[test]
    fn test_osmolarity_default_300_mosm() {
        let o = OsmolarityRegulation::default();
        assert!((o.osmolarity_mosm_l - 300.0).abs() < 1e-3);
        assert!(o.adh_pg_ml > 0.0);
    }

    #[test]
    fn test_osmolarity_high_stimulates_adh() {
        let mut o = OsmolarityRegulation::default();
        o.osmolarity_mosm_l = 320.0; // 高渗
        let before = o.adh_pg_ml;
        o.step(1.0);
        assert!(o.adh_pg_ml > before);
    }

    #[test]
    fn test_osmolarity_low_suppresses_adh() {
        let mut o = OsmolarityRegulation::default();
        o.osmolarity_mosm_l = 280.0; // 低渗
        let before = o.adh_pg_ml;
        o.step(1.0);
        assert!(o.adh_pg_ml < before);
    }

    #[test]
    fn test_osmolarity_returns_to_setpoint() {
        let mut o = OsmolarityRegulation::default();
        o.osmolarity_mosm_l = 330.0; // 高渗
        for _ in 0..5000 {
            o.step(0.1);
        }
        assert!((o.osmolarity_mosm_l - 300.0).abs() < 10.0, "osm = {}", o.osmolarity_mosm_l);
    }

    // --- 综合稳态系统 ---

    #[test]
    fn test_homeostatic_system_default() {
        let h = HomeostaticSystem::default();
        assert!((h.glucose.glucose_mmol_l - 5.0).abs() < 1e-3);
        assert!((h.sodium.sodium_mmol_l - 140.0).abs() < 1e-3);
        assert!((h.ph.ph - 7.40).abs() < 1e-2);
        assert!((h.osmolarity.osmolarity_mosm_l - 300.0).abs() < 1e-3);
        assert_eq!(h.tick, 0);
    }

    #[test]
    fn test_homeostatic_system_step_increments_tick() {
        let mut h = HomeostaticSystem::default();
        h.step(1.0);
        assert_eq!(h.tick, 1);
        h.step(1.0);
        assert_eq!(h.tick, 2);
    }

    #[test]
    fn test_homeostatic_system_integrates_all_subsystems() {
        let mut h = HomeostaticSystem::default();
        h.glucose.ingest_glucose(3.0);
        let before_g = h.glucose.glucose_mmol_l;
        h.step(1.0);
        // 血糖应开始下降 (因胰岛素分泌)
        assert!(h.glucose.glucose_mmol_l < before_g);
    }
}
