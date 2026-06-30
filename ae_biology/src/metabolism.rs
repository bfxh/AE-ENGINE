use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metabolism {
    pub basal_metabolic_rate: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub hydration: f32,
    pub max_hydration: f32,
    pub body_temperature: f32,
    pub optimal_temperature: f32,
    pub oxygen_level: f32,
    pub blood_ph: f32,
    pub calories_burned: f32,
    pub water_consumed: f32,
    pub toxins: Vec<Toxin>,
    pub nutrients: Nutrients,
    pub status: MetabolicStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nutrients {
    pub carbohydrates: f32,
    pub proteins: f32,
    pub fats: f32,
    pub vitamins: f32,
    pub minerals: f32,
    pub fiber: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toxin {
    pub name: String,
    pub concentration: f32,
    pub damage_per_second: f32,
    pub source: ToxinSource,
    pub half_life: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToxinSource {
    Radiation,
    Venom,
    Chemical,
    Biological,
    HeavyMetal,
    Mycotoxin,
    Bacterial,
    Viral,
    Prion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetabolicStatus {
    Normal,
    Starving,
    Dehydrated,
    Hyperthermic,
    Hypothermic,
    Toxic,
    Septic,
    Radiated,
    MetabolicAcidosis,
    MetabolicAlkalosis,
}

impl Default for Metabolism {
    fn default() -> Self {
        Self {
            basal_metabolic_rate: 2000.0,
            energy: 2000.0,
            max_energy: 3000.0,
            hydration: 100.0,
            max_hydration: 100.0,
            body_temperature: 310.0,
            optimal_temperature: 310.0,
            oxygen_level: 0.98,
            blood_ph: 7.4,
            calories_burned: 0.0,
            water_consumed: 0.0,
            toxins: Vec::new(),
            nutrients: Nutrients {
                carbohydrates: 500.0,
                proteins: 200.0,
                fats: 100.0,
                vitamins: 50.0,
                minerals: 30.0,
                fiber: 20.0,
            },
            status: MetabolicStatus::Normal,
        }
    }
}

impl Metabolism {
    pub fn update(&mut self, dt: f32) {
        let hours = dt / 3600.0;

        let activity_multiplier = 1.5;
        self.calories_burned = self.basal_metabolic_rate * activity_multiplier * hours;
        self.water_consumed = 2.0 * hours;

        self.energy -= self.calories_burned;
        self.hydration -= self.water_consumed;

        self.nutrients.carbohydrates -= self.calories_burned * 0.5;
        self.nutrients.fats -= self.calories_burned * 0.3;
        self.nutrients.proteins -= self.calories_burned * 0.2;

        self.nutrients.carbohydrates = self.nutrients.carbohydrates.max(0.0);
        self.nutrients.fats = self.nutrients.fats.max(0.0);
        self.nutrients.proteins = self.nutrients.proteins.max(0.0);

        self.update_toxins(dt);
        self.update_status();
    }

    fn update_toxins(&mut self, dt: f32) {
        for toxin in &mut self.toxins {
            let decay = (0.693 / toxin.half_life) * dt;
            toxin.concentration = (toxin.concentration - decay).max(0.0);
        }
        self.toxins.retain(|t| t.concentration > 0.001);
    }

    fn update_status(&mut self) {
        if self.energy <= 0.0 {
            self.status = MetabolicStatus::Starving;
        } else if self.hydration <= 0.0 {
            self.status = MetabolicStatus::Dehydrated;
        } else if self.body_temperature > self.optimal_temperature + 5.0 {
            self.status = MetabolicStatus::Hyperthermic;
        } else if self.body_temperature < self.optimal_temperature - 10.0 {
            self.status = MetabolicStatus::Hypothermic;
        } else if !self.toxins.is_empty() {
            let max_toxicity = self.toxins.iter().map(|t| t.concentration).fold(0.0f32, f32::max);
            if max_toxicity > 0.5 {
                self.status = MetabolicStatus::Toxic;
            } else {
                self.status = MetabolicStatus::Normal;
            }
        } else {
            self.status = MetabolicStatus::Normal;
        }
    }

    pub fn consume_food(&mut self, calories: f32, nutrients: Nutrients) {
        self.energy = (self.energy + calories).min(self.max_energy);
        self.nutrients.carbohydrates += nutrients.carbohydrates;
        self.nutrients.proteins += nutrients.proteins;
        self.nutrients.fats += nutrients.fats;
        self.nutrients.vitamins += nutrients.vitamins;
        self.nutrients.minerals += nutrients.minerals;
        self.nutrients.fiber += nutrients.fiber;
    }

    pub fn consume_water(&mut self, amount: f32) {
        self.hydration = (self.hydration + amount).min(self.max_hydration);
    }

    pub fn add_toxin(
        &mut self,
        name: String,
        concentration: f32,
        source: ToxinSource,
        half_life: f32,
    ) {
        let damage = match source {
            ToxinSource::Radiation => 0.1,
            ToxinSource::Venom => 0.5,
            ToxinSource::Chemical => 0.3,
            ToxinSource::Biological => 0.2,
            ToxinSource::HeavyMetal => 0.05,
            ToxinSource::Mycotoxin => 0.15,
            ToxinSource::Bacterial => 0.25,
            ToxinSource::Viral => 0.4,
            ToxinSource::Prion => 0.01,
        };

        self.toxins.push(Toxin {
            name,
            concentration,
            damage_per_second: damage,
            source,
            half_life,
        });
    }

    pub fn health_penalty(&self) -> f32 {
        let mut penalty = 0.0f32;

        if self.energy < self.basal_metabolic_rate * 0.5 {
            penalty += 1.0;
        }
        if self.hydration < 20.0 {
            penalty += 2.0;
        }
        let temp_diff = (self.body_temperature - self.optimal_temperature).abs();
        if temp_diff > 5.0 {
            penalty += temp_diff * 0.1;
        }
        for toxin in &self.toxins {
            penalty += toxin.concentration * toxin.damage_per_second * 10.0;
        }

        penalty
    }
}


// ============================================================================
// 扩展方法（2026-06-29）：稳态监测 + 营养分析 + 毒素评估
// ============================================================================

impl Metabolism {
    /// 是否处于稳态
    pub fn is_homeostatic(&self) -> bool {
        matches!(self.status, MetabolicStatus::Normal)
            && (self.body_temperature - self.optimal_temperature).abs() < 2.0
            && self.oxygen_level > 0.85
            && (self.blood_ph - 7.4).abs() < 0.1
    }

    /// 总营养储备（千卡近似）
    pub fn total_nutrition_kcal(&self) -> f32 {
        self.nutrients.carbohydrates * 4.0
            + self.nutrients.proteins * 4.0
            + self.nutrients.fats * 9.0
            + self.nutrients.fiber * 2.0
    }

    /// 三大营养素比例（碳水:蛋白:脂肪），归一化到 1.0
    pub fn macronutrient_ratio(&self) -> (f32, f32, f32) {
        let c = self.nutrients.carbohydrates.max(0.0);
        let p = self.nutrients.proteins.max(0.0);
        let f = self.nutrients.fats.max(0.0);
        let total = c + p + f;
        if total < 1e-6 { return (0.0, 0.0, 0.0); }
        (c / total, p / total, f / total)
    }

    /// 总毒素负荷
    pub fn toxin_load(&self) -> f32 {
        self.toxins.iter().map(|t| t.concentration * t.damage_per_second).sum()
    }

    /// 代谢效率（0-1）
    pub fn metabolic_efficiency(&self) -> f32 {
        let temp_factor = 1.0 - ((self.body_temperature - self.optimal_temperature).abs() / 10.0).min(1.0);
        let ph_factor = 1.0 - ((self.blood_ph - 7.4).abs() / 0.4).min(1.0);
        let oxygen_factor = self.oxygen_level.clamp(0.0, 1.0);
        let toxin_factor = 1.0 - (self.toxin_load() / 2.0).min(1.0);
        (temp_factor * ph_factor * oxygen_factor * toxin_factor).clamp(0.0, 1.0)
    }

    /// 应用辐射剂量
    pub fn apply_radiation(&mut self, dose_gy: f32) {
        if dose_gy <= 0.0 { return; }
        self.add_toxin(format!("Radiation {:.2}Gy", dose_gy), dose_gy.min(1.0), ToxinSource::Radiation, 86400.0);
        self.energy = (self.energy - dose_gy * 100.0).max(0.0);
    }

    /// 应用温度应激
    pub fn apply_temperature_stress(&mut self, external_temp_k: f32, dt: f32) {
        let delta = (external_temp_k - self.body_temperature) * 0.001 * dt;
        self.body_temperature += delta;
    }

    /// 血 pH 是否正常
    pub fn ph_normal(&self) -> bool {
        self.blood_ph >= 7.35 && self.blood_ph <= 7.45
    }

    /// 是否缺氧
    pub fn is_hypoxic(&self) -> bool {
        self.oxygen_level < 0.9
    }

    pub fn energy_fraction(&self) -> f32 {
        if self.max_energy <= 0.0 { return 0.0; }
        (self.energy / self.max_energy).clamp(0.0, 1.0)
    }

    pub fn hydration_fraction(&self) -> f32 {
        if self.max_hydration <= 0.0 { return 0.0; }
        (self.hydration / self.max_hydration).clamp(0.0, 1.0)
    }
}

impl Nutrients {
    pub fn total_mass_grams(&self) -> f32 {
        self.carbohydrates + self.proteins + self.fats + self.vitamins + self.minerals + self.fiber
    }

    pub fn is_balanced(&self) -> bool {
        let c = self.carbohydrates.max(0.0);
        let p = self.proteins.max(0.0);
        let f = self.fats.max(0.0);
        let total = c + p + f;
        if total < 1e-6 { return false; }
        let c_r = c / total;
        let p_r = p / total;
        let f_r = f / total;
        c_r >= 0.40 && c_r <= 0.70 && p_r >= 0.10 && p_r <= 0.40 && f_r >= 0.15 && f_r <= 0.40
    }

    pub fn micronutrient_adequacy(&self) -> f32 {
        let v = (self.vitamins / 50.0).clamp(0.0, 1.0);
        let m = (self.minerals / 30.0).clamp(0.0, 1.0);
        (v + m) * 0.5
    }
}

impl Toxin {
    pub fn effective_damage_per_second(&self) -> f32 {
        self.concentration * self.damage_per_second
    }

    pub fn is_biological(&self) -> bool {
        matches!(self.source, ToxinSource::Venom | ToxinSource::Biological | ToxinSource::Mycotoxin | ToxinSource::Bacterial | ToxinSource::Viral | ToxinSource::Prion)
    }

    pub fn clearance_time_seconds(&self) -> f32 {
        self.half_life * 5.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metabolism_default_normal() {
        let m = Metabolism::default();
        assert_eq!(m.status, MetabolicStatus::Normal);
        assert!(m.body_temperature > 300.0);
        assert!(m.oxygen_level > 0.9);
    }

    #[test]
    fn test_metabolism_update_reduces_energy() {
        let mut m = Metabolism::default();
        let initial = m.energy;
        m.update(3600.0);
        assert!(m.energy < initial);
        assert!(m.calories_burned > 0.0);
    }

    #[test]
    fn test_metabolism_update_reduces_hydration() {
        let mut m = Metabolism::default();
        let initial = m.hydration;
        m.update(3600.0);
        assert!(m.hydration < initial);
    }

    #[test]
    fn test_consume_food_increases_energy() {
        let mut m = Metabolism::default();
        m.energy = 100.0;
        let n = Nutrients { carbohydrates: 100.0, proteins: 50.0, fats: 30.0, vitamins: 10.0, minerals: 5.0, fiber: 5.0 };
        m.consume_food(500.0, n);
        assert!(m.energy > 100.0);
    }

    #[test]
    fn test_consume_water_capped() {
        let mut m = Metabolism::default();
        m.hydration = 90.0;
        m.consume_water(50.0);
        assert_eq!(m.hydration, 100.0);
    }

    #[test]
    fn test_add_toxin_radiation() {
        let mut m = Metabolism::default();
        m.add_toxin("Rad".to_string(), 0.8, ToxinSource::Radiation, 86400.0);
        assert_eq!(m.toxins.len(), 1);
        assert!(m.toxins[0].damage_per_second > 0.0);
    }

    #[test]
    fn test_health_penalty_increases_with_toxin() {
        let mut m = Metabolism::default();
        let base = m.health_penalty();
        m.add_toxin("Venom".to_string(), 0.5, ToxinSource::Venom, 3600.0);
        assert!(m.health_penalty() > base);
    }

    #[test]
    fn test_toxin_load_sums() {
        let mut m = Metabolism::default();
        m.add_toxin("A".to_string(), 0.5, ToxinSource::Venom, 3600.0);
        m.add_toxin("B".to_string(), 0.3, ToxinSource::Chemical, 7200.0);
        assert!(m.toxin_load() > 0.0);
    }

    #[test]
    fn test_metabolic_efficiency_normal_high() {
        let m = Metabolism::default();
        assert!(m.metabolic_efficiency() > 0.9);
    }

    #[test]
    fn test_metabolic_efficiency_drops_with_hypoxia() {
        let mut m = Metabolism::default();
        m.oxygen_level = 0.5;
        assert!(m.metabolic_efficiency() < 0.7);
    }

    #[test]
    fn test_is_homeostatic_default() {
        let m = Metabolism::default();
        assert!(m.is_homeostatic());
    }

    #[test]
    fn test_is_homeostatic_false_starving() {
        let mut m = Metabolism::default();
        m.energy = -10.0;
        m.update_status();
        assert!(!m.is_homeostatic());
    }

    #[test]
    fn test_macronutrient_ratio_sums_one() {
        let m = Metabolism::default();
        let (c, p, f) = m.macronutrient_ratio();
        assert!((c + p + f - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_total_nutrition_kcal_positive() {
        let m = Metabolism::default();
        assert!(m.total_nutrition_kcal() > 0.0);
    }

    #[test]
    fn test_nutrients_total_mass() {
        let n = Nutrients { carbohydrates: 100.0, proteins: 50.0, fats: 30.0, vitamins: 10.0, minerals: 5.0, fiber: 5.0 };
        assert_eq!(n.total_mass_grams(), 200.0);
    }

    #[test]
    fn test_nutrients_micronutrient_adequacy() {
        let n = Nutrients { carbohydrates: 0.0, proteins: 0.0, fats: 0.0, vitamins: 50.0, minerals: 30.0, fiber: 0.0 };
        assert!((n.micronutrient_adequacy() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_toxin_is_biological() {
        let t1 = Toxin { name: "Snake".to_string(), concentration: 0.5, damage_per_second: 0.5, source: ToxinSource::Venom, half_life: 3600.0 };
        assert!(t1.is_biological());
        let t2 = Toxin { name: "Lead".to_string(), concentration: 0.5, damage_per_second: 0.05, source: ToxinSource::HeavyMetal, half_life: 86400.0 };
        assert!(!t2.is_biological());
    }

    #[test]
    fn test_toxin_clearance_time() {
        let t = Toxin { name: "X".to_string(), concentration: 0.5, damage_per_second: 0.1, source: ToxinSource::Chemical, half_life: 3600.0 };
        assert_eq!(t.clearance_time_seconds(), 18000.0);
    }

    #[test]
    fn test_apply_radiation_effects() {
        let mut m = Metabolism::default();
        let initial = m.energy;
        m.apply_radiation(2.0);
        assert!(!m.toxins.is_empty());
        assert!(m.energy < initial);
    }

    #[test]
    fn test_apply_temperature_stress() {
        let mut m = Metabolism::default();
        let initial = m.body_temperature;
        m.apply_temperature_stress(320.0, 600.0);
        assert!(m.body_temperature != initial);
    }

    #[test]
    fn test_ph_normal_range() {
        let mut m = Metabolism::default();
        assert!(m.ph_normal());
        m.blood_ph = 7.2;
        assert!(!m.ph_normal());
    }

    #[test]
    fn test_is_hypoxic_threshold() {
        let mut m = Metabolism::default();
        assert!(!m.is_hypoxic());
        m.oxygen_level = 0.85;
        assert!(m.is_hypoxic());
    }
}