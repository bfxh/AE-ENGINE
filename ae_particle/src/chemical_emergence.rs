use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::particles::{BondType, ElementType, Particle, ParticleBond};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalEmergence {
    pub reaction_chains: Vec<ReactionChain>,
    pub activation_energy_base: f32,
    pub temperature_factor: f32,
    pub collision_energy_threshold: f32,
    pub conserved_elements: Vec<ElementType>,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionChain {
    pub id: Uuid,
    pub reactants: Vec<ElementType>,
    pub products: Vec<ElementType>,
    pub activation_energy: f32,
    pub energy_release: f32,
    pub rate: f32,
    pub catalyst: Option<ElementType>,
    pub active: bool,
    pub occurrence_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEvent {
    pub chain_id: Uuid,
    pub position: Vec3,
    pub energy_released: f32,
    pub reactants_consumed: Vec<Uuid>,
    pub products_created: Vec<ElementType>,
    pub tick: u64,
}

impl ChemicalEmergence {
    pub fn new() -> Self {
        Self {
            reaction_chains: Vec::new(),
            activation_energy_base: 10.0,
            temperature_factor: 0.008,
            collision_energy_threshold: 5.0,
            conserved_elements: vec![
                ElementType::Iron,
                ElementType::Carbon,
                ElementType::Oxygen,
                ElementType::Hydrogen,
                ElementType::Nitrogen,
                ElementType::Silicon,
            ],
            tick: 0,
        }
    }

    pub fn initialize_default_reactions(&mut self) {
        self.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen, ElementType::Oxygen],
            15.0,
            5.0,
            0.3,
            None,
        );

        self.add_reaction(
            vec![ElementType::Carbon, ElementType::Oxygen, ElementType::Oxygen],
            vec![ElementType::Carbon, ElementType::Oxygen, ElementType::Oxygen],
            20.0,
            10.0,
            0.4,
            None,
        );

        self.add_reaction(
            vec![ElementType::Hydrogen, ElementType::Hydrogen, ElementType::Oxygen],
            vec![ElementType::Hydrogen, ElementType::Oxygen],
            30.0,
            50.0,
            0.5,
            Some(ElementType::Iron),
        );

        self.add_reaction(
            vec![ElementType::Hydrogen, ElementType::Hydrogen],
            vec![ElementType::Hydrogen, ElementType::Hydrogen],
            25.0,
            8.0,
            0.2,
            None,
        );

        self.add_reaction(
            vec![
                ElementType::Nitrogen,
                ElementType::Hydrogen,
                ElementType::Hydrogen,
                ElementType::Hydrogen,
            ],
            vec![
                ElementType::Nitrogen,
                ElementType::Hydrogen,
                ElementType::Hydrogen,
                ElementType::Hydrogen,
            ],
            40.0,
            15.0,
            0.15,
            Some(ElementType::Iron),
        );
    }

    pub fn add_reaction(
        &mut self,
        reactants: Vec<ElementType>,
        products: Vec<ElementType>,
        activation_energy: f32,
        energy_release: f32,
        rate: f32,
        catalyst: Option<ElementType>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.reaction_chains.push(ReactionChain {
            id,
            reactants,
            products,
            activation_energy,
            energy_release,
            rate,
            catalyst,
            active: true,
            occurrence_count: 0,
        });
        id
    }

    pub fn step(&mut self, particles: &mut [Particle], dt: f32) -> Vec<ReactionEvent> {
        let mut events = Vec::new();
        let n = particles.len();

        let chain_indices: Vec<usize> = (0..self.reaction_chains.len()).collect();

        for i in 0..n {
            if !particles[i].active {
                continue;
            }

            for j in (i + 1)..n {
                if !particles[j].active {
                    continue;
                }

                let delta = particles[j].position - particles[i].position;
                let dist = delta.length();
                let collision_radius = particles[i].radius + particles[j].radius;

                if dist > collision_radius * 2.0 {
                    continue;
                }

                let rel_vel = (particles[j].velocity - particles[i].velocity).length();
                let collision_energy =
                    0.5 * (particles[i].mass.min(particles[j].mass)) * rel_vel * rel_vel;

                let avg_temp = (particles[i].temperature + particles[j].temperature) / 2.0;
                let thermal_energy = avg_temp * self.temperature_factor;

                for &ci in &chain_indices {
                    let (
                        chain_active,
                        chain_reactants,
                        chain_activation_energy,
                        chain_catalyst,
                        chain_rate,
                        chain_energy_release,
                        chain_id,
                    ) = {
                        let chain = &self.reaction_chains[ci];
                        (
                            chain.active,
                            chain.reactants.clone(),
                            chain.activation_energy,
                            chain.catalyst,
                            chain.rate,
                            chain.energy_release,
                            chain.id,
                        )
                    };

                    if !chain_active {
                        continue;
                    }

                    let ei = particles[i].element_type;
                    let ej = particles[j].element_type;
                    let has_reactants =
                        chain_reactants.contains(&ei) && chain_reactants.contains(&ej);

                    if !has_reactants {
                        continue;
                    }

                    let effective_activation =
                        chain_activation_energy * if chain_catalyst.is_some() { 0.5 } else { 1.0 };

                    let total_energy = collision_energy + thermal_energy;
                    if total_energy < effective_activation {
                        continue;
                    }

                    let reaction_prob =
                        (chain_rate * dt * (total_energy / effective_activation)).min(1.0);

                    if rand::random::<f32>() > reaction_prob {
                        continue;
                    }

                    let avg_pos = (particles[i].position + particles[j].position) / 2.0;

                    particles[i].temperature += chain_energy_release * 0.1;
                    particles[j].temperature += chain_energy_release * 0.1;

                    self.reaction_chains[ci].occurrence_count += 1;

                    let mut bond = ParticleBond {
                        target_id: particles[j].id,
                        bond_type: BondType::Covalent,
                        strength: chain_energy_release * 0.5,
                        equilibrium_distance: collision_radius,
                        max_distance: collision_radius * 2.0,
                    };
                    particles[i].bonds.push(bond.clone());
                    bond.target_id = particles[i].id;
                    particles[j].bonds.push(bond);

                    events.push(ReactionEvent {
                        chain_id,
                        position: avg_pos,
                        energy_released: chain_energy_release,
                        reactants_consumed: vec![particles[i].id, particles[j].id],
                        products_created: chain_reactants.clone(),
                        tick: self.tick,
                    });
                }
            }
        }

        self.prune_bonds(particles);

        self.tick += 1;
        events
    }

    fn prune_bonds(&self, particles: &mut [Particle]) {
        let max_bonds = 6;
        for p in particles.iter_mut() {
            if p.bonds.len() > max_bonds {
                p.bonds.sort_by(|a, b| {
                    b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal)
                });
                p.bonds.truncate(max_bonds);
            }
        }
    }

    pub fn discover_reactions(&mut self, particles: &[Particle]) -> Vec<ReactionChain> {
        let mut new_chains = Vec::new();

        let mut element_counts: std::collections::HashMap<ElementType, usize> =
            std::collections::HashMap::new();
        let mut pair_counts: std::collections::HashMap<(ElementType, ElementType), usize> =
            std::collections::HashMap::new();

        for p in particles.iter().filter(|p| p.active) {
            *element_counts.entry(p.element_type).or_insert(0) += 1;
        }

        for p in particles.iter().filter(|p| p.active) {
            for bond in &p.bonds {
                if let Some(target) = particles.iter().find(|t| t.id == bond.target_id) {
                    let key = (
                        p.element_type.min(target.element_type),
                        p.element_type.max(target.element_type),
                    );
                    *pair_counts.entry(key).or_insert(0) += 1;
                }
            }
        }

        for ((ea, eb), count) in &pair_counts {
            if *count < 10 {
                continue;
            }

            let ea_count = *element_counts.get(ea).unwrap_or(&0);
            let eb_count = *element_counts.get(eb).unwrap_or(&0);
            let total = ea_count + eb_count;

            if total == 0 {
                continue;
            }

            let bond_prob = *count as f32 / total as f32;

            if bond_prob > 0.3 {
                let activation = 20.0 / bond_prob;
                let energy = 10.0 * bond_prob;
                let chain = ReactionChain {
                    id: Uuid::new_v4(),
                    reactants: vec![*ea, *eb],
                    products: vec![*ea, *eb],
                    activation_energy: activation,
                    energy_release: energy,
                    rate: bond_prob,
                    catalyst: None,
                    active: true,
                    occurrence_count: 0,
                };

                if !self
                    .reaction_chains
                    .iter()
                    .any(|c| c.reactants == chain.reactants && c.products == chain.products)
                {
                    new_chains.push(chain.clone());
                    self.reaction_chains.push(chain);
                }
            }
        }

        new_chains
    }

    pub fn get_active_reactions(&self) -> Vec<&ReactionChain> {
        self.reaction_chains.iter().filter(|c| c.active && c.occurrence_count > 0).collect()
    }

    pub fn total_energy_released(&self) -> f32 {
        self.reaction_chains.iter().map(|c| c.energy_release * c.occurrence_count as f32).sum()
    }

    pub fn reaction_heatmap(&self) -> Vec<(Vec3, f32)> {
        Vec::new()
    }
}

impl Default for ChemicalEmergence {
    fn default() -> Self {
        let mut ce = Self::new();
        ce.initialize_default_reactions();
        ce
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Phase;
    use glam::Vec3;

    #[test]
    fn test_new() {
        let ce = ChemicalEmergence::new();
        assert!(ce.reaction_chains.is_empty());
        assert!((ce.activation_energy_base - 10.0).abs() < 0.001);
        assert!((ce.temperature_factor - 0.008).abs() < 0.001);
        assert!((ce.collision_energy_threshold - 5.0).abs() < 0.001);
        assert_eq!(ce.conserved_elements.len(), 6);
        assert_eq!(ce.tick, 0);
    }

    #[test]
    fn test_add_reaction() {
        let mut ce = ChemicalEmergence::new();
        let id = ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            0.5,
            None,
        );
        assert_eq!(ce.reaction_chains.len(), 1);
        assert_eq!(ce.reaction_chains[0].id, id);
        assert!(ce.reaction_chains[0].active);
    }

    #[test]
    fn test_initialize_default_reactions() {
        let mut ce = ChemicalEmergence::new();
        ce.initialize_default_reactions();
        assert_eq!(ce.reaction_chains.len(), 5);
    }

    #[test]
    fn test_step_no_reaction_when_apart() {
        let mut ce = ChemicalEmergence::new();
        ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            1.0,
            None,
        );
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Oxygen, Vec3::new(100.0, 0.0, 0.0), Phase::Solid),
        ];
        let events = ce.step(&mut particles, 0.016);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_step_no_reaction_wrong_elements() {
        let mut ce = ChemicalEmergence::new();
        ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            1.0,
            None,
        );
        let mut particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Hydrogen, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
        ];
        let events = ce.step(&mut particles, 0.016);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_get_active_reactions() {
        let mut ce = ChemicalEmergence::new();
        ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            0.5,
            None,
        );
        assert!(ce.get_active_reactions().is_empty());
        ce.reaction_chains[0].occurrence_count = 1;
        assert_eq!(ce.get_active_reactions().len(), 1);
    }

    #[test]
    fn test_total_energy_released() {
        let mut ce = ChemicalEmergence::new();
        ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            0.5,
            None,
        );
        ce.reaction_chains[0].occurrence_count = 3;
        assert!((ce.total_energy_released() - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_discover_reactions_no_bonds() {
        let mut ce = ChemicalEmergence::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        let new_chains = ce.discover_reactions(&particles);
        assert!(new_chains.is_empty());
    }

    #[test]
    fn test_step_inactive_particles_ignored() {
        let mut ce = ChemicalEmergence::new();
        ce.add_reaction(
            vec![ElementType::Iron, ElementType::Oxygen],
            vec![ElementType::Iron, ElementType::Oxygen],
            10.0,
            5.0,
            1.0,
            None,
        );
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Oxygen, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
        ];
        particles[0].active = false;
        let events = ce.step(&mut particles, 0.016);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_reaction_heatmap_empty() {
        let ce = ChemicalEmergence::new();
        let heatmap = ce.reaction_heatmap();
        assert!(heatmap.is_empty());
    }
}
