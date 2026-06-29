use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::world::RigidBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintType {
    Fixed,
    Hinge,
    Spring,
    Slider,
    Distance,
    BallSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: Uuid,
    pub constraint_type: ConstraintType,
    pub body_a: Uuid,
    pub body_b: Uuid,
    pub anchor_a: FixedVec3,
    pub anchor_b: FixedVec3,
    pub stiffness: FixedPoint,
    pub damping: FixedPoint,
    pub max_force: FixedPoint,
    pub active: bool,
    accumulated_impulse: FixedVec3,
    axis: FixedVec3,
    rest_distance: FixedPoint,
    axis_local_a: FixedVec3,
    axis_local_b: FixedVec3,
}

impl Constraint {
    pub fn new(
        constraint_type: ConstraintType,
        body_a: Uuid,
        body_b: Uuid,
        anchor_a: FixedVec3,
        anchor_b: FixedVec3,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            constraint_type,
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            stiffness: FixedPoint::from_f32(0.8),
            damping: FixedPoint::from_f32(0.1),
            max_force: FixedPoint::from_f32(1000.0),
            active: true,
            accumulated_impulse: FixedVec3::ZERO,
            axis: FixedVec3::new(FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO),
            rest_distance: (anchor_b - anchor_a).length(),
            axis_local_a: FixedVec3::new(FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO),
            axis_local_b: FixedVec3::new(FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO),
        }
    }

    pub fn with_limits(
        mut self,
        stiffness: FixedPoint,
        damping: FixedPoint,
        max_force: FixedPoint,
    ) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self.max_force = max_force;
        self
    }

    pub fn with_axis(mut self, axis: FixedVec3) -> Self {
        self.axis = axis;
        self.axis_local_a = FixedVec3::ZERO;
        self.axis_local_b = FixedVec3::ZERO;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSolver {
    pub constraints: Vec<Constraint>,
    pub position_iterations: u32,
    pub velocity_iterations: u32,
    pub warm_starting: bool,
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            position_iterations: 4,
            velocity_iterations: 2,
            warm_starting: true,
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) -> Uuid {
        let id = constraint.id;
        self.constraints.push(constraint);
        id
    }

    pub fn remove_constraint(&mut self, id: Uuid) -> bool {
        let len_before = self.constraints.len();
        self.constraints.retain(|c| c.id != id);
        self.constraints.len() < len_before
    }

    pub fn get_constraint(&self, id: Uuid) -> Option<&Constraint> {
        self.constraints.iter().find(|c| c.id == id)
    }

    pub fn get_constraint_mut(&mut self, id: Uuid) -> Option<&mut Constraint> {
        self.constraints.iter_mut().find(|c| c.id == id)
    }

    pub fn solve(&mut self, dt: FixedPoint, bodies: &mut [RigidBody]) {
        for constraint in &mut self.constraints {
            if !constraint.active {
                continue;
            }
            constraint.accumulated_impulse = FixedVec3::ZERO;
        }

        for _ in 0..self.position_iterations {
            self.solve_position(bodies);
        }

        for _ in 0..self.velocity_iterations {
            self.solve_velocity(dt, bodies);
        }
    }

    pub fn solve_position(&mut self, bodies: &mut [RigidBody]) {
        for constraint in &mut self.constraints {
            if !constraint.active {
                continue;
            }

            let idx_a = bodies.iter().position(|b| b.id == constraint.body_a);
            let idx_b = bodies.iter().position(|b| b.id == constraint.body_b);

            let (idx_a, idx_b) = match (idx_a, idx_b) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            if idx_a == idx_b {
                continue;
            }

            let (body_a, body_b) = unsafe {
                let ptr = bodies.as_mut_ptr();
                (&mut *ptr.add(idx_a), &mut *ptr.add(idx_b))
            };

            let world_a = body_a.position + body_a.rotation.rotate_vec3(constraint.anchor_a);
            let world_b = body_b.position + body_b.rotation.rotate_vec3(constraint.anchor_b);

            let delta = world_b - world_a;
            let error = match constraint.constraint_type {
                ConstraintType::Distance => {
                    let current_dist = delta.length();
                    if current_dist.raw == 0 {
                        continue;
                    }
                    let correction = FixedPoint::ONE - constraint.rest_distance / current_dist;
                    delta * correction
                },
                ConstraintType::BallSocket | ConstraintType::Fixed => delta,
                ConstraintType::Hinge => {
                    let axis_world = body_a.rotation.rotate_vec3(constraint.axis);
                    delta - axis_world * (delta.dot(axis_world))
                },
                ConstraintType::Slider => {
                    let axis_world = body_a.rotation.rotate_vec3(constraint.axis);
                    axis_world * (delta.dot(axis_world))
                },
                ConstraintType::Spring => {
                    let current_dist = delta.length();
                    if current_dist.raw == 0 {
                        continue;
                    }
                    let dir = delta / current_dist;
                    dir * (current_dist - constraint.rest_distance) * constraint.stiffness
                },
            };

            let inv_mass_a = if body_a.body_type == crate::world::BodyType::Static {
                FixedPoint::ZERO
            } else {
                FixedPoint::ONE / body_a.mass
            };
            let inv_mass_b = if body_b.body_type == crate::world::BodyType::Static {
                FixedPoint::ZERO
            } else {
                FixedPoint::ONE / body_b.mass
            };

            let total_inv_mass = inv_mass_a + inv_mass_b;
            if total_inv_mass.raw == 0 {
                continue;
            }

            let correction = error * constraint.stiffness;
            let correction_mag = correction.length();
            if correction_mag > constraint.max_force {
                constraint.accumulated_impulse = correction / correction_mag * constraint.max_force;
            } else {
                constraint.accumulated_impulse = correction;
            }

            if body_a.body_type != crate::world::BodyType::Static {
                body_a.position += constraint.accumulated_impulse * (inv_mass_a / total_inv_mass);
            }
            if body_b.body_type != crate::world::BodyType::Static {
                body_b.position -= constraint.accumulated_impulse * (inv_mass_b / total_inv_mass);
            }
        }
    }

    pub fn solve_velocity(&mut self, _dt: FixedPoint, bodies: &mut [RigidBody]) {
        for constraint in &mut self.constraints {
            if !constraint.active {
                continue;
            }

            let idx_a = bodies.iter().position(|b| b.id == constraint.body_a);
            let idx_b = bodies.iter().position(|b| b.id == constraint.body_b);

            let (idx_a, idx_b) = match (idx_a, idx_b) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            if idx_a == idx_b {
                continue;
            }

            let (body_a, body_b) = unsafe {
                let ptr = bodies.as_mut_ptr();
                (&mut *ptr.add(idx_a), &mut *ptr.add(idx_b))
            };

            let world_a = body_a.position + body_a.rotation.rotate_vec3(constraint.anchor_a);
            let world_b = body_b.position + body_b.rotation.rotate_vec3(constraint.anchor_b);

            let vel_a = body_a.velocity + body_a.angular_velocity.cross(world_a - body_a.position);
            let vel_b = body_b.velocity + body_b.angular_velocity.cross(world_b - body_b.position);

            let delta_vel = vel_b - vel_a;

            let inv_mass_a = if body_a.body_type == crate::world::BodyType::Static {
                FixedPoint::ZERO
            } else {
                FixedPoint::ONE / body_a.mass
            };
            let inv_mass_b = if body_b.body_type == crate::world::BodyType::Static {
                FixedPoint::ZERO
            } else {
                FixedPoint::ONE / body_b.mass
            };

            let total_inv_mass = inv_mass_a + inv_mass_b;
            if total_inv_mass.raw == 0 {
                continue;
            }

            let damping_impulse = delta_vel * constraint.damping;
            let impulse = damping_impulse * (FixedPoint::ONE / total_inv_mass);

            if body_a.body_type != crate::world::BodyType::Static {
                body_a.velocity += impulse * (inv_mass_a / total_inv_mass);
            }
            if body_b.body_type != crate::world::BodyType::Static {
                body_b.velocity -= impulse * (inv_mass_b / total_inv_mass);
            }
        }
    }

    pub fn find_constraints_for_body(&self, body_id: Uuid) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.body_a == body_id || c.body_b == body_id).collect()
    }

    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    pub fn active_count(&self) -> usize {
        self.constraints.iter().filter(|c| c.active).count()
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_point::FixedQuat;
    use crate::material::MaterialProperties;
    use crate::world::BodyType;

    fn make_test_body(id: Uuid, pos: FixedVec3, mass: FixedPoint) -> RigidBody {
        RigidBody {
            id,
            position: pos,
            rotation: FixedQuat::IDENTITY,
            velocity: FixedVec3::ZERO,
            angular_velocity: FixedVec3::ZERO,
            mass,
            material: MaterialProperties::default(),
            body_type: if mass.raw > 0 { BodyType::Dynamic } else { BodyType::Static },
            is_sleeping: false,
            sleep_timer: FixedPoint::ZERO,
            forces: FixedVec3::ZERO,
            torque: FixedVec3::ZERO,
            linear_damping: FixedPoint::from_f32(0.01),
            angular_damping: FixedPoint::from_f32(0.01),
            mpss_index: None,
        }
    }

    #[test]
    fn test_add_remove_constraint() {
        let mut solver = ConstraintSolver::new();
        let c = Constraint::new(
            ConstraintType::Distance,
            Uuid::new_v4(),
            Uuid::new_v4(),
            FixedVec3::ZERO,
            FixedVec3::new(FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO),
        );
        let id = solver.add_constraint(c);
        assert_eq!(solver.constraint_count(), 1);
        assert!(solver.get_constraint(id).is_some());
        assert!(solver.remove_constraint(id));
        assert_eq!(solver.constraint_count(), 0);
    }

    #[test]
    fn test_distance_constraint_solve() {
        let mut solver = ConstraintSolver::new();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let rest_dist = FixedPoint::from_f32(2.0);

        let mut c = Constraint::new(
            ConstraintType::Distance,
            id_a,
            id_b,
            FixedVec3::ZERO,
            FixedVec3::new(
                rest_dist * FixedPoint::from_f32(2.0),
                FixedPoint::ZERO,
                FixedPoint::ZERO,
            ),
        );
        c.rest_distance = rest_dist;
        solver.add_constraint(c);

        let mut bodies = vec![
            make_test_body(id_a, FixedVec3::ZERO, FixedPoint::ONE),
            make_test_body(
                id_b,
                FixedVec3::new(FixedPoint::from_f32(5.0), FixedPoint::ZERO, FixedPoint::ZERO),
                FixedPoint::ONE,
            ),
        ];

        solver.solve(FixedPoint::from_f32(0.016), &mut bodies);

        let dist = (bodies[1].position - bodies[0].position).length();
        assert!(dist < FixedPoint::from_f32(5.0));
    }

    #[test]
    fn test_constraint_active_toggle() {
        let mut solver = ConstraintSolver::new();
        let mut c = Constraint::new(
            ConstraintType::BallSocket,
            Uuid::new_v4(),
            Uuid::new_v4(),
            FixedVec3::ZERO,
            FixedVec3::new(FixedPoint::ONE, FixedPoint::ONE, FixedPoint::ONE),
        );
        c.active = false;
        solver.add_constraint(c);
        assert_eq!(solver.active_count(), 0);

        let c = solver.get_constraint_mut(solver.constraints[0].id).unwrap();
        c.active = true;
        assert_eq!(solver.active_count(), 1);
    }

    #[test]
    fn test_find_constraints_for_body() {
        let mut solver = ConstraintSolver::new();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();

        solver.add_constraint(Constraint::new(
            ConstraintType::Distance,
            id_a,
            id_b,
            FixedVec3::ZERO,
            FixedVec3::new(FixedPoint::ONE, FixedPoint::ONE, FixedPoint::ONE),
        ));
        solver.add_constraint(Constraint::new(
            ConstraintType::BallSocket,
            id_a,
            id_c,
            FixedVec3::ZERO,
            FixedVec3::new(FixedPoint::ONE, FixedPoint::ONE, FixedPoint::ONE),
        ));

        let found = solver.find_constraints_for_body(id_a);
        assert_eq!(found.len(), 2);
    }
}
