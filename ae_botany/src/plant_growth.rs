//! 植物生长模块
//!
//! 覆盖生长模型（Logistic / Von Bertalanffy / Gompertz / Weibull）、
//! 相对生长速率 RGR、资源分配（根茎叶果实种子）、环境因子影响
//! （温度 / 水分 / 光照 / 营养响应函数）、分生组织活动、生物量积累。
//!
//! 所有数值使用 f32，时间单位为 day（天），生物量单位为 g（克），
//! 光合有效辐射单位为 μmol/m²/s。

use serde::{Deserialize, Serialize};

// ============================================================================
// 生长模型枚举
// ============================================================================

/// 生长方程类型
///
/// 不同植物器官或不同生长阶段适用不同的数学模型：
/// - `Logistic`：S 型曲线，适合整体生物量积累
/// - `VonBertalanffy`：渐近增长，适合动物与某些果实
/// - `Gompertz`：不对称 S 型，早期增长更快
/// - `Weibull`：柔性形状，可拟合多种生长模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrowthModel {
    Logistic,
    VonBertalanffy,
    Gompertz,
    Weibull,
}

// ============================================================================
// 生长参数与状态
// ============================================================================

/// 生长参数集合
///
/// `k` 为生长速率常数，`t0` 为初始时间偏移，
/// `l_max` 为渐近最大尺寸或生物量，`beta` 为 Weibull 形状参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthParams {
    /// 生长速率常数 (1/day)
    pub k: f32,
    /// 初始时间 (day)
    pub t0: f32,
    /// 最大尺寸 / 生物量（渐近值）
    pub l_max: f32,
    /// Weibull 形状参数（>0）
    pub beta: f32,
}

impl Default for GrowthParams {
    fn default() -> Self {
        Self {
            k: 0.05,
            t0: 0.0,
            l_max: 100.0,
            beta: 1.5,
        }
    }
}

/// 生长状态快照
///
/// 用于描述某一时刻植物的生长状态，由 `integrate_growth` 数值积分产生。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthState {
    /// 当前时间 (day)
    pub time_day: f32,
    /// 当前尺寸（高度 / 长度等，单位与 `l_max` 一致）
    pub size: f32,
    /// 当前生物量 (g)
    pub biomass_g: f32,
    /// 相对生长速率 RGR (1/day)
    pub rgr: f32,
    /// 作物生长速率 CGR (g/m²/day)
    pub cgr: f32,
}

// ============================================================================
// 资源分配
// ============================================================================

/// 资源分配策略
///
/// 各器官分配比例，应当满足总和为 1。
/// 根冠比（root:shoot）会随环境胁迫动态调整。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStrategy {
    /// 根分配比例
    pub root_fraction: f32,
    /// 茎（shoot，含茎与叶柄等支撑组织）分配比例
    pub shoot_fraction: f32,
    /// 叶分配比例
    pub leaf_fraction: f32,
    /// 果实分配比例
    pub fruit_fraction: f32,
    /// 种子分配比例
    pub seed_fraction: f32,
}

impl Default for AllocationStrategy {
    /// 默认分配策略：根 0.20、茎 0.25、叶 0.30、果 0.15、种 0.10
    fn default() -> Self {
        Self {
            root_fraction: 0.20,
            shoot_fraction: 0.25,
            leaf_fraction: 0.30,
            fruit_fraction: 0.15,
            seed_fraction: 0.10,
        }
    }
}

impl AllocationStrategy {
    /// 校验各比例之和是否为 1（容差 1e-4）
    pub fn is_normalized(&self) -> bool {
        let sum = self.root_fraction
            + self.shoot_fraction
            + self.leaf_fraction
            + self.fruit_fraction
            + self.seed_fraction;
        (sum - 1.0).abs() < 1e-4
    }

    /// 归一化到总和为 1
    pub fn normalize(&mut self) {
        let sum = self.root_fraction
            + self.shoot_fraction
            + self.leaf_fraction
            + self.fruit_fraction
            + self.seed_fraction;
        if sum > 0.0 {
            let inv = 1.0 / sum;
            self.root_fraction *= inv;
            self.shoot_fraction *= inv;
            self.leaf_fraction *= inv;
            self.fruit_fraction *= inv;
            self.seed_fraction *= inv;
        }
    }
}

/// 资源分配结果（生物量绝对值）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationResult {
    /// 根生物量 (g)
    pub root_biomass: f32,
    /// 茎生物量 (g)
    pub shoot_biomass: f32,
    /// 叶生物量 (g)
    pub leaf_biomass: f32,
    /// 果实生物量 (g)
    pub fruit_biomass: f32,
    /// 种子生物量 (g)
    pub seed_biomass: f32,
}

impl AllocationResult {
    /// 总生物量
    pub fn total(&self) -> f32 {
        self.root_biomass
            + self.shoot_biomass
            + self.leaf_biomass
            + self.fruit_biomass
            + self.seed_biomass
    }
}

// ============================================================================
// 环境因子
// ============================================================================

/// 环境因子集合
///
/// 用于驱动生长速率修正与资源分配调整。
/// `water_stress` 范围 0..1，1 表示无胁迫（水分充足），0 表示完全干旱。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalFactors {
    /// 气温 (°C)
    pub temperature_c: f32,
    /// 水分胁迫指数 0..1（1=无胁迫）
    pub water_stress: f32,
    /// 光合有效辐射 PAR (μmol/m²/s)
    pub light_intensity: f32,
    /// 土壤速效氮 (mg/kg)
    pub nitrogen: f32,
    /// 土壤速效磷 (mg/kg)
    pub phosphorus: f32,
    /// 土壤速效钾 (mg/kg)
    pub potassium: f32,
    /// 大气 CO2 浓度 (ppm)
    pub co2_ppm: f32,
}

impl Default for EnvironmentalFactors {
    fn default() -> Self {
        Self {
            temperature_c: 22.0,
            water_stress: 1.0,
            light_intensity: 800.0,
            nitrogen: 50.0,
            phosphorus: 20.0,
            potassium: 80.0,
            co2_ppm: 400.0,
        }
    }
}

// ============================================================================
// 分生组织活动
// ============================================================================

/// 分生组织活动状态
///
/// 描述顶端分生组织与侧生分生组织的活动强度，
/// 用于决定营养生长与生殖生长的切换。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeristemActivity {
    /// 顶端分生组织活动强度 0..1
    pub apical_activity: f32,
    /// 侧生分生组织活动强度 0..1
    pub lateral_activity: f32,
    /// 花分生组织活动强度 0..1
    pub floral_activity: f32,
    /// 是否进入生殖生长阶段
    pub reproductive: bool,
}

impl Default for MeristemActivity {
    fn default() -> Self {
        Self {
            apical_activity: 1.0,
            lateral_activity: 0.5,
            floral_activity: 0.0,
            reproductive: false,
        }
    }
}

impl MeristemActivity {
    /// 根据环境因子与发育阶段更新分生组织活动
    ///
    /// `day_length_h` 为日长（小时），>12 触发开花倾向。
    /// `maturity` 为成熟度 0..1，>0.6 时花分生组织活动增强。
    pub fn update(&mut self, factors: &EnvironmentalFactors, day_length_h: f32, maturity: f32) {
        let temp_f = temperature_response(factors.temperature_c, 25.0, 40.0, 2.0);
        let water_f = water_response(factors.water_stress);
        let env = temp_f * water_f;

        self.apical_activity = (env * (1.0 - maturity * 0.5)).clamp(0.0, 1.0);
        self.lateral_activity = (env * 0.8).clamp(0.0, 1.0);

        let floral_signal = (day_length_h > 12.0) as i32 as f32;
        self.floral_activity = (maturity * floral_signal * env).clamp(0.0, 1.0);
        self.reproductive = self.floral_activity > 0.3;
    }
}

// ============================================================================
// 生长方程
// ============================================================================

/// 生长方程：根据模型计算 t 时刻的尺寸
///
/// # 公式
/// - Logistic:        `L(t) = L_max / (1 + exp(-k*(t-t0)))`
/// - VonBertalanffy:  `L(t) = L_max * (1 - exp(-k*(t-t0)))^3`
/// - Gompertz:        `L(t) = L_max * exp(-exp(-k*(t-t0)))`
/// - Weibull:         `L(t) = L_max * (1 - exp(-(k*(t-t0))^beta))`
///
/// 当 `t < t0` 时返回 0。
pub fn growth_curve(model: GrowthModel, params: &GrowthParams, t: f32) -> f32 {
    let dt = t - params.t0;
    if dt < 0.0 || params.l_max <= 0.0 {
        return 0.0;
    }
    let l_max = params.l_max;
    let k = params.k;
    match model {
        GrowthModel::Logistic => {
            let arg = -k * dt;
            l_max / (1.0 + arg.exp())
        }
        GrowthModel::VonBertalanffy => {
            let inner = 1.0 - (-k * dt).exp();
            if inner <= 0.0 {
                0.0
            } else {
                l_max * inner.powi(3)
            }
        }
        GrowthModel::Gompertz => {
            let arg = -((-k * dt).exp());
            l_max * arg.exp()
        }
        GrowthModel::Weibull => {
            let beta = if params.beta > 0.0 { params.beta } else { 1.0 };
            let base = k * dt;
            if base <= 0.0 {
                0.0
            } else {
                let inner = -base.powf(beta);
                l_max * (1.0 - inner.exp())
            }
        }
    }
}

/// 生长速率（导数 dL/dt）
///
/// 返回当前尺寸随时间的变化率，单位与 `l_max` 相同 / day。
pub fn growth_rate(model: GrowthModel, params: &GrowthParams, t: f32) -> f32 {
    let dt = t - params.t0;
    if dt < 0.0 || params.l_max <= 0.0 || params.k <= 0.0 {
        return 0.0;
    }
    let l_max = params.l_max;
    let k = params.k;
    match model {
        GrowthModel::Logistic => {
            // dL/dt = k * L * (1 - L/Lmax)
            let l = growth_curve(model, params, t);
            k * l * (1.0 - l / l_max)
        }
        GrowthModel::VonBertalanffy => {
            // dL/dt = 3 * k * Lmax * (1 - exp(-k dt))^2 * exp(-k dt)
            let e = (-k * dt).exp();
            let inner = 1.0 - e;
            if inner <= 0.0 {
                0.0
            } else {
                3.0 * k * l_max * inner * inner * e
            }
        }
        GrowthModel::Gompertz => {
            // dL/dt = k * L * ln(Lmax / L)
            let l = growth_curve(model, params, t);
            if l <= 0.0 {
                return 0.0;
            }
            let ratio = l_max / l;
            if ratio <= 0.0 {
                0.0
            } else {
                k * l * ratio.ln()
            }
        }
        GrowthModel::Weibull => {
            // dL/dt = Lmax * beta * k^beta * t^(beta-1) * exp(-(k t)^beta)
            let beta = if params.beta > 0.0 { params.beta } else { 1.0 };
            let base = k * dt;
            if base <= 0.0 {
                return 0.0;
            }
            let pow_beta = base.powf(beta);
            l_max * beta * k * (base.powf(beta - 1.0)) * (-pow_beta).exp()
        }
    }
}

/// 数值积分生长过程
///
/// 使用显式 Euler 法从 `t_start` 到 `t_end`，步长 `dt`，
/// 返回每个时间步的 `GrowthState`。
///
/// 假设尺寸与生物量等价（`biomass_g = size`），RGR 通过相邻样本对数差计算，
/// CGR 假设种植密度 10 株/m² 进行换算（可后续参数化）。
pub fn integrate_growth(
    model: GrowthModel,
    params: &GrowthParams,
    t_start: f32,
    t_end: f32,
    dt: f32,
) -> Vec<GrowthState> {
    if dt <= 0.0 || t_end <= t_start {
        return Vec::new();
    }
    let mut states = Vec::new();
    let mut t = t_start;
    let density_plants_per_m2: f32 = 10.0;
    let mut prev_mass: Option<f32> = None;

    while t <= t_end + 1e-6 {
        let size = growth_curve(model, params, t);
        let biomass = size.max(0.0);
        let rate = growth_rate(model, params, t);

        let rgr = match prev_mass {
            Some(pm) if pm > 1e-9 && biomass > 1e-9 && dt > 0.0 => {
                ((biomass / pm).ln()) / dt
            }
            _ => 0.0,
        };
        let cgr = rate * density_plants_per_m2;

        states.push(GrowthState {
            time_day: t,
            size,
            biomass_g: biomass,
            rgr,
            cgr,
        });

        prev_mass = Some(biomass);
        t += dt;
    }
    states
}

// ============================================================================
// 相对生长速率 RGR
// ============================================================================

/// 相对生长速率 RGR = (ln W2 - ln W1) / (t2 - t1)
///
/// 输入两次采样的生物量与时间间隔，返回 RGR (1/day)。
/// 当输入质量非正或 dt 非正时返回 0。
pub fn relative_growth_rate(mass_t1: f32, mass_t2: f32, dt: f32) -> f32 {
    if mass_t1 <= 0.0 || mass_t2 <= 0.0 || dt <= 0.0 {
        return 0.0;
    }
    (mass_t2.ln() - mass_t1.ln()) / dt
}

// ============================================================================
// 资源分配
// ============================================================================

/// 资源分配
///
/// 根据分配策略将总生物量切分到各器官。
/// 不强制归一化，若策略和不为 1，按比例线性分配（结果可能少于或大于总量）。
pub fn allocate_biomass(total_biomass: f32, strategy: &AllocationStrategy) -> AllocationResult {
    AllocationResult {
        root_biomass: total_biomass * strategy.root_fraction,
        shoot_biomass: total_biomass * strategy.shoot_fraction,
        leaf_biomass: total_biomass * strategy.leaf_fraction,
        fruit_biomass: total_biomass * strategy.fruit_fraction,
        seed_biomass: total_biomass * strategy.seed_fraction,
    }
}

// ============================================================================
// 环境响应函数
// ============================================================================

/// 温度响应函数（Q10 模型 + 高温抑制）
///
/// 当 `temp_c == t_opt` 时返回 1.0；
/// 低于最适温度时按 Q10 衰减；
/// 高于最适温度时按 `(t_max - temp) / (t_max - t_opt)` 线性下降到 0。
///
/// 参数：
/// - `temp_c`：当前气温
/// - `t_opt`：最适温度
/// - `t_max`：最高耐受温度
/// - `q10`：温度系数（每升高 10°C 反应速率倍数）
pub fn temperature_response(temp_c: f32, t_opt: f32, t_max: f32, q10: f32) -> f32 {
    if t_max <= t_opt {
        return 0.0;
    }
    if temp_c <= t_opt {
        // 低温段：Q10 衰减
        let delta = (t_opt - temp_c) / 10.0;
        q10.powf(-delta)
    } else if temp_c >= t_max {
        0.0
    } else {
        // 高温段：线性下降
        (t_max - temp_c) / (t_max - t_opt)
    }
}

/// 水分响应函数
///
/// `water_stress` 为 0..1，1=无胁迫。响应值随胁迫线性下降，
/// 并在胁迫较强时附加非线性抑制（平方项）以体现阈值效应。
pub fn water_response(water_stress: f32) -> f32 {
    let s = water_stress.clamp(0.0, 1.0);
    // 0..1：线性 + 轻微非线性
    s * (0.5 + 0.5 * s)
}

/// 光响应曲线（Mitscherlich 模型）
///
/// `A = Amax * (1 - exp(-ε * Q / Amax))`
///
/// 参数：
/// - `light_intensity`：PAR (μmol/m²/s)
/// - `a_max`：最大光合速率
/// - `epsilon`：初始光能利用率
pub fn light_response(light_intensity: f32, a_max: f32, epsilon: f32) -> f32 {
    if a_max <= 0.0 || epsilon < 0.0 || light_intensity <= 0.0 {
        return 0.0;
    }
    a_max * (1.0 - (-epsilon * light_intensity / a_max).exp())
}

/// 营养响应（Michaelis-Menten）
///
/// `R = C / (Km + C)`
///
/// 当 `C = Km` 时响应为 0.5，当 `C >> Km` 时趋近 1。
pub fn nutrient_response(concentration: f32, km: f32) -> f32 {
    if km <= 0.0 {
        return if concentration > 0.0 { 1.0 } else { 0.0 };
    }
    if concentration <= 0.0 {
        return 0.0;
    }
    concentration / (km + concentration)
}

/// 综合环境因子（0..1）
///
/// 取温度、水分、光照、N/P/K 五个响应的最小值（李比希最低率定律），
/// 再与 CO2 增益因子相乘。CO2 在 400ppm 时增益为 1，每升高 100ppm 增加 ~5%。
pub fn environmental_modifier(factors: &EnvironmentalFactors) -> f32 {
    let temp_f = temperature_response(factors.temperature_c, 25.0, 40.0, 2.0);
    let water_f = water_response(factors.water_stress);
    // 光响应归一化到 0..1：以 1500 μmol/m²/s 为饱和参考
    let light_f = light_response(factors.light_intensity, 1.0, 0.003).clamp(0.0, 1.0);
    let n_f = nutrient_response(factors.nitrogen, 10.0);
    let p_f = nutrient_response(factors.phosphorus, 5.0);
    let k_f = nutrient_response(factors.potassium, 15.0);

    let limiting = temp_f
        .min(water_f)
        .min(light_f)
        .min(n_f)
        .min(p_f)
        .min(k_f);

    // CO2 增益因子
    let co2_gain = 1.0 + ((factors.co2_ppm - 400.0) / 100.0).max(-0.5) * 0.05;
    let co2_gain = co2_gain.clamp(0.5, 1.5);

    (limiting * co2_gain).clamp(0.0, 1.0)
}

/// 最佳分配策略（根据环境调整根冠比）
///
/// - 水分胁迫增强 → 根分配增加（深根寻水）
/// - 氮素不足 → 根分配增加（拓展根际）
/// - 低光 → 茎与叶分配增加（向上生长争取光）
/// - 进入生殖阶段（通过环境暗示）→ 果实与种子分配增加
pub fn optimal_allocation(factors: &EnvironmentalFactors) -> AllocationStrategy {
    // 基线
    let mut root = 0.20_f32;
    let mut shoot = 0.25_f32;
    let mut leaf = 0.30_f32;
    let mut fruit = 0.15_f32;
    let mut seed = 0.10_f32;

    // 水分胁迫：从 1.0 → 0.0 时根比例提升最多 +0.15
    let water_deficit = (1.0 - factors.water_stress).clamp(0.0, 1.0);
    root += 0.15 * water_deficit;

    // 氮胁迫：低于 50 mg/kg 时根比例增加
    let n_factor = nutrient_response(factors.nitrogen, 50.0);
    root += 0.10 * (1.0 - n_factor);

    // 低光：茎增加（争取高度）、叶增加（扩大受光面）
    let light_norm = (factors.light_intensity / 1500.0).clamp(0.0, 1.0);
    let low_light = 1.0 - light_norm;
    shoot += 0.10 * low_light;
    leaf += 0.10 * low_light;

    // 温度适宜且水分充足时，生殖分配增加
    let temp_f = temperature_response(factors.temperature_c, 25.0, 40.0, 2.0);
    let env_repro = temp_f * factors.water_stress;
    if env_repro > 0.7 {
        let boost = (env_repro - 0.7) / 0.3;
        fruit += 0.10 * boost;
        seed += 0.05 * boost;
    }

    // 为保持总量守恒，从叶中扣除增加量
    let excess = root + shoot + leaf + fruit + seed - 1.0;
    if excess > 0.0 {
        leaf -= excess;
    }

    let mut s = AllocationStrategy {
        root_fraction: root.max(0.0),
        shoot_fraction: shoot.max(0.0),
        leaf_fraction: leaf.max(0.0),
        fruit_fraction: fruit.max(0.0),
        seed_fraction: seed.max(0.0),
    };
    s.normalize();
    s
}

// ============================================================================
// 生物量积累
// ============================================================================

/// 生物量积累驱动器
///
/// 结合生长模型与环境因子，按时间步长累积生物量。
/// 每步：`biomass += growth_rate * dt * env_modifier * meristem_factor`
///
/// 返回最终生物量（g）。
pub fn accumulate_biomass(
    model: GrowthModel,
    params: &GrowthParams,
    factors: &EnvironmentalFactors,
    meristem: &MeristemActivity,
    t_start: f32,
    t_end: f32,
    dt: f32,
) -> f32 {
    if dt <= 0.0 || t_end <= t_start {
        return growth_curve(model, params, t_start);
    }
    let env = environmental_modifier(factors);
    let meristem_factor = if meristem.reproductive {
        meristem.floral_activity.max(0.1)
    } else {
        meristem.apical_activity
    };

    let mut t = t_start;
    let mut biomass = growth_curve(model, params, t);

    while t < t_end {
        let rate = growth_rate(model, params, t);
        let increment = rate * dt * env * meristem_factor;
        biomass = (biomass + increment).max(0.0);
        t += dt;
    }
    biomass
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Logistic 模型在 t→∞ 时趋近 l_max
    #[test]
    fn test_logistic_growth() {
        let params = GrowthParams {
            k: 0.1,
            t0: 0.0,
            l_max: 100.0,
            beta: 1.0,
        };
        let l_far = growth_curve(GrowthModel::Logistic, &params, 1000.0);
        assert!(
            (l_far - 100.0).abs() < 1e-2,
            "Logistic 应趋近 l_max，得到 {}",
            l_far
        );

        // t = t0 时应当为 l_max / 2
        let l_mid = growth_curve(GrowthModel::Logistic, &params, 0.0);
        assert!((l_mid - 50.0).abs() < 1e-2);
    }

    /// 速率在拐点处最大（Logistic 拐点位于 t0）
    #[test]
    fn test_growth_rate() {
        let params = GrowthParams {
            k: 0.2,
            t0: 10.0,
            l_max: 100.0,
            beta: 1.0,
        };
        let rate_at_inflection = growth_rate(GrowthModel::Logistic, &params, 10.0);
        let rate_before = growth_rate(GrowthModel::Logistic, &params, 5.0);
        let rate_after = growth_rate(GrowthModel::Logistic, &params, 15.0);
        assert!(
            rate_at_inflection > rate_before,
            "拐点处速率应大于拐点前"
        );
        assert!(
            rate_at_inflection > rate_after,
            "拐点处速率应大于拐点后"
        );
        // 理论最大值 k * l_max / 4
        let theoretical_max = 0.2 * 100.0 / 4.0;
        assert!(
            (rate_at_inflection - theoretical_max).abs() < 1e-2,
            "拐点速率应接近 {}，得到 {}",
            theoretical_max,
            rate_at_inflection
        );
    }

    /// 分配比例和为 1
    #[test]
    fn test_allocation() {
        let strategy = AllocationStrategy::default();
        assert!(strategy.is_normalized(), "默认策略比例和应为 1");

        let result = allocate_biomass(1000.0, &strategy);
        let total = result.total();
        assert!(
            (total - 1000.0).abs() < 1e-2,
            "分配总生物量应为 1000，得到 {}",
            total
        );
        // 各分量非负
        assert!(result.root_biomass >= 0.0);
        assert!(result.shoot_biomass >= 0.0);
        assert!(result.leaf_biomass >= 0.0);
        assert!(result.fruit_biomass >= 0.0);
        assert!(result.seed_biomass >= 0.0);
    }

    /// 最适温度下响应为 1
    #[test]
    fn test_temperature_response() {
        let r_opt = temperature_response(25.0, 25.0, 40.0, 2.0);
        assert!(
            (r_opt - 1.0).abs() < 1e-6,
            "最适温度下响应应为 1，得到 {}",
            r_opt
        );

        // 高温超过 t_max 时为 0
        let r_hot = temperature_response(45.0, 25.0, 40.0, 2.0);
        assert!((r_hot - 0.0).abs() < 1e-6);

        // 低温时响应小于 1
        let r_cold = temperature_response(15.0, 25.0, 40.0, 2.0);
        assert!(r_cold > 0.0 && r_cold < 1.0);
    }

    /// 强光下趋近 Amax
    #[test]
    fn test_light_response() {
        // Amax = 1.0, 强光下趋近 1
        let r_strong = light_response(5000.0, 1.0, 0.0008);
        assert!(
            (r_strong - 1.0).abs() < 2e-2,
            "强光下应趋近 Amax，得到 {}",
            r_strong
        );

        // 零光下为 0
        let r_zero = light_response(0.0, 1.0, 0.0008);
        assert!((r_zero - 0.0).abs() < 1e-6);

        // 中等光强下应小于 Amax
        let r_mid = light_response(500.0, 1.0, 0.0008);
        assert!(r_mid > 0.0 && r_mid < 1.0);
    }

    /// 营养响应 Michaelis-Menten
    #[test]
    fn test_nutrient_response() {
        // C = Km 时响应为 0.5
        let r_half = nutrient_response(50.0, 50.0);
        assert!((r_half - 0.5).abs() < 1e-6);

        // C >> Km 时趋近 1
        let r_high = nutrient_response(5000.0, 50.0);
        assert!((r_high - 1.0).abs() < 1e-2);

        // C = 0 时为 0
        let r_zero = nutrient_response(0.0, 50.0);
        assert!((r_zero - 0.0).abs() < 1e-6);
    }

    /// 相对生长速率计算
    #[test]
    fn test_relative_growth_rate() {
        // 质量翻倍、dt=10 → RGR = ln(2)/10
        let rgr = relative_growth_rate(10.0, 20.0, 10.0);
        let expected = 2f32.ln() / 10.0;
        assert!((rgr - expected).abs() < 1e-6);

        // 非法输入返回 0
        assert_eq!(relative_growth_rate(0.0, 10.0, 10.0), 0.0);
        assert_eq!(relative_growth_rate(10.0, 20.0, 0.0), 0.0);
    }

    /// 数值积分生成多个状态点
    #[test]
    fn test_integrate_growth() {
        let params = GrowthParams {
            k: 0.1,
            t0: 0.0,
            l_max: 100.0,
            beta: 1.0,
        };
        let states = integrate_growth(GrowthModel::Logistic, &params, 0.0, 50.0, 5.0);
        assert!(!states.is_empty(), "应生成至少一个状态点");
        assert!(states.len() >= 10, "应有约 11 个点，得到 {}", states.len());

        // 第一个点为 t=0
        assert!((states[0].time_day - 0.0).abs() < 1e-6);
        // 最后一个点接近 t=50
        let last = states.last().unwrap();
        assert!((last.time_day - 50.0).abs() < 1e-3);

        // 生物量应单调非减
        for w in states.windows(2) {
            assert!(w[1].biomass_g >= w[0].biomass_g - 1e-6, "生物量应单调非减");
        }
    }

    /// 最佳分配策略在胁迫下根比例上升
    #[test]
    fn test_optimal_allocation_drought() {
        let mut factors = EnvironmentalFactors::default();
        let baseline = optimal_allocation(&factors);
        assert!(baseline.is_normalized());

        // 干旱胁迫
        factors.water_stress = 0.2;
        let drought = optimal_allocation(&factors);
        assert!(
            drought.root_fraction > baseline.root_fraction,
            "干旱下根分配应增加：baseline={} drought={}",
            baseline.root_fraction,
            drought.root_fraction
        );
        assert!(drought.is_normalized());
    }

    /// 综合环境因子在理想条件下接近 1
    #[test]
    fn test_environmental_modifier_ideal() {
        let factors = EnvironmentalFactors {
            temperature_c: 25.0,
            water_stress: 1.0,
            light_intensity: 2000.0,
            nitrogen: 200.0,
            phosphorus: 100.0,
            potassium: 300.0,
            co2_ppm: 400.0,
        };
        let m = environmental_modifier(&factors);
        assert!(m > 0.95, "理想环境下综合因子应接近 1，得到 {}", m);
    }
}
