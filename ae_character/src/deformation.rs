use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlasticDeformation {
    pub position: Vec3,
    pub depth: f32,
    pub timestamp: f32,
    pub source_bone_id: Uuid,
    pub target_bone_id: Uuid,
    pub force_magnitude: f32,
    pub direction: Vec3,
    pub source_material: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeformationHistory {
    pub records: Vec<PlasticDeformation>,
    pub max_records: usize,
    pub surface_roughness_change: f32,
    pub cumulative_strain: f32,
    pub wear_depth: f32,
    pub sharpness_loss: f32,
}

impl Default for DeformationHistory {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            max_records: 500,
            surface_roughness_change: 0.0,
            cumulative_strain: 0.0,
            wear_depth: 0.0,
            sharpness_loss: 0.0,
        }
    }
}

impl DeformationHistory {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_deformation(
        &mut self,
        position: Vec3,
        depth: f32,
        timestamp: f32,
        source_bone_id: Uuid,
        target_bone_id: Uuid,
        force_magnitude: f32,
        direction: Vec3,
        source_material: &str,
    ) {
        let record = PlasticDeformation {
            position,
            depth,
            timestamp,
            source_bone_id,
            target_bone_id,
            force_magnitude,
            direction,
            source_material: source_material.to_string(),
        };

        self.records.push(record);
        if self.records.len() > self.max_records {
            self.records.remove(0);
        }

        self.cumulative_strain += depth;
        self.wear_depth += depth * 0.01;
        self.surface_roughness_change += depth * 0.5;
        self.surface_roughness_change = self.surface_roughness_change.min(1.0);
        self.sharpness_loss += depth * 0.1;
        self.sharpness_loss = self.sharpness_loss.min(1.0);
    }

    pub fn total_deformations(&self) -> usize {
        self.records.len()
    }

    pub fn recent_deformations(&self, count: usize) -> &[PlasticDeformation] {
        let len = self.records.len();
        let start = len.saturating_sub(count);
        &self.records[start..]
    }

    pub fn deformations_at_position(
        &self,
        position: Vec3,
        radius: f32,
    ) -> Vec<&PlasticDeformation> {
        self.records.iter().filter(|d| d.position.distance(position) <= radius).collect()
    }

    pub fn is_worn(&self) -> bool {
        self.wear_depth >= 0.01
    }

    pub fn is_dulled(&self) -> bool {
        self.sharpness_loss > 0.3
    }

    pub fn is_heavily_deformed(&self) -> bool {
        self.cumulative_strain > 1.0
    }

    pub fn reset(&mut self) {
        self.records.clear();
        self.surface_roughness_change = 0.0;
        self.cumulative_strain = 0.0;
        self.wear_depth = 0.0;
        self.sharpness_loss = 0.0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeformationTracker {
    pub histories: Vec<(Uuid, DeformationHistory)>,
}

impl DeformationTracker {
    pub fn new() -> Self {
        Self { histories: Vec::new() }
    }

    pub fn get_or_create(&mut self, bone_id: Uuid) -> &mut DeformationHistory {
        if let Some(idx) = self.histories.iter().position(|(id, _)| *id == bone_id) {
            &mut self.histories[idx].1
        } else {
            self.histories.push((bone_id, DeformationHistory::new()));
            &mut self.histories.last_mut().unwrap().1
        }
    }

    pub fn get(&self, bone_id: Uuid) -> Option<&DeformationHistory> {
        self.histories.iter().find(|(id, _)| *id == bone_id).map(|(_, h)| h)
    }

    pub fn record_collision_deformation(
        &mut self,
        source_bone_id: Uuid,
        target_bone_id: Uuid,
        contact_position: Vec3,
        normal: Vec3,
        force_magnitude: f32,
        time: f32,
        material: &str,
    ) {
        let depth = force_magnitude * 0.0001;
        let history = self.get_or_create(target_bone_id);
        history.record_deformation(
            contact_position,
            depth,
            time,
            source_bone_id,
            target_bone_id,
            force_magnitude,
            -normal,
            material,
        );

        let history = self.get_or_create(source_bone_id);
        history.record_deformation(
            contact_position,
            depth * 0.1,
            time,
            target_bone_id,
            source_bone_id,
            force_magnitude,
            normal,
            material,
        );
    }
}

impl Default for DeformationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_deformation() {
        let mut history = DeformationHistory::new();
        history.record_deformation(
            Vec3::ZERO,
            0.001,
            0.0,
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000.0,
            Vec3::Y,
            "iron",
        );
        assert_eq!(history.total_deformations(), 1);
        assert!(history.cumulative_strain > 0.0);
    }

    #[test]
    fn test_deformation_wear_tracking() {
        let mut history = DeformationHistory::new();
        for _ in 0..200 {
            history.record_deformation(
                Vec3::ZERO,
                0.01,
                0.0,
                Uuid::new_v4(),
                Uuid::new_v4(),
                5000.0,
                Vec3::Y,
                "stone",
            );
        }
        assert!(history.is_worn());
        assert!(history.is_heavily_deformed());
    }

    #[test]
    fn test_tracker_collision_record() {
        let mut tracker = DeformationTracker::new();
        let bone_a = Uuid::new_v4();
        let bone_b = Uuid::new_v4();
        tracker.record_collision_deformation(
            bone_a,
            bone_b,
            Vec3::ZERO,
            Vec3::Y,
            10000.0,
            1.0,
            "iron",
        );
        assert!(tracker.get(bone_a).is_some());
        assert!(tracker.get(bone_b).is_some());
    }
}
