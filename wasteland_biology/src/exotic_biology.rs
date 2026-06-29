//! 跨物种奇特生物结构模块
//!
//! 基于真实生物学研究实现，涵盖壁虎刚毛附着、章鱼触手静水骨骼、
//! 电鳗电器官、生物发光、蜘蛛丝、变色龙色素细胞、水熊虫隐生、
//! 蝮蛇颊窝红外感知、鲨鱼劳伦氏壶腹电感知、候鸟磁感知等结构。

use serde::{Deserialize, Serialize};

// ============ 1. 壁虎刚毛附着（Van der Waals）============
// 来源：Autumn 2002, PNAS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoAdhesion {
    /// 刚毛数，每脚约 50 万根
    pub setae_count: u64,
    /// 刚毛长度，约 100 μm
    pub seta_length_um: f32,
    /// 刚毛直径，约 5 μm
    pub seta_diameter_um: f32,
    /// 每根刚毛末端的 spatula 数量，100-1000
    pub spatula_count_per_seta: u32,
    /// spatula 尺寸，约 200 nm
    pub spatula_size_nm: f32,
    /// 总接触面积 (m²)
    pub contact_area_m2: f32,
    /// 总附着力 (N)
    pub adhesion_force_n: f32,
}

impl GeckoAdhesion {
    /// 默认壁虎脚：50 万刚毛，每根 100μm 长 5μm 直径，500 spatulae
    pub fn new() -> Self {
        let setae_count: u64 = 500_000;
        let seta_length_um = 100.0;
        let seta_diameter_um = 5.0;
        let spatula_count_per_seta: u32 = 500;
        let spatula_size_nm = 200.0;
        // 单根 spatula 接触面积近似 π·r²，r = 100 nm = 1e-7 m
        let spatula_radius_m = spatula_size_nm * 0.5 * 1e-9;
        let spatula_area_m2 = std::f32::consts::PI * spatula_radius_m * spatula_radius_m;
        let total_spatulae = (setae_count as f32) * (spatula_count_per_seta as f32);
        let contact_area_m2 = spatula_area_m2 * total_spatulae;
        // Autumn 实测单根 seta 约 200 μN，500000 根 ≈ 100 N
        let adhesion_force_n = (setae_count as f32) * 200e-6;
        Self {
            setae_count,
            seta_length_um,
            seta_diameter_um,
            spatula_count_per_seta,
            spatula_size_nm,
            contact_area_m2,
            adhesion_force_n,
        }
    }

    /// Van der Waals 力：F = H/(6π·d³) · area
    /// H ≈ 0.4×10⁻¹⁹ J (Hamaker 常数)
    pub fn adhesion_force(&self, distance_nm: f32) -> f32 {
        let hamaker_j = 0.4e-19_f32;
        let d_m = (distance_nm.max(0.1)) * 1e-9;
        let pressure_pa = hamaker_j / (6.0 * std::f32::consts::PI * d_m * d_m * d_m);
        pressure_pa * self.contact_area_m2
    }

    /// 剪切力（壁虎能挂自身重量约 100 倍）
    pub fn shear_force(&self) -> f32 {
        // 实测 shear 约为 normal adhesion 的 2 倍
        self.adhesion_force_n * 2.0
    }
}

impl Default for GeckoAdhesion {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 2. 章鱼触手（静水骨骼 + 吸盘）============
// 来源：Kier 1985, Smith 2013
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OctopusTentacle {
    /// 触手长度，60-90 cm
    pub length_cm: f32,
    /// 触手直径
    pub diameter_cm: f32,
    /// 吸盘数量，约 240 个/触手
    pub suckers_count: u32,
    /// 吸盘直径
    pub sucker_diameter_mm: f32,
    /// 横肌（径向收缩，伸长）
    pub transverse_muscle: f32,
    /// 纵肌（缩短，弯曲）
    pub longitudinal_muscle: f32,
    /// 螺旋肌（扭转）
    pub helical_muscle: f32,
    /// 体腔压力
    pub pressure_kpa: f32,
}

impl OctopusTentacle {
    pub fn new() -> Self {
        Self {
            length_cm: 75.0,
            diameter_cm: 3.0,
            suckers_count: 240,
            sucker_diameter_mm: 15.0,
            transverse_muscle: 0.5,
            longitudinal_muscle: 0.5,
            helical_muscle: 0.0,
            pressure_kpa: 5.0,
        }
    }

    /// 弯曲：纵肌一侧收缩，对侧伸展，体腔压力维持刚度（antagonistic 控制）
    pub fn bend(&mut self, angle_rad: f32) {
        let s = (angle_rad.tanh() + 1.0) * 0.5; // 映射到 0..1
        self.longitudinal_muscle = s;
        self.transverse_muscle = 1.0 - s * 0.5;
        self.pressure_kpa = 5.0 + angle_rad.abs() * 2.0;
    }

    /// 伸长：横肌径向收缩 → 体积守恒 → 长度增加
    pub fn extend(&mut self, length_cm: f32) {
        self.length_cm = length_cm.clamp(30.0, 120.0);
        // 体积近似守恒：L1·D1² = L2·D2²
        let base_length = 75.0_f32;
        let base_diameter = 3.0_f32;
        let ratio = (base_length / self.length_cm).sqrt();
        self.diameter_cm = (base_diameter * ratio).max(1.5);
        self.transverse_muscle = 0.7;
        self.longitudinal_muscle = 0.3;
    }

    /// 吸盘吸附力 F = ΔP · A
    pub fn sucker_force(&self, suction_pressure_kpa: f32) -> f32 {
        let radius_m = self.sucker_diameter_mm * 0.5 * 1e-3;
        let area_m2 = std::f32::consts::PI * radius_m * radius_m;
        let pressure_pa = suction_pressure_kpa * 1e3;
        pressure_pa * area_m2 * (self.suckers_count as f32)
    }
}

impl Default for OctopusTentacle {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 3. 电鳗电器官（电板细胞）============
// 来源：Catania 2017, Science
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElectricPulse {
    pub voltage: f32,
    pub current: f32,
    pub duration_ms: f32,
    pub energy_j: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Electrocyte {
    /// 电板数量，5000-10000 个串联
    pub count: u32,
    /// 单电板电压，约 0.15 V
    pub voltage_per_cell_v: f32,
    /// 总电压，600-860 V
    pub total_voltage_v: f32,
    /// 电流，约 1 A
    pub current_a: f32,
    /// 脉冲时长，约 2 ms
    pub pulse_duration_ms: f32,
    /// 放电频率，约 400 Hz
    pub discharge_rate_hz: f32,
}

impl Electrocyte {
    pub fn new() -> Self {
        let count: u32 = 6000;
        let voltage_per_cell_v = 0.15;
        // 6000 × 0.15V ≈ 900V，钳制在 860V（实测上限）
        let total_voltage_v = ((count as f32) * voltage_per_cell_v).min(860.0);
        Self {
            count,
            voltage_per_cell_v,
            total_voltage_v,
            current_a: 1.0,
            pulse_duration_ms: 2.0,
            discharge_rate_hz: 400.0,
        }
    }

    /// 单次放电脉冲
    pub fn discharge(&self) -> ElectricPulse {
        let duration_s = self.pulse_duration_ms * 1e-3;
        let energy_j = self.total_voltage_v * self.current_a * duration_s;
        ElectricPulse {
            voltage: self.total_voltage_v,
            current: self.current_a,
            duration_ms: self.pulse_duration_ms,
            energy_j,
        }
    }

    /// 峰值功率 P = V·I，电鳗约 600 W
    pub fn power_output(&self) -> f32 {
        self.total_voltage_v * self.current_a
    }
}

impl Default for Electrocyte {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 4. 生物发光（荧光素酶）============
// 来源：Wilson 1976, Hastings 2012
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BioluminescenceType {
    /// 萤火虫（黄绿光 550nm）
    Firefly,
    /// 弧菌（蓝绿光 490nm）
    Bacterial,
    /// 甲藻（蓝色 474nm，海洋发光）
    Dinoflagellate,
    /// 腔肠素（水母，蓝光 460nm）
    Coelenterazine,
    /// 绿色荧光蛋白（维多利亚多管发光水母）
    GFP,
    /// 海萤（蓝色 460nm）
    Vargula,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bioluminescence {
    /// 荧光素浓度 (μM)
    pub luciferin_conc_um: f32,
    /// 荧光素酶浓度 (μM)
    pub luciferase_conc_um: f32,
    /// ATP 浓度 (μM)，萤火虫反应需要
    pub atp_conc_um: f32,
    /// 氧气浓度 (μM)
    pub oxygen_conc_um: f32,
    /// 发光波长 (nm)
    pub wavelength_nm: f32,
    /// 量子产率 0.1-0.9
    pub quantum_yield: f32,
    /// 当前亮度 (lux)
    pub brightness_lux: f32,
}

impl Bioluminescence {
    pub fn new(btype: BioluminescenceType) -> Self {
        // 各类生物发光的波长、量子产率、是否依赖 ATP
        let (wavelength_nm, quantum_yield, atp_required) = match btype {
            BioluminescenceType::Firefly => (550.0, 0.88, true),
            BioluminescenceType::Bacterial => (490.0, 0.10, false),
            BioluminescenceType::Dinoflagellate => (474.0, 0.20, false),
            BioluminescenceType::Coelenterazine => (460.0, 0.30, false),
            BioluminescenceType::GFP => (509.0, 0.80, false),
            BioluminescenceType::Vargula => (460.0, 0.28, false),
        };
        Self {
            luciferin_conc_um: 100.0,
            luciferase_conc_um: 50.0,
            atp_conc_um: if atp_required { 5000.0 } else { 0.0 },
            oxygen_conc_um: 250.0,
            wavelength_nm,
            quantum_yield,
            brightness_lux: 0.0,
        }
    }

    /// 萤火虫反应：luciferin + O2 + ATP → oxyluciferin + light
    /// 反应速率 k = k_max · [L] · [E] · [ATP] · [O2]
    /// 返回发光强度（任意单位）
    pub fn emit(&mut self, dt: f32) -> f32 {
        let k_max = 1e-3_f32;
        let atp_factor = if self.atp_conc_um > 0.0 { self.atp_conc_um } else { 1.0 };
        let rate = k_max
            * self.luciferin_conc_um
            * self.luciferase_conc_um
            * atp_factor
            * self.oxygen_conc_um;
        // 单步消耗不超过当前荧光素的 10%
        let consumed = (rate * dt).min(self.luciferin_conc_um * 0.1);
        self.luciferin_conc_um = (self.luciferin_conc_um - consumed).max(0.0);
        self.oxygen_conc_um = (self.oxygen_conc_um - consumed * 0.5).max(0.0);
        if self.atp_conc_um > 0.0 {
            self.atp_conc_um = (self.atp_conc_um - consumed * 0.5).max(0.0);
        }
        let intensity = consumed * self.quantum_yield * 1e3;
        self.brightness_lux = intensity;
        intensity
    }
}

// ============ 5. 蜘蛛丝（超强材料）============
// 来源：Gosline 1999, Omenetto 2010
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SilkType {
    /// 大壶腹丝（牵引丝，最强）
    MajorAmpullate,
    /// 小壶腹丝
    MinorAmpullate,
    /// 鞭状丝（捕获丝，弹性极高）
    Flagelliform,
    /// 卵囊丝
    Cylindriform,
    /// 葡状丝（包裹猎物）
    Aciniform,
    /// 梨状丝（附着盘）
    Pyriform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiderSilk {
    pub silk_type: SilkType,
    /// 直径，1-5 μm
    pub diameter_um: f32,
    /// 拉伸强度 (GPa)，钢铁 1.0, 凯夫拉 3.0
    pub tensile_strength_gpa: f32,
    /// 断裂延伸率，30-200%
    pub elongation_pct: f32,
    /// 杨氏模量 (GPa)
    pub youngs_modulus_gpa: f32,
    /// 韧性 (MJ/m³)，最强天然纤维
    pub toughness_mj_m3: f32,
}

impl SpiderSilk {
    pub fn new(silk_type: SilkType) -> Self {
        // 各类蜘蛛丝的力学参数（实测值）
        let (tensile_strength_gpa, elongation_pct, youngs_modulus_gpa, toughness_mj_m3, diameter_um) =
            match silk_type {
                SilkType::MajorAmpullate => (1.1, 30.0, 10.0, 160.0, 3.0),
                SilkType::MinorAmpullate => (1.0, 30.0, 11.0, 130.0, 2.0),
                SilkType::Flagelliform => (0.5, 200.0, 2.0, 150.0, 1.5),
                SilkType::Cylindriform => (0.4, 50.0, 8.0, 80.0, 4.0),
                SilkType::Aciniform => (0.7, 80.0, 7.0, 120.0, 1.0),
                SilkType::Pyriform => (0.5, 50.0, 6.0, 90.0, 2.5),
            };
        Self {
            silk_type,
            diameter_um,
            tensile_strength_gpa,
            elongation_pct,
            youngs_modulus_gpa,
            toughness_mj_m3,
        }
    }

    /// 应力-应变曲线（双线性模型：弹性段 + 硬化段）
    pub fn stress_strain(&self, strain: f32) -> f32 {
        let s = strain.max(0.0);
        let yield_strain = 0.02; // 2%
        let yield_stress = self.youngs_modulus_gpa * yield_strain;
        if s <= yield_strain {
            self.youngs_modulus_gpa * s
        } else {
            let max_strain = self.elongation_pct * 0.01;
            if s >= max_strain {
                0.0 // 断裂
            } else {
                let hardening =
                    (self.tensile_strength_gpa - yield_stress) / (max_strain - yield_strain);
                yield_stress + hardening * (s - yield_strain)
            }
        }
    }

    /// 断裂能（单位体积 MJ/m³）
    pub fn energy_to_break(&self) -> f32 {
        self.toughness_mj_m3
    }
}

// ============ 6. 变色龙色素细胞 ============
// 来源：Teyssier 2015, Nature Communications
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChromatophoreType {
    /// 黑色素细胞（黑/棕）
    Melanophore,
    /// 黄色素细胞（黄/橙）
    Xanthophore,
    /// 红色素细胞
    Erythrophore,
    /// 虹彩细胞（结构色，鸟嘌呤晶体）
    Iridophore,
    /// 白色素细胞
    Leucophore,
    /// 蓝色素细胞（罕见）
    Cyanophore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chromatophore {
    pub ctype: ChromatophoreType,
    pub pigment_density: f32,
    /// 细胞直径，100-200 μm
    pub cell_diameter_um: f32,
    /// 色素迁移状态：0.0=聚集（亮）, 1.0=分散（暗）
    pub migration_state: f32,
    pub neural_input: f32,
    pub hormone_input: f32,
}

impl Chromatophore {
    pub fn new(ctype: ChromatophoreType) -> Self {
        Self {
            ctype,
            pigment_density: 1.0,
            cell_diameter_um: 150.0,
            migration_state: 0.5,
            neural_input: 0.0,
            hormone_input: 0.0,
        }
    }

    /// 更新色素迁移：神经 + 激素 + 外部刺激驱动
    pub fn update(&mut self, dt: f32, stimulus: f32) {
        let total = self.neural_input + self.hormone_input + stimulus;
        let target = total.tanh().clamp(0.0, 1.0);
        // 一阶动态响应，时间常数 τ = 1s
        let tau = 1.0;
        let alpha = 1.0 - (-dt / tau).exp();
        self.migration_state += (target - self.migration_state) * alpha;
        self.migration_state = self.migration_state.clamp(0.0, 1.0);
    }

    /// 输出 RGB（简化模型，0.0-1.0）
    pub fn color_output(&self) -> (f32, f32, f32) {
        let d = self.migration_state.clamp(0.0, 1.0);
        match self.ctype {
            ChromatophoreType::Melanophore => (d * 0.1, d * 0.05, d * 0.0),
            ChromatophoreType::Xanthophore => (d * 1.0, d * 0.8, d * 0.1),
            ChromatophoreType::Erythrophore => (d * 0.9, d * 0.1, d * 0.05),
            ChromatophoreType::Iridophore => {
                // 结构色：随晶体间距变化（简化为青绿）
                (d * 0.2, d * 0.9, d * 0.8)
            }
            ChromatophoreType::Leucophore => (d, d, d),
            ChromatophoreType::Cyanophore => (d * 0.1, d * 0.6, d * 0.9),
        }
    }
}

// ============ 7. 水熊虫隐生（Cryptobiosis）============
// 来源：Jönsson 2003, Boothby 2017
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CryptobiosisType {
    /// 脱水隐生（最常见）
    Anhydrobiosis,
    /// 冷冻隐生
    Cryobiosis,
    /// 渗透隐生
    Osmobiosis,
    /// 缺氧隐生
    Anoxybiosis,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnvironmentCondition {
    pub temp_c: f32,
    pub pressure_mpa: f32,
    pub radiation_gy: f32,
    /// 水活度 0.0-1.0
    pub water_activity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TardigradeCryptobiosis {
    pub state: CryptobiosisType,
    /// 含水量：活跃 85%, 隐生 3%
    pub water_content_pct: f32,
    /// 海藻糖浓度（保护剂）
    pub trehalose_conc: f32,
    /// DPA（损伤保护蛋白）
    pub dpa_conc: f32,
    /// CAHS（无序蛋白）
    pub caahs_conc: f32,
    /// 隐生可耐 -273°C ~ +150°C
    pub survival_temp_c: f32,
    /// 隐生可耐 600 MPa
    pub survival_pressure_mpa: f32,
    /// 辐射耐受 5000 Gy（人类 LD50 = 4-10 Gy）
    pub radiation_dose_gy: f32,
    pub vacuum_survival_days: u32,
}

impl TardigradeCryptobiosis {
    pub fn new() -> Self {
        Self {
            state: CryptobiosisType::Anhydrobiosis,
            water_content_pct: 85.0,
            trehalose_conc: 0.0,
            dpa_conc: 0.0,
            caahs_conc: 0.0,
            survival_temp_c: 150.0,
            survival_pressure_mpa: 600.0,
            radiation_dose_gy: 5000.0,
            vacuum_survival_days: 10,
        }
    }

    /// 进入隐生：脱水 + 合成保护剂
    pub fn enter(&mut self) {
        self.water_content_pct = 3.0;
        self.trehalose_conc = 2.0; // 占干重 2-2.5%
        self.dpa_conc = 1.0;
        self.caahs_conc = 1.0;
    }

    /// 退出隐生：复水 + 降解保护剂
    pub fn exit(&mut self) {
        self.water_content_pct = 85.0;
        self.trehalose_conc = 0.0;
        self.dpa_conc = 0.0;
        self.caahs_conc = 0.0;
    }

    /// 在给定环境下隐生存活概率（0.0-1.0）
    pub fn survival_probability(&self, condition: EnvironmentCondition) -> f32 {
        let temp_ok = condition.temp_c >= -273.0 && condition.temp_c <= self.survival_temp_c;
        let p_ok = condition.pressure_mpa <= self.survival_pressure_mpa;
        let r_ok = condition.radiation_gy <= self.radiation_dose_gy;
        let w_ok = condition.water_activity >= 0.0 && condition.water_activity <= 1.0;
        if !(temp_ok && p_ok && r_ok && w_ok) {
            return 0.0;
        }
        // 各因素线性衰减
        let t_factor = 1.0 - (condition.temp_c.abs() / self.survival_temp_c.max(1.0)).min(0.95);
        let p_factor = 1.0 - (condition.pressure_mpa / self.survival_pressure_mpa).min(0.95);
        let r_factor = 1.0 - (condition.radiation_gy / self.radiation_dose_gy).min(0.95);
        // 隐生状态对低水活度更耐受
        let w_factor = if condition.water_activity < 0.6 { 1.0 } else { 0.5 };
        t_factor * p_factor * r_factor * w_factor
    }
}

impl Default for TardigradeCryptobiosis {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 8. 红外感知（蝮蛇颊窝）============
// 来源：Bakken 2007, Gracheva 2010
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraredPitOrgan {
    /// 膜厚度，约 15 μm
    pub membrane_thickness_um: f32,
    /// 热灵敏度，约 0.001 K（千分之一度）
    pub thermal_sensitivity_k: f32,
    /// 检测范围，0.5-1.0 m
    pub detection_range_m: f32,
    /// 角分辨率，约 5°
    pub angular_resolution_deg: f32,
    /// 神经末梢数量，约 7000
    pub channels: u32,
}

impl InfraredPitOrgan {
    pub fn new() -> Self {
        Self {
            membrane_thickness_um: 15.0,
            thermal_sensitivity_k: 0.001,
            detection_range_m: 1.0,
            angular_resolution_deg: 5.0,
            channels: 7000,
        }
    }

    /// Stefan-Boltzmann: P = σ·A·(T_target^4 - T_ambient^4) / (4π·r²)
    /// 返回膜上辐射通量密度 (W/m²)
    pub fn detect(&self, target_temp_k: f32, ambient_temp_k: f32, distance_m: f32) -> f32 {
        let sigma = 5.670374419e-8_f32; // Stefan-Boltzmann 常数
        let d = distance_m.max(0.001);
        let delta_t4 = target_temp_k.powi(4) - ambient_temp_k.powi(4);
        let flux = sigma * delta_t4 / (4.0 * std::f32::consts::PI * d * d);
        // 低于灵敏度阈值 → 无信号
        if flux.abs() < 1e-6 {
            0.0
        } else {
            flux
        }
    }
}

impl Default for InfraredPitOrgan {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 9. 电感知（鲨鱼劳伦氏壶腹）============
// 来源：Murray 1960, Kalmijn 1971
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmpullaeOfLorenzini {
    /// 壶腹数量，数百到数千
    pub count: u32,
    pub length_cm: f32,
    /// 孔径，0.1-2.0 mm
    pub pore_diameter_mm: f32,
    /// 灵敏度，约 5 nV/cm（极敏感）
    pub sensitivity_uv_per_cm: f32,
    pub detection_range_m: f32,
}

impl AmpullaeOfLorenzini {
    pub fn new() -> Self {
        Self {
            count: 1500,
            length_cm: 20.0,
            pore_diameter_mm: 0.5,
            // 5 nV/cm = 0.005 μV/cm
            sensitivity_uv_per_cm: 5e-3,
            detection_range_m: 0.5,
        }
    }

    /// 检测电场强度，返回归一化信号强度（0.0-1.0）
    pub fn detect_field(&self, field_strength_v_per_m: f32, distance_m: f32) -> f32 {
        let d = distance_m.max(0.001);
        // 偶极子场 1/r³ 衰减
        let field_at_sensor = field_strength_v_per_m / (d * d * d);
        // 灵敏度阈值 μV/cm 转 V/m：1 μV/cm = 1e-4 V/m
        let threshold_v_per_m = self.sensitivity_uv_per_cm * 1e-4;
        if field_at_sensor.abs() < threshold_v_per_m {
            0.0
        } else {
            (field_at_sensor / threshold_v_per_m).tanh()
        }
    }
}

impl Default for AmpullaeOfLorenzini {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 10. 磁感知（候鸟磁罗盘）============
// 来源：Wiltschko 1972, Hore 2016
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MagnetoreceptionType {
    /// 自由基对（隐花色素 cryptochrome）
    RadicalPair,
    /// 磁铁矿颗粒
    Magnetite,
    /// 电磁感应（鲨鱼）
    Induction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Magnetoreception {
    pub mechanism: MagnetoreceptionType,
    /// 灵敏度 (nT)
    pub sensitivity_nt: f32,
    /// 地磁场强度，约 50 μT
    pub field_strength_ut: f32,
    /// 是否能感知倾角
    pub inclination_detection: bool,
    /// 定向精度 (度)
    pub directional_accuracy_deg: f32,
}

impl Magnetoreception {
    pub fn new(mech: MagnetoreceptionType) -> Self {
        let (sensitivity_nt, inclination_detection, directional_accuracy_deg) = match mech {
            // 自由基对机制：高灵敏 + 倾角感知
            MagnetoreceptionType::RadicalPair => (10.0, true, 5.0),
            // 磁铁矿颗粒：中等灵敏，无倾角
            MagnetoreceptionType::Magnetite => (50.0, false, 10.0),
            // 电磁感应：低灵敏，无倾角
            MagnetoreceptionType::Induction => (1000.0, false, 30.0),
        };
        Self {
            mechanism: mech,
            sensitivity_nt,
            field_strength_ut: 50.0,
            inclination_detection,
            directional_accuracy_deg,
        }
    }

    /// 返回 (方位角, 仰角) 弧度
    pub fn sense_direction(&self, field_ut: f32, inclination_deg: f32) -> (f32, f32) {
        let field = if field_ut < 1e-3 { self.field_strength_ut } else { field_ut };
        let incl_rad = inclination_deg.to_radians();
        // 简化：方位角基于场强比，仰角 = 倾角（若可感知）
        let azimuth = (field / self.field_strength_ut).atan2(1.0);
        let elevation = if self.inclination_detection { incl_rad } else { 0.0 };
        (azimuth, elevation)
    }
}

// ============ 11. 其他奇特结构汇总 ============
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoticCapability {
    pub name: String,
    pub organism: String,
    pub capability: String,
    pub biomimetic_applications: Vec<String>,
}

/// 跨物种奇特生物能力数据库
pub fn exotic_capabilities_database() -> Vec<ExoticCapability> {
    vec![
        ExoticCapability {
            name: "GeckoAdhesion".into(),
            organism: "壁虎".into(),
            capability: "Van der Waals 刚毛附着".into(),
            biomimetic_applications: vec!["干胶带".into(), "爬墙机器人".into()],
        },
        ExoticCapability {
            name: "OctopusTentacle".into(),
            organism: "章鱼".into(),
            capability: "静水骨骼 + 吸盘".into(),
            biomimetic_applications: vec!["软体机器人".into()],
        },
        ExoticCapability {
            name: "Electrocyte".into(),
            organism: "电鳗".into(),
            capability: "电器官放电 600V/1A".into(),
            biomimetic_applications: vec!["生物电池".into(), "柔性电源".into()],
        },
        ExoticCapability {
            name: "Bioluminescence".into(),
            organism: "萤火虫/甲藻/水母".into(),
            capability: "荧光素酶冷光发光".into(),
            biomimetic_applications: vec!["生物传感器".into(), "无光源照明".into()],
        },
        ExoticCapability {
            name: "SpiderSilk".into(),
            organism: "蜘蛛".into(),
            capability: "高强度高韧性纤维".into(),
            biomimetic_applications: vec!["防弹衣".into(), "医用缝线".into()],
        },
        ExoticCapability {
            name: "Chromatophore".into(),
            organism: "变色龙/章鱼".into(),
            capability: "色素细胞动态变色".into(),
            biomimetic_applications: vec!["自适应迷彩".into(), "智能织物".into()],
        },
        ExoticCapability {
            name: "TardigradeCryptobiosis".into(),
            organism: "水熊虫".into(),
            capability: "隐生抗极端环境".into(),
            biomimetic_applications: vec!["疫苗干燥保存".into(), "太空生存".into()],
        },
        ExoticCapability {
            name: "InfraredPitOrgan".into(),
            organism: "蝮蛇".into(),
            capability: "颊窝红外温差检测 0.001K".into(),
            biomimetic_applications: vec!["红外传感器".into()],
        },
        ExoticCapability {
            name: "AmpullaeOfLorenzini".into(),
            organism: "鲨鱼".into(),
            capability: "劳伦氏壶腹电场感知 5nV/cm".into(),
            biomimetic_applications: vec!["水下电场探测".into()],
        },
        ExoticCapability {
            name: "Magnetoreception".into(),
            organism: "候鸟".into(),
            capability: "磁罗盘定向".into(),
            biomimetic_applications: vec!["GPS-free 导航".into()],
        },
        ExoticCapability {
            name: "Echolocation".into(),
            organism: "蝙蝠".into(),
            capability: "回声定位".into(),
            biomimetic_applications: vec!["超声成像".into(), "声纳".into()],
        },
        ExoticCapability {
            name: "SalamanderRegeneration".into(),
            organism: "蝾螈".into(),
            capability: "肢体再生".into(),
            biomimetic_applications: vec!["再生医学".into()],
        },
        ExoticCapability {
            name: "HydrothermalVentExtremophile".into(),
            organism: "嗜极菌".into(),
            capability: "高温高压酶系统".into(),
            biomimetic_applications: vec!["工业酶".into(), "PCR 酶".into()],
        },
        ExoticCapability {
            name: "Venom".into(),
            organism: "蛇/蝎/蜘蛛/水母".into(),
            capability: "神经毒素/溶血毒素".into(),
            biomimetic_applications: vec!["药物开发".into(), "止痛药".into()],
        },
    ]
}


#[cfg(test)]
mod tests {
    use super::*;

    // ===== GeckoAdhesion =====
    #[test]
    fn test_gecko_adhesion_new_and_default_field_values() {
        let g = GeckoAdhesion::new();
        assert_eq!(g.setae_count, 500_000);
        assert_eq!(g.seta_length_um, 100.0);
        assert_eq!(g.seta_diameter_um, 5.0);
        assert_eq!(g.spatula_count_per_seta, 500);
        assert_eq!(g.spatula_size_nm, 200.0);
        assert!(g.adhesion_force_n > 0.0);
        assert!(g.contact_area_m2 > 0.0);
        // Default 应等价于 new
        let d = GeckoAdhesion::default();
        assert_eq!(d.setae_count, g.setae_count);
        assert_eq!(d.adhesion_force_n, g.adhesion_force_n);
        assert_eq!(d.contact_area_m2, g.contact_area_m2);
    }

    #[test]
    fn test_gecko_adhesion_force_clamps_zero_distance() {
        let g = GeckoAdhesion::new();
        // 0.0 nm 应被钳制到 0.1 nm，不 panic
        let f_zero = g.adhesion_force(0.0);
        let f_min = g.adhesion_force(0.1);
        assert!(f_zero.is_finite());
        assert!(f_min.is_finite());
        assert!(f_zero > 0.0);
        // 钳制后两者应相等
        assert!((f_zero - f_min).abs() < 1e-6);
    }

    #[test]
    fn test_gecko_adhesion_shear_force_double_adhesion() {
        let g = GeckoAdhesion::new();
        // shear = adhesion_force_n * 2.0
        assert!((g.shear_force() - g.adhesion_force_n * 2.0).abs() < 1e-6);
    }

    // ===== OctopusTentacle =====
    #[test]
    fn test_octopus_tentacle_new_defaults() {
        let t = OctopusTentacle::new();
        assert_eq!(t.length_cm, 75.0);
        assert_eq!(t.diameter_cm, 3.0);
        assert_eq!(t.suckers_count, 240);
        assert_eq!(t.sucker_diameter_mm, 15.0);
        assert_eq!(t.transverse_muscle, 0.5);
        assert_eq!(t.longitudinal_muscle, 0.5);
        assert_eq!(t.helical_muscle, 0.0);
        assert_eq!(t.pressure_kpa, 5.0);
    }

    #[test]
    fn test_octopus_tentacle_bend_increases_pressure() {
        let mut t = OctopusTentacle::new();
        let p0 = t.pressure_kpa;
        t.bend(1.0);
        // bend(1.0) 后压力 = 5.0 + |1.0|*2.0 = 7.0
        assert!((t.pressure_kpa - (p0 + 2.0)).abs() < 1e-6);
        // longitudinal_muscle 应被更新 (tanh(1)+1)/2 ≈ 0.88 > 0.5
        assert!(t.longitudinal_muscle > 0.5);
    }

    #[test]
    fn test_octopus_tentacle_extend_clamps_to_range() {
        let mut t_high = OctopusTentacle::new();
        t_high.extend(200.0); // 应钳制到 120
        assert_eq!(t_high.length_cm, 120.0);

        let mut t_low = OctopusTentacle::new();
        t_low.extend(10.0); // 应钳制到 30
        assert_eq!(t_low.length_cm, 30.0);
    }

    #[test]
    fn test_octopus_tentacle_sucker_force_scales_with_pressure() {
        let t = OctopusTentacle::new();
        let f_low = t.sucker_force(10.0);
        let f_high = t.sucker_force(20.0);
        assert!(f_low > 0.0);
        // 压力翻倍 → 吸附力翻倍
        assert!((f_high - f_low * 2.0).abs() < 1e-3);
    }

    // ===== Electrocyte =====
    #[test]
    fn test_electrocyte_new_voltage_capped_at_860() {
        let e = Electrocyte::new();
        // 6000 × 0.15 = 900，但钳制到 860
        assert_eq!(e.count, 6000);
        assert_eq!(e.voltage_per_cell_v, 0.15);
        assert_eq!(e.total_voltage_v, 860.0);
        assert_eq!(e.current_a, 1.0);
        assert_eq!(e.pulse_duration_ms, 2.0);
        assert_eq!(e.discharge_rate_hz, 400.0);
    }

    #[test]
    fn test_electrocyte_discharge_energy_calculation() {
        let e = Electrocyte::new();
        let pulse = e.discharge();
        assert_eq!(pulse.voltage, 860.0);
        assert_eq!(pulse.current, 1.0);
        assert_eq!(pulse.duration_ms, 2.0);
        // E = V·I·t = 860 × 1 × 2e-3 = 1.72 J
        assert!((pulse.energy_j - 1.72).abs() < 1e-4);
    }

    // ===== Bioluminescence =====
    #[test]
    fn test_bioluminescence_new_wavelength_and_atp_dependency() {
        let firefly = Bioluminescence::new(BioluminescenceType::Firefly);
        assert_eq!(firefly.wavelength_nm, 550.0);
        assert_eq!(firefly.quantum_yield, 0.88);
        assert_eq!(firefly.atp_conc_um, 5000.0); // 萤火虫反应需要 ATP

        let bacterial = Bioluminescence::new(BioluminescenceType::Bacterial);
        assert_eq!(bacterial.wavelength_nm, 490.0);
        assert_eq!(bacterial.atp_conc_um, 0.0); // 细菌发光不依赖 ATP
    }

    #[test]
    fn test_bioluminescence_emit_consumes_luciferin() {
        let mut b = Bioluminescence::new(BioluminescenceType::Firefly);
        let before = b.luciferin_conc_um;
        let intensity = b.emit(1e-6);
        assert!(intensity > 0.0);
        assert!(b.luciferin_conc_um < before);
        // brightness_lux 应被同步设置为返回的强度
        assert_eq!(b.brightness_lux, intensity);
    }

    // ===== SpiderSilk =====
    #[test]
    fn test_spider_silk_stress_strain_fracture_returns_zero() {
        let s = SpiderSilk::new(SilkType::MajorAmpullate);
        // MajorAmpullate elongation=30% → max_strain=0.30
        // strain >= 0.30 → 断裂，返回 0
        assert_eq!(s.stress_strain(0.30), 0.0);
        assert_eq!(s.stress_strain(0.50), 0.0);
    }

    #[test]
    fn test_spider_silk_stress_strain_elastic_linear() {
        let s = SpiderSilk::new(SilkType::MajorAmpullate);
        // 弹性段：strain <= 0.02，stress = youngs_modulus * strain
        // MajorAmpullate youngs_modulus = 10.0
        assert!((s.stress_strain(0.01) - 0.10).abs() < 1e-6);
        assert!((s.stress_strain(0.005) - 0.05).abs() < 1e-6);
        assert!((s.stress_strain(0.0)).abs() < 1e-6);
    }

    // ===== Chromatophore =====
    #[test]
    fn test_chromatophore_color_output_leucophore_full() {
        let mut c = Chromatophore::new(ChromatophoreType::Leucophore);
        assert_eq!(c.migration_state, 0.5);
        c.migration_state = 1.0;
        let (r, g, b) = c.color_output();
        // Leucophore 在 migration=1.0 时返回 (1,1,1)
        assert!((r - 1.0).abs() < 1e-6);
        assert!((g - 1.0).abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);
    }

    // ===== TardigradeCryptobiosis =====
    #[test]
    fn test_tardigrade_enter_drops_water_and_synthesizes_protectants() {
        let mut t = TardigradeCryptobiosis::new();
        assert_eq!(t.water_content_pct, 85.0);
        assert_eq!(t.trehalose_conc, 0.0);
        t.enter();
        assert_eq!(t.water_content_pct, 3.0);
        assert_eq!(t.trehalose_conc, 2.0);
        assert_eq!(t.dpa_conc, 1.0);
        assert_eq!(t.caahs_conc, 1.0);
    }

    #[test]
    fn test_tardigrade_survival_zero_when_temp_exceeds_limit() {
        let t = TardigradeCryptobiosis::new();
        let cond = EnvironmentCondition {
            temp_c: 200.0, // 超过 survival_temp_c=150
            pressure_mpa: 0.1,
            radiation_gy: 0.0,
            water_activity: 0.3,
        };
        assert_eq!(t.survival_probability(cond), 0.0);
    }

    // ===== InfraredPitOrgan =====
    #[test]
    fn test_infrared_pit_organ_detect_zero_when_equal_temps() {
        let ir = InfraredPitOrgan::new();
        // 目标温度 == 环境温度 → delta_t4=0 → flux=0 → 返回 0
        let flux = ir.detect(300.0, 300.0, 0.5);
        assert_eq!(flux, 0.0);
    }

    // ===== AmpullaeOfLorenzini =====
    #[test]
    fn test_ampullae_detect_field_threshold_behavior() {
        let a = AmpullaeOfLorenzini::new();
        // 极小电场 + 远距离 → 低于灵敏度阈值 → 0
        assert_eq!(a.detect_field(1e-10, 1.0), 0.0);
        // 较强电场 → 高于阈值 → tanh 饱和，返回 (0,1] 内正值
        let signal = a.detect_field(1e-3, 0.5);
        assert!(signal > 0.0);
        assert!(signal <= 1.0);
    }

    // ===== Magnetoreception =====
    #[test]
    fn test_magnetoreception_radical_pair_detects_inclination() {
        let m = Magnetoreception::new(MagnetoreceptionType::RadicalPair);
        assert!(m.inclination_detection);
        assert_eq!(m.sensitivity_nt, 10.0);
        // 方位角/仰角：能感知倾角 → elevation = 45° in rad
        let (_azimuth, elevation) = m.sense_direction(50.0, 45.0);
        assert!((elevation - 45.0_f32.to_radians()).abs() < 1e-6);

        // Magnetite 无倾角感知 → elevation 应为 0
        let mag = Magnetoreception::new(MagnetoreceptionType::Magnetite);
        assert!(!mag.inclination_detection);
        let (_, elev2) = mag.sense_direction(50.0, 45.0);
        assert!(elev2.abs() < 1e-6);
    }

    // ===== exotic_capabilities_database =====
    #[test]
    fn test_exotic_capabilities_database_count_fourteen() {
        let db = exotic_capabilities_database();
        assert_eq!(db.len(), 14);
        // 抽样校验若干条目
        assert!(db.iter().any(|c| c.name == "GeckoAdhesion"));
        assert!(db.iter().any(|c| c.name == "Magnetoreception"));
        assert!(db.iter().any(|c| c.name == "Venom"));
        // 每条都应有非空 organism / capability / biomimetic_applications
        for c in &db {
            assert!(!c.organism.is_empty());
            assert!(!c.capability.is_empty());
            assert!(!c.biomimetic_applications.is_empty());
        }
    }
}
