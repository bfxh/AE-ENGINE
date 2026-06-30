use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicheSpace {
    pub dimensions: Vec<NicheDimension>,
    pub overlap_matrix: Vec<Vec<f32>>,
    pub species_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicheDimension {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub optimum: f32,
    pub tolerance: f32,
}

impl NicheSpace {
    pub fn new() -> Self {
        Self { dimensions: Vec::new(), overlap_matrix: Vec::new(), species_count: 0 }
    }

    pub fn add_species(&mut self, dimensions: Vec<NicheDimension>) {
        self.dimensions.extend(dimensions);
        self.species_count += 1;
        self.rebuild_overlap_matrix();
    }

    pub fn rebuild_overlap_matrix(&mut self) {
        let n = self.species_count;
        self.overlap_matrix = vec![vec![0.0; n]; n];
    }

    pub fn niche_overlap(&self, species_a: usize, species_b: usize) -> f32 {
        if species_a >= self.species_count || species_b >= self.species_count {
            return 0.0;
        }

        let dims_per_species = self.dimensions.len() / self.species_count;
        let start_a = species_a * dims_per_species;
        let start_b = species_b * dims_per_species;

        let mut total_overlap = 0.0;
        for i in 0..dims_per_species {
            let dim_a = &self.dimensions[start_a + i];
            let dim_b = &self.dimensions[start_b + i];

            let overlap = Self::dimension_overlap(dim_a, dim_b);
            total_overlap += overlap;
        }

        total_overlap / dims_per_species as f32
    }

    fn dimension_overlap(a: &NicheDimension, b: &NicheDimension) -> f32 {
        let overlap_min = a.min.max(b.min);
        let overlap_max = a.max.min(b.max);

        if overlap_min >= overlap_max {
            return 0.0;
        }

        let overlap_range = overlap_max - overlap_min;
        let a_range = a.max - a.min;
        let b_range = b.max - b.min;

        if a_range <= 0.0 || b_range <= 0.0 {
            return 0.0;
        }

        (overlap_range / a_range).min(overlap_range / b_range)
    }
}

impl Default for NicheSpace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Competition {
    pub species_a: String,
    pub species_b: String,
    pub competition_coefficient: f32,
    pub resource_competition: ResourceCompetition,
    pub interference: f32,
    pub outcome: CompetitionOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetitionOutcome {
    SpeciesAWins,
    SpeciesBWins,
    Coexistence,
    Unstable,
    Undetermined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCompetition {
    pub shared_resources: Vec<String>,
    pub resource_use_efficiency: f32,
    pub resource_partitioning: f32,
}

impl Competition {
    pub fn new(species_a: &str, species_b: &str, alpha: f32) -> Self {
        Self {
            species_a: species_a.to_string(),
            species_b: species_b.to_string(),
            competition_coefficient: alpha,
            resource_competition: ResourceCompetition {
                shared_resources: Vec::new(),
                resource_use_efficiency: 0.5,
                resource_partitioning: 0.0,
            },
            interference: 0.0,
            outcome: CompetitionOutcome::Undetermined,
        }
    }

    pub fn determine_outcome(
        &self,
        carrying_capacity_a: f32,
        carrying_capacity_b: f32,
        alpha: f32,
        beta: f32,
    ) -> CompetitionOutcome {
        let k1_k2 = carrying_capacity_a / carrying_capacity_b.max(0.001);
        let k2_k1 = carrying_capacity_b / carrying_capacity_a.max(0.001);

        if k1_k2 > alpha && k2_k1 < beta {
            CompetitionOutcome::SpeciesAWins
        } else if k1_k2 < alpha && k2_k1 > beta {
            CompetitionOutcome::SpeciesBWins
        } else if k1_k2 < alpha && k2_k1 < beta {
            CompetitionOutcome::Coexistence
        } else {
            CompetitionOutcome::Unstable
        }
    }

    pub fn competitive_exclusion(
        &self,
        population_a: &mut super::population::Population,
        population_b: &mut super::population::Population,
        dt: f32,
    ) {
        let r1 = population_a.growth_rate;
        let r2 = population_b.growth_rate;
        let k1 = population_a.carrying_capacity;
        let k2 = population_b.carrying_capacity;
        let n1 = population_a.count;
        let n2 = population_b.count;
        let alpha = self.competition_coefficient;

        let dn1 = r1 * n1 * (1.0 - (n1 + alpha * n2) / k1) * dt;
        let dn2 = r2 * n2 * (1.0 - (n2 + alpha * n1) / k2) * dt;

        population_a.count = (population_a.count + dn1).max(0.0);
        population_b.count = (population_b.count + dn2).max(0.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionNetwork {
    pub competitions: Vec<Competition>,
    pub species_ids: Vec<String>,
}

impl CompetitionNetwork {
    pub fn new() -> Self {
        Self { competitions: Vec::new(), species_ids: Vec::new() }
    }

    pub fn add_competition(&mut self, competition: Competition) {
        if !self.species_ids.contains(&competition.species_a) {
            self.species_ids.push(competition.species_a.clone());
        }
        if !self.species_ids.contains(&competition.species_b) {
            self.species_ids.push(competition.species_b.clone());
        }
        self.competitions.push(competition);
    }

    pub fn competition_pressure(&self, species_id: &str) -> f32 {
        self.competitions
            .iter()
            .filter(|c| c.species_a == species_id || c.species_b == species_id)
            .map(|c| c.competition_coefficient)
            .sum()
    }

    pub fn dominant_species(&self) -> Option<String> {
        let mut scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        for comp in &self.competitions {
            match comp.outcome {
                CompetitionOutcome::SpeciesAWins => {
                    *scores.entry(comp.species_a.clone()).or_insert(0.0) += 1.0;
                    *scores.entry(comp.species_b.clone()).or_insert(0.0) -= 1.0;
                },
                CompetitionOutcome::SpeciesBWins => {
                    *scores.entry(comp.species_b.clone()).or_insert(0.0) += 1.0;
                    *scores.entry(comp.species_a.clone()).or_insert(0.0) -= 1.0;
                },
                _ => {},
            }
        }
        scores.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(id, _)| id)
    }
}

impl Default for CompetitionNetwork {
    fn default() -> Self {
        Self::new()
    }
}
