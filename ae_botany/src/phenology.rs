//! 物候学模块
//!
//! 本模块负责模拟植物的物候发育进程，覆盖以下核心内容：
//! - 物候期（萌发、展叶、开花、结果、落叶、休眠等）
//! - 积温模型（Growing Degree Days, GDD）
//! - 光周期反应（Photoperiodism）
//! - 春化作用（Vernalization）
//! - BBCH 物候期标准化 scale
//! - 气候变化对物候的影响
//! - 物候预测模型
//!
//! 设计参考：
//! - BBCH 量表（Meier 1997, BBCH Monoograph）
//! - 单三角形 GDD 法（Sevacherian et al. 1977）
//! - 春化模型（Chew 2012, Static Wheat Model）
//! - 临界日长光周期模型（Thomas & Vince-Prue 1997）

use serde::{Deserialize, Serialize};

// ============================================================================
// 物候期定义
// ============================================================================

/// 植物物候期枚举
///
/// 描述植物在一年中生命周期所处的发育阶段。
/// 顺序大体遵循：休眠 → 萌芽 → 展叶 → 拔节 → 开花 → 坐果 → 果实发育 → 成熟 → 衰老 → 落叶。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhenologicalPhase {
    /// 休眠（芽休眠），由低温或短日诱导
    Dormancy,
    /// 萌芽，芽鳞开裂、萌动
    BudBurst,
    /// 展叶，幼叶展开至完全展开
    LeafExpansion,
    /// 拔节/茎伸长，节间伸长
    StemElongation,
    /// 开花，花冠开放
    Flowering,
    /// 坐果，授粉受精后果实始膨大
    FruitSet,
    /// 果实发育，果实膨大至生理成熟前
    FruitDevelopment,
    /// 成熟，果实/种子达到生理或工艺成熟
    Ripening,
    /// 衰老，叶片黄化、组织退化
    Senescence,
    /// 落叶，叶片脱落
    LeafFall,
}

impl PhenologicalPhase {
    /// 物候期的中文显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dormancy => "休眠期",
            Self::BudBurst => "萌芽期",
            Self::LeafExpansion => "展叶期",
            Self::StemElongation => "拔节期",
            Self::Flowering => "开花期",
            Self::FruitSet => "坐果期",
            Self::FruitDevelopment => "果实发育期",
            Self::Ripening => "成熟期",
            Self::Senescence => "衰老期",
            Self::LeafFall => "落叶期",
        }
    }

    /// 该物候期是否属于营养生长阶段
    pub fn is_vegetative(&self) -> bool {
        matches!(
            self,
            Self::BudBurst | Self::LeafExpansion | Self::StemElongation
        )
    }

    /// 该物候期是否属于生殖生长阶段
    pub fn is_reproductive(&self) -> bool {
        matches!(
            self,
            Self::Flowering | Self::FruitSet | Self::FruitDevelopment | Self::Ripening
        )
    }
}

// ============================================================================
// BBCH 物候期标准化 scale
// ============================================================================

/// BBCH 物候期阶段
///
/// 国际通用农作物发育阶段标准化量表，由主阶段（0-9）和次阶段（0-9）组成。
/// 主阶段：
/// - 0: 发芽/萌芽
/// - 1: 叶发育
/// - 2: 分蘖/侧枝形成
/// - 3: 茎伸长
/// - 4: 抽穗/-booting
/// - 5: 花序发育
/// - 6: 开花
/// - 7: 果实发育
/// - 8: 果实成熟
/// - 9: 衰老/开始休眠
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BBCHStage {
    /// 主阶段（0-9）
    pub principal_stage: u8,
    /// 次阶段（0-9）
    pub secondary_stage: u8,
}

impl BBCHStage {
    /// 构造新的 BBCH 阶段
    pub fn new(principal: u8, secondary: u8) -> Self {
        Self {
            principal_stage: principal.min(9),
            secondary_stage: secondary.min(9),
        }
    }

    /// 返回两位 BBCH 代码字符串，例如 "13"、"65"
    pub fn code(&self) -> String {
        format!("{:02}", self.principal_stage * 10 + self.secondary_stage)
    }

    /// 从 0-99 的整数代码解析 BBCH 阶段
    pub fn from_code(code: u8) -> Self {
        let p = (code / 10).min(9);
        let s = (code % 10).min(9);
        Self {
            principal_stage: p,
            secondary_stage: s,
        }
    }

    /// 主阶段中文名称
    pub fn principal_name(&self) -> &'static str {
        match self.principal_stage {
            0 => "发芽/萌芽",
            1 => "叶发育",
            2 => "分蘖/侧枝",
            3 => "茎伸长",
            4 => "抽穗",
            5 => "花序发育",
            6 => "开花",
            7 => "果实发育",
            8 => "果实成熟",
            9 => "衰老",
            _ => "未知",
        }
    }
}

// ============================================================================
// 光周期反应
// ============================================================================

/// 光周期反应类型
///
/// 植物开花对日长的响应类型，依据临界日长进行分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhotoperiodResponse {
    /// 日中性植物（Day Neutral Plant, DNP），开花不受日长影响
    DayNeutral,
    /// 短日植物（Short Day Plant, SDP），日长短于临界值时开花
    ShortDay,
    /// 长日植物（Long Day Plant, LDP），日长长于临界值时开花
    LongDay,
    /// 短长日植物（Short-Long Day Plant, SLDP），需先短日后长日
    ShortLongDay,
    /// 长短日植物（Long-Short Day Plant, LSDP），需先长日后短日
    LongShortDay,
}

impl PhotoperiodResponse {
    /// 当前日长是否对该反应类型构成诱导
    pub fn is_inductive(&self, daylength_h: f32, critical_h: f32) -> bool {
        match self {
            Self::DayNeutral => true,
            Self::ShortDay | Self::LongShortDay => daylength_h <= critical_h,
            Self::LongDay | Self::ShortLongDay => daylength_h >= critical_h,
        }
    }
}

/// 光周期状态
///
/// 跟踪植物对光周期的累计诱导量，判定是否达到开花转换阈值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoperiodState {
    /// 光周期反应类型
    pub response: PhotoperiodResponse,
    /// 临界日长（小时）
    pub critical_daylength_h: f32,
    /// 诱导所需的累计天数
    pub induction_days: u32,
    /// 当前累计诱导量
    pub accumulated_induction: f32,
}

impl PhotoperiodState {
    /// 构造默认日中性植物的光周期状态
    pub fn day_neutral() -> Self {
        Self {
            response: PhotoperiodResponse::DayNeutral,
            critical_daylength_h: 12.0,
            induction_days: 0,
            accumulated_induction: 0.0,
        }
    }

    /// 是否已完成光周期诱导
    pub fn is_induced(&self) -> bool {
        self.accumulated_induction >= self.induction_days as f32
    }

    /// 重置诱导状态
    pub fn reset(&mut self) {
        self.accumulated_induction = 0.0;
    }
}

// ============================================================================
// 春化作用
// ============================================================================

/// 春化状态
///
/// 跟踪植物对低温的累计响应量。许多冬性植物（如冬小麦、需冷果树）
/// 必须积累足够的低温量才能完成花芽分化或打破芽休眠。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VernalizationState {
    /// 完成春化所需的累计需冷量（< 7°C 累计小时数）
    pub required_chill_hours: f32,
    /// 当前累计冷温（有效春化小时数）
    pub accumulated_chill: f32,
    /// 是否已完成春化
    pub vernalized: bool,
    /// 最适春化温度（°C）
    pub optimal_temp_c: f32,
}

impl VernalizationState {
    /// 春化进度 [0, 1]
    pub fn progress(&self) -> f32 {
        if self.required_chill_hours <= 0.0 {
            return 1.0;
        }
        (self.accumulated_chill / self.required_chill_hours).clamp(0.0, 1.0)
    }

    /// 重置春化状态
    pub fn reset(&mut self) {
        self.accumulated_chill = 0.0;
        self.vernalized = false;
    }
}

// ============================================================================
// 积温（Growing Degree Days）
// ============================================================================

/// 积温模型参数
///
/// GDD（生长度日）是作物发育常用的热量单位，定义为：
///   GDD = ((T_max + T_min) / 2) - T_base
/// 并对上限进行截断。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowingDegreeDays {
    /// 基础温度（发育下限温度，°C）
    pub base_temp_c: f32,
    /// 发育上限温度（°C）
    pub upper_temp_c: f32,
    /// 当前累计积温（°C·d）
    pub accumulated_gdd: f32,
    /// 达到目标阶段所需积温（°C·d）
    pub required_gdd: f32,
}

impl GrowingDegreeDays {
    /// 进度 [0, 1]
    pub fn progress(&self) -> f32 {
        if self.required_gdd <= 0.0 {
            return 1.0;
        }
        (self.accumulated_gdd / self.required_gdd).clamp(0.0, 1.0)
    }

    /// 是否达到目标积温
    pub fn is_complete(&self) -> bool {
        self.accumulated_gdd >= self.required_gdd
    }
}

// ============================================================================
// 物候模型与阶段转换
// ============================================================================

/// 物候发育综合模型
///
/// 综合考虑 GDD、光周期、春化、冷温积累等因素，
/// 用于驱动植物个体级别的物候推进。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhenologyModel {
    /// 当前物候期
    pub current_phase: PhenologicalPhase,
    /// 积温模型
    pub gdd: GrowingDegreeDays,
    /// 光周期状态
    pub photoperiod: PhotoperiodState,
    /// 春化状态
    pub vernalization: VernalizationState,
    /// 冷温累计（< 7°C 小时数，用于打破芽休眠）
    pub chill_accumulation: f32,
    /// 当前年内日序（1-365）
    pub day_of_year: u32,
    /// 所在纬度（度）
    pub latitude_deg: f32,
}

impl PhenologyModel {
    /// 构造新模型实例
    pub fn new(latitude_deg: f32, day_of_year: u32) -> Self {
        Self {
            current_phase: PhenologicalPhase::Dormancy,
            gdd: GrowingDegreeDays {
                base_temp_c: 5.0,
                upper_temp_c: 30.0,
                accumulated_gdd: 0.0,
                required_gdd: 1500.0,
            },
            photoperiod: PhotoperiodState::day_neutral(),
            vernalization: VernalizationState {
                required_chill_hours: 1200.0,
                accumulated_chill: 0.0,
                vernalized: false,
                optimal_temp_c: 5.0,
            },
            chill_accumulation: 0.0,
            day_of_year,
            latitude_deg,
        }
    }

    /// 是否完成春化
    pub fn is_vernalized(&self) -> bool {
        self.vernalization.vernalized
    }

    /// 是否完成光周期诱导
    pub fn is_photoperiod_induced(&self) -> bool {
        self.photoperiod.is_induced()
    }

    /// 当前日长（基于纬度和日序）
    pub fn current_daylength(&self) -> f32 {
        day_length(self.latitude_deg, self.day_of_year)
    }
}

/// 阶段转换规则
///
/// 描述从某物候期进入下一阶段所需满足的阈值条件。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseTransition {
    /// 起始阶段
    pub from_phase: PhenologicalPhase,
    /// 目标阶段
    pub to_phase: PhenologicalPhase,
    /// 所需累计 GDD 阈值（°C·d）
    pub gdd_threshold: f32,
    /// 所需日长阈值（小时），None 表示不限制
    pub daylength_required: Option<f32>,
    /// 所需冷温累计阈值（小时），None 表示不限制
    pub chill_required: Option<f32>,
}

impl PhaseTransition {
    /// 构造仅依据 GDD 的转换规则
    pub fn by_gdd(from: PhenologicalPhase, to: PhenologicalPhase, gdd: f32) -> Self {
        Self {
            from_phase: from,
            to_phase: to,
            gdd_threshold: gdd,
            daylength_required: None,
            chill_required: None,
        }
    }
}

// ============================================================================
// 核心算法函数
// ============================================================================

/// 计算给定纬度和日序的日长（小时）
///
/// 采用标准天文公式：
///   declination δ = 23.45° × sin(360° × (doy + 284) / 365)
///   sunset angle ω₀ = acos(-tan(φ) × tan(δ))
///   daylength = (2 / 15°) × ω₀ = 24/π × ω₀ (rad)
///
/// # 参数
/// - `latitude_deg`: 纬度（度），北半球为正
/// - `day_of_year`: 年内日序（1-365）
///
/// # 返回
/// 日长小时数 [0, 24]，极夜返回 0，极昼返回 24
pub fn day_length(latitude_deg: f32, day_of_year: u32) -> f32 {
    let lat_rad = latitude_deg.to_radians();
    // 太阳赤纬
    let angle = (360.0 / 365.0) * (day_of_year as f32 + 284.0);
    let declination = (23.45_f32).to_radians() * angle.to_radians().sin();

    let cos_h = -lat_rad.tan() * declination.tan();

    // 数值边界处理
    if cos_h >= 1.0 {
        // 极夜
        return 0.0;
    }
    if cos_h <= -1.0 {
        // 极昼
        return 24.0;
    }

    let half_day_angle = cos_h.acos();
    // 1 弧度 = 180/π 度，地球每小时转 15°，故日长 = 2 × half_day_angle × (180/π) / 15 = 24/π × half_day_angle
    24.0 / std::f32::consts::PI * half_day_angle
}

/// 单日 GDD 计算（单三角形法）
///
/// 采用 Sevacherian et al. (1977) 提出的单三角形法：
/// - 当 T_max ≤ T_base 时，GDD = 0
/// - 当 T_min ≥ T_base 时，GDD = ((T_max + T_min) / 2) - T_base
/// - 当 T_min < T_base < T_max 时，使用三角形积分：
///       GDD = (T_max - T_base)² / (2 × (T_max - T_min))
///
/// 上限温度 T_upper 对 T_max 进行截断处理。
///
/// # 参数
/// - `t_min`: 当日最低气温（°C）
/// - `t_max`: 当日最高气温（°C）
/// - `base`: 基础温度（°C）
/// - `upper`: 上限温度（°C）
pub fn daily_gdd(t_min: f32, t_max: f32, base: f32, upper: f32) -> f32 {
    // 数值保护
    if t_max <= t_min || upper <= base {
        return 0.0;
    }

    // 上限截断
    let t_max_capped = t_max.min(upper);
    if t_max_capped <= base {
        return 0.0;
    }

    if t_min >= base {
        // 整日温度高于基础温度，使用平均法
        let t_avg = (t_min + t_max_capped) / 2.0;
        return (t_avg - base).max(0.0);
    }

    // 三角形法：T_min < T_base < T_max
    let gdd = (t_max_capped - base).powi(2) / (2.0 * (t_max_capped - t_min));
    gdd.max(0.0)
}

/// 更新物候模型（推进一天）
///
/// 综合处理温度、光周期、春化、冷温等累计量，并推进 day_of_year。
///
/// # 参数
/// - `model`: 物候模型可变引用
/// - `t_min`: 当日最低气温（°C）
/// - `t_max`: 当日最高气温（°C）
pub fn update_phenology(model: &mut PhenologyModel, t_min: f32, t_max: f32) {
    // 1. 累计 GDD
    let gdd = daily_gdd(t_min, t_max, model.gdd.base_temp_c, model.gdd.upper_temp_c);
    model.gdd.accumulated_gdd += gdd;

    // 2. 日均温
    let t_avg = (t_min + t_max) / 2.0;

    // 3. 春化更新（按全天 24 小时累计，使用日均温近似）
    update_vernalization(&mut model.vernalization, t_avg, 24.0);

    // 4. 冷温累计（< 7°C 才计入，用于打破芽休眠）
    if t_avg < 7.0 {
        model.chill_accumulation += 24.0;
    }

    // 5. 光周期诱导更新
    let daylen = day_length(model.latitude_deg, model.day_of_year);
    let _ = update_photoperiod_induction(&mut model.photoperiod, daylen);

    // 6. 推进日序（按 365 天循环）
    model.day_of_year = if model.day_of_year >= 365 {
        1
    } else {
        model.day_of_year + 1
    };
}

/// 检查阶段转换
///
/// 遍历阶段转换规则表，返回第一个满足条件的下一阶段。
/// 若当前模型尚未达到任何转换阈值，返回 None。
pub fn check_phase_transition(
    model: &PhenologyModel,
    transitions: &[PhaseTransition],
) -> Option<PhenologicalPhase> {
    let current_daylength = day_length(model.latitude_deg, model.day_of_year);

    for t in transitions {
        // 仅匹配当前阶段的转换规则
        if t.from_phase != model.current_phase {
            continue;
        }
        // GDD 阈值检查
        if model.gdd.accumulated_gdd < t.gdd_threshold {
            continue;
        }
        // 日长阈值检查
        if let Some(dl_req) = t.daylength_required {
            if current_daylength < dl_req {
                continue;
            }
        }
        // 冷温阈值检查
        if let Some(chill_req) = t.chill_required {
            if model.chill_accumulation < chill_req {
                continue;
            }
        }
        return Some(t.to_phase);
    }
    None
}

/// 春化进程更新
///
/// 根据当前温度和暴露小时数，按效率曲线累计有效春化量。
/// 温度效率曲线在最适温度附近取最大值 1.0，远离最适温度递减。
///
/// # 参数
/// - `state`: 春化状态可变引用
/// - `temp_c`: 当前温度（°C）
/// - `hours`: 暴露持续小时数
pub fn update_vernalization(state: &mut VernalizationState, temp_c: f32, hours: f32) {
    // 0°C 以下或 12°C 以上不产生有效春化
    if temp_c < 0.0 || temp_c > 12.0 || state.vernalized {
        return;
    }

    let optimal = state.optimal_temp_c;
    let efficiency = if temp_c <= optimal {
        // 0 ~ optimal: 线性升至 1.0
        if optimal <= 0.0 {
            1.0
        } else {
            temp_c / optimal
        }
    } else {
        // optimal ~ 12: 线性降至 0
        let denom = 12.0 - optimal;
        if denom <= 0.0 {
            0.0
        } else {
            1.0 - (temp_c - optimal) / denom
        }
    };

    if efficiency <= 0.0 {
        return;
    }

    let chill = efficiency * hours;
    state.accumulated_chill = (state.accumulated_chill + chill).min(state.required_chill_hours);

    if state.accumulated_chill >= state.required_chill_hours {
        state.vernalized = true;
    }
}

/// 光周期诱导更新
///
/// 根据当前日长判断是否构成诱导，累计诱导量。
/// 非诱导日会以一定速率产生逆化（de-vernalization 效应的类比）。
///
/// # 参数
/// - `state`: 光周期状态可变引用
/// - `daylength_h`: 当日日长（小时）
///
/// # 返回
/// 是否已完成光周期诱导
pub fn update_photoperiod_induction(
    state: &mut PhotoperiodState,
    daylength_h: f32,
) -> bool {
    let is_inductive = state
        .response
        .is_inductive(daylength_h, state.critical_daylength_h);

    if is_inductive {
        state.accumulated_induction += 1.0;
    } else {
        // 非诱导日部分逆化，避免偶发长/短日误判
        state.accumulated_induction = (state.accumulated_induction - 0.5).max(0.0);
    }

    // 上限保护
    if state.accumulated_induction > state.induction_days as f32 + 5.0 {
        state.accumulated_induction = state.induction_days as f32 + 5.0;
    }

    state.is_induced()
}

/// 物候预测（基于历史气候数据预测开花日）
///
/// 从当前累计 GDD 出发，按给定每日温度序列推算达到目标 GDD 所需日数。
///
/// # 参数
/// - `model`: 物候模型
/// - `daily_temps`: 每日 (T_min, T_max) 序列
///
/// # 返回
/// 达到目标 GDD 所需日数；若序列耗尽仍未达到，返回序列长度
pub fn predict_flowering_date(
    model: &PhenologyModel,
    daily_temps: &[(f32, f32)],
) -> u32 {
    let target = model.gdd.required_gdd;
    let mut gdd = model.gdd.accumulated_gdd;

    if gdd >= target {
        return 0;
    }

    for (i, (t_min, t_max)) in daily_temps.iter().enumerate() {
        gdd += daily_gdd(*t_min, *t_max, model.gdd.base_temp_c, model.gdd.upper_temp_c);
        if gdd >= target {
            return (i + 1) as u32;
        }
    }

    daily_temps.len() as u32
}

/// 气候变化对物候的影响
///
/// 简化模型：假设基线生育期约 90 天，平均日 GDD = baseline_gdd / 90。
/// 升温 w°C 后日 GDD 平均增加 w，由此推算开花提前天数。
///
/// # 参数
/// - `warming_c`: 升温幅度（°C）
/// - `baseline_gdd`: 基线所需 GDD（°C·d）
///
/// # 返回
/// 提前开花的天数（正值表示提前）
pub fn climate_shift_effect(warming_c: f32, baseline_gdd: f32) -> f32 {
    if warming_c <= 0.0 || baseline_gdd <= 0.0 {
        return 0.0;
    }

    let season_days = 90.0_f32;
    let avg_daily_gdd = baseline_gdd / season_days;
    if avg_daily_gdd <= 0.0 {
        return 0.0;
    }

    let baseline_days = baseline_gdd / avg_daily_gdd;
    let new_daily_gdd = avg_daily_gdd + warming_c;
    let new_days = baseline_gdd / new_daily_gdd;

    let advance = baseline_days - new_days;

    // 物候不可能提前超过整个生育期
    advance.clamp(0.0, season_days)
}

/// BBCH 阶段转换为物候期
///
/// 提供 BBCH 主阶段到 PhenologicalPhase 的简化映射。
pub fn bbch_to_phase(bbch: &BBCHStage) -> PhenologicalPhase {
    match bbch.principal_stage {
        0 => PhenologicalPhase::BudBurst,
        1 => PhenologicalPhase::LeafExpansion,
        2 => PhenologicalPhase::LeafExpansion,
        3 => PhenologicalPhase::StemElongation,
        4 => PhenologicalPhase::StemElongation,
        5 => PhenologicalPhase::StemElongation,
        6 => PhenologicalPhase::Flowering,
        7 => PhenologicalPhase::FruitSet,
        8 => PhenologicalPhase::Ripening,
        9 => PhenologicalPhase::Senescence,
        _ => PhenologicalPhase::Dormancy,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试单日 GDD：温度高于 base 时 GDD > 0
    #[test]
    fn test_daily_gdd() {
        // 完全高于基础温度
        let g1 = daily_gdd(15.0, 25.0, 10.0, 30.0);
        assert!(
            g1 > 0.0,
            "温度高于 base 时 GDD 必须为正，实际: {}",
            g1
        );
        // 期望值 ((15 + 25) / 2) - 10 = 10
        assert!((g1 - 10.0).abs() < 1e-5, "平均法期望 10，实际: {}", g1);

        // 最高温低于 base，GDD 应为 0
        let g2 = daily_gdd(2.0, 8.0, 10.0, 30.0);
        assert!(
            g2.abs() < 1e-6,
            "最高温低于 base 时 GDD 应为 0，实际: {}",
            g2
        );

        // 三角形法：T_min < base < T_max
        // 期望 (20 - 10)^2 / (2 * (20 - 5)) = 100 / 30 ≈ 3.333
        let g3 = daily_gdd(5.0, 20.0, 10.0, 30.0);
        assert!(
            (g3 - 3.3333333).abs() < 1e-3,
            "三角形法期望约 3.333，实际: {}",
            g3
        );

        // 上限截断：T_max 超过 upper 应被截断
        let g4 = daily_gdd(20.0, 40.0, 10.0, 30.0);
        // 截断后 t_max = 30, t_min = 20, 平均 25, GDD = 15
        assert!(
            (g4 - 15.0).abs() < 1e-5,
            "上限截后期望 15，实际: {}",
            g4
        );
    }

    /// 测试日长：夏至日（约第 172 天）北半球日长最长
    #[test]
    fn test_day_length() {
        // 北纬 40°，夏至（约 6 月 21 日，日序 172）
        let summer = day_length(40.0, 172);
        // 北纬 40°，冬至（约 12 月 21 日，日序 355）
        let winter = day_length(40.0, 355);

        assert!(
            summer > winter,
            "夏至日长应大于冬至，夏: {}, 冬: {}",
            summer,
            winter
        );
        // 北纬 40° 夏至日长约 14.8 小时
        assert!(
            summer > 14.0 && summer < 16.0,
            "夏至日长应在 14-16 小时之间，实际: {}",
            summer
        );
        // 北纬 40° 冬至日长约 9.2 小时
        assert!(
            winter > 8.0 && winter < 10.0,
            "冬至日长应在 8-10 小时之间，实际: {}",
            winter
        );

        // 赤道日长全年接近 12 小时
        let equator_summer = day_length(0.0, 172);
        assert!(
            (equator_summer - 12.0).abs() < 0.1,
            "赤道日长应接近 12 小时，实际: {}",
            equator_summer
        );

        // 春分日（约日序 80）日长接近 12 小时
        let equinox = day_length(40.0, 80);
        assert!(
            (equinox - 12.0).abs() < 0.2,
            "春分日长应接近 12 小时，实际: {}",
            equinox
        );
    }

    /// 测试春化：冷温累计达标后完成春化
    #[test]
    fn test_vernalization() {
        let mut v = VernalizationState {
            required_chill_hours: 100.0,
            accumulated_chill: 0.0,
            vernalized: false,
            optimal_temp_c: 5.0,
        };

        // 高于 12°C 不产生春化
        update_vernalization(&mut v, 15.0, 100.0);
        assert!(
            !v.vernalized,
            "高温不应产生春化"
        );
        assert!(
            v.accumulated_chill.abs() < 1e-6,
            "高温下累计冷温应为 0"
        );

        // 最适温度下效率为 1.0
        update_vernalization(&mut v, 5.0, 50.0);
        assert!(
            (v.accumulated_chill - 50.0).abs() < 1e-5,
            "最适温度下累计冷温应等于暴露小时数，实际: {}",
            v.accumulated_chill
        );

        // 继续累计直到达标
        update_vernalization(&mut v, 5.0, 60.0);
        assert!(
            v.vernalized,
            "累计达到 100 小时后应完成春化"
        );
        assert!(
            v.accumulated_chill <= 100.0 + 1e-5,
            "累计冷温不应超过需冷量"
        );
        assert!(
            (v.progress() - 1.0).abs() < 1e-6,
            "完成春化后进度应为 1.0"
        );
    }

    /// 测试光周期：短日植物在短日下诱导
    #[test]
    fn test_photoperiod() {
        let mut state = PhotoperiodState {
            response: PhotoperiodResponse::ShortDay,
            critical_daylength_h: 12.0,
            induction_days: 3,
            accumulated_induction: 0.0,
        };

        // 短日（10h）构成诱导
        let r1 = update_photoperiod_induction(&mut state, 10.0);
        assert!(!r1, "第一天不应完成诱导");
        assert!((state.accumulated_induction - 1.0).abs() < 1e-6);

        // 长日（14h）不应构成诱导，且产生部分逆化
        update_photoperiod_induction(&mut state, 14.0);
        assert!(
            state.accumulated_induction < 1.0,
            "长日应使诱导量减少"
        );

        // 连续三天短日应完成诱导
        state.accumulated_induction = 0.0;
        let _ = update_photoperiod_induction(&mut state, 10.0);
        let _ = update_photoperiod_induction(&mut state, 10.0);
        let r3 = update_photoperiod_induction(&mut state, 10.0);
        assert!(r3, "连续 3 天短日应完成诱导");
        assert!(state.is_induced());

        // 长日植物测试
        let mut ldp = PhotoperiodState {
            response: PhotoperiodResponse::LongDay,
            critical_daylength_h: 14.0,
            induction_days: 2,
            accumulated_induction: 0.0,
        };
        let _ = update_photoperiod_induction(&mut ldp, 15.0);
        let r_ldp = update_photoperiod_induction(&mut ldp, 15.0);
        assert!(r_ldp, "长日植物在长日下应完成诱导");

        // 日中性植物总是诱导
        let mut dnp = PhotoperiodState::day_neutral();
        dnp.induction_days = 1;
        let r_dnp = update_photoperiod_induction(&mut dnp, 8.0);
        assert!(r_dnp, "日中性植物任何日长均应诱导");
    }

    /// 测试气候变化影响：升温导致 GDD 累积提前
    #[test]
    fn test_climate_shift() {
        // 基线 1500 GDD，升温 1°C 应提前若干天
        let advance_1 = climate_shift_effect(1.0, 1500.0);
        assert!(
            advance_1 > 0.0,
            "升温应导致物候提前，实际: {}",
            advance_1
        );
        // 期望 ≈ 90 - 1500 / (1500/90 + 1) = 90 - 1500/17.667 ≈ 90 - 84.91 ≈ 5.09
        assert!(
            (advance_1 - 5.09).abs() < 0.5,
            "升温 1°C 期望提前约 5 天，实际: {}",
            advance_1
        );

        // 升温越多，提前越多
        let advance_2 = climate_shift_effect(2.0, 1500.0);
        assert!(
            advance_2 > advance_1,
            "升温越多提前应越多，1°C: {}, 2°C: {}",
            advance_1,
            advance_2
        );

        // 不升温应返回 0
        let no_change = climate_shift_effect(0.0, 1500.0);
        assert!(no_change.abs() < 1e-6, "零升温应返回 0");

        // 极端升温不应超过生育期
        let extreme = climate_shift_effect(1000.0, 1500.0);
        assert!(
            extreme <= 90.0 + 1e-5,
            "提前天数不应超过生育期 90 天"
        );
    }

    /// 测试 BBCH 编码与物候期映射
    #[test]
    fn test_bbch_code_and_mapping() {
        let s = BBCHStage::new(6, 5);
        assert_eq!(s.code(), "65", "BBCH 65 应输出 '65'");
        assert_eq!(s.principal_name(), "开花");

        let parsed = BBCHStage::from_code(13);
        assert_eq!(parsed.principal_stage, 1);
        assert_eq!(parsed.secondary_stage, 3);

        // 物候期映射
        assert_eq!(bbch_to_phase(&BBCHStage::new(0, 0)), PhenologicalPhase::BudBurst);
        assert_eq!(bbch_to_phase(&BBCHStage::new(1, 0)), PhenologicalPhase::LeafExpansion);
        assert_eq!(bbch_to_phase(&BBCHStage::new(3, 0)), PhenologicalPhase::StemElongation);
        assert_eq!(bbch_to_phase(&BBCHStage::new(6, 0)), PhenologicalPhase::Flowering);
        assert_eq!(bbch_to_phase(&BBCHStage::new(7, 0)), PhenologicalPhase::FruitSet);
        assert_eq!(bbch_to_phase(&BBCHStage::new(8, 0)), PhenologicalPhase::Ripening);
        assert_eq!(bbch_to_phase(&BBCHStage::new(9, 0)), PhenologicalPhase::Senescence);
    }

    /// 测试物候模型更新与阶段转换
    #[test]
    fn test_phenology_update_and_transition() {
        let mut model = PhenologyModel::new(40.0, 80);
        model.current_phase = PhenologicalPhase::Dormancy;
        model.gdd.required_gdd = 200.0;
        model.vernalization.required_chill_hours = 100.0;

        // 先用冷温完成春化（春化需 0-12°C，最适 5°C）
        for _ in 0..5 {
            update_phenology(&mut model, 0.0, 10.0);
        }
        assert!(model.is_vernalized(), "应通过冷温完成春化");

        // 再用温暖日累计 GDD
        for _ in 0..30 {
            update_phenology(&mut model, 12.0, 22.0);
        }
        assert!(model.gdd.accumulated_gdd > 0.0, "温暖日应累计 GDD");
        assert!(model.gdd.is_complete(), "应已达到目标 GDD");

        // 阶段转换规则
        let transitions = [
            PhaseTransition::by_gdd(
                PhenologicalPhase::Dormancy,
                PhenologicalPhase::BudBurst,
                100.0,
            ),
            PhaseTransition::by_gdd(
                PhenologicalPhase::BudBurst,
                PhenologicalPhase::LeafExpansion,
                200.0,
            ),
        ];

        let next = check_phase_transition(&model, &transitions);
        assert_eq!(next, Some(PhenologicalPhase::BudBurst));
    }

    /// 测试开花日期预测
    #[test]
    fn test_predict_flowering_date() {
        let mut model = PhenologyModel::new(40.0, 80);
        model.gdd.base_temp_c = 5.0;
        model.gdd.upper_temp_c = 30.0;
        model.gdd.required_gdd = 500.0;
        model.gdd.accumulated_gdd = 0.0;

        // 模拟 60 天温暖气候 (10, 20) → 日均 GDD = 10
        let mut temps = Vec::new();
        for _ in 0..60 {
            temps.push((10.0, 20.0));
        }

        let days = predict_flowering_date(&model, &temps);
        // 每日 GDD = 10，需 500 → 50 天
        assert_eq!(
            days, 50,
            "期望 50 天达到开花，实际: {}",
            days
        );

        // 已经达成时应返回 0
        model.gdd.accumulated_gdd = 600.0;
        let d0 = predict_flowering_date(&model, &temps);
        assert_eq!(d0, 0, "已超阈值应返回 0");
    }
}
