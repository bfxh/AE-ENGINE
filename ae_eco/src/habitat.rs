use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitatPatch {
    pub id: String,
    pub position: Vec3,
    pub area: f32,
    pub quality: f32,
    pub habitat_type: HabitatType,
    pub resources: ResourcePool,
    pub fragmentation: f32,
    pub edge_ratio: f32,
    pub connectivity: f32,
    pub disturbance_frequency: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HabitatType {
    Forest,
    Grassland,
    Wetland,
    Desert,
    Tundra,
    Aquatic,
    Coastal,
    Urban,
    Agricultural,
    Riparian,
}

impl HabitatType {
    pub fn base_carrying_capacity(&self) -> f32 {
        match self {
            HabitatType::Forest => 1000.0,
            HabitatType::Grassland => 500.0,
            HabitatType::Wetland => 800.0,
            HabitatType::Desert => 50.0,
            HabitatType::Tundra => 100.0,
            HabitatType::Aquatic => 2000.0,
            HabitatType::Coastal => 1500.0,
            HabitatType::Urban => 30.0,
            HabitatType::Agricultural => 300.0,
            HabitatType::Riparian => 600.0,
        }
    }

    pub fn resilience(&self) -> f32 {
        match self {
            HabitatType::Forest => 0.3,
            HabitatType::Grassland => 0.5,
            HabitatType::Wetland => 0.2,
            HabitatType::Desert => 0.1,
            HabitatType::Tundra => 0.1,
            HabitatType::Aquatic => 0.4,
            HabitatType::Coastal => 0.3,
            HabitatType::Urban => 0.8,
            HabitatType::Agricultural => 0.6,
            HabitatType::Riparian => 0.35,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    pub water: f32,
    pub nutrients: f32,
    pub light: f32,
    pub shelter: f32,
    pub food: f32,
    pub nesting_sites: f32,
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self { water: 0.5, nutrients: 0.5, light: 0.5, shelter: 0.5, food: 0.5, nesting_sites: 0.5 }
    }
}

impl ResourcePool {
    pub fn total_resources(&self) -> f32 {
        self.water + self.nutrients + self.light + self.shelter + self.food + self.nesting_sites
    }

    pub fn limiting_resource(&self) -> (&str, f32) {
        let resources = [
            ("water", self.water),
            ("nutrients", self.nutrients),
            ("light", self.light),
            ("shelter", self.shelter),
            ("food", self.food),
            ("nesting_sites", self.nesting_sites),
        ];
        resources
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(n, v)| (*n, *v))
            .unwrap()
    }
}

impl HabitatPatch {
    pub fn new(id: &str, position: Vec3, area: f32, habitat_type: HabitatType) -> Self {
        Self {
            id: id.to_string(),
            position,
            area,
            quality: 0.8,
            habitat_type,
            resources: ResourcePool::default(),
            fragmentation: 0.0,
            edge_ratio: 0.0,
            connectivity: 0.0,
            disturbance_frequency: 0.0,
        }
    }

    pub fn effective_area(&self) -> f32 {
        let core_area_ratio = 1.0 - self.edge_ratio;
        let quality_factor = self.quality;
        let fragment_factor = 1.0 - self.fragmentation;
        self.area * core_area_ratio * quality_factor * fragment_factor
    }

    pub fn carrying_capacity(&self, species_body_mass: f32) -> f32 {
        let base = self.habitat_type.base_carrying_capacity();
        let area_factor = self.effective_area() / 10000.0;
        let mass_factor = 1.0 / species_body_mass.powf(0.75);
        let resource_factor = self.resources.total_resources() / 6.0;
        base * area_factor * mass_factor * resource_factor * self.quality
    }

    pub fn habitat_suitability(&self, species_requirements: &SpeciesRequirements) -> f32 {
        let water_match = self.resources.water.min(species_requirements.water_need)
            / species_requirements.water_need.max(0.001);
        let food_match = self.resources.food.min(species_requirements.food_need)
            / species_requirements.food_need.max(0.001);
        let shelter_match = self.resources.shelter.min(species_requirements.shelter_need)
            / species_requirements.shelter_need.max(0.001);
        let area_match = if self.area >= species_requirements.min_area {
            1.0
        } else {
            self.area / species_requirements.min_area
        };

        let type_match =
            if self.habitat_type == species_requirements.preferred_habitat { 1.0 } else { 0.3 };

        (water_match * 0.3
            + food_match * 0.3
            + shelter_match * 0.2
            + area_match * 0.1
            + type_match * 0.1)
            * self.quality
            * (1.0 - self.fragmentation)
    }

    pub fn apply_disturbance(&mut self, severity: f32) {
        self.quality = (self.quality - severity * 0.3).max(0.0);
        self.resources.food *= 1.0 - severity * 0.5;
        self.resources.shelter *= 1.0 - severity * 0.4;
        self.resources.nesting_sites *= 1.0 - severity * 0.3;
        self.disturbance_frequency += severity * 0.1;
        self.fragmentation = (self.fragmentation + severity * 0.2).min(1.0);
    }

    pub fn recover(&mut self, dt: f32) {
        let recovery_rate = self.habitat_type.resilience() * 0.001;
        self.quality = (self.quality + recovery_rate * dt).min(1.0);
        self.resources.food = (self.resources.food + recovery_rate * dt).min(1.0);
        self.resources.shelter = (self.resources.shelter + recovery_rate * dt).min(1.0);
        self.resources.nesting_sites = (self.resources.nesting_sites + recovery_rate * dt).min(1.0);
        self.fragmentation = (self.fragmentation - recovery_rate * 0.5 * dt).max(0.0);
        self.disturbance_frequency = (self.disturbance_frequency - 0.0001 * dt).max(0.0);
    }

    pub fn update_resources(&mut self, rainfall: f32, temperature: f32, dt: f32) {
        let temp_factor = (temperature - 273.0).max(0.0) / 30.0;
        self.resources.water =
            (self.resources.water + rainfall * 0.1 * dt - 0.001 * dt).clamp(0.0, 1.0);
        self.resources.nutrients =
            (self.resources.nutrients + 0.0001 * dt - 0.00005 * temp_factor * dt).clamp(0.0, 1.0);
        self.resources.light = (self.resources.light + 0.001 * dt).clamp(0.0, 1.0);
        self.resources.food =
            (self.resources.food + 0.0005 * temp_factor * dt - 0.0002 * dt).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesRequirements {
    pub water_need: f32,
    pub food_need: f32,
    pub shelter_need: f32,
    pub min_area: f32,
    pub preferred_habitat: HabitatType,
    pub temperature_range: (f32, f32),
    pub ph_range: (f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitatNetwork {
    pub patches: Vec<HabitatPatch>,
    pub connectivity_matrix: Vec<Vec<f32>>,
    pub corridors: Vec<HabitatCorridor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitatCorridor {
    pub from_id: String,
    pub to_id: String,
    pub width: f32,
    pub quality: f32,
    pub length: f32,
    pub permeability: f32,
}

impl HabitatNetwork {
    pub fn new() -> Self {
        Self { patches: Vec::new(), connectivity_matrix: Vec::new(), corridors: Vec::new() }
    }

    pub fn add_patch(&mut self, patch: HabitatPatch) {
        self.patches.push(patch);
        self.rebuild_connectivity();
    }

    pub fn add_corridor(&mut self, corridor: HabitatCorridor) {
        self.corridors.push(corridor);
    }

    pub fn rebuild_connectivity(&mut self) {
        let n = self.patches.len();
        self.connectivity_matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dist = (self.patches[i].position - self.patches[j].position).length();
                let connectivity = (-dist * 0.001).exp();
                self.connectivity_matrix[i][j] = connectivity;
                self.connectivity_matrix[j][i] = connectivity;
            }
        }
    }

    pub fn patch_connectivity(&self, patch_idx: usize) -> f32 {
        if patch_idx >= self.patches.len() {
            return 0.0;
        }
        self.connectivity_matrix[patch_idx].iter().sum::<f32>() / self.patches.len().max(1) as f32
    }

    pub fn landscape_connectivity(&self) -> f32 {
        let n = self.patches.len();
        if n == 0 {
            return 0.0;
        }
        let total: f32 = self.connectivity_matrix.iter().flat_map(|row| row.iter()).sum();
        total / (n * n) as f32
    }

    pub fn fragmentation_index(&self) -> f32 {
        let n = self.patches.len() as f32;
        if n <= 1.0 {
            return 0.0;
        }
        let total_area: f32 = self.patches.iter().map(|p| p.area).sum();
        let mean_area = total_area / n;
        let variance: f32 =
            self.patches.iter().map(|p| (p.area - mean_area).powi(2)).sum::<f32>() / n;
        variance.sqrt() / mean_area.max(0.001)
    }

    pub fn largest_patch_index(&self) -> f32 {
        let max_area = self.patches.iter().map(|p| p.area).fold(0.0f32, f32::max);
        let total_area: f32 = self.patches.iter().map(|p| p.area).sum();
        if total_area > 0.0 { max_area / total_area } else { 0.0 }
    }

    pub fn metapopulation_capacity(&self, species_body_mass: f32) -> f32 {
        self.patches.iter().map(|p| p.carrying_capacity(species_body_mass)).sum()
    }

    pub fn step(&mut self, rainfall: f32, temperature: f32, dt: f32) {
        for patch in &mut self.patches {
            patch.update_resources(rainfall, temperature, dt);
            patch.recover(dt);
        }
    }
}

impl Default for HabitatNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_habitat_patch_creation() {
        let patch = HabitatPatch::new(
            "forest_1",
            Vec3::new(100.0, 0.0, 200.0),
            10000.0,
            HabitatType::Forest,
        );
        assert_eq!(patch.id, "forest_1");
        assert_eq!(patch.area, 10000.0);
        assert!(patch.quality > 0.0);
    }

    #[test]
    fn test_carrying_capacity() {
        let patch = HabitatPatch::new("grass_1", Vec3::ZERO, 50000.0, HabitatType::Grassland);
        let capacity = patch.carrying_capacity(50.0);
        assert!(capacity > 0.0);
    }

    #[test]
    fn test_habitat_suitability() {
        let mut patch = HabitatPatch::new("forest_1", Vec3::ZERO, 20000.0, HabitatType::Forest);
        patch.resources.water = 0.8;
        patch.resources.food = 0.7;
        patch.resources.shelter = 0.9;

        let requirements = SpeciesRequirements {
            water_need: 0.5,
            food_need: 0.5,
            shelter_need: 0.5,
            min_area: 10000.0,
            preferred_habitat: HabitatType::Forest,
            temperature_range: (273.0, 310.0),
            ph_range: (5.0, 8.0),
        };

        let suitability = patch.habitat_suitability(&requirements);
        assert!(suitability > 0.0);
        assert!(suitability <= 1.0);
    }

    #[test]
    fn test_disturbance_and_recovery() {
        let mut patch = HabitatPatch::new("forest_1", Vec3::ZERO, 10000.0, HabitatType::Forest);
        let initial_quality = patch.quality;
        patch.apply_disturbance(0.5);
        assert!(patch.quality < initial_quality);
        patch.recover(100.0);
        assert!(patch.quality > 0.0);
    }

    #[test]
    fn test_habitat_network() {
        let mut network = HabitatNetwork::new();
        network.add_patch(HabitatPatch::new(
            "a",
            Vec3::new(0.0, 0.0, 0.0),
            10000.0,
            HabitatType::Forest,
        ));
        network.add_patch(HabitatPatch::new(
            "b",
            Vec3::new(100.0, 0.0, 0.0),
            15000.0,
            HabitatType::Forest,
        ));
        network.add_patch(HabitatPatch::new(
            "c",
            Vec3::new(0.0, 0.0, 100.0),
            8000.0,
            HabitatType::Grassland,
        ));

        assert_eq!(network.patches.len(), 3);
        let conn = network.landscape_connectivity();
        assert!(conn > 0.0);
    }

    #[test]
    fn test_resource_update() {
        let mut patch = HabitatPatch::new("wetland_1", Vec3::ZERO, 5000.0, HabitatType::Wetland);
        patch.update_resources(0.01, 290.0, 1.0);
        assert!(patch.resources.water > 0.0);
    }

    #[test]
    fn test_effective_area() {
        let mut patch = HabitatPatch::new("forest_1", Vec3::ZERO, 10000.0, HabitatType::Forest);
        patch.edge_ratio = 0.3;
        patch.fragmentation = 0.2;
        let effective = patch.effective_area();
        assert!(effective < 10000.0);
        assert!(effective > 0.0);
    }
}
