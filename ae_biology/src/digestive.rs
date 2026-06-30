//! 消化系统模块
//!
//! 基于: Johnson, Gastrointestinal Physiology (9th Edition)
//! 参考: Michaelis-Menten 酶动力学 (Michaelis & Menten, 1913)
//!       胃排空、小肠吸收、酶活性 pH 依赖性、肠道菌群多样性 (Shannon 指数)
//! 单位约定: pH 无量纲, 胃排空率 ml/min, 转运时间 h, 食物量 g
//!
//! 核心公式:
//!   - Michaelis-Menten: v = Vmax × [S] / (Km + [S])
//!   - 胃排空受食物成分影响 (脂肪延缓排空)
//!   - 胃蛋白酶最适 pH ≈ 2.0

use serde::{Deserialize, Serialize};

/// 消化系统生理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestiveState {
    /// 禁食状态
    Fasting,
    /// 消化中 (食物在胃内)
    Digesting,
    /// 吸收中 (食糜进入小肠)
    Absorbing,
    /// 排空
    Empty,
}

/// 食物类型 (影响排空速率与营养分布)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoodType {
    Carbohydrate,
    Protein,
    Fat,
    Mixed,
}

/// 营养吸收量 (克)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NutrientAbsorption {
    pub carbohydrates: f32,
    pub proteins: f32,
    pub fats: f32,
    pub vitamins: f32,
    pub minerals: f32,
    pub water: f32,
}

impl NutrientAbsorption {
    pub fn total(&self) -> f32 {
        self.carbohydrates + self.proteins + self.fats
            + self.vitamins + self.minerals + self.water
    }

    pub fn zero() -> Self {
        Self {
            carbohydrates: 0.0,
            proteins: 0.0,
            fats: 0.0,
            vitamins: 0.0,
            minerals: 0.0,
            water: 0.0,
        }
    }
}

/// Michaelis-Menten 酶动力学参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnzymeKinetics {
    /// 米氏常数 (底物浓度, v = Vmax/2 处)
    pub km: f32,
    /// 最大反应速率
    pub vmax: f32,
}

impl EnzymeKinetics {
    pub fn new(km: f32, vmax: f32) -> Self {
        Self { km: km.max(0.0), vmax: vmax.max(0.0) }
    }

    /// Michaelis-Menten 速率: v = Vmax × [S] / (Km + [S])
    pub fn rate(&self, substrate_concentration: f32) -> f32 {
        let s = substrate_concentration.max(0.0);
        let denom = self.km + s;
        if denom <= 0.0 {
            0.0
        } else {
            self.vmax * s / denom
        }
    }

    /// 半饱和浓度下的速率应等于 Vmax/2
    pub fn half_max_rate(&self) -> f32 {
        self.vmax / 2.0
    }
}

impl Default for EnzymeKinetics {
    fn default() -> Self {
        // 典型消化酶参数 (近似淀粉酶)
        Self::new(1.0, 1.0)
    }
}

/// 消化系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestiveSystem {
    /// 胃 pH (禁食 ≈ 2.0)
    pub stomach_ph: f32,
    /// 胃排空速率 (ml/min)
    pub gastric_emptying_rate: f32,
    /// 小肠转运时间 (h)
    pub intestinal_transit_time: f32,
    /// 酶活性 (0..1)
    pub enzyme_activity: f32,
    /// 营养吸收效率 (0..1)
    pub nutrient_absorption_efficiency: f32,
    /// 肠道菌群多样性 (Shannon 指数, 健康 ≈ 3.5)
    pub gut_microbiome_diversity: f32,
    /// 当前消化状态
    pub state: DigestiveState,
    /// 胃内食物量 (g)
    pub food_in_stomach: f32,
}

impl DigestiveSystem {
    /// 创建健康成人默认消化系统
    pub fn new() -> Self {
        Self {
            stomach_ph: 2.0,
            gastric_emptying_rate: 10.0,
            intestinal_transit_time: 4.0,
            enzyme_activity: 0.8,
            nutrient_absorption_efficiency: 0.9,
            gut_microbiome_diversity: 3.5,
            state: DigestiveState::Fasting,
            food_in_stomach: 0.0,
        }
    }

    /// 默认酶动力学速率 (基于系统酶活性)
    /// 使用 Km=1.0, Vmax=酶活性
    pub fn enzyme_rate(&self, substrate_concentration: f32) -> f32 {
        let kinetics = EnzymeKinetics::new(1.0, self.enzyme_activity);
        kinetics.rate(substrate_concentration)
    }

    /// 消化食物, 返回该时段吸收的营养量
    pub fn digest(&mut self, food_type: FoodType, amount: f32, dt: f32) -> NutrientAbsorption {
        if amount <= 0.0 {
            return NutrientAbsorption::zero();
        }
        // 食物进入胃, 状态进入消化
        self.food_in_stomach += amount;
        self.state = DigestiveState::Digesting;

        // 食物缓冲胃酸, pH 短暂上升
        let ph_rise = (amount / 100.0) * 0.5;
        self.stomach_ph = (self.stomach_ph + ph_rise).min(6.0);

        // 排空速率受食物成分影响 (脂肪显著延缓排空)
        let emptying_factor = match food_type {
            FoodType::Carbohydrate => 1.2,
            FoodType::Protein => 1.0,
            FoodType::Fat => 0.4,
            FoodType::Mixed => 0.8,
        };
        let emptied = (self.gastric_emptying_rate * emptying_factor * dt).min(self.food_in_stomach);
        self.food_in_stomach -= emptied;

        // 酶动力学分解 (底物浓度 = 排空量/10)
        let substrate = emptied / 10.0;
        let breakdown = self.enzyme_rate(substrate);

        // 营养吸收 = 排空量 × 吸收效率 × 分解率
        let eff = self.nutrient_absorption_efficiency * breakdown;
        let absorbed_total = emptied * eff;

        let (carbs, proteins, fats) = match food_type {
            FoodType::Carbohydrate => {
                (absorbed_total * 0.8, absorbed_total * 0.1, absorbed_total * 0.1)
            }
            FoodType::Protein => {
                (absorbed_total * 0.1, absorbed_total * 0.8, absorbed_total * 0.1)
            }
            FoodType::Fat => {
                (absorbed_total * 0.1, absorbed_total * 0.1, absorbed_total * 0.8)
            }
            FoodType::Mixed => {
                (absorbed_total * 0.5, absorbed_total * 0.3, absorbed_total * 0.2)
            }
        };

        // 状态转移
        if self.stomach_ph > 4.0 {
            self.state = DigestiveState::Absorbing;
        }
        if self.food_in_stomach < 1.0 {
            self.state = DigestiveState::Empty;
        }

        NutrientAbsorption {
            carbohydrates: carbs,
            proteins,
            fats,
            vitamins: absorbed_total * 0.05,
            minerals: absorbed_total * 0.05,
            water: emptied * 0.5,
        }
    }

    /// 每帧更新: pH 与菌群缓慢回归基线, 持续排空
    pub fn update(&mut self, dt: f32) {
        // 胃 pH 回归禁食酸性基线
        self.stomach_ph += (2.0 - self.stomach_ph) * 0.01 * dt;
        self.stomach_ph = self.stomach_ph.clamp(1.0, 7.0);

        // 持续排空
        if self.food_in_stomach > 0.0 {
            let emptied = (self.gastric_emptying_rate * dt).min(self.food_in_stomach);
            self.food_in_stomach -= emptied;
            self.state = DigestiveState::Absorbing;
        } else {
            self.state = DigestiveState::Fasting;
        }

        // 菌群多样性缓慢恢复
        self.gut_microbiome_diversity += (3.5 - self.gut_microbiome_diversity) * 0.001 * dt;
    }
}

impl Default for DigestiveSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stomach_ph() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.stomach_ph, 2.0);
    }

    #[test]
    fn test_default_emptying_rate() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.gastric_emptying_rate, 10.0);
    }

    #[test]
    fn test_default_transit_time() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.intestinal_transit_time, 4.0);
    }

    #[test]
    fn test_default_enzyme_activity() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.enzyme_activity, 0.8);
    }

    #[test]
    fn test_default_absorption_efficiency() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.nutrient_absorption_efficiency, 0.9);
    }

    #[test]
    fn test_default_microbiome_diversity() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.gut_microbiome_diversity, 3.5);
    }

    #[test]
    fn test_default_state_fasting() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.state, DigestiveState::Fasting);
    }

    #[test]
    fn test_enzyme_rate_zero_substrate() {
        let sys = DigestiveSystem::new();
        assert_eq!(sys.enzyme_rate(0.0), 0.0);
    }

    #[test]
    fn test_enzyme_rate_high_substrate_approaches_vmax() {
        let sys = DigestiveSystem::new();
        let rate = sys.enzyme_rate(1000.0);
        // 高底物浓度时接近 Vmax=0.8
        assert!((rate - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_enzyme_rate_at_km_is_half_vmax() {
        let sys = DigestiveSystem::new();
        // Km=1.0, 在 [S]=Km=1.0 时 v = Vmax/2 = 0.4
        let rate = sys.enzyme_rate(1.0);
        assert!((rate - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_digest_adds_food_to_stomach() {
        let mut sys = DigestiveSystem::new();
        sys.digest(FoodType::Mixed, 100.0, 1.0);
        assert!(sys.food_in_stomach > 0.0);
    }

    #[test]
    fn test_digest_returns_absorption() {
        let mut sys = DigestiveSystem::new();
        let absorption = sys.digest(FoodType::Mixed, 200.0, 1.0);
        assert!(absorption.total() > 0.0);
    }

    #[test]
    fn test_digest_carbohydrate_yields_more_carbs() {
        let mut sys = DigestiveSystem::new();
        let absorption = sys.digest(FoodType::Carbohydrate, 200.0, 1.0);
        assert!(absorption.carbohydrates > absorption.proteins);
        assert!(absorption.carbohydrates > absorption.fats);
    }

    #[test]
    fn test_digest_fat_yields_more_fats() {
        let mut sys = DigestiveSystem::new();
        let absorption = sys.digest(FoodType::Fat, 200.0, 1.0);
        assert!(absorption.fats > absorption.carbohydrates);
    }

    #[test]
    fn test_digest_sets_digesting_state() {
        let mut sys = DigestiveSystem::new();
        sys.digest(FoodType::Mixed, 100.0, 0.1);
        assert_ne!(sys.state, DigestiveState::Fasting);
    }

    #[test]
    fn test_digest_fat_slows_emptying() {
        let mut sys_fat = DigestiveSystem::new();
        let mut sys_carb = DigestiveSystem::new();
        sys_fat.digest(FoodType::Fat, 200.0, 1.0);
        sys_carb.digest(FoodType::Carbohydrate, 200.0, 1.0);
        // 脂肪排空慢, 胃内残留应更多
        assert!(sys_fat.food_in_stomach > sys_carb.food_in_stomach);
    }

    #[test]
    fn test_digest_large_amount_clamps_ph() {
        let mut sys = DigestiveSystem::new();
        sys.digest(FoodType::Mixed, 1000.0, 1.0);
        assert!(sys.stomach_ph <= 6.0);
    }

    #[test]
    fn test_digest_zero_amount_returns_zero() {
        let mut sys = DigestiveSystem::new();
        let absorption = sys.digest(FoodType::Mixed, 0.0, 1.0);
        assert_eq!(absorption.total(), 0.0);
    }

    #[test]
    fn test_update_lowers_ph_toward_baseline() {
        let mut sys = DigestiveSystem::new();
        sys.stomach_ph = 5.0;
        sys.update(60.0);
        assert!(sys.stomach_ph < 5.0);
    }

    #[test]
    fn test_update_continues_emptying() {
        let mut sys = DigestiveSystem::new();
        sys.food_in_stomach = 50.0;
        sys.update(1.0);
        assert!(sys.food_in_stomach < 50.0);
    }

    #[test]
    fn test_update_sets_fasting_when_empty() {
        let mut sys = DigestiveSystem::new();
        sys.food_in_stomach = 0.0;
        sys.update(1.0);
        assert_eq!(sys.state, DigestiveState::Fasting);
    }

    #[test]
    fn test_enzyme_kinetics_struct_rate() {
        let k = EnzymeKinetics::new(2.0, 5.0);
        // [S]=2 (Km) → v = Vmax/2 = 2.5
        assert!((k.rate(2.0) - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_enzyme_kinetics_negative_substrate_returns_zero() {
        let k = EnzymeKinetics::new(1.0, 1.0);
        assert_eq!(k.rate(-5.0), 0.0);
    }

    #[test]
    fn test_nutrient_absorption_total() {
        let n = NutrientAbsorption {
            carbohydrates: 10.0,
            proteins: 5.0,
            fats: 3.0,
            vitamins: 1.0,
            minerals: 1.0,
            water: 8.0,
        };
        assert!((n.total() - 28.0).abs() < 0.001);
    }

    #[test]
    fn test_serialization_round_trip() {
        let sys = DigestiveSystem::new();
        let json = serde_json::to_string(&sys).unwrap();
        let restored: DigestiveSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.stomach_ph, sys.stomach_ph);
        assert_eq!(restored.enzyme_activity, sys.enzyme_activity);
    }
}
