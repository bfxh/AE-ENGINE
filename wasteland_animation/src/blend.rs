use glam::{Quat, Vec3};

#[derive(Debug, Clone, Copy, Default)]
pub struct BoneTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl BoneTransform {
    pub fn identity() -> Self {
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.lerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pose {
    pub bones: Vec<BoneTransform>,
    pub root_motion: Vec3,
}

impl Pose {
    pub fn new(bone_count: usize) -> Self {
        Self { bones: vec![BoneTransform::identity(); bone_count], root_motion: Vec3::ZERO }
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let count = self.bones.len().min(other.bones.len());
        let mut bones = Vec::with_capacity(count);
        for i in 0..count {
            bones.push(self.bones[i].lerp(&other.bones[i], t));
        }
        Self { bones, root_motion: self.root_motion.lerp(other.root_motion, t) }
    }

    pub fn nlerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let count = self.bones.len().min(other.bones.len());
        let mut bones = Vec::with_capacity(count);
        for i in 0..count {
            bones.push(BoneTransform {
                translation: self.bones[i].translation.lerp(other.bones[i].translation, t),
                rotation: self.bones[i].rotation.slerp(other.bones[i].rotation, t),
                scale: self.bones[i].scale.lerp(other.bones[i].scale, t),
            });
        }
        Self { bones, root_motion: self.root_motion.lerp(other.root_motion, t) }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlendSpace1D {
    pub poses: Vec<(f32, Pose)>,
}

impl BlendSpace1D {
    pub fn new() -> Self {
        Self { poses: Vec::new() }
    }

    pub fn add_pose(&mut self, param: f32, pose: Pose) {
        self.poses.push((param, pose));
        self.poses.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn sample(&self, param: f32) -> Option<Pose> {
        if self.poses.is_empty() {
            return None;
        }
        if self.poses.len() == 1 {
            return Some(self.poses[0].1.clone());
        }

        if param <= self.poses[0].0 {
            return Some(self.poses[0].1.clone());
        }
        if param >= self.poses.last().unwrap().0 {
            return Some(self.poses.last().unwrap().1.clone());
        }

        for i in 0..(self.poses.len() - 1) {
            let (p0, pose0) = &self.poses[i];
            let (p1, pose1) = &self.poses[i + 1];
            if param >= *p0 && param <= *p1 {
                let t = (param - p0) / (p1 - p0);
                return Some(pose0.nlerp(pose1, t));
            }
        }

        Some(self.poses[0].1.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlendSpace2D {
    pub poses: Vec<([f32; 2], Pose)>,
}

impl BlendSpace2D {
    pub fn new() -> Self {
        Self { poses: Vec::new() }
    }

    pub fn add_pose(&mut self, x: f32, y: f32, pose: Pose) {
        self.poses.push(([x, y], pose));
    }

    pub fn sample(&self, x: f32, y: f32) -> Option<Pose> {
        if self.poses.is_empty() {
            return None;
        }
        if self.poses.len() == 1 {
            return Some(self.poses[0].1.clone());
        }

        let mut best = None;
        let mut best_dist = f32::MAX;

        for (i, (params_i, _)) in self.poses.iter().enumerate() {
            let di = (params_i[0] - x) * (params_i[0] - x) + (params_i[1] - y) * (params_i[1] - y);
            if di < best_dist {
                best_dist = di;
                best = Some(i);
            }
        }

        let best_i = best?;
        let mut closest = vec![best_i];

        for (j, (params_j, _)) in self.poses.iter().enumerate() {
            if j == best_i {
                continue;
            }
            let dj = (params_j[0] - x) * (params_j[0] - x) + (params_j[1] - y) * (params_j[1] - y);
            if closest.len() < 3 {
                closest.push(j);
            } else {
                let max_idx = closest
                    .iter()
                    .enumerate()
                    .max_by(|(_, &a), (_, &b)| {
                        let da =
                            (self.poses[a].0[0] - x).powi(2) + (self.poses[a].0[1] - y).powi(2);
                        let db =
                            (self.poses[b].0[0] - x).powi(2) + (self.poses[b].0[1] - y).powi(2);
                        da.partial_cmp(&db).unwrap()
                    })
                    .map(|(idx, _)| idx)?;
                if dj < best_dist {
                    closest[max_idx] = j;
                }
            }
        }

        let weights = if closest.len() == 3 {
            let p0 = self.poses[closest[0]].0;
            let p1 = self.poses[closest[1]].0;
            let p2 = self.poses[closest[2]].0;
            Self::barycentric_weights([x, y], p0, p1, p2)
        } else if closest.len() == 2 {
            let d0 = ((self.poses[closest[0]].0[0] - x).powi(2)
                + (self.poses[closest[0]].0[1] - y).powi(2))
            .sqrt();
            let d1 = ((self.poses[closest[1]].0[0] - x).powi(2)
                + (self.poses[closest[1]].0[1] - y).powi(2))
            .sqrt();
            let sum = d0 + d1;
            if sum < 0.0001 { [0.5, 0.5, 0.0] } else { [d1 / sum, d0 / sum, 0.0] }
        } else {
            return Some(self.poses[closest[0]].1.clone());
        };

        let count = self.poses[closest[0]].1.bones.len();
        let mut bones = vec![BoneTransform::identity(); count];
        for (i, bone) in bones.iter_mut().enumerate() {
            let w0 = self.poses[closest[0]].1.bones[i].scale * weights[0];
            let w1 = if closest.len() > 1 {
                self.poses[closest[1]].1.bones[i].scale * weights[1]
            } else {
                Vec3::ZERO
            };
            let w2 = if closest.len() > 2 {
                self.poses[closest[2]].1.bones[i].scale * weights[2]
            } else {
                Vec3::ZERO
            };
            bone.scale = w0 + w1 + w2;
        }

        Some(Pose { bones, root_motion: self.poses[closest[0]].1.root_motion * weights[0] })
    }

    fn barycentric_weights(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> [f32; 3] {
        let denom = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
        if denom.abs() < 0.0001 {
            return [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        }

        let w0 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / denom;
        let w1 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / denom;
        let w0 = w0.clamp(0.0, 1.0);
        let w1 = w1.clamp(0.0, 1.0);
        let w2 = (1.0 - w0 - w1).max(0.0);
        let sum = w0 + w1 + w2;
        if sum > 0.0 { [w0 / sum, w1 / sum, w2 / sum] } else { [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bone_transform_lerp() {
        let a =
            BoneTransform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
        let b = BoneTransform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let mid = a.lerp(&b, 0.5);
        assert!((mid.translation.x - 0.5).abs() < 0.01);
        assert!((mid.scale.x - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_blend_space_1d() {
        let pose_a = Pose::new(1);
        let pose_b = Pose::new(1);
        let mut bs = BlendSpace1D::new();
        bs.add_pose(0.0, pose_a);
        bs.add_pose(1.0, pose_b);
        let sample = bs.sample(0.5);
        assert!(sample.is_some());
    }

    #[test]
    fn test_blend_space_1d_extrapolate() {
        let pose_a = Pose::new(1);
        let pose_b = Pose::new(1);
        let mut bs = BlendSpace1D::new();
        bs.add_pose(0.0, pose_a);
        bs.add_pose(1.0, pose_b);
        let sample = bs.sample(2.0);
        assert!(sample.is_some());
    }
}
