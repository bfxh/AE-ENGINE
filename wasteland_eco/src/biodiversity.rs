use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiodiversityMetrics {
    pub species_count: usize,
    pub shannon_index: f32,
    pub simpson_index: f32,
    pub evenness: f32,
    pub margalef_richness: f32,
    pub berger_parker_dominance: f32,
    pub alpha_diversity: f32,
    pub beta_diversity: f32,
    pub gamma_diversity: f32,
}

impl BiodiversityMetrics {
    pub fn compute(species_abundances: &[f32]) -> Self {
        let total: f32 = species_abundances.iter().sum();
        if total <= 0.0 {
            return Self {
                species_count: 0,
                shannon_index: 0.0,
                simpson_index: 0.0,
                evenness: 0.0,
                margalef_richness: 0.0,
                berger_parker_dominance: 0.0,
                alpha_diversity: 0.0,
                beta_diversity: 0.0,
                gamma_diversity: 0.0,
            };
        }

        let proportions: Vec<f32> = species_abundances.iter().map(|&a| a / total).collect();
        let species_count = species_abundances.iter().filter(|&&a| a > 0.0).count();

        let shannon: f32 = proportions.iter().filter(|&&p| p > 0.0).map(|&p| -p * p.ln()).sum();

        let simpson: f32 = 1.0 - proportions.iter().map(|&p| p * p).sum::<f32>();

        let max_shannon = (species_count as f32).ln();
        let evenness = if max_shannon > 0.0 { shannon / max_shannon } else { 0.0 };

        let margalef =
            if species_count > 0 { (species_count as f32 - 1.0) / total.ln() } else { 0.0 };

        let max_p = proportions.iter().fold(0.0f32, |a, &b| a.max(b));
        let berger_parker = max_p;

        Self {
            species_count,
            shannon_index: shannon,
            simpson_index: simpson,
            evenness,
            margalef_richness: margalef,
            berger_parker_dominance: berger_parker,
            alpha_diversity: species_count as f32,
            beta_diversity: 0.0,
            gamma_diversity: species_count as f32,
        }
    }

    pub fn compute_beta_diversity(community_a: &[f32], community_b: &[f32]) -> f32 {
        let shared: usize = community_a
            .iter()
            .zip(community_b.iter())
            .filter(|(&a, &b)| a > 0.0 && b > 0.0)
            .count();

        let total_a: usize = community_a.iter().filter(|&&a| a > 0.0).count();
        let total_b: usize = community_b.iter().filter(|&&a| a > 0.0).count();
        let total_species = total_a.max(total_b);

        if total_species == 0 {
            return 0.0;
        }

        shared as f32 / total_species as f32
    }

    pub fn update(&mut self, species_abundances: &[f32]) {
        *self = Self::compute(species_abundances);
    }

    pub fn biodiversity_quality(&self) -> BiodiversityQuality {
        if self.shannon_index > 3.0 && self.evenness > 0.7 {
            BiodiversityQuality::High
        } else if self.shannon_index > 1.5 && self.evenness > 0.4 {
            BiodiversityQuality::Moderate
        } else if self.shannon_index > 0.5 {
            BiodiversityQuality::Low
        } else {
            BiodiversityQuality::Degraded
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiodiversityQuality {
    High,
    Moderate,
    Low,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesPool {
    pub species: Vec<PoolSpecies>,
    pub total_pool_size: usize,
    pub immigration_rate: f32,
    pub extinction_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSpecies {
    pub name: String,
    pub abundance: f32,
    pub niche_width: f32,
    pub dispersal_ability: f32,
    pub competitive_ability: f32,
    pub stress_tolerance: f32,
}

impl SpeciesPool {
    pub fn new() -> Self {
        Self {
            species: Vec::new(),
            total_pool_size: 0,
            immigration_rate: 0.01,
            extinction_rate: 0.005,
        }
    }

    pub fn add_species(&mut self, species: PoolSpecies) {
        self.species.push(species);
        self.total_pool_size = self.species.len();
    }

    pub fn island_biogeography_equilibrium(&self) -> f32 {
        if self.immigration_rate + self.extinction_rate <= 0.0 {
            return 0.0;
        }
        let i = self.immigration_rate;
        let e = self.extinction_rate;
        self.total_pool_size as f32 * i / (i + e)
    }

    pub fn expected_species(&self, area: f32, distance: f32) -> f32 {
        let s_max = self.total_pool_size as f32;
        let z = 0.25;
        let c = s_max / area.powf(z);
        let immigration_factor = (-0.001 * distance).exp();
        c * area.powf(z) * immigration_factor
    }

    pub fn extinction_debt(&self, habitat_loss: f32) -> f32 {
        let current = self.species.iter().filter(|s| s.abundance > 0.0).count() as f32;
        let expected = current * (1.0 - habitat_loss).powf(0.25);
        current - expected
    }
}

impl Default for SpeciesPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalDiversity {
    pub trait_matrix: Vec<Vec<f32>>,
    pub species_names: Vec<String>,
    pub trait_names: Vec<String>,
    pub functional_richness: f32,
    pub functional_evenness: f32,
    pub functional_divergence: f32,
    pub rao_quadratic_entropy: f32,
}

impl FunctionalDiversity {
    pub fn new(trait_names: Vec<String>) -> Self {
        Self {
            trait_matrix: Vec::new(),
            species_names: Vec::new(),
            trait_names,
            functional_richness: 0.0,
            functional_evenness: 0.0,
            functional_divergence: 0.0,
            rao_quadratic_entropy: 0.0,
        }
    }

    pub fn add_species(&mut self, name: &str, traits: Vec<f32>) {
        self.species_names.push(name.to_string());
        self.trait_matrix.push(traits);
    }

    pub fn compute_convex_hull_volume(&self) -> f32 {
        if self.trait_matrix.len() < 2 {
            return 0.0;
        }

        let n_traits = self.trait_names.len();
        let mut min_vals = vec![f32::MAX; n_traits];
        let mut max_vals = vec![f32::MIN; n_traits];

        for traits in &self.trait_matrix {
            for (i, &t) in traits.iter().enumerate() {
                min_vals[i] = min_vals[i].min(t);
                max_vals[i] = max_vals[i].max(t);
            }
        }

        min_vals.iter().zip(max_vals.iter()).map(|(min, max)| (max - min).max(0.0)).product()
    }

    pub fn trait_distance(&self, species_a: usize, species_b: usize) -> f32 {
        if species_a >= self.trait_matrix.len() || species_b >= self.trait_matrix.len() {
            return 0.0;
        }

        let a = &self.trait_matrix[species_a];
        let b = &self.trait_matrix[species_b];

        a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt()
    }

    pub fn compute_rao_q(&mut self, abundances: &[f32]) {
        let total: f32 = abundances.iter().sum();
        if total <= 0.0 {
            self.rao_quadratic_entropy = 0.0;
            return;
        }

        let proportions: Vec<f32> = abundances.iter().map(|&a| a / total).collect();
        let n = self.trait_matrix.len();
        let mut q = 0.0;

        for i in 0..n {
            for j in 0..n {
                let d = self.trait_distance(i, j);
                q += proportions[i] * proportions[j] * d;
            }
        }

        self.rao_quadratic_entropy = q;
    }

    pub fn functional_redundancy(&self, abundances: &[f32]) -> f32 {
        let total: f32 = abundances.iter().sum();
        if total <= 0.0 {
            return 0.0;
        }

        let n = self.trait_matrix.len();
        let mut redundant_pairs = 0;
        let mut total_pairs = 0;

        for i in 0..n {
            for j in (i + 1)..n {
                if abundances[i] > 0.0 && abundances[j] > 0.0 {
                    total_pairs += 1;
                    let dist = self.trait_distance(i, j);
                    if dist < 0.1 {
                        redundant_pairs += 1;
                    }
                }
            }
        }

        if total_pairs == 0 { 0.0 } else { redundant_pairs as f32 / total_pairs as f32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_index() {
        let abundances = vec![10.0, 20.0, 30.0, 40.0];
        let metrics = BiodiversityMetrics::compute(&abundances);
        assert!(metrics.shannon_index > 0.0);
        assert_eq!(metrics.species_count, 4);
    }

    #[test]
    fn test_simpson_index() {
        let abundances = vec![50.0, 50.0];
        let metrics = BiodiversityMetrics::compute(&abundances);
        assert!(metrics.simpson_index > 0.0);
        assert!(metrics.simpson_index <= 1.0);
    }

    #[test]
    fn test_beta_diversity() {
        let community_a = vec![10.0, 0.0, 5.0, 0.0];
        let community_b = vec![0.0, 10.0, 5.0, 0.0];
        let beta = BiodiversityMetrics::compute_beta_diversity(&community_a, &community_b);
        assert!(beta > 0.0);
    }

    #[test]
    fn test_species_pool() {
        let mut pool = SpeciesPool::new();
        pool.add_species(PoolSpecies {
            name: "Oak".to_string(),
            abundance: 100.0,
            niche_width: 0.5,
            dispersal_ability: 0.3,
            competitive_ability: 0.8,
            stress_tolerance: 0.6,
        });
        pool.add_species(PoolSpecies {
            name: "Pine".to_string(),
            abundance: 50.0,
            niche_width: 0.7,
            dispersal_ability: 0.9,
            competitive_ability: 0.4,
            stress_tolerance: 0.8,
        });
        assert_eq!(pool.total_pool_size, 2);
        let eq = pool.island_biogeography_equilibrium();
        assert!(eq > 0.0);
    }

    #[test]
    fn test_functional_diversity() {
        let mut fd =
            FunctionalDiversity::new(vec!["leaf_area".to_string(), "wood_density".to_string()]);
        fd.add_species("Oak", vec![0.8, 0.7]);
        fd.add_species("Pine", vec![0.3, 0.5]);
        fd.add_species("Birch", vec![0.6, 0.4]);

        let vol = fd.compute_convex_hull_volume();
        assert!(vol > 0.0);

        let dist = fd.trait_distance(0, 1);
        assert!(dist > 0.0);

        fd.compute_rao_q(&[10.0, 20.0, 15.0]);
        assert!(fd.rao_quadratic_entropy > 0.0);
    }

    #[test]
    fn test_biodiversity_quality() {
        let metrics = BiodiversityMetrics {
            species_count: 10,
            shannon_index: 3.5,
            simpson_index: 0.9,
            evenness: 0.8,
            margalef_richness: 2.0,
            berger_parker_dominance: 0.2,
            alpha_diversity: 10.0,
            beta_diversity: 0.5,
            gamma_diversity: 15.0,
        };
        assert_eq!(metrics.biodiversity_quality(), BiodiversityQuality::High);
    }

    #[test]
    fn test_extinction_debt() {
        let mut pool = SpeciesPool::new();
        for i in 0..5 {
            pool.add_species(PoolSpecies {
                name: format!("Species_{}", i),
                abundance: 10.0,
                niche_width: 0.5,
                dispersal_ability: 0.5,
                competitive_ability: 0.5,
                stress_tolerance: 0.5,
            });
        }
        let debt = pool.extinction_debt(0.5);
        assert!(debt > 0.0);
    }
}
