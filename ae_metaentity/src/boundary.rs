use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use ae_physics::fixed_point::FixedPoint;
use ae_weave::constraint::{ConstraintInput, ConstraintType};
use ae_weave::network::ConstraintNetwork;

use crate::meta_entity::{
    ChemistryAttributes, MetaEntity, MetaEntityState, PhysicsAttributes, StructuralFieldParams,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SplitConfig {
    pub stress_ratio_threshold: f32,
    pub min_child_mass_fraction: f32,
    pub max_children: usize,
    pub crack_propagation_depth: u32,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            stress_ratio_threshold: 1.2,
            min_child_mass_fraction: 0.1,
            max_children: 4,
            crack_propagation_depth: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MergeConfig {
    pub max_distance: f32,
    pub temperature_ratio_to_melting: f32,
    pub chemical_compatibility_threshold: f32,
    pub max_merged_volume_ratio: f32,
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            max_distance: 0.1,
            temperature_ratio_to_melting: 0.8,
            chemical_compatibility_threshold: 0.7,
            max_merged_volume_ratio: 3.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundaryManager {
    pub split_config: SplitConfig,
    pub merge_config: MergeConfig,
}

impl BoundaryManager {
    pub fn new(split_config: SplitConfig, merge_config: MergeConfig) -> Self {
        Self { split_config, merge_config }
    }

    pub fn should_split(&self, entity: &MetaEntity) -> Option<SplitDecision> {
        let stress = entity.current_stress();
        let ultimate = entity.physics.ultimate_strength;

        if stress <= ultimate * self.split_config.stress_ratio_threshold {
            return None;
        }

        let excess_ratio = (stress - ultimate) / ultimate.max(0.001);

        let crack_depth = if let Some(sf) = &entity.structural_field {
            (sf.depth_in_hierarchy + 1).min(self.split_config.crack_propagation_depth)
        } else {
            1
        };

        let num_children =
            ((excess_ratio * 2.0) as usize + 1).clamp(2, self.split_config.max_children);

        let total_mass = entity.physics.mass;
        let min_child_mass = total_mass * self.split_config.min_child_mass_fraction;
        if total_mass / (num_children as f32) < min_child_mass {
            return None;
        }

        let child_masses = self.distribute_masses(total_mass, num_children, excess_ratio);

        let split_axis = entity.velocity.normalize_or_zero();
        let split_axis = if split_axis.length_squared() < 0.001 { Vec3::X } else { split_axis };

        let split_plane_offset = (excess_ratio - 1.0).clamp(-0.4, 0.4);

        let fracture_energy = entity.physics.toughness
            * entity.physics.mass
            * excess_ratio
            * crack_depth as f32
            * 0.01;

        Some(SplitDecision {
            parent_id: entity.id,
            num_children,
            child_masses,
            split_axis,
            split_plane_offset,
            excess_ratio,
            fracture_energy,
            crack_depth,
        })
    }

    fn distribute_masses(
        &self,
        total_mass: f32,
        num_children: usize,
        excess_ratio: f32,
    ) -> Vec<f32> {
        let mut masses = Vec::with_capacity(num_children);
        let base = total_mass / num_children as f32;
        let asymmetry = (excess_ratio - 1.0).clamp(0.0, 0.5);

        for i in 0..num_children {
            let t = i as f32 / (num_children - 1).max(1) as f32;
            let bias = (t - 0.5) * 2.0 * asymmetry;
            masses.push(base * (1.0 + bias));
        }

        let sum: f32 = masses.iter().sum();
        let scale = total_mass / sum;
        for m in &mut masses {
            *m *= scale;
        }
        masses
    }

    pub fn execute_split(
        &mut self,
        entity: &mut MetaEntity,
        decision: &SplitDecision,
        tick: u64,
        network: Option<&mut ConstraintNetwork>,
    ) -> Vec<MetaEntity> {
        let spacing = 0.05;
        let mut children = Vec::with_capacity(decision.num_children);

        let perp = if decision.split_axis.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let secondary_axis = decision.split_axis.cross(perp).normalize();

        for i in 0..decision.num_children {
            let t = i as f32 / (decision.num_children - 1).max(1) as f32;
            let primary_offset =
                (t - 0.5 + decision.split_plane_offset) * decision.num_children as f32 * spacing;
            let child_pos = entity.position
                + decision.split_axis * primary_offset
                + secondary_axis * (i as f32 % 2.0 - 0.5) * spacing * 0.5;

            let mut child_physics = entity.physics;
            child_physics.mass = decision.child_masses[i];
            child_physics.temperature += decision.fracture_energy
                / (child_physics.mass * child_physics.specific_heat_capacity.max(1.0));

            let mut child = MetaEntity::new(child_pos, child_physics, tick);
            child.parent_id = Some(entity.id);
            child.chemistry = entity.chemistry.clone();
            child.biology = entity.biology.clone();
            child.velocity = entity.velocity
                + decision.split_axis * (t - 0.5) * 2.0
                + secondary_axis * (i as f32 % 2.0 - 0.5) * 1.0;
            child.angular_velocity = entity.angular_velocity + secondary_axis * (t - 0.5) * 3.0;

            let residual_stress = decision.excess_ratio * 0.3 * entity.physics.ultimate_strength;
            child.structural_field = Some(StructuralFieldParams {
                depth_in_hierarchy: decision.crack_depth,
                current_stress: residual_stress,
                max_stress_before_yield: entity.physics.yield_strength,
                max_stress_before_fracture: entity.physics.ultimate_strength,
                ..Default::default()
            });

            child.state = MetaEntityState::Active;
            children.push(child);
        }

        if let Some(network) = network {
            self.update_network_for_split(entity, &children, decision, network);
        }

        entity.state = MetaEntityState::Destroyed;
        entity.children = children.iter().map(|c| c.id).collect();

        children
    }

    fn update_network_for_split(
        &self,
        parent: &MetaEntity,
        children: &[MetaEntity],
        decision: &SplitDecision,
        network: &mut ConstraintNetwork,
    ) {
        let child_nodes: Vec<_> = children
            .iter()
            .map(|c| {
                let pos = [
                    FixedPoint::from_f32(c.position.x),
                    FixedPoint::from_f32(c.position.y),
                    FixedPoint::from_f32(c.position.z),
                ];
                let inv_mass = if c.physics.mass > 0.0 {
                    FixedPoint::from_f32(1.0 / c.physics.mass)
                } else {
                    FixedPoint::ZERO
                };
                let radius =
                    FixedPoint::from_f32((c.physics.mass / c.physics.density).cbrt() * 0.5);
                network.add_node(pos, inv_mass, radius)
            })
            .collect();

        let stiffness = FixedPoint::from_f32(
            parent.physics.elastic_modulus * 1e-9 * decision.crack_depth as f32,
        );
        let compliance = FixedPoint::from_f32(0.01 / decision.crack_depth.max(1) as f32);

        for i in 0..child_nodes.len() {
            for j in (i + 1)..child_nodes.len() {
                let dist = children[i].distance_to(&children[j]);
                network.add_edge(ConstraintInput {
                    node_a: child_nodes[i],
                    node_b: child_nodes[j],
                    constraint_type: ConstraintType::Variable,
                    rest_length: FixedPoint::from_f32(dist),
                    stiffness,
                    compliance,
                    group: ae_weave::constraint::ConstraintGroup::Structural,
                    surface_params: None,
                });
            }
        }
    }

    pub fn should_merge(
        &self,
        entity_a: &MetaEntity,
        entity_b: &MetaEntity,
    ) -> Option<MergeDecision> {
        if !entity_a.is_active() || !entity_b.is_active() {
            return None;
        }

        if entity_a.id == entity_b.id {
            return None;
        }

        let distance = entity_a.distance_to(entity_b);
        if distance > self.merge_config.max_distance {
            return None;
        }

        let melt_temp_a = self.estimate_melting_point(entity_a);
        let melt_temp_b = self.estimate_melting_point(entity_b);

        let effective_melt = melt_temp_a.min(melt_temp_b);
        let ratio_a = entity_a.physics.temperature / melt_temp_a.max(1.0);
        let ratio_b = entity_b.physics.temperature / melt_temp_b.max(1.0);
        let min_ratio = ratio_a.min(ratio_b);

        if min_ratio < self.merge_config.temperature_ratio_to_melting {
            return None;
        }

        let compatibility = self.chemical_compatibility(entity_a, entity_b);
        if compatibility < self.merge_config.chemical_compatibility_threshold {
            return None;
        }

        let merged_mass = entity_a.physics.mass + entity_b.physics.mass;
        let max_mass = entity_a.physics.mass.max(entity_b.physics.mass)
            * self.merge_config.max_merged_volume_ratio;
        if merged_mass > max_mass {
            return None;
        }

        let latent_heat_required = merged_mass
            * entity_a.physics.specific_heat_capacity.max(entity_b.physics.specific_heat_capacity)
            * 250.0
            * (1.0 - compatibility);

        let phase_transition_energy =
            merged_mass * 0.01 * (1.0 - min_ratio / self.merge_config.temperature_ratio_to_melting);

        Some(MergeDecision {
            entity_a_id: entity_a.id,
            entity_b_id: entity_b.id,
            merged_position: (entity_a.position + entity_b.position) * 0.5,
            merged_mass,
            compatibility,
            melt_temperature: effective_melt,
            latent_heat_required,
            phase_transition_energy,
        })
    }

    fn estimate_melting_point(&self, entity: &MetaEntity) -> f32 {
        if entity.physics.density > 7000.0 {
            1800.0
        } else if entity.physics.density > 2000.0 {
            1500.0
        } else if entity.physics.hardness < 0.5 {
            273.0
        } else {
            800.0
        }
    }

    fn chemical_compatibility(&self, a: &MetaEntity, b: &MetaEntity) -> f32 {
        if a.chemistry.elemental_composition.is_empty()
            || b.chemistry.elemental_composition.is_empty()
        {
            return 1.0;
        }

        let a_elements: Vec<_> =
            a.chemistry.elemental_composition.iter().map(|ef| ef.element).collect();

        let mut common = 0;
        for ef in &b.chemistry.elemental_composition {
            if a_elements.contains(&ef.element) {
                common += 1;
            }
        }

        let total = a
            .chemistry
            .elemental_composition
            .len()
            .max(b.chemistry.elemental_composition.len())
            .max(1);

        common as f32 / total as f32
    }

    pub fn execute_merge(
        &mut self,
        entity_a: &mut MetaEntity,
        entity_b: &mut MetaEntity,
        decision: &MergeDecision,
        tick: u64,
        network: Option<&mut ConstraintNetwork>,
    ) -> MetaEntity {
        let mut merged_physics = self.average_physics(entity_a, entity_b, decision.merged_mass);

        let heat_deficit = decision.latent_heat_required
            / (decision.merged_mass * merged_physics.specific_heat_capacity.max(1.0));
        merged_physics.temperature = (merged_physics.temperature - heat_deficit).max(0.0);

        let transition_heat = decision.phase_transition_energy
            / (decision.merged_mass * merged_physics.specific_heat_capacity.max(1.0));
        merged_physics.temperature = (merged_physics.temperature + transition_heat).max(0.0);

        let mut merged = MetaEntity::new(decision.merged_position, merged_physics, tick);
        merged.chemistry = self.merge_chemistry(&entity_a.chemistry, &entity_b.chemistry);
        merged.biology = self.merge_biology(&entity_a.biology, &entity_b.biology);
        merged.velocity = (entity_a.velocity * entity_a.physics.mass
            + entity_b.velocity * entity_b.physics.mass)
            / decision.merged_mass;
        merged.angular_velocity = entity_a.angular_velocity.lerp(entity_b.angular_velocity, 0.5);
        merged.parent_id = entity_a.parent_id.or(entity_b.parent_id);

        merged.structural_field = Some(StructuralFieldParams {
            depth_in_hierarchy: entity_a
                .structural_field
                .as_ref()
                .map(|sf| sf.depth_in_hierarchy)
                .unwrap_or(0)
                .max(
                    entity_b.structural_field.as_ref().map(|sf| sf.depth_in_hierarchy).unwrap_or(0),
                ),
            ..Default::default()
        });

        merged.state = MetaEntityState::Active;

        if let Some(network) = network {
            self.update_network_for_merge(&merged, entity_a, entity_b, network);
        }

        entity_a.state = MetaEntityState::Destroyed;
        entity_b.state = MetaEntityState::Destroyed;

        merged
    }

    fn average_physics(
        &self,
        a: &MetaEntity,
        b: &MetaEntity,
        merged_mass: f32,
    ) -> PhysicsAttributes {
        let wa = a.physics.mass / merged_mass;
        let wb = b.physics.mass / merged_mass;

        PhysicsAttributes {
            mass: merged_mass,
            density: a.physics.density * wa + b.physics.density * wb,
            hardness: a.physics.hardness * wa + b.physics.hardness * wb,
            toughness: a.physics.toughness * wa + b.physics.toughness * wb,
            elastic_modulus: a.physics.elastic_modulus * wa + b.physics.elastic_modulus * wb,
            yield_strength: a.physics.yield_strength * wa + b.physics.yield_strength * wb,
            ultimate_strength: a.physics.ultimate_strength * wa + b.physics.ultimate_strength * wb,
            poisson_ratio: a.physics.poisson_ratio * wa + b.physics.poisson_ratio * wb,
            friction_coefficient: a.physics.friction_coefficient * wa
                + b.physics.friction_coefficient * wb,
            restitution: a.physics.restitution * wa + b.physics.restitution * wb,
            temperature: (a.physics.temperature * a.physics.mass
                + b.physics.temperature * b.physics.mass)
                / merged_mass,
            thermal_conductivity: a.physics.thermal_conductivity * wa
                + b.physics.thermal_conductivity * wb,
            specific_heat_capacity: a.physics.specific_heat_capacity * wa
                + b.physics.specific_heat_capacity * wb,
            electrical_conductivity: a.physics.electrical_conductivity * wa
                + b.physics.electrical_conductivity * wb,
            magnetic_permeability: a.physics.magnetic_permeability * wa
                + b.physics.magnetic_permeability * wb,
        }
    }

    fn merge_chemistry(
        &self,
        a: &ChemistryAttributes,
        b: &ChemistryAttributes,
    ) -> ChemistryAttributes {
        let combined_elements = {
            let mut elements = a.elemental_composition.clone();
            for ef in &b.elemental_composition {
                if let Some(existing) = elements.iter_mut().find(|e| e.element == ef.element) {
                    existing.fraction = (existing.fraction + ef.fraction) * 0.5;
                } else {
                    elements.push(*ef);
                }
            }
            elements
        };

        let bond_types = {
            let mut bonds = a.bond_types.clone();
            for bt in &b.bond_types {
                if !bonds.contains(bt) {
                    bonds.push(*bt);
                }
            }
            bonds
        };

        ChemistryAttributes {
            elemental_composition: combined_elements,
            bond_types,
            reactivity: (a.reactivity + b.reactivity) * 0.5,
            ph: (a.ph + b.ph) * 0.5,
            redox_potential: (a.redox_potential + b.redox_potential) * 0.5,
            oxidation_state: (a.oxidation_state + b.oxidation_state) * 0.5,
            corrosion_depth: a.corrosion_depth.min(b.corrosion_depth),
            chemical_stain: a.chemical_stain.max(b.chemical_stain),
            solubility: (a.solubility + b.solubility) * 0.5,
            flammability: (a.flammability + b.flammability) * 0.5,
            toxicity: (a.toxicity + b.toxicity) * 0.5,
        }
    }

    fn merge_biology(
        &self,
        a: &crate::meta_entity::BiologyAttributes,
        b: &crate::meta_entity::BiologyAttributes,
    ) -> crate::meta_entity::BiologyAttributes {
        let gene_tokens = {
            let mut genes = a.gene_tokens.clone();
            for gt in &b.gene_tokens {
                if let Some(existing) = genes.iter_mut().find(|g| g.name == gt.name) {
                    existing.expression_level =
                        (existing.expression_level + gt.expression_level) * 0.5;
                } else {
                    genes.push(gt.clone());
                }
            }
            genes
        };

        crate::meta_entity::BiologyAttributes {
            gene_tokens,
            metabolic_rate: (a.metabolic_rate + b.metabolic_rate) * 0.5,
            growth_rate: (a.growth_rate + b.growth_rate) * 0.5,
            repair_rate: (a.repair_rate + b.repair_rate) * 0.5,
            neural_signal_strength: (a.neural_signal_strength + b.neural_signal_strength) * 0.5,
            health: (a.health + b.health) * 0.5,
            max_health: a.max_health.max(b.max_health),
            radiation_dose: (a.radiation_dose + b.radiation_dose) * 0.5,
            toxin_level: (a.toxin_level + b.toxin_level) * 0.5,
            nutrient_level: (a.nutrient_level + b.nutrient_level) * 0.5,
            hydration: (a.hydration + b.hydration) * 0.5,
            cell_type: a.cell_type,
            tissue_density: (a.tissue_density + b.tissue_density) * 0.5,
        }
    }

    fn update_network_for_merge(
        &self,
        merged: &MetaEntity,
        entity_a: &MetaEntity,
        entity_b: &MetaEntity,
        network: &mut ConstraintNetwork,
    ) {
        let pos = [
            FixedPoint::from_f32(merged.position.x),
            FixedPoint::from_f32(merged.position.y),
            FixedPoint::from_f32(merged.position.z),
        ];
        let inv_mass = if merged.physics.mass > 0.0 {
            FixedPoint::from_f32(1.0 / merged.physics.mass)
        } else {
            FixedPoint::ZERO
        };
        let radius =
            FixedPoint::from_f32((merged.physics.mass / merged.physics.density).cbrt() * 0.5);
        let merged_node = network.add_node(pos, inv_mass, radius);

        let a_pos = [
            FixedPoint::from_f32(entity_a.position.x),
            FixedPoint::from_f32(entity_a.position.y),
            FixedPoint::from_f32(entity_a.position.z),
        ];
        let b_pos = [
            FixedPoint::from_f32(entity_b.position.x),
            FixedPoint::from_f32(entity_b.position.y),
            FixedPoint::from_f32(entity_b.position.z),
        ];

        let all_nodes: Vec<_> = network.nodes.iter().map(|(id, _)| id).collect();
        for node_id in all_nodes {
            if node_id == merged_node {
                continue;
            }
            if let Some(node) = network.nodes.get(node_id) {
                let dist_a = ((node.position[0] - a_pos[0]).to_f32().powi(2)
                    + (node.position[1] - a_pos[1]).to_f32().powi(2)
                    + (node.position[2] - a_pos[2]).to_f32().powi(2))
                .sqrt();
                let dist_b = ((node.position[0] - b_pos[0]).to_f32().powi(2)
                    + (node.position[1] - b_pos[1]).to_f32().powi(2)
                    + (node.position[2] - b_pos[2]).to_f32().powi(2))
                .sqrt();
                let min_dist = dist_a.min(dist_b);

                if min_dist < 1.0 {
                    network.add_edge(ConstraintInput {
                        node_a: merged_node,
                        node_b: node_id,
                        constraint_type: ConstraintType::Elastic,
                        rest_length: FixedPoint::from_f32(min_dist),
                        stiffness: FixedPoint::from_f32(merged.physics.elastic_modulus * 1e-9),
                        compliance: FixedPoint::from_f32(0.01),
                        group: ae_weave::constraint::ConstraintGroup::Structural,
                        surface_params: None,
                    });
                }
            }
        }
    }
}

impl Default for BoundaryManager {
    fn default() -> Self {
        Self::new(SplitConfig::default(), MergeConfig::default())
    }
}

#[derive(Debug, Clone)]
pub struct SplitDecision {
    pub parent_id: Uuid,
    pub num_children: usize,
    pub child_masses: Vec<f32>,
    pub split_axis: Vec3,
    pub split_plane_offset: f32,
    pub excess_ratio: f32,
    pub fracture_energy: f32,
    pub crack_depth: u32,
}

#[derive(Debug, Clone)]
pub struct MergeDecision {
    pub entity_a_id: Uuid,
    pub entity_b_id: Uuid,
    pub merged_position: Vec3,
    pub merged_mass: f32,
    pub compatibility: f32,
    pub melt_temperature: f32,
    pub latent_heat_required: f32,
    pub phase_transition_energy: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_entity::*;

    #[test]
    fn test_no_split_below_threshold() {
        let bm = BoundaryManager::default();
        let entity = MetaEntity::iron(Vec3::ZERO, 0);
        assert!(bm.should_split(&entity).is_none());
    }

    #[test]
    fn test_split_under_high_stress() {
        let bm = BoundaryManager::default();
        let mut entity = MetaEntity::iron(Vec3::ZERO, 0);
        entity.velocity = Vec3::X;
        entity.structural_field = Some(StructuralFieldParams {
            current_stress: entity.physics.ultimate_strength * 2.0,
            depth_in_hierarchy: 1,
            max_stress_before_yield: entity.physics.yield_strength,
            max_stress_before_fracture: entity.physics.ultimate_strength,
            ..Default::default()
        });
        let decision = bm.should_split(&entity);
        assert!(decision.is_some());
        let d = decision.unwrap();
        assert!(d.num_children >= 2);
        assert!(!d.child_masses.is_empty());
        assert!(d.fracture_energy > 0.0);
        assert!(d.crack_depth >= 1);
    }

    #[test]
    fn test_should_merge_same_entity() {
        let bm = BoundaryManager::default();
        let a = MetaEntity::iron(Vec3::ZERO, 0);
        let b = MetaEntity::iron(Vec3::ZERO, 0);
        assert!(bm.should_merge(&a, &b).is_none());
    }

    #[test]
    fn test_no_merge_too_far() {
        let bm = BoundaryManager::default();
        let a = MetaEntity::iron(Vec3::ZERO, 0);
        let b = MetaEntity::iron(Vec3::new(10.0, 0.0, 0.0), 0);
        assert!(bm.should_merge(&a, &b).is_none());
    }

    #[test]
    fn test_no_merge_cold() {
        let mut bm = BoundaryManager::default();
        bm.merge_config.temperature_ratio_to_melting = 0.8;
        let a = MetaEntity::iron(Vec3::ZERO, 0);
        let b = MetaEntity::iron(Vec3::new(0.05, 0.0, 0.0), 0);
        assert!(bm.should_merge(&a, &b).is_none());
    }

    #[test]
    fn test_merge_hot_iron() {
        let bm = BoundaryManager::default();
        let mut a = MetaEntity::iron(Vec3::ZERO, 0);
        a.physics.temperature = 2000.0;
        let mut b = MetaEntity::iron(Vec3::new(0.05, 0.0, 0.0), 0);
        b.physics.temperature = 2000.0;
        let decision = bm.should_merge(&a, &b);
        assert!(decision.is_some());
        let d = decision.unwrap();
        assert!(d.latent_heat_required >= 0.0);
        assert!(d.compatibility > 0.9);
    }

    #[test]
    fn test_execute_split_creates_children() {
        let mut bm = BoundaryManager::default();
        let child_masses = vec![3000.0, 3000.0, 3000.0];
        let decision = SplitDecision {
            parent_id: Uuid::new_v4(),
            num_children: 3,
            child_masses,
            split_axis: Vec3::X,
            split_plane_offset: 0.0,
            excess_ratio: 0.5,
            fracture_energy: 100.0,
            crack_depth: 2,
        };
        let mut parent = MetaEntity::iron(Vec3::ZERO, 0);
        let children = bm.execute_split(&mut parent, &decision, 1, None);
        assert_eq!(children.len(), 3);
        assert!(parent.is_destroyed());
        assert_eq!(parent.children.len(), 3);
    }

    #[test]
    fn test_execute_merge_combines() {
        let mut bm = BoundaryManager::default();
        let mut a = MetaEntity::iron(Vec3::ZERO, 0);
        let mut b = MetaEntity::iron(Vec3::new(0.05, 0.0, 0.0), 0);

        let decision = MergeDecision {
            entity_a_id: a.id,
            entity_b_id: b.id,
            merged_position: Vec3::new(0.025, 0.0, 0.0),
            merged_mass: a.physics.mass + b.physics.mass,
            compatibility: 1.0,
            melt_temperature: 1800.0,
            latent_heat_required: 5000.0,
            phase_transition_energy: 100.0,
        };

        let merged = bm.execute_merge(&mut a, &mut b, &decision, 1, None);
        assert!(a.is_destroyed());
        assert!(b.is_destroyed());
        assert!(merged.is_active());
        assert_eq!(merged.physics.mass, a.physics.mass + b.physics.mass);
    }

    #[test]
    fn test_split_preserves_chemistry() {
        let mut bm = BoundaryManager::default();
        let child_masses = vec![4000.0, 4000.0];
        let decision = SplitDecision {
            parent_id: Uuid::new_v4(),
            num_children: 2,
            child_masses,
            split_axis: Vec3::X,
            split_plane_offset: 0.0,
            excess_ratio: 0.3,
            fracture_energy: 50.0,
            crack_depth: 1,
        };
        let mut parent = MetaEntity::iron(Vec3::ZERO, 0);
        let children = bm.execute_split(&mut parent, &decision, 1, None);
        assert_eq!(children.len(), 2);
        for child in &children {
            assert!(!child.chemistry.elemental_composition.is_empty());
            assert_eq!(child.chemistry.elemental_composition[0].element, Element::Fe);
        }
    }

    #[test]
    fn test_merge_chemistry_combines() {
        let mut bm = BoundaryManager::default();
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        iron.physics.temperature = 2000.0;
        let mut water = MetaEntity::water(Vec3::new(0.05, 0.0, 0.0), 0);
        water.physics.temperature = 2000.0;

        let decision = MergeDecision {
            entity_a_id: iron.id,
            entity_b_id: water.id,
            merged_position: Vec3::new(0.025, 0.0, 0.0),
            merged_mass: iron.physics.mass + water.physics.mass,
            compatibility: 0.5,
            melt_temperature: 1800.0,
            latent_heat_required: 10000.0,
            phase_transition_energy: 200.0,
        };

        let merged = bm.execute_merge(&mut iron, &mut water, &decision, 1, None);
        assert!(merged.is_active());
        assert!(merged.chemistry.elemental_composition.len() >= 2);
    }

    #[test]
    fn test_distribute_masses() {
        let bm = BoundaryManager::default();
        let masses = bm.distribute_masses(100.0, 3, 0.3);
        assert_eq!(masses.len(), 3);
        let sum: f32 = masses.iter().sum();
        assert!((sum - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_chemical_compatibility() {
        let bm = BoundaryManager::default();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let water = MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0);
        let compat = bm.chemical_compatibility(&iron, &water);
        assert!((0.0..=1.0).contains(&compat));
    }
}
