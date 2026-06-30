use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::particles::{ElementType, Particle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfOrganizationSystem {
    pub cluster_threshold: f32,
    pub alignment_strength: f32,
    pub cohesion_strength: f32,
    pub separation_strength: f32,
    pub neighborhood_radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleCluster {
    pub particles: Vec<usize>,
    pub center: Vec3,
    pub velocity: Vec3,
    pub element_counts: Vec<(ElementType, usize)>,
    pub average_temperature: f32,
}

impl SelfOrganizationSystem {
    pub fn new() -> Self {
        Self {
            cluster_threshold: 2.0,
            alignment_strength: 0.1,
            cohesion_strength: 0.05,
            separation_strength: 0.15,
            neighborhood_radius: 3.0,
        }
    }

    pub fn find_clusters(&self, particles: &[Particle]) -> Vec<ParticleCluster> {
        let n = particles.len();
        let mut visited = vec![false; n];
        let mut clusters = Vec::new();

        for i in 0..n {
            if !particles[i].active || visited[i] {
                continue;
            }

            let mut cluster_particles = Vec::new();
            let mut stack = vec![i];
            visited[i] = true;

            while let Some(idx) = stack.pop() {
                cluster_particles.push(idx);

                for j in 0..n {
                    if !particles[j].active || visited[j] {
                        continue;
                    }

                    let dist = (particles[idx].position - particles[j].position).length();
                    if dist < self.cluster_threshold {
                        visited[j] = true;
                        stack.push(j);
                    }
                }
            }

            if cluster_particles.len() >= 2 {
                let mut center = Vec3::ZERO;
                let mut velocity = Vec3::ZERO;
                let mut temp_sum = 0.0;
                let mut element_map: std::collections::HashMap<ElementType, usize> =
                    std::collections::HashMap::new();

                for &idx in &cluster_particles {
                    center += particles[idx].position;
                    velocity += particles[idx].velocity;
                    temp_sum += particles[idx].temperature;
                    *element_map.entry(particles[idx].element_type).or_insert(0) += 1;
                }

                let count = cluster_particles.len() as f32;
                center /= count;
                velocity /= count;
                temp_sum /= count;

                clusters.push(ParticleCluster {
                    particles: cluster_particles,
                    center,
                    velocity,
                    element_counts: element_map.into_iter().collect(),
                    average_temperature: temp_sum,
                });
            }
        }

        clusters
    }

    pub fn apply_boids(&self, particles: &mut [Particle], dt: f32) {
        let n = particles.len();

        for i in 0..n {
            if !particles[i].active {
                continue;
            }

            let mut alignment = Vec3::ZERO;
            let mut cohesion = Vec3::ZERO;
            let mut separation = Vec3::ZERO;
            let mut neighbor_count = 0usize;

            for j in 0..n {
                if i == j || !particles[j].active {
                    continue;
                }

                let delta = particles[j].position - particles[i].position;
                let dist = delta.length();

                if dist < self.neighborhood_radius && dist > 0.001 {
                    alignment += particles[j].velocity;
                    cohesion += particles[j].position;
                    separation -= delta / (dist * dist);
                    neighbor_count += 1;
                }
            }

            if neighbor_count > 0 {
                let nf = neighbor_count as f32;

                alignment /= nf;
                let alignment_force = (alignment - particles[i].velocity) * self.alignment_strength;

                cohesion /= nf;
                let cohesion_force = (cohesion - particles[i].position) * self.cohesion_strength;

                let separation_force = separation * self.separation_strength;

                let total_force = alignment_force + cohesion_force + separation_force;
                particles[i].apply_force(total_force, dt);
            }
        }
    }

    pub fn compute_entropy(&self, particles: &[Particle]) -> f32 {
        let active: Vec<&Particle> = particles.iter().filter(|p| p.active).collect();
        if active.len() < 2 {
            return 0.0;
        }

        let mut velocities: Vec<f32> = active.iter().map(|p| p.velocity.length()).collect();
        velocities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mean = velocities.iter().sum::<f32>() / velocities.len() as f32;
        let variance =
            velocities.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / velocities.len() as f32;

        variance
    }

    pub fn order_parameter(&self, particles: &[Particle]) -> f32 {
        let active: Vec<&Particle> = particles.iter().filter(|p| p.active).collect();
        if active.is_empty() {
            return 0.0;
        }

        let total_speed: f32 = active.iter().map(|p| p.velocity.length()).sum();
        if total_speed < 1e-6 {
            return 0.0;
        }

        let avg_dir: Vec3 = active
            .iter()
            .map(|p| {
                let speed = p.velocity.length();
                if speed < 1e-6 { Vec3::ZERO } else { p.velocity / speed }
            })
            .sum();

        avg_dir.length() / active.len() as f32
    }
}

impl Default for SelfOrganizationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Phase;
    use glam::Vec3;

    #[test]
    fn test_new_defaults() {
        let system = SelfOrganizationSystem::new();
        assert!((system.cluster_threshold - 2.0).abs() < 0.001);
        assert!((system.alignment_strength - 0.1).abs() < 0.001);
        assert!((system.cohesion_strength - 0.05).abs() < 0.001);
        assert!((system.separation_strength - 0.15).abs() < 0.001);
        assert!((system.neighborhood_radius - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_find_clusters_nearby() {
        let system = SelfOrganizationSystem::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
        ];
        let clusters = system.find_clusters(&particles);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].particles.len(), 2);
    }

    #[test]
    fn test_find_clusters_far_apart() {
        let system = SelfOrganizationSystem::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(10.0, 0.0, 0.0), Phase::Solid),
        ];
        let clusters = system.find_clusters(&particles);
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_find_clusters_ignores_inactive() {
        let system = SelfOrganizationSystem::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
        ];
        particles[1].active = false;
        let clusters = system.find_clusters(&particles);
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_find_clusters_three_particles() {
        let system = SelfOrganizationSystem::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.5, 0.0), Phase::Solid),
        ];
        let clusters = system.find_clusters(&particles);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].particles.len(), 3);
    }

    #[test]
    fn test_apply_boids_no_crash() {
        let system = SelfOrganizationSystem::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.0, 1.0, 0.0), Phase::Solid),
        ];
        system.apply_boids(&mut particles, 0.016);
        assert!(particles.iter().all(|p| p.active));
    }

    #[test]
    fn test_apply_boids_single_particle() {
        let system = SelfOrganizationSystem::new();
        let mut particles =
            vec![Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid)];
        let pos_before = particles[0].position;
        system.apply_boids(&mut particles, 0.016);
        assert_eq!(particles[0].position, pos_before);
    }

    #[test]
    fn test_compute_entropy_empty() {
        let system = SelfOrganizationSystem::new();
        let particles: Vec<Particle> = vec![];
        let entropy = system.compute_entropy(&particles);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_compute_entropy_moving() {
        let system = SelfOrganizationSystem::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        particles[1].velocity = Vec3::new(2.0, 0.0, 0.0);
        let entropy = system.compute_entropy(&particles);
        assert!(entropy > 0.0);
    }

    #[test]
    fn test_order_parameter_aligned() {
        let system = SelfOrganizationSystem::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        particles[1].velocity = Vec3::new(1.0, 0.0, 0.0);
        let order = system.order_parameter(&particles);
        assert!((order - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_order_parameter_opposite() {
        let system = SelfOrganizationSystem::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        particles[1].velocity = Vec3::new(-1.0, 0.0, 0.0);
        let order = system.order_parameter(&particles);
        assert!((order - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_order_parameter_empty() {
        let system = SelfOrganizationSystem::new();
        let particles: Vec<Particle> = vec![];
        let order = system.order_parameter(&particles);
        assert_eq!(order, 0.0);
    }

    #[test]
    fn test_cluster_center_computation() {
        let system =
            SelfOrganizationSystem { cluster_threshold: 5.0, ..SelfOrganizationSystem::new() };
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(2.0, 0.0, 0.0), Phase::Solid),
        ];
        let clusters = system.find_clusters(&particles);
        assert_eq!(clusters.len(), 1);
        assert!((clusters[0].center.x - 1.0).abs() < 0.001);
        assert!((clusters[0].center.y - 0.0).abs() < 0.001);
    }
}
