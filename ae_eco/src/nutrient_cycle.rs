use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientPool {
    pub nitrogen: NitrogenPool,
    pub phosphorus: PhosphorusPool,
    pub potassium: PotassiumPool,
    pub carbon: CarbonPool,
    pub organic_matter: f32,
    pub soil_ph: f32,
    pub cation_exchange_capacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitrogenPool {
    pub organic_n: f32,
    pub ammonium: f32,
    pub nitrate: f32,
    pub atmospheric_n: f32,
    pub plant_uptake_n: f32,
    pub denitrification: f32,
    pub nitrification: f32,
    pub mineralization: f32,
    pub fixation: f32,
    pub leaching: f32,
    pub volatilization: f32,
}

impl Default for NitrogenPool {
    fn default() -> Self {
        Self {
            organic_n: 100.0,
            ammonium: 5.0,
            nitrate: 10.0,
            atmospheric_n: 0.0,
            plant_uptake_n: 0.0,
            denitrification: 0.0,
            nitrification: 0.0,
            mineralization: 0.0,
            fixation: 0.0,
            leaching: 0.0,
            volatilization: 0.0,
        }
    }
}

impl NitrogenPool {
    pub fn total_nitrogen(&self) -> f32 {
        self.organic_n + self.ammonium + self.nitrate
    }

    pub fn available_nitrogen(&self) -> f32 {
        self.ammonium + self.nitrate
    }

    pub fn update(&mut self, temperature: f32, moisture: f32, ph: f32, dt: f32) {
        let temp_factor = ((temperature - 273.0) / 25.0).clamp(0.1, 2.0);
        let moisture_factor = moisture.clamp(0.1, 1.0);
        let ph_factor = if ph < 5.0 {
            0.5
        } else if ph > 8.0 {
            0.7
        } else {
            1.0
        };

        self.mineralization = self.organic_n * 0.001 * temp_factor * moisture_factor * dt;
        self.organic_n -= self.mineralization;
        self.ammonium += self.mineralization;

        self.nitrification = self.ammonium * 0.002 * temp_factor * moisture_factor * ph_factor * dt;
        self.ammonium -= self.nitrification;
        self.nitrate += self.nitrification;

        self.denitrification = self.nitrate * 0.0005 * moisture_factor * temp_factor * dt;
        self.nitrate -= self.denitrification;
        self.atmospheric_n += self.denitrification;

        self.leaching = self.nitrate * 0.0003 * moisture * dt;
        self.nitrate -= self.leaching;

        self.volatilization = self.ammonium * 0.0001 * temp_factor * dt;
        self.ammonium -= self.volatilization;

        self.fixation = 0.001 * moisture_factor * dt;
        self.ammonium += self.fixation;

        self.ammonium = self.ammonium.max(0.0);
        self.nitrate = self.nitrate.max(0.0);
        self.organic_n = self.organic_n.max(0.0);
    }

    pub fn plant_uptake(&mut self, demand: f32) -> f32 {
        let available = self.ammonium + self.nitrate;
        let uptake = demand.min(available);
        let nitrate_uptake = uptake * 0.7;
        let ammonium_uptake = uptake * 0.3;

        self.nitrate = (self.nitrate - nitrate_uptake).max(0.0);
        self.ammonium = (self.ammonium - ammonium_uptake).max(0.0);
        self.plant_uptake_n = uptake;
        uptake
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhosphorusPool {
    pub organic_p: f32,
    pub labile_p: f32,
    pub fixed_p: f32,
    pub mineral_p: f32,
    pub plant_uptake_p: f32,
    pub mineralization: f32,
    pub immobilization: f32,
    pub adsorption: f32,
    pub desorption: f32,
    pub weathering: f32,
}

impl Default for PhosphorusPool {
    fn default() -> Self {
        Self {
            organic_p: 50.0,
            labile_p: 3.0,
            fixed_p: 20.0,
            mineral_p: 100.0,
            plant_uptake_p: 0.0,
            mineralization: 0.0,
            immobilization: 0.0,
            adsorption: 0.0,
            desorption: 0.0,
            weathering: 0.0,
        }
    }
}

impl PhosphorusPool {
    pub fn total_phosphorus(&self) -> f32 {
        self.organic_p + self.labile_p + self.fixed_p + self.mineral_p
    }

    pub fn available_phosphorus(&self) -> f32 {
        self.labile_p
    }

    pub fn update(&mut self, temperature: f32, moisture: f32, ph: f32, dt: f32) {
        let temp_factor = ((temperature - 273.0) / 25.0).clamp(0.1, 2.0);
        let moisture_factor = moisture.clamp(0.1, 1.0);

        self.mineralization = self.organic_p * 0.0008 * temp_factor * moisture_factor * dt;
        self.organic_p -= self.mineralization;
        self.labile_p += self.mineralization;

        let ph_factor = if !(5.5..=7.5).contains(&ph) { 0.5 } else { 1.0 };
        self.adsorption = self.labile_p * 0.001 * (1.0 - ph_factor) * dt;
        self.labile_p -= self.adsorption;
        self.fixed_p += self.adsorption;

        self.desorption = self.fixed_p * 0.0002 * ph_factor * dt;
        self.fixed_p -= self.desorption;
        self.labile_p += self.desorption;

        self.weathering = self.mineral_p * 0.00005 * moisture_factor * dt;
        self.mineral_p -= self.weathering;
        self.labile_p += self.weathering;

        self.labile_p = self.labile_p.max(0.0);
        self.organic_p = self.organic_p.max(0.0);
        self.fixed_p = self.fixed_p.max(0.0);
        self.mineral_p = self.mineral_p.max(0.0);
    }

    pub fn plant_uptake(&mut self, demand: f32) -> f32 {
        let uptake = demand.min(self.labile_p);
        self.labile_p -= uptake;
        self.plant_uptake_p = uptake;
        uptake
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotassiumPool {
    pub exchangeable_k: f32,
    pub fixed_k: f32,
    pub mineral_k: f32,
    pub solution_k: f32,
    pub plant_uptake_k: f32,
    pub leaching: f32,
    pub weathering: f32,
}

impl Default for PotassiumPool {
    fn default() -> Self {
        Self {
            exchangeable_k: 20.0,
            fixed_k: 50.0,
            mineral_k: 200.0,
            solution_k: 2.0,
            plant_uptake_k: 0.0,
            leaching: 0.0,
            weathering: 0.0,
        }
    }
}

impl PotassiumPool {
    pub fn total_potassium(&self) -> f32 {
        self.exchangeable_k + self.fixed_k + self.mineral_k + self.solution_k
    }

    pub fn available_potassium(&self) -> f32 {
        self.exchangeable_k + self.solution_k
    }

    pub fn update(&mut self, moisture: f32, dt: f32) {
        let moisture_factor = moisture.clamp(0.1, 1.0);

        self.weathering = self.mineral_k * 0.00003 * moisture_factor * dt;
        self.mineral_k -= self.weathering;
        self.exchangeable_k += self.weathering;

        let exchange = (self.exchangeable_k - self.solution_k * 10.0) * 0.001 * dt;
        self.exchangeable_k -= exchange;
        self.solution_k = (self.solution_k + exchange).max(0.0);

        self.leaching = self.solution_k * 0.0002 * moisture * dt;
        self.solution_k -= self.leaching;

        self.exchangeable_k = self.exchangeable_k.max(0.0);
        self.solution_k = self.solution_k.max(0.0);
        self.mineral_k = self.mineral_k.max(0.0);
    }

    pub fn plant_uptake(&mut self, demand: f32) -> f32 {
        let available = self.exchangeable_k + self.solution_k;
        let uptake = demand.min(available);
        let sol_uptake = uptake * 0.6;
        let exc_uptake = uptake * 0.4;

        self.solution_k = (self.solution_k - sol_uptake).max(0.0);
        self.exchangeable_k = (self.exchangeable_k - exc_uptake).max(0.0);
        self.plant_uptake_k = uptake;
        uptake
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonPool {
    pub soil_organic_carbon: f32,
    pub microbial_biomass_c: f32,
    pub dissolved_organic_c: f32,
    pub particulate_organic_c: f32,
    pub humus_c: f32,
    pub respiration: f32,
    pub litter_input: f32,
    pub decomposition: f32,
    pub humification: f32,
}

impl Default for CarbonPool {
    fn default() -> Self {
        Self {
            soil_organic_carbon: 500.0,
            microbial_biomass_c: 50.0,
            dissolved_organic_c: 5.0,
            particulate_organic_c: 100.0,
            humus_c: 200.0,
            respiration: 0.0,
            litter_input: 0.0,
            decomposition: 0.0,
            humification: 0.0,
        }
    }
}

impl CarbonPool {
    pub fn total_carbon(&self) -> f32 {
        self.soil_organic_carbon
            + self.microbial_biomass_c
            + self.dissolved_organic_c
            + self.particulate_organic_c
            + self.humus_c
    }

    pub fn update(&mut self, temperature: f32, moisture: f32, litter_input: f32, dt: f32) {
        let temp_factor = ((temperature - 273.0) / 25.0).clamp(0.1, 3.0);
        let moisture_factor = moisture.clamp(0.1, 1.0);

        self.litter_input = litter_input * dt;
        self.particulate_organic_c += self.litter_input;

        self.decomposition =
            self.particulate_organic_c * 0.001 * temp_factor * moisture_factor * dt;
        self.particulate_organic_c -= self.decomposition;
        self.microbial_biomass_c += self.decomposition * 0.3;
        self.dissolved_organic_c += self.decomposition * 0.2;

        self.respiration =
            self.decomposition * 0.5 + self.microbial_biomass_c * 0.0005 * temp_factor * dt;

        self.humification = self.microbial_biomass_c * 0.0002 * dt;
        self.microbial_biomass_c -= self.humification;
        self.humus_c += self.humification;

        self.soil_organic_carbon = self.particulate_organic_c
            + self.microbial_biomass_c
            + self.dissolved_organic_c
            + self.humus_c;

        self.particulate_organic_c = self.particulate_organic_c.max(0.0);
        self.microbial_biomass_c = self.microbial_biomass_c.max(0.0);
        self.dissolved_organic_c = self.dissolved_organic_c.max(0.0);
        self.humus_c = self.humus_c.max(0.0);
    }
}

impl Default for NutrientPool {
    fn default() -> Self {
        Self {
            nitrogen: NitrogenPool::default(),
            phosphorus: PhosphorusPool::default(),
            potassium: PotassiumPool::default(),
            carbon: CarbonPool::default(),
            organic_matter: 5.0,
            soil_ph: 6.5,
            cation_exchange_capacity: 15.0,
        }
    }
}

impl NutrientPool {
    pub fn update(&mut self, temperature: f32, moisture: f32, litter_input: f32, dt: f32) {
        self.nitrogen.update(temperature, moisture, self.soil_ph, dt);
        self.phosphorus.update(temperature, moisture, self.soil_ph, dt);
        self.potassium.update(moisture, dt);
        self.carbon.update(temperature, moisture, litter_input, dt);

        self.organic_matter = self.carbon.total_carbon() * 0.58;

        if self.organic_matter > 0.0 {
            self.soil_ph += (7.0 - self.soil_ph) * 0.0001 * dt;
            self.soil_ph = self.soil_ph.clamp(4.0, 9.0);
        }
    }

    pub fn plant_uptake(&mut self, n_demand: f32, p_demand: f32, k_demand: f32) -> (f32, f32, f32) {
        let n_uptake = self.nitrogen.plant_uptake(n_demand);
        let p_uptake = self.phosphorus.plant_uptake(p_demand);
        let k_uptake = self.potassium.plant_uptake(k_demand);
        (n_uptake, p_uptake, k_uptake)
    }

    pub fn fertility_index(&self) -> f32 {
        let n_factor = (self.nitrogen.available_nitrogen() / 20.0).min(1.0);
        let p_factor = (self.phosphorus.available_phosphorus() / 5.0).min(1.0);
        let k_factor = (self.potassium.available_potassium() / 25.0).min(1.0);
        let c_factor = (self.organic_matter / 10.0).min(1.0);
        n_factor * 0.3 + p_factor * 0.25 + k_factor * 0.25 + c_factor * 0.2
    }

    pub fn c_n_ratio(&self) -> f32 {
        let total_n = self.nitrogen.total_nitrogen();
        if total_n > 0.0 { self.carbon.total_carbon() / total_n } else { f32::MAX }
    }

    pub fn nutrient_limitation(&self) -> Vec<(&str, f32)> {
        let mut limitations = Vec::new();
        let n_avail = self.nitrogen.available_nitrogen();
        let p_avail = self.phosphorus.available_phosphorus();
        let k_avail = self.potassium.available_potassium();

        let max_n = 20.0;
        let max_p = 5.0;
        let max_k = 25.0;

        limitations.push(("N", n_avail / max_n));
        limitations.push(("P", p_avail / max_p));
        limitations.push(("K", k_avail / max_k));
        limitations.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        limitations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nitrogen_pool() {
        let mut n = NitrogenPool::default();
        let _initial = n.total_nitrogen();
        n.update(298.0, 0.5, 6.5, 1.0);
        assert!(n.ammonium > 0.0 || n.nitrate > 0.0);
    }

    #[test]
    fn test_nitrogen_uptake() {
        let mut n = NitrogenPool::default();
        let uptake = n.plant_uptake(5.0);
        assert!(uptake > 0.0);
        assert!(uptake <= 5.0);
    }

    #[test]
    fn test_phosphorus_pool() {
        let mut p = PhosphorusPool::default();
        let _initial = p.total_phosphorus();
        p.update(298.0, 0.5, 6.5, 1.0);
        assert!(p.labile_p > 0.0);
    }

    #[test]
    fn test_potassium_pool() {
        let mut k = PotassiumPool::default();
        let _initial = k.total_potassium();
        k.update(0.5, 1.0);
        assert!(k.available_potassium() > 0.0);
    }

    #[test]
    fn test_carbon_pool() {
        let mut c = CarbonPool::default();
        let _initial = c.total_carbon();
        c.update(298.0, 0.5, 1.0, 1.0);
        assert!(c.respiration > 0.0);
    }

    #[test]
    fn test_nutrient_pool() {
        let mut pool = NutrientPool::default();
        pool.update(298.0, 0.6, 2.0, 1.0);
        let fertility = pool.fertility_index();
        assert!(fertility > 0.0);
        assert!(fertility <= 1.0);
    }

    #[test]
    fn test_plant_uptake() {
        let mut pool = NutrientPool::default();
        let (n, p, k) = pool.plant_uptake(10.0, 2.0, 5.0);
        assert!(n > 0.0);
        assert!(p > 0.0);
        assert!(k > 0.0);
    }

    #[test]
    fn test_nutrient_limitation() {
        let pool = NutrientPool::default();
        let limitations = pool.nutrient_limitation();
        assert_eq!(limitations.len(), 3);
    }
}
