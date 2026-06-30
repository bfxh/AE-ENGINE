use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationModel {
    pub species_id: String,
    pub source_position: Vec3,
    pub destination: Vec3,
    pub migration_speed: f32,
    pub population_fraction: f32,
    pub energy_cost: f32,
    pub route: Vec<Vec3>,
    pub current_position: Vec3,
    pub progress: f32,
    pub is_active: bool,
    pub mortality_risk: f32,
}

impl MigrationModel {
    pub fn new(
        species_id: &str,
        source: Vec3,
        destination: Vec3,
        speed: f32,
        fraction: f32,
    ) -> Self {
        let distance = (destination - source).length();
        let steps = (distance / speed).ceil() as usize;
        let mut route = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = if steps > 0 { i as f32 / steps as f32 } else { 0.0 };
            route.push(source + (destination - source) * t);
        }

        Self {
            species_id: species_id.to_string(),
            source_position: source,
            destination,
            migration_speed: speed,
            population_fraction: fraction,
            energy_cost: distance * 0.01,
            route,
            current_position: source,
            progress: 0.0,
            is_active: true,
            mortality_risk: 0.01,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_active {
            return;
        }

        let total_distance = (self.destination - self.source_position).length();
        if total_distance < 0.001 {
            self.is_active = false;
            return;
        }

        let distance_moved = self.migration_speed * dt;
        self.progress += distance_moved / total_distance;
        self.progress = self.progress.min(1.0);

        self.current_position =
            self.source_position + (self.destination - self.source_position) * self.progress;
        self.energy_cost += distance_moved * 0.01;
        self.mortality_risk = 0.01 + self.progress * 0.05;

        if self.progress >= 1.0 {
            self.is_active = false;
            self.current_position = self.destination;
        }
    }

    pub fn distance_remaining(&self) -> f32 {
        (self.destination - self.current_position).length()
    }

    pub fn estimated_time_remaining(&self) -> f32 {
        if self.migration_speed > 0.0 {
            self.distance_remaining() / self.migration_speed
        } else {
            f32::MAX
        }
    }

    pub fn apply_barrier(&mut self, barrier_position: Vec3, barrier_radius: f32, detour_cost: f32) {
        let dist_to_barrier = (self.current_position - barrier_position).length();
        if dist_to_barrier < barrier_radius {
            let dir = (self.current_position - barrier_position).normalize();
            let perpendicular = Vec3::new(-dir.z, 0.0, dir.x);
            self.destination += perpendicular * barrier_radius * 2.0;
            self.energy_cost += detour_cost;
            self.rebuild_route();
        }
    }

    fn rebuild_route(&mut self) {
        let distance = (self.destination - self.current_position).length();
        let steps = (distance / self.migration_speed).ceil() as usize;
        self.route.clear();
        for i in 0..=steps {
            let t = if steps > 0 { i as f32 / steps as f32 } else { 0.0 };
            self.route.push(self.current_position + (self.destination - self.current_position) * t);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispersalKernel {
    pub kernel_type: DispersalType,
    pub mean_distance: f32,
    pub max_distance: f32,
    pub shape_parameter: f32,
    pub directionality: Vec3,
    pub anisotropy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispersalType {
    Gaussian,
    Exponential,
    FatTailed,
    Uniform,
    Directional,
    WindAssisted,
    AnimalAssisted,
}

impl DispersalKernel {
    pub fn new(kernel_type: DispersalType, mean_distance: f32) -> Self {
        Self {
            kernel_type,
            mean_distance,
            max_distance: mean_distance * 5.0,
            shape_parameter: 1.0,
            directionality: Vec3::ZERO,
            anisotropy: 0.0,
        }
    }

    pub fn probability(&self, distance: f32) -> f32 {
        if distance > self.max_distance {
            return 0.0;
        }

        match self.kernel_type {
            DispersalType::Gaussian => {
                let sigma = self.mean_distance / 2.0;
                (-distance.powi(2) / (2.0 * sigma.powi(2))).exp()
            },
            DispersalType::Exponential => {
                let lambda = 1.0 / self.mean_distance;
                (-lambda * distance).exp()
            },
            DispersalType::FatTailed => {
                let alpha = self.shape_parameter;
                1.0 / (1.0 + (distance / self.mean_distance).powf(alpha))
            },
            DispersalType::Uniform => {
                if distance <= self.max_distance {
                    1.0
                } else {
                    0.0
                }
            },
            DispersalType::Directional => {
                let base = self.probability_gaussian(distance);
                let wind_factor =
                    if self.directionality.length() > 0.0 { self.anisotropy } else { 0.0 };
                base * (1.0 + wind_factor)
            },
            DispersalType::WindAssisted => {
                self.probability_fat_tailed(distance) * (1.0 + self.anisotropy * 2.0)
            },
            DispersalType::AnimalAssisted => {
                let base = self.probability_exponential(distance);
                let carry_factor = 1.0 + self.anisotropy * 3.0;
                base * carry_factor
            },
        }
    }

    fn probability_gaussian(&self, distance: f32) -> f32 {
        let sigma = self.mean_distance / 2.0;
        (-distance.powi(2) / (2.0 * sigma.powi(2))).exp()
    }

    fn probability_exponential(&self, distance: f32) -> f32 {
        let lambda = 1.0 / self.mean_distance;
        (-lambda * distance).exp()
    }

    fn probability_fat_tailed(&self, distance: f32) -> f32 {
        let alpha = self.shape_parameter;
        1.0 / (1.0 + (distance / self.mean_distance).powf(alpha))
    }

    pub fn sample_distance(&self, rng: f32) -> f32 {
        match self.kernel_type {
            DispersalType::Gaussian => {
                let sigma = self.mean_distance / 2.0;
                (rng * 5.0 - 2.5).abs() * sigma
            },
            DispersalType::Exponential => -self.mean_distance * (1.0 - rng).ln(),
            DispersalType::FatTailed => {
                self.mean_distance * (rng / (1.0 - rng)).powf(1.0 / self.shape_parameter)
            },
            DispersalType::Uniform => rng * self.max_distance,
            _ => rng * self.mean_distance * 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSystem {
    pub migrations: Vec<MigrationModel>,
    pub dispersal_kernels: std::collections::HashMap<String, DispersalKernel>,
    pub seasonal_trigger: f32,
    pub season: Season,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl MigrationSystem {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
            dispersal_kernels: std::collections::HashMap::new(),
            seasonal_trigger: 0.5,
            season: Season::Spring,
        }
    }

    pub fn add_migration(&mut self, migration: MigrationModel) {
        self.migrations.push(migration);
    }

    pub fn add_dispersal_kernel(&mut self, species_id: &str, kernel: DispersalKernel) {
        self.dispersal_kernels.insert(species_id.to_string(), kernel);
    }

    pub fn update(&mut self, dt: f32) {
        for migration in &mut self.migrations {
            migration.update(dt);
        }
        self.migrations.retain(|m| m.is_active);
    }

    pub fn active_migrations(&self) -> usize {
        self.migrations.iter().filter(|m| m.is_active).count()
    }

    pub fn total_energy_cost(&self) -> f32 {
        self.migrations.iter().map(|m| m.energy_cost).sum()
    }

    pub fn dispersal_probability(&self, species_id: &str, source: Vec3, target: Vec3) -> f32 {
        let kernel = match self.dispersal_kernels.get(species_id) {
            Some(k) => k,
            None => return 0.0,
        };
        let distance = (target - source).length();
        kernel.probability(distance)
    }

    pub fn advance_season(&mut self, day_of_year: f32) {
        let prev = self.season;
        self.season = match day_of_year {
            d if d < 90.0 => Season::Winter,
            d if d < 180.0 => Season::Spring,
            d if d < 270.0 => Season::Summer,
            _ => Season::Autumn,
        };

        if prev != self.season {
            self.trigger_seasonal_migration();
        }
    }

    fn trigger_seasonal_migration(&mut self) {
        match self.season {
            Season::Spring => {
                for migration in &mut self.migrations {
                    if !migration.is_active {
                        migration.is_active = true;
                        migration.progress = 0.0;
                        migration.current_position = migration.source_position;
                    }
                }
            },
            Season::Autumn => {
                for migration in &mut self.migrations {
                    if migration.progress >= 1.0 {
                        std::mem::swap(&mut migration.source_position, &mut migration.destination);
                        migration.is_active = true;
                        migration.progress = 0.0;
                        migration.current_position = migration.source_position;
                    }
                }
            },
            _ => {},
        }
    }
}

impl Default for MigrationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creation() {
        let migration =
            MigrationModel::new("bird", Vec3::ZERO, Vec3::new(1000.0, 0.0, 0.0), 10.0, 0.5);
        assert!(migration.is_active);
        assert!(migration.distance_remaining() > 0.0);
    }

    #[test]
    fn test_migration_update() {
        let mut migration =
            MigrationModel::new("bird", Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 50.0, 0.5);
        migration.update(1.0);
        assert!(migration.progress > 0.0);
        assert!(migration.progress <= 1.0);
    }

    #[test]
    fn test_migration_completion() {
        let mut migration =
            MigrationModel::new("bird", Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 100.0, 0.5);
        migration.update(10.0);
        assert!(!migration.is_active);
        assert_eq!(migration.progress, 1.0);
    }

    #[test]
    fn test_dispersal_kernel() {
        let kernel = DispersalKernel::new(DispersalType::Gaussian, 100.0);
        let prob_near = kernel.probability(10.0);
        let prob_far = kernel.probability(500.0);
        assert!(prob_near > prob_far);
    }

    #[test]
    fn test_dispersal_exponential() {
        let kernel = DispersalKernel::new(DispersalType::Exponential, 50.0);
        let prob = kernel.probability(25.0);
        assert!(prob > 0.0);
        assert!(prob <= 1.0);
    }

    #[test]
    fn test_migration_system() {
        let mut system = MigrationSystem::new();
        let migration =
            MigrationModel::new("wildebeest", Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0), 5.0, 0.8);
        system.add_migration(migration);
        system.update(10.0);
        assert!(system.total_energy_cost() > 0.0);
    }

    #[test]
    fn test_season_advance() {
        let mut system = MigrationSystem::new();
        let mut migration =
            MigrationModel::new("bird", Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 5.0, 0.5);
        migration.update(100.0);
        system.add_migration(migration);
        system.advance_season(180.0);
        assert_eq!(system.season, Season::Summer);
    }

    #[test]
    fn test_barrier_avoidance() {
        let mut migration =
            MigrationModel::new("fish", Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 5.0, 0.3);
        migration.update(10.0);
        let original_dest = migration.destination;
        migration.apply_barrier(Vec3::new(50.0, 0.0, 0.0), 20.0, 10.0);
        assert!(migration.destination != original_dest);
        assert!(migration.energy_cost > 10.0);
    }
}
