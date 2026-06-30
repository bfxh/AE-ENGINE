use ae_physics::fixed_point::FixedPoint;

use crate::constraint::{ConstraintEdge, ConstraintType};
use crate::network::ConstraintNetwork;

#[derive(Debug, Clone)]
pub struct FractureConfig {
    pub stress_threshold: FixedPoint,
    pub damage_accumulation_rate: FixedPoint,
    pub healing_rate: FixedPoint,
    pub min_compliance: FixedPoint,
    pub max_compliance: FixedPoint,
    pub crack_propagation_probability: FixedPoint,
}

impl Default for FractureConfig {
    fn default() -> Self {
        Self {
            stress_threshold: FixedPoint::from_f32(10.0),
            damage_accumulation_rate: FixedPoint::from_f32(0.01),
            healing_rate: FixedPoint::from_f32(0.0),
            min_compliance: FixedPoint::from_f32(0.0001),
            max_compliance: FixedPoint::from_f32(1.0),
            crack_propagation_probability: FixedPoint::from_f32(0.3),
        }
    }
}

pub struct FractureSystem {
    config: FractureConfig,
}

impl FractureSystem {
    pub fn new(config: FractureConfig) -> Self {
        Self { config }
    }

    pub fn evaluate_stress(&self, edge: &ConstraintEdge, current_length: FixedPoint) -> FixedPoint {
        let strain =
            (current_length - edge.rest_length).abs() / edge.rest_length.max(FixedPoint::EPSILON);
        strain * edge.stiffness
    }

    pub fn process_fracture(
        &mut self,
        network: &mut ConstraintNetwork,
        rng: &mut impl rand::Rng,
    ) -> Vec<crate::constraint::ConstraintId> {
        let mut fractured = Vec::new();

        let edge_ids: Vec<_> = network.edges.keys().collect();
        for edge_id in edge_ids {
            let edge = &network.edges[edge_id];
            if !edge.active || edge.constraint_type == ConstraintType::Free {
                continue;
            }

            let (na, nb) = (edge.node_a, edge.node_b);
            let (node_a, node_b) = match network.nodes.get(na).zip(network.nodes.get(nb)) {
                Some(pair) => pair,
                None => continue,
            };

            let mut current_length_sq = FixedPoint::ZERO;
            for i in 0..3 {
                let d = node_a.position[i] - node_b.position[i];
                current_length_sq += d * d;
            }
            let current_length = current_length_sq.sqrt();

            let stress = self.evaluate_stress(edge, current_length);

            if stress > self.config.stress_threshold * (FixedPoint::ONE + edge.damage) {
                let damage_delta =
                    (stress - self.config.stress_threshold) * self.config.damage_accumulation_rate;

                let edge_mut = &mut network.edges[edge_id];
                edge_mut.apply_damage(damage_delta);

                let compliance_delta = damage_delta * edge_mut.compliance;
                edge_mut.compliance =
                    (edge_mut.compliance + compliance_delta).min(self.config.max_compliance);

                if !edge_mut.active {
                    fractured.push(edge_id);
                    self.propagate_crack(network, edge_id, rng, &mut fractured);
                }
            } else if edge.damage > FixedPoint::ZERO && self.config.healing_rate > FixedPoint::ZERO
            {
                let edge_mut = &mut network.edges[edge_id];
                edge_mut.damage =
                    (edge_mut.damage - self.config.healing_rate).max(FixedPoint::ZERO);
                edge_mut.compliance = (edge_mut.compliance
                    - self.config.healing_rate * edge_mut.compliance)
                    .max(self.config.min_compliance);
            }
        }

        fractured
    }

    fn propagate_crack(
        &mut self,
        network: &mut ConstraintNetwork,
        source_edge_id: crate::constraint::ConstraintId,
        rng: &mut impl rand::Rng,
        fractured: &mut Vec<crate::constraint::ConstraintId>,
    ) {
        let edge = &network.edges[source_edge_id];
        let nodes = [edge.node_a, edge.node_b];

        for &node_id in &nodes {
            let neighbor_edges: Vec<_> = network.get_neighbors(node_id).to_vec();

            for &neighbor_id in &neighbor_edges {
                let neighbor_edge = &network.edges[neighbor_id];
                if !neighbor_edge.active || neighbor_edge.damage >= FixedPoint::ONE {
                    continue;
                }

                let roll: f32 = rng.gen();
                if FixedPoint::from_f32(roll) < self.config.crack_propagation_probability {
                    let damage = FixedPoint::from_f32(rng.gen::<f32>()) * FixedPoint::from_f32(0.5);
                    let edge_mut = &mut network.edges[neighbor_id];
                    edge_mut.apply_damage(damage);

                    if !edge_mut.active {
                        fractured.push(neighbor_id);
                    }
                }
            }
        }
    }
}

impl Default for FractureSystem {
    fn default() -> Self {
        Self::new(FractureConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{ConstraintGroup, ConstraintInput, ConstraintType};

    #[test]
    fn test_stress_evaluation() {
        let fs = FractureSystem::default();
        let edge = ConstraintEdge::new(
            crate::constraint::ConstraintId::default(),
            crate::constraint::NodeId::default(),
            crate::constraint::NodeId::default(),
            ConstraintType::Elastic,
            FixedPoint::ONE,
            FixedPoint::from_f32(50.0),
            FixedPoint::from_f32(0.001),
            ConstraintGroup::Structural,
        );

        let stress = fs.evaluate_stress(&edge, FixedPoint::from_f32(1.5));
        assert!(stress.to_f32() > 0.0);
    }

    #[test]
    fn test_no_fracture_below_threshold() {
        let mut network = ConstraintNetwork::new();
        let na =
            network.add_node([FixedPoint::ZERO; 3], FixedPoint::ONE, FixedPoint::from_f32(0.1));
        let nb = network.add_node(
            [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(1.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        let mut fs = FractureSystem::default();
        let mut rng = rand::thread_rng();
        let fractured = fs.process_fracture(&mut network, &mut rng);

        assert_eq!(network.active_edge_count(), 1);
        assert!(fractured.is_empty());
    }

    #[test]
    fn test_high_stress_causes_damage() {
        let mut network = ConstraintNetwork::new();
        let na =
            network.add_node([FixedPoint::ZERO; 3], FixedPoint::ONE, FixedPoint::from_f32(0.1));
        let nb = network.add_node(
            [FixedPoint::from_f32(10.0), FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        let mut fs = FractureSystem::default();
        let mut rng = rand::thread_rng();
        let _fractured = fs.process_fracture(&mut network, &mut rng);

        let edge = network.edges.values().next().unwrap();
        assert!(edge.damage > FixedPoint::ZERO, "high strain should cause damage");
    }
}
