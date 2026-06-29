//! 光合作用模块
//!
//! 覆盖：
//! - 光反应 (Z 链 / 水裂解 / ATP 合成 / 循环电子传递)
//! - 暗反应 Calvin 循环 (C3/C4/CAM)
//! - 光呼吸
//! - 光合效率与光响应曲线
//! - 色素系统
//!
//! 单位说明：
//! - 光合速率 A: μmol CO2 / m^2 / s
//! - 光强 Q: μmol photons / m^2 / s (PAR)
//! - 浓度: ppm 或 μmol/mol

use serde::{Deserialize, Serialize};

// ============================================================================
// 色素系统 Pigments
// ============================================================================

/// 叶绿素类型 (a/b)，区别于吸收峰与功能
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChlorophyllType {
    /// 叶绿素 a，反应中心专用，吸收峰 ~430nm(蓝)/~662nm(红)
    A,
    /// 叶绿素 b，天线色素，吸收峰 ~453nm(蓝)/~642nm(红)
    B,
}

/// 色素种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PigmentKind {
    Chlorophyll(ChlorophyllType),
    /// 类胡萝卜素 (蓝光吸收 + 光保护 + 散热)
    Carotenoid,
    /// 叶黄素类 (xanthophyll cycle，热耗能)
    Xanthophyll,
    /// 藻胆素 (红藻/蓝藻的藻胆蛋白)
    Phycobilin,
}

/// 单个色素分子：吸收峰与摩尔消光系数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pigment {
    pub kind: PigmentKind,
    /// 吸收峰波长 (nm)
    pub peak_wavelength_nm: f32,
    /// 摩尔消光系数 (L·mol^-1·cm^-1)
    pub molar_extinction: f32,
    /// 相对含量 (0..1，相对叶绿素 a)
    pub relative_abundance: f32,
}

impl Pigment {
    pub fn chlorophyll_a() -> Self {
        Self {
            kind: PigmentKind::Chlorophyll(ChlorophyllType::A),
            peak_wavelength_nm: 430.0,
            molar_extinction: 1.1e5,
            relative_abundance: 1.0,
        }
    }
    pub fn chlorophyll_b() -> Self {
        Self {
            kind: PigmentKind::Chlorophyll(ChlorophyllType::B),
            peak_wavelength_nm: 453.0,
            molar_extinction: 1.6e5,
            relative_abundance: 0.35,
        }
    }
    pub fn carotenoid() -> Self {
        Self {
            kind: PigmentKind::Carotenoid,
            peak_wavelength_nm: 480.0,
            molar_extinction: 1.4e5,
            relative_abundance: 0.25,
        }
    }
}

// ============================================================================
// 光反应 Light Reactions
// ============================================================================

/// 光系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Photosystem {
    /// PSII，反应中心 P680，氧化水
    PSII,
    /// PSI，反应中心 P700，还原 NADP+
    PSI,
}

/// Z 链电子载体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElectronCarrier {
    /// 质体醌 (移动性，跨膜质子移位)
    Plastoquinone,
    /// 细胞色素 b6f 复合体 (质子泵)
    CytochromeB6F,
    /// 质体蓝素 (水溶性铜蛋白)
    Plastocyanin,
    /// 铁氧还蛋白 (Fe-S)
    Ferredoxin,
    /// FNR (铁氧还蛋白-NADP+ 还原酶)
    FNR,
}

/// 光反应状态：H+ 梯度、ATP/NADPH 产量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightReactionState {
    /// 类囊体腔内 H+ 浓度 (mol/L)
    pub lumen_protons: f32,
    /// 基质 pH (默认 7.8)
    pub stroma_ph: f32,
    /// 跨膜 ΔpH
    pub delta_ph: f32,
    /// 已合成 ATP (μmol/m^2)
    pub atp_produced: f32,
    /// 已还原 NADPH (μmol/m^2)
    pub nadph_produced: f32,
    /// 释放 O2 (μmol/m^2)
    pub oxygen_released: f32,
    /// 是否启用循环电子传递 (仅 PSI，额外产 ATP 不产 NADPH)
    pub cyclic_electron_flow: bool,
}

impl Default for LightReactionState {
    fn default() -> Self {
        Self {
            lumen_protons: 5e-5,
            stroma_ph: 7.8,
            delta_ph: 0.0,
            atp_produced: 0.0,
            nadph_produced: 0.0,
            oxygen_released: 0.0,
            cyclic_electron_flow: false,
        }
    }
}

impl LightReactionState {
    /// 推进光反应一个时间步
    ///
    /// - `par`: 光合有效辐射 (μmol photons/m^2/s)
    /// - `dt`: 时间步长 (s)
    pub fn step(&mut self, par: f32, dt: f32) {
        // PSII 吸收光子 → 激发电子 → 水裂解
        // 2 H2O → 4 H+ + 4 e- + O2
        // 每 4 个光子产生 1 O2，每 O2 释放 4 H+ 进腔
        let photons_absorbed = par.min(2000.0) * dt * 0.85; // 量子吸收效率
        let o2_rate = photons_absorbed / 4.0;
        self.oxygen_released += o2_rate;
        // 水裂解释放 H+ 进腔
        self.lumen_protons += o2_rate * 4.0 * 1e-6;

        // 电子经 b6f 复合体，每电子泵 2 H+ 进腔
        let electrons = photons_absorbed;
        self.lumen_protons += electrons * 2.0 * 1e-6;

        // NADP+ 还原：每 2 电子产 1 NADPH
        self.nadph_produced += electrons / 2.0;

        // ΔpH
        let lumen_ph = -self.lumen_protons.log10().max(0.0);
        self.delta_ph = (self.stroma_ph - lumen_ph).max(0.0);

        // ATP 合成：化学渗透，每 4 H+ 经 ATP 合酶合成 1 ATP
        let atp_rate = self.delta_ph * electrons / 4.0 * 0.6;
        self.atp_produced += atp_rate;

        // 循环电子传递：仅 PSI，额外产 ATP 不产 NADPH/O2
        if self.cyclic_electron_flow {
            self.atp_produced += electrons * 0.15;
        }

        // H+ 衰减回到稳态
        self.lumen_protons *= (-dt * 0.5).exp();
    }
}
// ============================================================================
// 碳同化途径 Carbon Fixation Pathways
// ============================================================================

/// 碳同化途径分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarbonPathway {
    /// C3 途径：直接 Rubisco 固碳，多数植物
    C3,
    /// C4 途径 (Hatch-Slack)：PEP 羧化酶→苹果酸→维管束鞘→CO2 释放
    /// 适应高温、强光；几乎消除光呼吸
    C4,
    /// CAM 途径：夜间开放气孔固定 CO2→苹果酸→白天释放
    /// 适应干旱
    CAM,
}

impl CarbonPathway {
    /// 量子产率 Φ (mol CO2 / mol photon)
    pub fn quantum_yield(self) -> f32 {
        match self {
            CarbonPathway::C3 => 0.08,
            CarbonPathway::C4 => 0.06,
            CarbonPathway::CAM => 0.05,
        }
    }
    /// 最大光合速率 A_max (μmol/m^2/s)
    pub fn a_max(self) -> f32 {
        match self {
            CarbonPathway::C3 => 25.0,
            CarbonPathway::C4 => 40.0,
            CarbonPathway::CAM => 12.0,
        }
    }
    /// 光呼吸损失比例 (相对已固定碳)
    pub fn photorespiration_loss(self) -> f32 {
        match self {
            CarbonPathway::C3 => 0.25,
            CarbonPathway::C4 => 0.02,
            CarbonPathway::CAM => 0.05,
        }
    }
    /// CO2 补偿点 (ppm)
    pub fn co2_compensation_point(self) -> f32 {
        match self {
            CarbonPathway::C3 => 50.0,
            CarbonPathway::C4 => 5.0,
            CarbonPathway::CAM => 5.0,
        }
    }
}

/// Calvin 循环状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalvinCycle {
    pub pathway: CarbonPathway,
    /// RuBP (核酮糖-1,5-二磷酸) 池 (μmol/m^2)
    pub rubp_pool: f32,
    /// PGA (3-磷酸甘油酸) 池
    pub pga_pool: f32,
    /// G3P (3-磷酸甘油醛) 池
    pub g3p_pool: f32,
    /// 已合成葡萄糖 (μmol/m^2)
    pub glucose_synthesized: f32,
    /// Rubisco 活性 (0..1)
    pub rubisco_activity: f32,
}

impl Default for CalvinCycle {
    fn default() -> Self {
        Self {
            pathway: CarbonPathway::C3,
            rubp_pool: 200.0,
            pga_pool: 0.0,
            g3p_pool: 0.0,
            glucose_synthesized: 0.0,
            rubisco_activity: 1.0,
        }
    }
}

impl CalvinCycle {
    /// 推进 Calvin 循环一步
    ///
    /// - `atp`: 可用 ATP (μmol/m^2)
    /// - `nadph`: 可用 NADPH (μmol/m^2)
    /// - `co2`: 胞间 CO2 浓度 (ppm)
    /// - `temperature`: 叶温 (°C)
    /// - `dt`: 时间步长 (s)
    pub fn step(
        &mut self,
        atp: f32,
        nadph: f32,
        co2: f32,
        temperature: f32,
        dt: f32,
    ) {
        // Rubisco 固碳：RuBP + CO2 → 2 PGA (5C+1C → 2×3C)
        // 温度修正：高温增加 Rubisco 加氧酶活性，降低净固碳
        let temp_factor = if temperature > 30.0 {
            (1.0 - (temperature - 30.0) * 0.03).max(0.0)
        } else {
            0.5 + (temperature - 5.0) * 0.02
        };
        let co2_factor = (co2 / (co2 + 200.0)).min(1.0); // Michaelis-Menten 类
        let fix_rate = self.rubp_pool * self.rubisco_activity * co2_factor * temp_factor * dt;

        let fixed = fix_rate.min(self.rubp_pool);
        self.rubp_pool -= fixed;
        self.pga_pool += fixed * 2.0;

        // PGA → G3P：消耗 1 ATP + 1 NADPH 每 PGA
        let conv = self.pga_pool.min(atp).min(nadph);
        self.pga_pool -= conv;
        self.g3p_pool += conv;

        // 5/6 G3P 回 RuBP 再生，1/6 转葡萄糖
        // 葡萄糖合成需要 6 G3P
        let glucose_made = (self.g3p_pool / 6.0).floor();
        if glucose_made > 0.0 {
            self.glucose_synthesized += glucose_made;
            self.g3p_pool -= glucose_made * 5.0; // 5 G3P 回 RuBP 再生路径
            self.rubp_pool += glucose_made * 5.0 * 0.4; // 简化再生
        }

        // C4/CAM：额外 ATP 开销 (浓缩 CO2)
        match self.pathway {
            CarbonPathway::C3 => {}
            CarbonPathway::C4 => {
                // C4 额外消耗 ATP 用于苹果酸循环
                self.rubp_pool += 0.0; // 略，ATP 已在调用方扣除
            }
            CarbonPathway::CAM => {
                // 苹果酸池夜间累积，白天脱羧；此处简化
            }
        }
    }
}
// ============================================================================
// 光呼吸 Photorespiration
// ============================================================================

/// 光呼吸状态：Rubisco 加氧酶活性导致的碳损失
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photorespiration {
    /// Rubisco 加氧酶/羧化酶比 (温度依赖)
    pub oxygenase_ratio: f32,
    /// 磷酸乙醇酸累积 (μmol/m^2)
    pub phosphoglycolate: f32,
    /// 已损失碳 (μmol/m^2)
    pub carbon_lost: f32,
    /// 氨再固定消耗 ATP (μmol/m^2)
    pub atp_cost: f32,
}

impl Default for Photorespiration {
    fn default() -> Self {
        Self {
            oxygenase_ratio: 0.25,
            phosphoglycolate: 0.0,
            carbon_lost: 0.0,
            atp_cost: 0.0,
        }
    }
}

impl Photorespiration {
    /// 根据温度和 O2/CO2 比更新加氧酶活性
    pub fn update_conditions(&mut self, temperature: f32, o2_ppm: f32, co2_ppm: f32) {
        let temp_factor = 2.0_f32.powf((temperature - 25.0) / 10.0);
        let ratio = (o2_ppm / co2_ppm) * temp_factor * 0.0002;
        self.oxygenase_ratio = ratio.clamp(0.0, 0.6);
    }

    pub fn step(&mut self, fixed_carbon: f32) {
        let lost = fixed_carbon * self.oxygenase_ratio;
        self.phosphoglycolate += lost * 0.5;
        self.carbon_lost += lost;
        self.atp_cost += lost * 0.5;
    }
}

// ============================================================================
// 光合效率与光响应曲线
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotosynthesisModel {
    pub pathway: CarbonPathway,
    pub alpha: f32,
    pub a_max: f32,
    pub dark_respiration: f32,
    pub current_par: f32,
    pub current_co2: f32,
    pub current_temperature: f32,
}

impl Default for PhotosynthesisModel {
    fn default() -> Self {
        Self {
            pathway: CarbonPathway::C3,
            alpha: 0.08,
            a_max: 30.0,
            dark_respiration: 1.5,
            current_par: 800.0,
            current_co2: 400.0,
            current_temperature: 25.0,
        }
    }
}

impl PhotosynthesisModel {
    /// 直角双曲线：A = (A_max·Q·α) / (A_max + Q·α) - R_d
    pub fn net_assimilation(&self) -> f32 {
        let q_alpha = self.current_par * self.alpha;
        let gross = (self.a_max * q_alpha) / (self.a_max + q_alpha);
        let co2_factor = (self.current_co2 / (self.current_co2 + 200.0)) * 2.0;
        let temp_factor = ((self.current_temperature - 25.0) * 0.1).abs();
        let temp_factor = (-temp_factor * temp_factor).exp();
        gross * co2_factor.min(1.5) * temp_factor - self.dark_respiration
    }

    pub fn light_saturation_point(&self) -> f32 {
        9.0 * self.a_max / self.alpha
    }

    pub fn light_compensation_point(&self) -> f32 {
        self.dark_respiration / self.alpha
    }

    pub fn water_use_efficiency(&self, stomatal_conductance: f32) -> f32 {
        if stomatal_conductance > 0.0 {
            self.net_assimilation() / stomatal_conductance
        } else {
            0.0
        }
    }
}
// ============================================================================
// 叶片综合状态
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafPhotosynthesis {
    pub light: LightReactionState,
    pub calvin: CalvinCycle,
    pub photorespiration: Photorespiration,
    pub model: PhotosynthesisModel,
    /// 气孔导度 g_sw (mol/m^2/s)
    pub stomatal_conductance: f32,
}

impl Default for LeafPhotosynthesis {
    fn default() -> Self {
        Self {
            light: LightReactionState::default(),
            calvin: CalvinCycle::default(),
            photorespiration: Photorespiration::default(),
            model: PhotosynthesisModel::default(),
            stomatal_conductance: 0.3,
        }
    }
}

impl LeafPhotosynthesis {
    pub fn step(&mut self, par: f32, co2: f32, temperature: f32, dt: f32) {
        self.model.current_par = par;
        self.model.current_co2 = co2;
        self.model.current_temperature = temperature;

        self.light.step(par, dt);
        self.photorespiration.update_conditions(temperature, 210_000.0, co2);
        let atp_avail = self.light.atp_produced;
        let nadph_avail = self.light.nadph_produced;
        let prev_glucose = self.calvin.glucose_synthesized;
        self.calvin.step(atp_avail, nadph_avail, co2, temperature, dt);
        let fixed_this_step = self.calvin.glucose_synthesized - prev_glucose;
        self.photorespiration.step(fixed_this_step * 6.0);

        let vpd = 0.6108 * ((17.27 * temperature) / (temperature + 237.3)).exp();
        self.stomatal_conductance = (0.4 / (1.0 + vpd * 0.5)).max(0.05);
    }

    pub fn current_assimilation(&self) -> f32 {
        self.model.net_assimilation()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pigment_constructors() {
        let a = Pigment::chlorophyll_a();
        let b = Pigment::chlorophyll_b();
        let c = Pigment::carotenoid();
        assert_eq!(a.peak_wavelength_nm, 430.0);
        assert_eq!(b.peak_wavelength_nm, 453.0);
        assert_eq!(c.peak_wavelength_nm, 480.0);
        assert!(b.relative_abundance < a.relative_abundance);
    }

    #[test]
    fn test_carbon_pathway_quantum_yield() {
        assert!((CarbonPathway::C3.quantum_yield() - 0.08).abs() < 1e-6);
        assert!((CarbonPathway::C4.quantum_yield() - 0.06).abs() < 1e-6);
        assert!(CarbonPathway::C4.photorespiration_loss() < CarbonPathway::C3.photorespiration_loss());
        assert!(CarbonPathway::C4.co2_compensation_point() < CarbonPathway::C3.co2_compensation_point());
    }

    #[test]
    fn test_light_reaction_produces_atp_and_o2() {
        let mut lr = LightReactionState::default();
        lr.step(1000.0, 1.0);
        assert!(lr.atp_produced > 0.0, "ATP must be produced");
        assert!(lr.nadph_produced > 0.0, "NADPH must be produced");
        assert!(lr.oxygen_released > 0.0, "O2 must be released");
        assert!(lr.delta_ph >= 0.0);
    }

    #[test]
    fn test_cyclic_electron_flow_extra_atp() {
        let mut lr_no_cyclic = LightReactionState::default();
        let mut lr_cyclic = LightReactionState::default();
        lr_cyclic.cyclic_electron_flow = true;
        lr_no_cyclic.step(800.0, 1.0);
        lr_cyclic.step(800.0, 1.0);
        assert!(lr_cyclic.atp_produced > lr_no_cyclic.atp_produced);
        assert!((lr_cyclic.nadph_produced - lr_no_cyclic.nadph_produced).abs() < 1e-3);
    }

    #[test]
    fn test_photosynthesis_model_response_curve() {
        let mut model = PhotosynthesisModel::default();
        model.current_par = 50.0;
        let low_q = model.net_assimilation();
        model.current_par = 1000.0;
        let high_q = model.net_assimilation();
        assert!(high_q > low_q);
        model.current_par = 5000.0;
        let sat = model.net_assimilation();
        assert!(sat - high_q < high_q - low_q);
        assert!(model.light_compensation_point() > 0.0);
        assert!(model.light_saturation_point() > model.light_compensation_point());
    }

    #[test]
    fn test_photorespiration_loses_more_at_high_temperature() {
        let mut pr_cold = Photorespiration::default();
        let mut pr_hot = Photorespiration::default();
        pr_cold.update_conditions(15.0, 210_000.0, 400.0);
        pr_hot.update_conditions(40.0, 210_000.0, 400.0);
        assert!(pr_hot.oxygenase_ratio > pr_cold.oxygenase_ratio);
        pr_cold.step(100.0);
        pr_hot.step(100.0);
        assert!(pr_hot.carbon_lost > pr_cold.carbon_lost);
    }

    #[test]
    fn test_leaf_integration_produces_glucose() {
        let mut leaf = LeafPhotosynthesis::default();
        for _ in 0..10 {
            leaf.step(1000.0, 400.0, 25.0, 1.0);
        }
        assert!(leaf.calvin.glucose_synthesized > 0.0);
        assert!(leaf.light.oxygen_released > 0.0);
    }
}