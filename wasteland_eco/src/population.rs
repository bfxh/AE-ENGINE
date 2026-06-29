use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    pub species_id: String,
    pub count: f32,
    pub carrying_capacity: f32,
    pub growth_rate: f32,
    pub death_rate: f32,
    pub birth_rate: f32,
    pub biomass: f32,
    pub age_distribution: AgeDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeDistribution {
    pub juvenile: f32,
    pub adult: f32,
    pub senescent: f32,
}

impl Default for AgeDistribution {
    fn default() -> Self {
        Self { juvenile: 0.3, adult: 0.5, senescent: 0.2 }
    }
}

impl Population {
    pub fn new(
        species_id: &str,
        initial_count: f32,
        growth_rate: f32,
        carrying_capacity: f32,
    ) -> Self {
        Self {
            species_id: species_id.to_string(),
            count: initial_count,
            carrying_capacity,
            growth_rate,
            death_rate: 0.0,
            birth_rate: growth_rate,
            biomass: initial_count * 10.0,
            age_distribution: AgeDistribution::default(),
        }
    }

    pub fn logistic_growth(&mut self, dt: f32) {
        let r = self.growth_rate;
        let k = self.carrying_capacity;
        let n = self.count;

        let dn = r * n * (1.0 - n / k) * dt;
        self.count = (self.count + dn).max(0.0);
        self.birth_rate = r * n;
        self.death_rate = r * n * n / k;
        self.biomass = self.count * 10.0;
    }

    pub fn apply_mortality(&mut self, mortality_rate: f32, dt: f32) {
        self.count -= self.count * mortality_rate * dt;
        self.count = self.count.max(0.0);
        self.death_rate += mortality_rate;
    }

    pub fn apply_migration(&mut self, immigration: f32, emigration: f32, dt: f32) {
        self.count += (immigration - emigration) * dt;
        self.count = self.count.max(0.0);
    }

    pub fn update_age(&mut self, dt: f32) {
        let aging_rate = 0.001 * dt;
        self.age_distribution.juvenile = (self.age_distribution.juvenile - aging_rate).max(0.0);
        self.age_distribution.adult =
            (self.age_distribution.adult + aging_rate - aging_rate * 0.5).max(0.0);
        self.age_distribution.senescent =
            (self.age_distribution.senescent + aging_rate * 0.5).min(0.5);
    }

    pub fn reproductive_rate(&self) -> f32 {
        self.birth_rate * self.age_distribution.adult
    }

    pub fn is_extinct(&self) -> bool {
        self.count < 1.0
    }

    pub fn is_overpopulated(&self) -> bool {
        self.count > self.carrying_capacity * 0.9
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotkaVolterra {
    pub prey: Population,
    pub predator: Population,
    pub predation_rate: f32,
    pub conversion_efficiency: f32,
    pub predator_mortality: f32,
}

impl LotkaVolterra {
    pub fn new(
        prey: Population,
        predator: Population,
        predation_rate: f32,
        conversion_efficiency: f32,
        predator_mortality: f32,
    ) -> Self {
        Self { prey, predator, predation_rate, conversion_efficiency, predator_mortality }
    }

    pub fn step(&mut self, dt: f32) {
        let p = self.prey.count;
        let q = self.predator.count;
        let a = self.predation_rate;
        let b = self.conversion_efficiency;
        let m = self.predator_mortality;

        let dp = (self.prey.growth_rate * p - a * p * q) * dt;
        let dq = (b * a * p * q - m * q) * dt;

        self.prey.count = (self.prey.count + dp).max(0.0);
        self.predator.count = (self.predator.count + dq).max(0.0);

        self.prey.biomass = self.prey.count * 10.0;
        self.predator.biomass = self.predator.count * 10.0;
    }

    pub fn phase_portrait(&self) -> (f32, f32) {
        (self.prey.count, self.predator.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_population_creation() {
        let pop = Population::new("rabbit", 100.0, 0.1, 1000.0);
        assert_eq!(pop.species_id, "rabbit");
        assert_eq!(pop.count, 100.0);
        assert_eq!(pop.growth_rate, 0.1);
        assert_eq!(pop.carrying_capacity, 1000.0);
        assert!(!pop.is_extinct());
    }

    #[test]
    fn test_population_logistic_growth() {
        let mut pop = Population::new("deer", 100.0, 0.2, 1000.0);
        pop.logistic_growth(1.0);
        assert!(pop.count > 100.0);
        assert!(pop.birth_rate > 0.0);
        assert!(pop.death_rate > 0.0);
    }

    #[test]
    fn test_population_mortality_and_extinction() {
        let mut pop = Population::new("doomed", 10.0, 0.0, 100.0);
        pop.apply_mortality(0.5, 1.0);
        assert_eq!(pop.count, 5.0);
        pop.apply_mortality(1.0, 10.0);
        assert!(pop.is_extinct());
    }
}
