use glam::Vec3;

#[derive(Debug, Clone)]
pub struct IKBone {
    pub position: Vec3,
    pub length: f32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IKChain {
    pub bones: Vec<IKBone>,
    pub max_iterations: usize,
    pub tolerance: f32,
}

impl IKChain {
    pub fn new(bones: Vec<IKBone>, max_iterations: usize, tolerance: f32) -> Self {
        Self { bones, max_iterations, tolerance }
    }

    pub fn total_length(&self) -> f32 {
        self.bones.iter().map(|b| b.length).sum()
    }

    pub fn solve_fabrik(&mut self, target: Vec3) -> bool {
        if self.bones.is_empty() {
            return false;
        }

        let root_pos = self.bones[0].position;
        let total_len = self.total_length();

        if (target - root_pos).length() > total_len {
            let dir = (target - root_pos).normalize_or_zero();
            let mut current = root_pos;
            for bone in &mut self.bones[1..] {
                current += dir * bone.length;
                bone.position = current;
            }
            return true;
        }

        for _ in 0..self.max_iterations {
            self.backward_pass(target);
            self.forward_pass(root_pos);

            let end_pos = self.bones.last().unwrap().position;
            if (end_pos - target).length() <= self.tolerance {
                return true;
            }
        }

        let end_pos = self.bones.last().unwrap().position;
        (end_pos - target).length() <= self.tolerance
    }

    fn backward_pass(&mut self, target: Vec3) {
        let last_idx = self.bones.len() - 1;
        self.bones[last_idx].position = target;

        for i in (1..=last_idx).rev() {
            let dir = (self.bones[i - 1].position - self.bones[i].position).normalize_or_zero();
            self.bones[i - 1].position = self.bones[i].position + dir * self.bones[i - 1].length;
        }
    }

    fn forward_pass(&mut self, root: Vec3) {
        self.bones[0].position = root;

        for i in 1..self.bones.len() {
            let dir = (self.bones[i].position - self.bones[i - 1].position).normalize_or_zero();
            self.bones[i].position = self.bones[i - 1].position + dir * self.bones[i - 1].length;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TwoBoneIK {
    pub upper_length: f32,
    pub lower_length: f32,
    pub pole_vector: Vec3,
}

impl TwoBoneIK {
    pub fn new(upper_length: f32, lower_length: f32) -> Self {
        Self { upper_length, lower_length, pole_vector: Vec3::Y }
    }

    pub fn solve(&self, root: Vec3, target: Vec3) -> Option<(Vec3, Vec3)> {
        let to_target = target - root;
        let dist = to_target.length();
        let max_reach = self.upper_length + self.lower_length;
        let min_reach = (self.upper_length - self.lower_length).abs();

        if dist > max_reach || dist < min_reach {
            return None;
        }

        let a = self.upper_length;
        let b = self.lower_length;
        let c = dist;

        let cos_b = (a * a + c * c - b * b) / (2.0 * a * c);
        let cos_b = cos_b.clamp(-1.0, 1.0);
        let angle_b = cos_b.acos();

        let to_target_n = to_target.normalize_or_zero();
        let pole = self.pole_vector.normalize_or_zero();

        let rotation_axis = to_target_n.cross(pole).normalize_or_zero();
        if rotation_axis.length_squared() < 0.001 {
            let fallback_axis = if to_target_n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            let rot = glam::Quat::from_axis_angle(fallback_axis, angle_b);
            let elbow = root + rot * (to_target_n * a);
            return Some((elbow, target));
        }

        let rot = glam::Quat::from_axis_angle(rotation_axis, angle_b);
        let elbow = root + rot * (to_target_n * a);

        Some((elbow, target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fabrik_simple_chain() {
        let bones = vec![
            IKBone { position: Vec3::ZERO, length: 0.5, name: "root".into() },
            IKBone { position: Vec3::new(0.0, 0.5, 0.0), length: 0.5, name: "mid".into() },
            IKBone { position: Vec3::new(0.0, 1.0, 0.0), length: 0.0, name: "end".into() },
        ];
        let mut chain = IKChain::new(bones, 10, 0.001);
        let target = Vec3::new(0.3, 0.7, 0.0);
        let solved = chain.solve_fabrik(target);
        assert!(solved);
        let end = chain.bones.last().unwrap().position;
        assert!((end - target).length() < 0.1);
    }

    #[test]
    fn test_fabrik_out_of_reach() {
        let bones = vec![
            IKBone { position: Vec3::ZERO, length: 0.5, name: "root".into() },
            IKBone { position: Vec3::new(0.0, 0.5, 0.0), length: 0.5, name: "mid".into() },
            IKBone { position: Vec3::new(0.0, 1.0, 0.0), length: 0.0, name: "end".into() },
        ];
        let mut chain = IKChain::new(bones, 10, 0.001);
        let target = Vec3::new(0.0, 5.0, 0.0);
        let solved = chain.solve_fabrik(target);
        assert!(solved);
    }

    #[test]
    fn test_two_bone_ik() {
        let ik = TwoBoneIK::new(0.5, 0.3);
        let result = ik.solve(Vec3::ZERO, Vec3::new(0.5, 0.4, 0.0));
        assert!(result.is_some());
    }

    #[test]
    fn test_two_bone_ik_too_far() {
        let ik = TwoBoneIK::new(0.3, 0.2);
        let result = ik.solve(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        assert!(result.is_none());
    }
}
