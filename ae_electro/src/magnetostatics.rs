use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::properties::VACUUM_PERMEABILITY;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentElement {
    pub position: Vec3,
    pub direction: Vec3,
    pub current: f32,
    pub length: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentLoop {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub current: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagnetostaticSolver {
    pub time_step: f32,
}

impl Default for MagnetostaticSolver {
    fn default() -> Self {
        Self { time_step: 1.0 / 60.0 }
    }
}

impl MagnetostaticSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn biot_savart(&self, element: &CurrentElement, target_pos: Vec3) -> Vec3 {
        let r_vec = target_pos - element.position;
        let r = r_vec.length();
        if r < 1e-9 {
            return Vec3::ZERO;
        }
        let r_hat = r_vec / r;
        let dl = element.direction.normalize() * element.length;
        let cross = dl.cross(r_hat);
        (VACUUM_PERMEABILITY / (4.0 * std::f32::consts::PI)) * element.current * cross / (r * r)
    }

    pub fn magnetic_field_from_elements(
        &self,
        elements: &[CurrentElement],
        target_pos: Vec3,
    ) -> Vec3 {
        elements.iter().map(|el| self.biot_savart(el, target_pos)).sum()
    }

    pub fn magnetic_field_loop(&self, loop_: &CurrentLoop, target_pos: Vec3) -> Vec3 {
        let n = loop_.normal.normalize();
        let r = loop_.radius;
        let i = loop_.current;
        let center = loop_.center;

        let offset = target_pos - center;
        let axial = offset.dot(n);
        let radial = offset - axial * n;
        let radial_dist = radial.length();

        let num_segments = 36;
        let mut b = Vec3::ZERO;
        let d_theta = 2.0 * std::f32::consts::PI / num_segments as f32;

        let u = if radial_dist > 1e-6 { radial.normalize() } else { Vec3::X };
        let v = n.cross(u);

        for seg in 0..num_segments {
            let theta = seg as f32 * d_theta;
            let theta_next = (seg as f32 + 1.0) * d_theta;
            let theta_mid = (theta + theta_next) * 0.5;

            let pos = center + u * (r * theta_mid.cos()) + v * (r * theta_mid.sin());
            let tangent = -u * theta_mid.sin() + v * theta_mid.cos();
            let dl = tangent * r * d_theta;

            let r_vec = target_pos - pos;
            let dist = r_vec.length();
            if dist < 1e-9 {
                continue;
            }
            let r_hat = r_vec / dist;
            let cross = dl.cross(r_hat);
            b += (VACUUM_PERMEABILITY / (4.0 * std::f32::consts::PI)) * i * cross / (dist * dist);
        }

        b
    }

    pub fn lorentz_force(&self, q: f32, v: Vec3, b: Vec3) -> Vec3 {
        q * v.cross(b)
    }

    pub fn magnetic_force_on_wire(
        &self,
        current: f32,
        wire_direction: Vec3,
        wire_length: f32,
        b: Vec3,
    ) -> Vec3 {
        current
            * wire_direction.normalize()
            * wire_length
            * b.length()
            * wire_direction.normalize().cross(b.normalize()).length()
            * wire_direction.cross(b).normalize_or_zero()
    }

    pub fn magnetic_dipole_moment(&self, loop_: &CurrentLoop) -> Vec3 {
        let area = std::f32::consts::PI * loop_.radius * loop_.radius;
        loop_.current * area * loop_.normal.normalize()
    }

    pub fn torque_on_dipole(&self, magnetic_moment: Vec3, b: Vec3) -> Vec3 {
        magnetic_moment.cross(b)
    }

    pub fn magnetic_field_infinite_wire(
        &self,
        current: f32,
        wire_pos: Vec3,
        wire_dir: Vec3,
        target_pos: Vec3,
    ) -> Vec3 {
        let w = wire_dir.normalize();
        let r = target_pos - wire_pos;
        let perp = r - r.dot(w) * w;
        let perp_dist = perp.length();
        if perp_dist < 1e-9 {
            return Vec3::ZERO;
        }
        let b_dir = w.cross(perp.normalize());
        (VACUUM_PERMEABILITY * current) / (2.0 * std::f32::consts::PI * perp_dist) * b_dir
    }

    pub fn magnetic_field_solenoid_axis(
        &self,
        n_turns: f32,
        current: f32,
        length: f32,
        axial_dist: f32,
    ) -> f32 {
        let n = n_turns / length;
        VACUUM_PERMEABILITY * n * current * axial_dist.signum()
    }

    pub fn force_between_parallel_wires(
        &self,
        i1: f32,
        i2: f32,
        length: f32,
        separation: f32,
    ) -> f32 {
        if separation < 1e-9 {
            return 0.0;
        }
        VACUUM_PERMEABILITY * i1 * i2 * length / (2.0 * std::f32::consts::PI * separation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biot_savart_straight_wire() {
        let solver = MagnetostaticSolver::new();
        let element = CurrentElement {
            position: Vec3::new(0.0, 0.0, -1.0),
            direction: Vec3::Z,
            current: 1.0,
            length: 2.0,
        };
        let b = solver.biot_savart(&element, Vec3::new(1.0, 0.0, 0.0));
        assert!(b.z.abs() < 1e-6);
    }

    #[test]
    fn test_lorentz_force_perpendicular() {
        let solver = MagnetostaticSolver::new();
        let f = solver.lorentz_force(1.0, Vec3::X, Vec3::Y);
        assert!(f.z > 0.0);
    }

    #[test]
    fn test_lorentz_force_parallel() {
        let solver = MagnetostaticSolver::new();
        let f = solver.lorentz_force(1.0, Vec3::X, Vec3::X);
        assert!(f.length() < 1e-9);
    }

    #[test]
    fn test_magnetic_dipole_moment() {
        let solver = MagnetostaticSolver::new();
        let loop_ = CurrentLoop { center: Vec3::ZERO, normal: Vec3::Z, radius: 0.1, current: 1.0 };
        let m = solver.magnetic_dipole_moment(&loop_);
        assert!(m.z > 0.0);
    }

    #[test]
    fn test_infinite_wire_field() {
        let solver = MagnetostaticSolver::new();
        let b =
            solver.magnetic_field_infinite_wire(1.0, Vec3::ZERO, Vec3::Z, Vec3::new(1.0, 0.0, 0.0));
        assert!(b.y.abs() > 0.0);
    }

    #[test]
    fn test_parallel_wire_force() {
        let solver = MagnetostaticSolver::new();
        let f = solver.force_between_parallel_wires(1.0, 1.0, 1.0, 0.1);
        assert!(f > 0.0);
    }

    #[test]
    fn test_torque_on_dipole() {
        let solver = MagnetostaticSolver::new();
        let m = Vec3::Z;
        let b = Vec3::X;
        let torque = solver.torque_on_dipole(m, b);
        assert!(torque.y.abs() > 0.0);
    }
}
