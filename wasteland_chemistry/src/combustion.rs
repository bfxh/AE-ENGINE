use serde::{Deserialize, Serialize};

// ===== 生物质组分 =====
// 木材典型组成：纤维素 40-50%, 半纤维素 20-30%, 木质素 20-30%
// 来源：Miller & Bellan 1997

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BiomassComposition {
    /// 纤维素质量 (kg) - C6H10O5
    pub cellulose: f32,
    /// 半纤维素质量 (kg)
    pub hemicellulose: f32,
    /// 木质素质量 (kg)
    pub lignin: f32,
    /// 含水质量 (kg)
    pub moisture: f32,
}

impl BiomassComposition {
    /// 木材典型组分（按 1 kg 基准）
    /// 纤维素 45%, 半纤维素 25%, 木质素 25%, 水分 5%
    pub fn typical_wood() -> Self {
        Self {
            cellulose: 0.45,
            hemicellulose: 0.25,
            lignin: 0.25,
            moisture: 0.05,
        }
    }

    /// 总质量 (kg)
    pub fn total_mass(&self) -> f32 {
        self.cellulose + self.hemicellulose + self.lignin + self.moisture
    }
}

impl Default for BiomassComposition {
    fn default() -> Self {
        Self::typical_wood()
    }
}

// ===== Arrhenius 参数 =====
// k = A·exp(-Ea/(R·T))
// 来源：Ranzi 2008 三组分模型

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ArrheniusParams {
    /// 指前因子 A (s⁻¹)
    pub A: f32,
    /// 活化能 Ea (J/mol)
    pub Ea: f32,
}

impl ArrheniusParams {
    pub const fn new(A: f32, Ea: f32) -> Self {
        Self { A, Ea }
    }
}

/// 纤维素热解参数 - Miller & Bellan 1997
/// Ea = 200 kJ/mol, A = 2×10¹⁶ s⁻¹
pub fn cellulose_params() -> ArrheniusParams {
    ArrheniusParams::new(2.0e16, 2.0e5)
}

/// 半纤维素热解参数 - Miller & Bellan 1997
/// Ea = 100 kJ/mol, A = 1×10⁸ s⁻¹
pub fn hemicellulose_params() -> ArrheniusParams {
    ArrheniusParams::new(1.0e8, 1.0e5)
}

/// 木质素热解参数 - Miller & Bellan 1997
/// Ea = 120 kJ/mol, A = 5×10⁵ s⁻¹
pub fn lignin_params() -> ArrheniusParams {
    ArrheniusParams::new(5.0e5, 1.2e5)
}

/// 焦炭氧化参数 - 6 反应骨架
/// C + O2 → CO2, Ea = 120 kJ/mol, A = 1×10⁵
pub fn char_oxidation_params() -> ArrheniusParams {
    ArrheniusParams::new(1.0e5, 1.2e5)
}

/// CO 氧化参数 - 6 反应骨架
/// CO + 0.5O2 → CO2, Ea = 50 kJ/mol, A = 2×10⁸
pub fn co_oxidation_params() -> ArrheniusParams {
    ArrheniusParams::new(2.0e8, 5.0e4)
}

// ===== 燃烧状态 =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombustionState {
    /// 温度 (K)
    pub temperature: f32,
    /// 密度 (kg/m³)
    pub density: f32,
    /// 生物质组分（质量 kg）
    pub biomass_composition: BiomassComposition,
    /// 焦炭质量 (kg)
    pub char_mass: f32,
    /// 挥发分质量 (kg)
    pub volatiles_mass: f32,
    /// CO 质量 (kg)
    pub co_mass: f32,
    /// CO2 质量 (kg)
    pub co2_mass: f32,
    /// 水蒸气质量 (kg)
    pub h2o_vapor_mass: f32,
    /// O2 质量 (kg)
    pub o2_mass: f32,
    /// 烟雾颗粒质量 (kg)
    pub smoke_mass: f32,
    /// 灰分质量 (kg)
    pub ash_mass: f32,
}

impl CombustionState {
    /// 总质量 (kg)
    pub fn total_mass(&self) -> f32 {
        self.biomass_composition.total_mass()
            + self.char_mass
            + self.volatiles_mass
            + self.co_mass
            + self.co2_mass
            + self.h2o_vapor_mass
            + self.o2_mass
            + self.smoke_mass
            + self.ash_mass
    }
}

impl Default for CombustionState {
    fn default() -> Self {
        Self {
            temperature: 300.0,
            density: 1.0,
            biomass_composition: BiomassComposition::typical_wood(),
            char_mass: 0.0,
            volatiles_mass: 0.0,
            co_mass: 0.0,
            co2_mass: 0.0,
            h2o_vapor_mass: 0.0,
            o2_mass: 0.0,
            smoke_mass: 0.0,
            ash_mass: 0.0,
        }
    }
}

// ===== 燃烧报告 =====
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CombustionReport {
    /// 释热总量 (J)，正值为放热
    pub heat_release: f32,
    /// 挥发分生成量 (kg)
    pub volatiles_produced: f32,
    /// 焦炭生成量 (kg)
    pub char_produced: f32,
    /// 烟雾生成量 (kg)
    pub smoke_produced: f32,
    /// 步末温度 (K)
    pub new_temperature: f32,
}

// ===== 燃烧模型 =====
// 6 反应骨架（来源：调研报告）
//  1. 水分蒸发     H2O(l) → H2O(g)        吸热 2257 kJ/kg, T_evap = 373K
//  2. 半纤维素热解 Hemi → Vol + Char      吸热 200 kJ/kg
//  3. 纤维素热解   Cell → Vol + Char      吸热 200 kJ/kg
//  4. 木质素热解   Lign → Vol + Char      吸热 150 kJ/kg
//  5. 焦炭氧化     C + O2 → CO2           放热 32.8 MJ/kg
//  6. CO 氧化      CO + 0.5O2 → CO2       放热 10.1 MJ/kg

pub struct CombustionModel {
    /// 气体常数 R = 8.314 J/(mol·K)
    pub R: f32,
    /// 空气定压比热 cp_air = 1.005 kJ/(kg·K)
    pub cp_air: f32,
    /// 化学计量空气燃料比 (木材 ≈ 5.5)
    pub phi_air: f32,
    /// 默认时间步长 (s)
    pub dt: f32,
}

impl Default for CombustionModel {
    fn default() -> Self {
        Self::new()
    }
}

// 热解产物质量分数
const CHAR_YIELD: f32 = 0.10; // 焦炭产率
const SMOKE_YIELD: f32 = 0.03; // 烟雾产率

// 反应热 (kJ/kg)
const DH_EVAP: f32 = 2257.0; // 水蒸发 吸热
const DH_HEMI: f32 = 200.0; // 半纤维素热解 吸热
const DH_CELL: f32 = 200.0; // 纤维素热解 吸热
const DH_LIGN: f32 = 150.0; // 木质素热解 吸热
const DH_CHAR_OX: f32 = 32800.0; // 焦炭氧化 放热 (32.8 MJ/kg)
const DH_CO_OX: f32 = 10100.0; // CO 氧化 放热 (10.1 MJ/kg)
const T_EVAP: f32 = 373.0; // 水沸点 (K)

// 化学计量比 (kg/kg)
const O2_PER_CHAR: f32 = 32.0 / 12.0; // C + O2 → CO2
const CO2_PER_CHAR: f32 = 44.0 / 12.0;
const O2_PER_CO: f32 = 16.0 / 28.0; // CO + 0.5O2 → CO2
const CO2_PER_CO: f32 = 44.0 / 28.0;
const ASH_PER_CHAR: f32 = 0.05; // 焦炭中 5% 不可燃矿物成为灰分
const CHAR_CARBON_FRAC: f32 = 0.95; // 焦炭可燃碳分数

impl CombustionModel {
    pub fn new() -> Self {
        Self {
            R: 8.314,
            cp_air: 1.005, // kJ/(kg·K)
            phi_air: 5.5,
            dt: 0.016, // ~60fps
        }
    }

    /// Arrhenius 反应速率 k = A·exp(-Ea/(R·T))
    /// 来源：Ranzi 2008
    #[inline]
    pub fn arrhenius_rate(&self, params: &ArrheniusParams, T: f32) -> f32 {
        if T <= 0.0 || params.Ea <= 0.0 {
            return 0.0;
        }
        let exponent = -params.Ea / (self.R * T);
        // 防止 exp 上溢：指数过负时速率趋近 0
        if exponent < -80.0 {
            return 0.0;
        }
        params.A * exponent.exp()
    }

    /// 绝热火焰温度
    /// T_ad = T_0 + (-ΔH_rxn)·Y_fuel / (cp·(1 + φ_air·Y_fuel))
    /// 参数 fuel_mass 解释为燃料在燃料-空气混合物中的质量分数 Y_fuel (0-1)
    pub fn adiabatic_flame_temperature(&self, fuel_mass: f32, T_0: f32) -> f32 {
        // 木材燃烧焓 ≈ -18 MJ/kg = -18000 kJ/kg
        const DH_WOOD: f32 = -18000.0; // kJ/kg
        let y = fuel_mass.clamp(0.0, 1.0);
        if y <= 0.0 {
            return T_0;
        }
        let denom = self.cp_air * (1.0 + self.phi_air * y);
        T_0 + (-DH_WOOD) * y / denom
    }

    /// 火焰传播速率 (Rothermel 模型)
    /// R = I_R·ξ·(1 + φ_w + φ_s) / (ρ_b·ε·Q_ig)
    /// 输出单位 m/s；典型野外火灾 0.1-2.0 m/s
    pub fn flame_propagation_rate(&self, wind_speed: f32, slope: f32, density: f32) -> f32 {
        // 反应强度 I_R (kW/m² = kJ/(s·m²)) - 木材火灾典型值
        const I_R: f32 = 1000.0;
        // 传播热通量比 ξ
        const XI: f32 = 0.1;
        // 有效加热数 ε
        const EPS: f32 = 0.8;
        // 预点燃热 Q_ig (kJ/kg)
        const Q_IG: f32 = 500.0;
        // Rothermel 风因子系数
        const C_W: f32 = 0.025;
        const B_W: f32 = 1.5;
        // Rothermel 坡度因子系数
        const C_S: f32 = 5.275;

        if density <= 0.0 {
            return 0.0;
        }
        let phi_w = C_W * wind_speed.max(0.0).powf(B_W);
        let phi_s = C_S * slope.max(0.0).min(1.0).powi(2);
        let numerator = I_R * XI * (1.0 + phi_w + phi_s);
        let denominator = density * EPS * Q_IG;
        // 单位: (kJ/(s·m²)) / (kg/m³ * kJ/kg) = m/s
        numerator / denominator
    }

    /// 热解单步计算（私有辅助）
    /// 返回 (消耗质量 kg, 焦炭产率 kg, 挥发分产率 kg, 烟雾产率 kg, 反应热 kJ)
    #[inline]
    fn pyrolyze_step(&self, mass: f32, k: f32, dh: f32, dt: f32) -> (f32, f32, f32, f32, f32) {
        if mass <= 0.0 || k <= 0.0 {
            return (0.0, 0.0, 0.0, 0.0, 0.0);
        }
        let rate = k * mass; // kg/s
        let consumed = (rate * dt).min(mass);
        let char_prod = consumed * CHAR_YIELD;
        let smoke_prod = consumed * SMOKE_YIELD;
        let vol_prod = consumed - char_prod - smoke_prod;
        // 吸热反应：dh > 0 表示吸热，热量贡献为负
        let heat = -consumed * dh;
        (consumed, char_prod, vol_prod, smoke_prod, heat)
    }

    /// 单步推进燃烧状态
    ///
    /// 6 反应按顺序更新：水分蒸发 → 半纤维素 → 纤维素 → 木质素 → 焦炭氧化 → CO 氧化
    /// O2 不足时焦炭氧化和 CO 氧化按比例减速
    pub fn step(&self, state: &mut CombustionState, dt: f32) -> CombustionReport {
        let mut report = CombustionReport::default();
        if dt <= 0.0 {
            report.new_temperature = state.temperature;
            return report;
        }

        let t = state.temperature;

        // 净热累积 (kJ)，正值为放热
        let mut q_net: f32 = 0.0;

        // ========== 反应 1: 水分蒸发 ==========
        // H2O(l) → H2O(g), 吸热 2257 kJ/kg, T_evap = 373K
        if t >= T_EVAP && state.biomass_composition.moisture > 0.0 {
            // 蒸发速率系数 (1/s)，随温差线性增加
            let k_evap = 0.05 * (t - T_EVAP);
            let rate = k_evap * state.biomass_composition.moisture; // kg/s
            let consumed = (rate * dt).min(state.biomass_composition.moisture);
            state.biomass_composition.moisture -= consumed;
            state.h2o_vapor_mass += consumed;
            q_net -= consumed * DH_EVAP;
        }

        // ========== 反应 2: 半纤维素热解 ==========
        // Hemicellulose → Volatiles + Char, ΔH = +200 kJ/kg
        {
            let k = self.arrhenius_rate(&hemicellulose_params(), t);
            let (consumed, char_p, vol_p, smoke_p, heat) =
                self.pyrolyze_step(state.biomass_composition.hemicellulose, k, DH_HEMI, dt);
            state.biomass_composition.hemicellulose -= consumed;
            state.char_mass += char_p;
            state.volatiles_mass += vol_p;
            state.smoke_mass += smoke_p;
            report.char_produced += char_p;
            report.volatiles_produced += vol_p;
            report.smoke_produced += smoke_p;
            q_net += heat;
        }

        // ========== 反应 3: 纤维素热解 ==========
        // Cellulose → Volatiles + Char, ΔH = +200 kJ/kg
        {
            let k = self.arrhenius_rate(&cellulose_params(), t);
            let (consumed, char_p, vol_p, smoke_p, heat) =
                self.pyrolyze_step(state.biomass_composition.cellulose, k, DH_CELL, dt);
            state.biomass_composition.cellulose -= consumed;
            state.char_mass += char_p;
            state.volatiles_mass += vol_p;
            state.smoke_mass += smoke_p;
            report.char_produced += char_p;
            report.volatiles_produced += vol_p;
            report.smoke_produced += smoke_p;
            q_net += heat;
        }

        // ========== 反应 4: 木质素热解 ==========
        // Lignin → Volatiles + Char, ΔH = +150 kJ/kg
        {
            let k = self.arrhenius_rate(&lignin_params(), t);
            let (consumed, char_p, vol_p, smoke_p, heat) =
                self.pyrolyze_step(state.biomass_composition.lignin, k, DH_LIGN, dt);
            state.biomass_composition.lignin -= consumed;
            state.char_mass += char_p;
            state.volatiles_mass += vol_p;
            state.smoke_mass += smoke_p;
            report.char_produced += char_p;
            report.volatiles_produced += vol_p;
            report.smoke_produced += smoke_p;
            q_net += heat;
        }

        // ========== 反应 5: 焦炭氧化 ==========
        // C + O2 → CO2, 放热 32.8 MJ/kg
        // O2 不足时按比例减速
        if state.char_mass > 0.0 && state.o2_mass > 0.0 {
            let k = self.arrhenius_rate(&char_oxidation_params(), t);
            let rate_char = k * state.char_mass; // kg/s
            let o2_needed = rate_char * dt * O2_PER_CHAR * CHAR_CARBON_FRAC;
            let o2_factor = if o2_needed > 0.0 {
                (state.o2_mass / o2_needed).min(1.0)
            } else {
                0.0
            };
            let char_consumed = (rate_char * dt).min(state.char_mass) * o2_factor;
            let carbon_burned = char_consumed * CHAR_CARBON_FRAC;
            let o2_consumed = carbon_burned * O2_PER_CHAR;
            let co2_prod = carbon_burned * CO2_PER_CHAR;
            let ash_prod = char_consumed * ASH_PER_CHAR;
            state.char_mass -= char_consumed;
            state.o2_mass -= o2_consumed;
            state.co2_mass += co2_prod;
            state.ash_mass += ash_prod;
            q_net += carbon_burned * DH_CHAR_OX;
        }

        // ========== 反应 6: CO 氧化 ==========
        // CO + 0.5O2 → CO2, 放热 10.1 MJ/kg
        // O2 不足时按比例减速
        if state.co_mass > 0.0 && state.o2_mass > 0.0 {
            let k = self.arrhenius_rate(&co_oxidation_params(), t);
            let rate_co = k * state.co_mass; // kg/s
            let o2_needed = rate_co * dt * O2_PER_CO;
            let o2_factor = if o2_needed > 0.0 {
                (state.o2_mass / o2_needed).min(1.0)
            } else {
                0.0
            };
            let co_consumed = (rate_co * dt).min(state.co_mass) * o2_factor;
            let o2_consumed = co_consumed * O2_PER_CO;
            let co2_prod = co_consumed * CO2_PER_CO;
            state.co_mass -= co_consumed;
            state.o2_mass -= o2_consumed;
            state.co2_mass += co2_prod;
            q_net += co_consumed * DH_CO_OX;
        }

        // ========== 温度更新 ==========
        // T_new = T + Q_net / (m_total · cp)
        // 其中 Q_net = Σ(ΔH_i · consumed_i) 是本步总释热 (kJ)
        let m_total = state.total_mass();
        if m_total > 0.0 && self.cp_air > 0.0 {
            let dt_temp = q_net / (m_total * self.cp_air);
            state.temperature = (state.temperature + dt_temp).max(0.0);
        }

        // 报告：heat_release 输出为 J
        report.heat_release = q_net * 1000.0;
        report.new_temperature = state.temperature;

        report
    }
}
