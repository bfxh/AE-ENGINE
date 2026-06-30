//! 营养学模块 — 宏量/微量营养素、能量平衡与营养不良模型
//!
//! 生物学背景:
//!   营养学涉及能量摄入与消耗的平衡、宏量营养素（蛋白质、碳水、脂肪）的代谢贡献，
//!   以及微量营养素（维生素、矿物质）的生理功能。长期失衡会导致营养不良（肥胖或消瘦），
//!   进而引发代谢综合征、免疫缺陷、内分泌紊乱。
//!
//! 论文来源:
//! - Mifflin, M. D., St Jeor, S. T., Hill, L. A., Scott, B. J., Daugherty, S. A., Koh, Y. O.
//!   (1990). "A new predictive equation for resting energy expenditure in healthy individuals."
//!   Am. J. Clin. Nutr. 51(2): 241-247. (Mifflin-St Jeor BMR 方程)
//! - WHO (1995). "Physical status: the use and interpretation of anthropometry."
//!   WHO Tech. Rep. Ser. 854. (BMI 营养不良分度)
//! - National Research Council (1989). "Recommended Dietary Allowances." 10th ed.
//!   (RDA 推荐膳食供给量)
//! - Atwater, W. O. (1896). "The potential energy of food." Science 3(67): 653-657.
//!   (能量密度: 蛋白质 4 kcal/g, 碳水 4 kcal/g, 脂肪 9 kcal/g)
//! - WHO (2000). "Obesity: preventing and managing the global epidemic."
//!   WHO Tech. Rep. Ser. 894. (BMI >= 30 肥胖分度)
//!
//! 物理量单位:
//!   - 能量: kcal (1 kcal = 4184 J)
//!   - 质量: g
//!   - 时间: s
//!   - BMR: kcal/day

use serde::{Deserialize, Serialize};

/// 三大宏量营养素类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Nutrient {
    /// 蛋白质, 4 kcal/g (Atwater 1896)
    Protein,
    /// 碳水化合物, 4 kcal/g
    Carbohydrate,
    /// 脂肪, 9 kcal/g
    Fat,
    /// 维生素 A (脂溶性)
    VitaminA,
    /// 维生素 C (水溶性)
    VitaminC,
    /// 维生素 D
    VitaminD,
    /// 铁
    Iron,
    /// 钙
    Calcium,
    /// 锌
    Zinc,
}

impl Nutrient {
    /// 宏量营养素的能量密度 (kcal/g), 微量营养素返回 0
    /// 来源: Atwater 1896
    pub fn energy_density_kcal_per_g(&self) -> f32 {
        match self {
            Self::Protein => 4.0,
            Self::Carbohydrate => 4.0,
            Self::Fat => 9.0,
            _ => 0.0,
        }
    }

    /// 成人 RDA 推荐膳食供给量 (g/day 或 mg/day)
    /// 来源: NRC 1989 RDA 第10版
    pub fn rda_adult(&self) -> f32 {
        match self {
            Self::Protein => 56.0,            // g/day (成年男性)
            Self::Carbohydrate => 130.0,       // g/day
            Self::Fat => 65.0,                 // g/day
            Self::VitaminA => 900.0,           // ug/day
            Self::VitaminC => 90.0,            // mg/day
            Self::VitaminD => 15.0,            // ug/day (600 IU)
            Self::Iron => 8.0,                 // mg/day (成年男性)
            Self::Calcium => 1000.0,           // mg/day
            Self::Zinc => 11.0,                // mg/day (成年男性)
        }
    }

    /// 是否为宏量营养素
    pub fn is_macronutrient(&self) -> bool {
        matches!(self, Self::Protein | Self::Carbohydrate | Self::Fat)
    }

    /// 是否为微量营养素
    pub fn is_micronutrient(&self) -> bool {
        !self.is_macronutrient()
    }
}

/// 宏量营养素分布档案 (按能量百分比)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MacronutrientProfile {
    /// 蛋白质供能比 (0..1)
    pub protein_ratio: f32,
    /// 碳水供能比 (0..1)
    pub carbohydrate_ratio: f32,
    /// 脂肪供能比 (0..1)
    pub fat_ratio: f32,
}

impl Default for MacronutrientProfile {
    fn default() -> Self {
        // AMDR 推荐范围中位数: 蛋白 15%, 碳水 55%, 脂肪 30%
        Self {
            protein_ratio: 0.15,
            carbohydrate_ratio: 0.55,
            fat_ratio: 0.30,
        }
    }
}

impl MacronutrientProfile {
    /// 三大宏量营养素比例之和应为 1.0
    pub fn sum(&self) -> f32 {
        self.protein_ratio + self.carbohydrate_ratio + self.fat_ratio
    }

    /// 是否合法 (和为 1.0 ± 0.01, 各分量在 [0,1])
    pub fn is_valid(&self) -> bool {
        let s = self.sum();
        (s - 1.0).abs() < 0.01
            && self.protein_ratio >= 0.0
            && self.protein_ratio <= 1.0
            && self.carbohydrate_ratio >= 0.0
            && self.carbohydrate_ratio <= 1.0
            && self.fat_ratio >= 0.0
            && self.fat_ratio <= 1.0
    }

    /// 给定总能量 (kcal/day) 返回各宏量营养素质量 (g)
    pub fn to_grams(&self, total_kcal_per_day: f32) -> (f32, f32, f32) {
        let protein_g = (total_kcal_per_day * self.protein_ratio) / 4.0;
        let carb_g = (total_kcal_per_day * self.carbohydrate_ratio) / 4.0;
        let fat_g = (total_kcal_per_day * self.fat_ratio) / 9.0;
        (protein_g, carb_g, fat_g)
    }
}

/// 微量营养素档案 (mg/day 或 ug/day)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MicronutrientProfile {
    pub vitamin_a_ug: f32,
    pub vitamin_c_mg: f32,
    pub vitamin_d_ug: f32,
    pub iron_mg: f32,
    pub calcium_mg: f32,
    pub zinc_mg: f32,
}

impl Default for MicronutrientProfile {
    fn default() -> Self {
        Self {
            vitamin_a_ug: 900.0,
            vitamin_c_mg: 90.0,
            vitamin_d_ug: 15.0,
            iron_mg: 8.0,
            calcium_mg: 1000.0,
            zinc_mg: 11.0,
        }
    }
}

impl MicronutrientProfile {
    /// 计算各微量营养素的 RDA 满足度 (摄入/RDA), 1.0 = 刚好满足
    pub fn rda_satisfaction(&self) -> MicronutrientProfile {
        let d = MicronutrientProfile::default();
        Self {
            vitamin_a_ug: self.vitamin_a_ug / d.vitamin_a_ug,
            vitamin_c_mg: self.vitamin_c_mg / d.vitamin_c_mg,
            vitamin_d_ug: self.vitamin_d_ug / d.vitamin_d_ug,
            iron_mg: self.iron_mg / d.iron_mg,
            calcium_mg: self.calcium_mg / d.calcium_mg,
            zinc_mg: self.zinc_mg / d.zinc_mg,
        }
    }

    /// 全部满足 RDA (>= 1.0)
    pub fn all_met(&self) -> bool {
        let r = self.rda_satisfaction();
        r.vitamin_a_ug >= 1.0
            && r.vitamin_c_mg >= 1.0
            && r.vitamin_d_ug >= 1.0
            && r.iron_mg >= 1.0
            && r.calcium_mg >= 1.0
            && r.zinc_mg >= 1.0
    }
}

/// 性别 (用于 BMR 计算)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiologicalSex {
    Male,
    Female,
}

/// 能量平衡 — 摄入与消耗之差
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnergyBalance {
    /// 摄入能量 (kcal/day)
    pub intake_kcal_per_day: f32,
    /// 消耗能量 (kcal/day), 包含 BMR + 活动热量 + TEF
    pub expenditure_kcal_per_day: f32,
}

impl Default for EnergyBalance {
    fn default() -> Self {
        Self {
            intake_kcal_per_day: 2000.0,
            expenditure_kcal_per_day: 2000.0,
        }
    }
}

impl EnergyBalance {
    /// 净能量 (kcal/day), 正 = 盈余, 负 = 缺口
    pub fn net_kcal_per_day(&self) -> f32 {
        self.intake_kcal_per_day - self.expenditure_kcal_per_day
    }

    /// 是否为正能量平衡 (体重增加)
    pub fn is_positive(&self) -> bool {
        self.net_kcal_per_day() > 0.0
    }

    /// 是否为负能量平衡 (体重减少)
    pub fn is_negative(&self) -> bool {
        self.net_kcal_per_day() < 0.0
    }

    /// 显式 Euler 积分: 1 天后的体重变化 (kg)
    /// 假设 1 kg 脂肪组织 = 7700 kcal (Hall 2008)
    pub fn daily_weight_delta_kg(&self) -> f32 {
        self.net_kcal_per_day() / 7700.0
    }

    /// 30 天累积体重变化 (kg)
    pub fn monthly_weight_delta_kg(&self) -> f32 {
        self.daily_weight_delta_kg() * 30.0
    }
}

/// BMR 计算 (Mifflin-St Jeor 1990 方程)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BmrParams {
    pub sex: BiologicalSex,
    /// 体重 (kg)
    pub weight_kg: f32,
    /// 身高 (cm)
    pub height_cm: f32,
    /// 年龄 (年)
    pub age_years: f32,
}

impl Default for BmrParams {
    fn default() -> Self {
        Self {
            sex: BiologicalSex::Male,
            weight_kg: 70.0,
            height_cm: 175.0,
            age_years: 30.0,
        }
    }
}

impl BmrParams {
    /// Mifflin-St Jeor BMR (kcal/day)
    /// 男性: BMR = 10*W + 6.25*H - 5*A + 5
    /// 女性: BMR = 10*W + 6.25*H - 5*A - 161
    pub fn bmr_kcal_per_day(&self) -> f32 {
        let base = 10.0 * self.weight_kg + 6.25 * self.height_cm - 5.0 * self.age_years;
        match self.sex {
            BiologicalSex::Male => base + 5.0,
            BiologicalSex::Female => base - 161.0,
        }
    }

    /// 总能量消耗 TDEE (kcal/day) = BMR * 活动因子
    /// 活动因子: 1.2 久坐, 1.375 轻度, 1.55 中度, 1.725 高度, 1.9 极高强度
    pub fn tdee_kcal_per_day(&self, activity_factor: f32) -> f32 {
        self.bmr_kcal_per_day() * activity_factor.max(0.0)
    }
}

/// 营养不良等级 (基于 BMI, WHO 1995/2000)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MalnutritionGrade {
    /// BMI < 16.0 重度消瘦
    SevereUnderweight,
    /// 16.0 <= BMI < 17.0 中度消瘦
    ModerateUnderweight,
    /// 17.0 <= BMI < 18.5 轻度消瘦
    MildUnderweight,
    /// 18.5 <= BMI < 25.0 正常
    Normal,
    /// 25.0 <= BMI < 30.0 超重
    Overweight,
    /// 30.0 <= BMI < 35.0 一级肥胖
    ObeseClass1,
    /// 35.0 <= BMI < 40.0 二级肥胖
    ObeseClass2,
    /// BMI >= 40.0 三级肥胖
    ObeseClass3,
}

impl MalnutritionGrade {
    /// 根据 BMI (kg/m^2) 判定营养不良等级
    pub fn from_bmi(bmi: f32) -> Self {
        if bmi < 16.0 {
            Self::SevereUnderweight
        } else if bmi < 17.0 {
            Self::ModerateUnderweight
        } else if bmi < 18.5 {
            Self::MildUnderweight
        } else if bmi < 25.0 {
            Self::Normal
        } else if bmi < 30.0 {
            Self::Overweight
        } else if bmi < 35.0 {
            Self::ObeseClass1
        } else if bmi < 40.0 {
            Self::ObeseClass2
        } else {
            Self::ObeseClass3
        }
    }

    /// 是否属于营养不良 (任何异常)
    pub fn is_malnourished(&self) -> bool {
        !matches!(self, Self::Normal)
    }

    /// 是否为体重过轻 (BMI < 18.5)
    pub fn is_underweight(&self) -> bool {
        matches!(
            self,
            Self::SevereUnderweight | Self::ModerateUnderweight | Self::MildUnderweight
        )
    }

    /// 是否为肥胖 (BMI >= 30)
    pub fn is_obese(&self) -> bool {
        matches!(
            self,
            Self::ObeseClass1 | Self::ObeseClass2 | Self::ObeseClass3
        )
    }
}

/// 营养状态综合档案
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NutritionState {
    pub macro_profile: MacronutrientProfile,
    pub micro_profile: MicronutrientProfile,
    pub energy_balance: EnergyBalance,
    pub bmr_params: BmrParams,
    /// 当前 BMI (kg/m^2)
    pub bmi: f32,
}

impl Default for NutritionState {
    fn default() -> Self {
        let bmr = BmrParams::default();
        let height_m = bmr.height_cm / 100.0;
        let bmi = bmr.weight_kg / (height_m * height_m);
        Self {
            macro_profile: MacronutrientProfile::default(),
            micro_profile: MicronutrientProfile::default(),
            energy_balance: EnergyBalance::default(),
            bmr_params: bmr,
            bmi,
        }
    }
}

impl NutritionState {
    /// 计算当前营养不良等级
    pub fn malnutrition_grade(&self) -> MalnutritionGrade {
        MalnutritionGrade::from_bmi(self.bmi)
    }

    /// 显式 Euler 单步积分: 1 天后的营养状态
    /// - 体重根据能量平衡变化
    /// - BMI 根据新体重和固定身高重算
    pub fn step_one_day(&self) -> Self {
        let delta_kg = self.energy_balance.daily_weight_delta_kg();
        let new_weight = (self.bmr_params.weight_kg + delta_kg).max(1.0);
        let height_m = self.bmr_params.height_cm / 100.0;
        let new_bmi = new_weight / (height_m * height_m);
        let mut new_state = *self;
        new_state.bmr_params.weight_kg = new_weight;
        new_state.bmi = new_bmi;
        new_state
    }

    /// RDA 整体满足度 (宏量营养素按 g 计算)
    pub fn macro_rda_satisfaction(&self, total_kcal_per_day: f32) -> (f32, f32, f32) {
        let (p, c, f) = self.macro_profile.to_grams(total_kcal_per_day);
        (p / Nutrient::Protein.rda_adult(), c / Nutrient::Carbohydrate.rda_adult(), f / Nutrient::Fat.rda_adult())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nutrient_default_energy_density_protein() {
        assert_eq!(Nutrient::Protein.energy_density_kcal_per_g(), 4.0);
    }

    #[test]
    fn test_nutrient_default_energy_density_carbohydrate() {
        assert_eq!(Nutrient::Carbohydrate.energy_density_kcal_per_g(), 4.0);
    }

    #[test]
    fn test_nutrient_default_energy_density_fat() {
        assert_eq!(Nutrient::Fat.energy_density_kcal_per_g(), 9.0);
    }

    #[test]
    fn test_micronutrient_zero_energy_density() {
        assert_eq!(Nutrient::VitaminC.energy_density_kcal_per_g(), 0.0);
        assert_eq!(Nutrient::Iron.energy_density_kcal_per_g(), 0.0);
    }

    #[test]
    fn test_nutrient_classification() {
        assert!(Nutrient::Protein.is_macronutrient());
        assert!(!Nutrient::Protein.is_micronutrient());
        assert!(Nutrient::VitaminA.is_micronutrient());
        assert!(!Nutrient::VitaminA.is_macronutrient());
    }

    #[test]
    fn test_rda_adult_values() {
        assert_eq!(Nutrient::Protein.rda_adult(), 56.0);
        assert_eq!(Nutrient::Calcium.rda_adult(), 1000.0);
    }

    #[test]
    fn test_macronutrient_profile_default_sums_to_one() {
        let p = MacronutrientProfile::default();
        assert!((p.sum() - 1.0).abs() < 1e-5);
        assert!(p.is_valid());
    }

    #[test]
    fn test_macronutrient_profile_invalid_when_negative() {
        let p = MacronutrientProfile {
            protein_ratio: -0.1,
            carbohydrate_ratio: 0.55,
            fat_ratio: 0.30,
        };
        assert!(!p.is_valid());
    }

    #[test]
    fn test_macronutrient_to_grams_2000kcal() {
        let p = MacronutrientProfile::default();
        let (protein, carb, fat) = p.to_grams(2000.0);
        // 2000 * 0.15 / 4 = 75 g 蛋白
        assert!((protein - 75.0).abs() < 1e-3);
        // 2000 * 0.55 / 4 = 275 g 碳水
        assert!((carb - 275.0).abs() < 1e-3);
        // 2000 * 0.30 / 9 = 66.667 g 脂肪
        assert!((fat - 66.6667).abs() < 1e-2);
    }

    #[test]
    fn test_energy_balance_default_is_zero() {
        let eb = EnergyBalance::default();
        assert!(eb.net_kcal_per_day().abs() < 1e-5);
        assert!(!eb.is_positive());
        assert!(!eb.is_negative());
    }

    #[test]
    fn test_energy_balance_positive_surplus() {
        let eb = EnergyBalance {
            intake_kcal_per_day: 2500.0,
            expenditure_kcal_per_day: 2000.0,
        };
        assert!(eb.is_positive());
        assert!((eb.net_kcal_per_day() - 500.0).abs() < 1e-5);
    }

    #[test]
    fn test_energy_balance_negative_deficit() {
        let eb = EnergyBalance {
            intake_kcal_per_day: 1500.0,
            expenditure_kcal_per_day: 2000.0,
        };
        assert!(eb.is_negative());
        assert!((eb.net_kcal_per_day() + 500.0).abs() < 1e-5);
    }

    #[test]
    fn test_daily_weight_delta_7700_kcal_per_kg() {
        let eb = EnergyBalance {
            intake_kcal_per_day: 2770.0,
            expenditure_kcal_per_day: 770.0,
        };
        // 净 +2000 kcal/day → 2000/7700 = 0.2597 kg/day
        assert!((eb.daily_weight_delta_kg() - 2000.0 / 7700.0).abs() < 1e-5);
    }

    #[test]
    fn test_monthly_weight_delta_kg() {
        let eb = EnergyBalance {
            intake_kcal_per_day: 2770.0,
            expenditure_kcal_per_day: 770.0,
        };
        assert!((eb.monthly_weight_delta_kg() - 30.0 * 2000.0 / 7700.0).abs() < 1e-4);
    }

    #[test]
    fn test_bmr_male_mifflin_st_jeor() {
        let p = BmrParams {
            sex: BiologicalSex::Male,
            weight_kg: 70.0,
            height_cm: 175.0,
            age_years: 30.0,
        };
        // 10*70 + 6.25*175 - 5*30 + 5 = 700 + 1093.75 - 150 + 5 = 1648.75
        assert!((p.bmr_kcal_per_day() - 1648.75).abs() < 1e-3);
    }

    #[test]
    fn test_bmr_female_mifflin_st_jeor() {
        let p = BmrParams {
            sex: BiologicalSex::Female,
            weight_kg: 60.0,
            height_cm: 165.0,
            age_years: 30.0,
        };
        // 10*60 + 6.25*165 - 5*30 - 161 = 600 + 1031.25 - 150 - 161 = 1320.25
        assert!((p.bmr_kcal_per_day() - 1320.25).abs() < 1e-3);
    }

    #[test]
    fn test_tdee_with_activity_factor() {
        let p = BmrParams::default();
        let bmr = p.bmr_kcal_per_day();
        assert!((p.tdee_kcal_per_day(1.2) - bmr * 1.2).abs() < 1e-3);
    }

    #[test]
    fn test_tdee_clamps_negative_factor() {
        let p = BmrParams::default();
        assert!(p.tdee_kcal_per_day(-1.0) >= 0.0);
    }

    #[test]
    fn test_malnutrition_severe_underweight() {
        assert_eq!(MalnutritionGrade::from_bmi(15.0), MalnutritionGrade::SevereUnderweight);
        assert!(MalnutritionGrade::SevereUnderweight.is_underweight());
    }

    #[test]
    fn test_malnutrition_mild_underweight() {
        assert_eq!(MalnutritionGrade::from_bmi(17.5), MalnutritionGrade::MildUnderweight);
    }

    #[test]
    fn test_malnutrition_normal_range() {
        assert_eq!(MalnutritionGrade::from_bmi(18.5), MalnutritionGrade::Normal);
        assert_eq!(MalnutritionGrade::from_bmi(22.0), MalnutritionGrade::Normal);
        assert_eq!(MalnutritionGrade::from_bmi(24.9), MalnutritionGrade::Normal);
        assert!(!MalnutritionGrade::Normal.is_malnourished());
    }

    #[test]
    fn test_malnutrition_overweight() {
        assert_eq!(MalnutritionGrade::from_bmi(25.0), MalnutritionGrade::Overweight);
        assert_eq!(MalnutritionGrade::from_bmi(29.9), MalnutritionGrade::Overweight);
        assert!(!MalnutritionGrade::Overweight.is_obese());
        assert!(MalnutritionGrade::Overweight.is_malnourished());
    }

    #[test]
    fn test_malnutrition_obese_class1() {
        assert_eq!(MalnutritionGrade::from_bmi(30.0), MalnutritionGrade::ObeseClass1);
        assert!(MalnutritionGrade::ObeseClass1.is_obese());
    }

    #[test]
    fn test_malnutrition_obese_class3() {
        assert_eq!(MalnutritionGrade::from_bmi(45.0), MalnutritionGrade::ObeseClass3);
        assert!(MalnutritionGrade::ObeseClass3.is_obese());
    }

    #[test]
    fn test_micronutrient_profile_default_all_met() {
        let m = MicronutrientProfile::default();
        assert!(m.all_met());
    }

    #[test]
    fn test_micronutrient_profile_deficient() {
        let m = MicronutrientProfile {
            vitamin_c_mg: 30.0, // 低于 RDA 90
            ..MicronutrientProfile::default()
        };
        assert!(!m.all_met());
        let r = m.rda_satisfaction();
        assert!((r.vitamin_c_mg - 1.0 / 3.0).abs() < 1e-3);
    }

    #[test]
    fn test_nutrition_state_default_malnutrition_grade() {
        let s = NutritionState::default();
        assert_eq!(s.malnutrition_grade(), MalnutritionGrade::Normal);
    }

    #[test]
    fn test_nutrition_state_step_one_day_no_change_when_balanced() {
        let mut s = NutritionState::default();
        s.energy_balance = EnergyBalance::default();
        let s2 = s.step_one_day();
        // 净能量 0, 体重不变
        assert!((s2.bmr_params.weight_kg - s.bmr_params.weight_kg).abs() < 1e-5);
    }

    #[test]
    fn test_nutrition_state_step_one_day_surplus_increases_weight() {
        let mut s = NutritionState::default();
        s.energy_balance = EnergyBalance {
            intake_kcal_per_day: 2770.0,
            expenditure_kcal_per_day: 770.0,
        };
        let s2 = s.step_one_day();
        assert!(s2.bmr_params.weight_kg > s.bmr_params.weight_kg);
        assert!(s2.bmi > s.bmi);
    }

    #[test]
    fn test_nutrition_state_step_one_day_deficit_decreases_weight() {
        let mut s = NutritionState::default();
        s.energy_balance = EnergyBalance {
            intake_kcal_per_day: 770.0,
            expenditure_kcal_per_day: 2770.0,
        };
        let s2 = s.step_one_day();
        assert!(s2.bmr_params.weight_kg < s.bmr_params.weight_kg);
        assert!(s2.bmi < s.bmi);
    }

    #[test]
    fn test_macro_rda_satisfaction_default_profile() {
        let s = NutritionState::default();
        let (p, _c, _f) = s.macro_rda_satisfaction(2000.0);
        // 2000 * 0.15 / 4 = 75 g 蛋白; 75 / 56 = 1.339
        assert!((p - 75.0 / 56.0).abs() < 1e-3);
    }
}
