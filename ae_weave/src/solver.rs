use ae_physics::fixed_point::FixedPoint;

use crate::constraint::ConstraintType;
use crate::network::ConstraintNetwork;

#[derive(Debug, Clone)]
pub struct SolverConfig {
    pub substeps: u32,
    pub compliance_relaxation: FixedPoint,
    pub max_iterations: u32,
    pub convergence_threshold: FixedPoint,
    pub group_priorities: Vec<(crate::constraint::ConstraintGroup, u32)>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            substeps: 8,
            compliance_relaxation: FixedPoint::from_f32(0.5),
            max_iterations: 64,
            convergence_threshold: FixedPoint::from_f32(0.0001),
            group_priorities: vec![
                (crate::constraint::ConstraintGroup::Contact, 0),
                (crate::constraint::ConstraintGroup::Structural, 1),
                (crate::constraint::ConstraintGroup::Fluid, 2),
                (crate::constraint::ConstraintGroup::Thermal, 3),
            ],
        }
    }
}

pub struct WeaveSolver {
    config: SolverConfig,
}

impl WeaveSolver {
    pub fn new(config: SolverConfig) -> Self {
        Self { config }
    }

    pub fn step(&self, network: &mut ConstraintNetwork, dt: FixedPoint) {
        let sub_dt = dt / FixedPoint::from_i32(self.config.substeps as i32);

        for _ in 0..self.config.substeps {
            network.predict_positions(sub_dt);

            for iteration in 0..self.config.max_iterations {
                let max_error = self.solve_constraints(network, sub_dt);
                if max_error < self.config.convergence_threshold && iteration > 0 {
                    break;
                }
            }

            network.update_velocities(FixedPoint::ONE / sub_dt);
        }

        self.process_plastic_deformation(network);
    }

    fn process_plastic_deformation(&self, network: &mut ConstraintNetwork) {
        use crate::constraint::ConstraintType;

        let edge_ids: Vec<_> = network.edges.keys().collect();
        for edge_id in edge_ids {
            let edge = match network.edges.get(edge_id) {
                Some(e) => e,
                None => continue,
            };
            if !edge.active || edge.constraint_type != ConstraintType::Surface {
                continue;
            }
            let sp = match edge.surface_params {
                Some(ref sp) => *sp,
                None => continue,
            };
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

            let strain = (current_length - edge.rest_length).abs()
                / edge.rest_length.max(FixedPoint::EPSILON);
            let stress = strain * edge.stiffness;
            let pressure = stress / sp.contact_area.max(FixedPoint::EPSILON);

            if pressure > sp.yield_strength && pressure <= sp.ultimate_strength {
                let plastic_rate = FixedPoint::from_f32(0.1);
                let plastic_strain = (strain * plastic_rate).min(FixedPoint::from_f32(0.05));
                if let Some(edge_mut) = network.edges.get_mut(edge_id) {
                    if let Some(ref mut sp_mut) = edge_mut.surface_params {
                        sp_mut.plastic_strain += plastic_strain;
                        if current_length > edge_mut.rest_length {
                            edge_mut.rest_length += plastic_strain * edge_mut.rest_length;
                        } else {
                            edge_mut.rest_length -= plastic_strain * edge_mut.rest_length;
                        }
                    }
                }
            }
        }
    }

    fn solve_constraints(&self, network: &mut ConstraintNetwork, dt: FixedPoint) -> FixedPoint {
        let dt_sq = dt * dt;
        let mut max_error = FixedPoint::ZERO;

        for &(ref group, _priority) in &self.config.group_priorities {
            let edge_ids: Vec<_> = network.edge_groups.get(group).cloned().unwrap_or_default();

            for edge_id in edge_ids {
                if let Some(error) = self.solve_edge(network, edge_id, dt_sq) {
                    if error > max_error {
                        max_error = error;
                    }
                }
            }
        }

        let ungrouped_ids: Vec<_> = network
            .edges
            .iter()
            .filter(|(_, e)| !self.config.group_priorities.iter().any(|(g, _)| e.group == *g))
            .map(|(id, _)| id)
            .collect();

        for edge_id in ungrouped_ids {
            if let Some(error) = self.solve_edge(network, edge_id, dt_sq) {
                if error > max_error {
                    max_error = error;
                }
            }
        }

        max_error
    }

    fn solve_edge(
        &self,
        network: &mut ConstraintNetwork,
        edge_id: crate::constraint::ConstraintId,
        dt_sq: FixedPoint,
    ) -> Option<FixedPoint> {
        let edge = network.edges.get(edge_id)?;
        if !edge.active {
            return None;
        }

        let (na_id, nb_id) = (edge.node_a, edge.node_b);
        let (node_a, node_b) = network.nodes.get(na_id).zip(network.nodes.get(nb_id))?;

        let mut delta = [FixedPoint::ZERO; 3];
        let mut current_length_sq = FixedPoint::ZERO;
        for (i, delta_i) in delta.iter_mut().enumerate() {
            *delta_i = node_a.position[i] - node_b.position[i];
            current_length_sq += *delta_i * *delta_i;
        }

        let current_length = current_length_sq.sqrt();
        if current_length < FixedPoint::EPSILON {
            return Some(FixedPoint::ZERO);
        }

        let error = current_length - edge.rest_length;
        let abs_error = error.abs();

        let inv_mass_sum = node_a.inv_mass + node_b.inv_mass;
        if inv_mass_sum < FixedPoint::EPSILON {
            return Some(abs_error);
        }

        let correction = match edge.constraint_type {
            ConstraintType::Fixed => {
                if error.abs() > FixedPoint::EPSILON {
                    error * edge.stiffness
                } else {
                    FixedPoint::ZERO
                }
            },
            ConstraintType::Elastic => {
                let alpha = edge.compliance / dt_sq;
                let lambda = -error / (inv_mass_sum + alpha);
                let force = lambda.abs();
                if force > edge.max_force { FixedPoint::ZERO } else { lambda }
            },
            ConstraintType::Variable => {
                let alpha = edge.compliance / dt_sq;
                let adjusted_compliance = alpha + edge.damage * edge.compliance;
                -error / (inv_mass_sum + adjusted_compliance)
            },
            ConstraintType::Repulsion => {
                if error > FixedPoint::ZERO {
                    FixedPoint::ZERO
                } else {
                    let alpha = edge.compliance / dt_sq;
                    -error / (inv_mass_sum + alpha)
                }
            },
            ConstraintType::Attraction => {
                if error < FixedPoint::ZERO {
                    FixedPoint::ZERO
                } else {
                    let alpha = edge.compliance / dt_sq;
                    -error / (inv_mass_sum + alpha)
                }
            },
            ConstraintType::Free => FixedPoint::ZERO,
            ConstraintType::Surface => {
                let alpha = edge.compliance / dt_sq;
                let lambda = -error / (inv_mass_sum + alpha);
                let force = lambda.abs();
                if let Some(ref sp) = edge.surface_params {
                    let pressure = force / sp.contact_area.max(FixedPoint::EPSILON);
                    if pressure > sp.ultimate_strength { FixedPoint::ZERO } else { lambda }
                } else {
                    lambda
                }
            },
        };

        if correction == FixedPoint::ZERO {
            return Some(abs_error);
        }

        let inv_length = FixedPoint::ONE / current_length;
        let mut gradient = [FixedPoint::ZERO; 3];
        for (i, gradient_i) in gradient.iter_mut().enumerate() {
            *gradient_i = delta[i] * inv_length;
        }

        let (inv_mass_a, inv_mass_b, pinned_a, pinned_b, pos_a, pos_b) = {
            let na = network.nodes.get(na_id)?;
            let nb = network.nodes.get(nb_id)?;
            (na.inv_mass, nb.inv_mass, na.pinned, nb.pinned, na.position, nb.position)
        };

        for (i, gradient_i) in gradient.iter().enumerate() {
            let dx = *gradient_i * correction;
            if !pinned_a {
                let new_pos = pos_a[i] + dx * inv_mass_a;
                if let Some(node) = network.nodes.get_mut(na_id) {
                    node.position[i] = new_pos;
                }
            }
            if !pinned_b {
                let new_pos = pos_b[i] - dx * inv_mass_b;
                if let Some(node) = network.nodes.get_mut(nb_id) {
                    node.position[i] = new_pos;
                }
            }
        }

        Some(abs_error)
    }
}

impl Default for WeaveSolver {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{ConstraintGroup, ConstraintInput, ConstraintType};

    #[test]
    fn test_solver_basic() {
        let mut network = ConstraintNetwork::new();
        let na = network.add_node(
            [FixedPoint::ZERO, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.pin_node(na);

        let nb = network.add_node(
            [FixedPoint::ZERO, FixedPoint::from_f32(-2.0), FixedPoint::ZERO],
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

        let solver = WeaveSolver::default();
        solver.step(&mut network, FixedPoint::from_f32(1.0 / 60.0));

        let node_b = network.nodes.get(nb).unwrap();
        let dist = node_b.position[1].abs();
        assert!(dist.to_f32() < 2.0, "node should be pulled toward pinned node");
    }

    #[test]
    fn test_repulsion_constraint() {
        let mut network = ConstraintNetwork::new();
        let na = network.add_node(
            [FixedPoint::ZERO, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.pin_node(na);

        let nb = network.add_node(
            [FixedPoint::ZERO, FixedPoint::from_f32(-0.5), FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );

        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Repulsion,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Contact,
            surface_params: None,
        });

        let solver = WeaveSolver::default();
        solver.step(&mut network, FixedPoint::from_f32(1.0 / 60.0));

        let node_b = network.nodes.get(nb).unwrap();
        let dist = node_b.position[1].abs();
        assert!(dist.to_f32() >= 0.5, "node should be pushed away");
    }

    #[test]
    fn test_fixed_constraint() {
        let mut network = ConstraintNetwork::new();
        let na = network.add_node(
            [FixedPoint::ZERO, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.pin_node(na);

        let nb = network.add_node(
            [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.pin_node(nb);

        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Fixed,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(10.0),
            compliance: FixedPoint::ZERO,
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        let solver = WeaveSolver::default();
        solver.step(&mut network, FixedPoint::from_f32(1.0 / 60.0));

        let node_a = network.nodes.get(na).unwrap();
        let node_b = network.nodes.get(nb).unwrap();
        let mut dist_sq = FixedPoint::ZERO;
        for (pa, pb) in node_a.position.iter().zip(node_b.position.iter()) {
            let d = *pa - *pb;
            dist_sq += d * d;
        }
        let dist = dist_sq.sqrt();
        assert!((dist.to_f32() - 1.0).abs() < 0.1, "fixed constraint should maintain distance");
    }

    #[test]
    fn test_convergence() {
        let mut network = ConstraintNetwork::new();
        let na = network.add_node(
            [FixedPoint::ZERO, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.pin_node(na);

        let nb = network.add_node(
            [FixedPoint::ZERO, FixedPoint::from_f32(-5.0), FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );

        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.0001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        let config = SolverConfig { substeps: 16, max_iterations: 128, ..Default::default() };
        let solver = WeaveSolver::new(config);
        solver.step(&mut network, FixedPoint::from_f32(1.0 / 60.0));

        let node_b = network.nodes.get(nb).unwrap();
        let dist = node_b.position[1].abs();
        assert!((dist.to_f32() - 1.0).abs() < 0.5, "should converge near rest length");
    }
}
