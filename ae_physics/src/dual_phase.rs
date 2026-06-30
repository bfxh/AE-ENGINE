use glam::IVec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::destruction::{VoxelFlags, VoxelGrid};
use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::material::MaterialProperties;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Particle,
    Voxel,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPhaseEntity {
    pub id: Uuid,
    pub material: MaterialProperties,
    pub active_phase: Phase,
    pub particle_phase: ParticlePhase,
    pub voxel_phase: VoxelPhase,
    pub transform: DualPhaseTransform,
    pub last_sync_tick: u64,
    pub sync_threshold: FixedPoint,
    pub priority: PhasePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhasePriority {
    ParticleDominant,
    VoxelDominant,
    Balanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticlePhase {
    pub particles: Vec<DualPhaseParticle>,
    pub total_mass: FixedPoint,
    pub density_threshold: FixedPoint,
    pub kernel_radius: FixedPoint,
    pub rest_density: FixedPoint,
    pub particle_spacing: FixedPoint,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DualPhaseParticle {
    pub position: FixedVec3,
    pub velocity: FixedVec3,
    pub density: FixedPoint,
    pub pressure: FixedPoint,
    pub mass: FixedPoint,
    pub radius: FixedPoint,
    pub active: bool,
    pub chemical_state: [FixedPoint; 4],
}

impl Default for DualPhaseParticle {
    fn default() -> Self {
        Self {
            position: FixedVec3::ZERO,
            velocity: FixedVec3::ZERO,
            density: FixedPoint::ONE,
            pressure: FixedPoint::ZERO,
            mass: FixedPoint::ONE,
            radius: FixedPoint::from_f32(0.5),
            active: true,
            chemical_state: [FixedPoint::ZERO; 4],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelPhase {
    pub grid: Option<VoxelGrid>,
    pub grid_resolution: IVec3,
    pub voxel_size: FixedPoint,
    pub origin: FixedVec3,
    pub dirty_regions: Vec<DirtyRegion>,
    pub chemical_distribution: HashMap<u32, Vec<f32>>,
    pub phase_boundaries: Vec<PhaseBoundary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DirtyRegion {
    pub min: FixedVec3,
    pub max: FixedVec3,
    pub tick_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseBoundary {
    pub center: FixedVec3,
    pub normal: FixedVec3,
    pub particle_side: FixedVec3,
    pub voxel_side: FixedVec3,
    pub transition_width: FixedPoint,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DualPhaseTransform {
    pub position: FixedVec3,
    pub rotation: glam::Quat,
    pub scale: FixedVec3,
}

impl Default for DualPhaseTransform {
    fn default() -> Self {
        Self {
            position: FixedVec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: FixedVec3::new(FixedPoint::ONE, FixedPoint::ONE, FixedPoint::ONE),
        }
    }
}

impl DualPhaseEntity {
    pub fn new(
        material: MaterialProperties,
        resolution: IVec3,
        voxel_size: FixedPoint,
        origin: FixedVec3,
    ) -> Self {
        let grid = VoxelGrid::new(resolution, voxel_size, origin, material);
        Self {
            id: Uuid::new_v4(),
            material,
            active_phase: Phase::Voxel,
            particle_phase: ParticlePhase {
                particles: Vec::new(),
                total_mass: FixedPoint::ZERO,
                density_threshold: FixedPoint::from_f32(0.3),
                kernel_radius: voxel_size * FixedPoint::from_f32(2.0),
                rest_density: FixedPoint::from_f32(1000.0),
                particle_spacing: voxel_size,
            },
            voxel_phase: VoxelPhase {
                grid: Some(grid),
                grid_resolution: resolution,
                voxel_size,
                origin,
                dirty_regions: Vec::new(),
                chemical_distribution: HashMap::new(),
                phase_boundaries: Vec::new(),
            },
            transform: DualPhaseTransform::default(),
            last_sync_tick: 0,
            sync_threshold: FixedPoint::from_f32(10.0),
            priority: PhasePriority::VoxelDominant,
        }
    }

    pub fn particles_to_voxels(&mut self) {
        if self.particle_phase.particles.is_empty() {
            return;
        }

        let grid = match &mut self.voxel_phase.grid {
            Some(g) => g,
            None => {
                self.voxel_phase.grid = Some(VoxelGrid::new(
                    self.voxel_phase.grid_resolution,
                    self.voxel_phase.voxel_size,
                    self.voxel_phase.origin,
                    self.material,
                ));
                self.voxel_phase.grid.as_mut().unwrap()
            },
        };

        let kernel_radius = self.particle_phase.kernel_radius;
        let particles = &self.particle_phase.particles;

        for z in 0..grid.resolution.z {
            for y in 0..grid.resolution.y {
                for x in 0..grid.resolution.x {
                    let pos = IVec3::new(x, y, z);
                    let world_center = grid.voxel_to_world(pos);
                    let mut density = FixedPoint::ZERO;
                    let mut temp_sum = FixedPoint::ZERO;
                    let mut count = 0i32;

                    for p in particles.iter().filter(|p| p.active) {
                        let dist = (p.position - world_center).length();
                        if dist < kernel_radius {
                            let weight = FixedPoint::from_f32(poly6_kernel(
                                dist.to_f32(),
                                kernel_radius.to_f32(),
                            ));
                            density += p.mass * weight;
                            temp_sum += p.density;
                            count += 1;
                        }
                    }

                    if density > self.particle_phase.density_threshold {
                        if let Some(voxel) = grid.get_voxel_mut(pos) {
                            voxel.flags.insert(VoxelFlags::ACTIVE);
                            voxel.density = density / FixedPoint::from_f32(particles.len() as f32);
                            if count > 0 {
                                voxel.temperature = temp_sum / FixedPoint::from_i32(count)
                                    * FixedPoint::from_f32(300.0)
                                    + FixedPoint::from_f32(293.0);
                            }
                        }
                    }
                }
            }
        }

        self.active_phase = Phase::Mixed;
        let extent = FixedVec3::new(
            FixedPoint::from_i32(grid.resolution.x) * grid.voxel_size,
            FixedPoint::from_i32(grid.resolution.y) * grid.voxel_size,
            FixedPoint::from_i32(grid.resolution.z) * grid.voxel_size,
        );
        self.voxel_phase.dirty_regions.push(DirtyRegion {
            min: self.voxel_phase.origin,
            max: self.voxel_phase.origin + extent,
            tick_updated: 0,
        });
    }

    pub fn voxels_to_particles(&mut self) {
        let grid = match &self.voxel_phase.grid {
            Some(ref g) => g,
            None => return,
        };

        let mut new_particles = Vec::new();
        let spacing = self.particle_phase.particle_spacing;

        for z in 0..grid.resolution.z {
            for y in 0..grid.resolution.y {
                for x in 0..grid.resolution.x {
                    let pos = IVec3::new(x, y, z);
                    if let Some(voxel) = grid.get_voxel(pos) {
                        if voxel.flags.contains(VoxelFlags::ACTIVE)
                            && !voxel.flags.contains(VoxelFlags::DESTROYED)
                        {
                            let has_empty_neighbor = Self::has_empty_neighbor(grid, pos);
                            let is_boundary = x == 0
                                || x == grid.resolution.x - 1
                                || y == 0
                                || y == grid.resolution.y - 1
                                || z == 0
                                || z == grid.resolution.z - 1;

                            if has_empty_neighbor || is_boundary {
                                let world_pos = grid.voxel_to_world(pos);
                                new_particles.push(DualPhaseParticle {
                                    position: world_pos,
                                    velocity: FixedVec3::ZERO,
                                    density: voxel.density,
                                    pressure: FixedPoint::ZERO,
                                    mass: voxel.density * spacing.powi(3),
                                    radius: spacing * FixedPoint::from_f32(0.5),
                                    active: true,
                                    chemical_state: [
                                        voxel.temperature / FixedPoint::from_f32(1000.0),
                                        voxel.radiation_level / FixedPoint::from_f32(1000.0),
                                        FixedPoint::from_f32(voxel.chemical_stain as f32 / 255.0),
                                        voxel.health / voxel.max_health,
                                    ],
                                });
                            }
                        }
                    }
                }
            }
        }

        for z in 1..grid.resolution.z - 1 {
            for y in 1..grid.resolution.y - 1 {
                for x in 1..grid.resolution.x - 1 {
                    let pos = IVec3::new(x, y, z);
                    if let Some(voxel) = grid.get_voxel(pos) {
                        if voxel.flags.contains(VoxelFlags::ACTIVE)
                            && !voxel.flags.contains(VoxelFlags::DESTROYED)
                            && !Self::has_empty_neighbor(grid, pos)
                        {
                            let random_offset = FixedPoint::from_f32(
                                ((x + y * 13 + z * 37) as f32 * 0.618034) % 1.0,
                            );
                            if random_offset < FixedPoint::from_f32(0.1) {
                                let world_pos = grid.voxel_to_world(pos);
                                new_particles.push(DualPhaseParticle {
                                    position: world_pos,
                                    velocity: FixedVec3::ZERO,
                                    density: voxel.density,
                                    pressure: FixedPoint::ZERO,
                                    mass: voxel.density * spacing.powi(3),
                                    radius: spacing * FixedPoint::from_f32(0.3),
                                    active: true,
                                    chemical_state: [
                                        voxel.temperature / FixedPoint::from_f32(1000.0),
                                        voxel.radiation_level / FixedPoint::from_f32(1000.0),
                                        FixedPoint::from_f32(voxel.chemical_stain as f32 / 255.0),
                                        voxel.health / voxel.max_health,
                                    ],
                                });
                            }
                        }
                    }
                }
            }
        }

        self.particle_phase.total_mass =
            new_particles.iter().fold(FixedPoint::ZERO, |a, p| a + p.mass);
        self.particle_phase.particles = new_particles;
        self.active_phase = Phase::Mixed;
    }

    pub fn sync_phases(&mut self, tick: u64) {
        if (tick - self.last_sync_tick) < self.sync_threshold.to_f32() as u64 {
            return;
        }
        self.last_sync_tick = tick;

        match self.priority {
            PhasePriority::ParticleDominant => {
                self.particles_to_voxels();
            },
            PhasePriority::VoxelDominant => {
                self.voxels_to_particles();
            },
            PhasePriority::Balanced => {
                if tick.is_multiple_of(2) {
                    self.particles_to_voxels();
                } else {
                    self.voxels_to_particles();
                }
            },
        }
    }

    pub fn mark_dirty_region(&mut self, min: FixedVec3, max: FixedVec3, tick: u64) {
        self.voxel_phase.dirty_regions.push(DirtyRegion { min, max, tick_updated: tick });
    }

    pub fn determine_phase_transition(&self) -> Option<PhaseTransition> {
        let solid_voxel_ratio = if let Some(ref grid) = self.voxel_phase.grid {
            FixedPoint::from_f32(grid.active_voxel_count() as f32 / grid.voxels.len() as f32)
        } else {
            FixedPoint::ZERO
        };

        let particle_count = FixedPoint::from_f32(self.particle_phase.particles.len() as f32);
        let average_spacing = if particle_count > FixedPoint::ZERO {
            self.particle_spacing_estimate()
        } else {
            self.particle_phase.particle_spacing * FixedPoint::from_f32(10.0)
        };

        if solid_voxel_ratio > FixedPoint::from_f32(0.8) && particle_count == FixedPoint::ZERO {
            Some(PhaseTransition::VoxelToParticle)
        } else if average_spacing < self.particle_phase.particle_spacing * FixedPoint::from_f32(1.2)
            && particle_count > FixedPoint::from_f32(10.0)
            && solid_voxel_ratio < FixedPoint::from_f32(0.3)
        {
            Some(PhaseTransition::ParticleToVoxel)
        } else {
            None
        }
    }

    fn particle_spacing_estimate(&self) -> FixedPoint {
        let particles = &self.particle_phase.particles;
        if particles.len() < 2 {
            return FixedPoint::MAX;
        }

        let mut total_dist = FixedPoint::ZERO;
        let mut pair_count = 0i32;

        for i in 0..particles.len().min(100) {
            let mut min_dist = FixedPoint::MAX;
            for j in 0..particles.len().min(100) {
                if i != j {
                    let dist = (particles[i].position - particles[j].position).length();
                    if dist < min_dist {
                        min_dist = dist;
                    }
                }
            }
            if min_dist < FixedPoint::MAX {
                total_dist += min_dist;
                pair_count += 1;
            }
        }

        if pair_count > 0 {
            total_dist / FixedPoint::from_i32(pair_count)
        } else {
            self.particle_phase.particle_spacing
        }
    }

    fn has_empty_neighbor(grid: &VoxelGrid, pos: IVec3) -> bool {
        let neighbors = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        for dir in &neighbors {
            let npos = pos + *dir;
            if npos.x < 0
                || npos.x >= grid.resolution.x
                || npos.y < 0
                || npos.y >= grid.resolution.y
                || npos.z < 0
                || npos.z >= grid.resolution.z
            {
                return true;
            }
            if let Some(v) = grid.get_voxel(npos) {
                if !v.flags.contains(VoxelFlags::ACTIVE) || v.flags.contains(VoxelFlags::DESTROYED)
                {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseTransition {
    ParticleToVoxel,
    VoxelToParticle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPhaseManager {
    pub entities: Vec<DualPhaseEntity>,
    pub max_entities: usize,
    pub tick: u64,
}

impl DualPhaseManager {
    pub fn new(max_entities: usize) -> Self {
        Self { entities: Vec::with_capacity(max_entities), max_entities, tick: 0 }
    }

    pub fn add_entity(&mut self, entity: DualPhaseEntity) -> Option<Uuid> {
        if self.entities.len() >= self.max_entities {
            return None;
        }
        let id = entity.id;
        self.entities.push(entity);
        Some(id)
    }

    pub fn step(&mut self) {
        self.tick += 1;

        for entity in &mut self.entities {
            entity.sync_phases(self.tick);

            if let Some(transition) = entity.determine_phase_transition() {
                match transition {
                    PhaseTransition::ParticleToVoxel => {
                        entity.particles_to_voxels();
                    },
                    PhaseTransition::VoxelToParticle => {
                        entity.voxels_to_particles();
                    },
                }
            }
        }
    }

    pub fn find_at_position(&self, world_pos: FixedVec3) -> Vec<Uuid> {
        self.entities
            .iter()
            .filter(|e| {
                if let Some(ref grid) = e.voxel_phase.grid {
                    grid.world_to_voxel(world_pos)
                        .and_then(|vp| grid.get_voxel(vp))
                        .map(|v| v.flags.contains(VoxelFlags::ACTIVE))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .map(|e| e.id)
            .collect()
    }

    pub fn active_voxel_count(&self) -> usize {
        self.entities
            .iter()
            .filter_map(|e| e.voxel_phase.grid.as_ref())
            .map(|g| g.active_voxel_count())
            .sum()
    }

    pub fn total_particle_count(&self) -> usize {
        self.entities
            .iter()
            .map(|e| e.particle_phase.particles.iter().filter(|p| p.active).count())
            .sum()
    }
}

fn poly6_kernel(r: f32, h: f32) -> f32 {
    if r >= h {
        return 0.0;
    }
    let h2 = h * h;
    let r2 = r * r;
    let coeff = 315.0 / (64.0 * std::f32::consts::PI * h.powi(9));
    coeff * (h2 - r2).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_phase_creation() {
        let material = MaterialProperties::concrete();
        let entity =
            DualPhaseEntity::new(material, IVec3::splat(8), FixedPoint::ONE, FixedVec3::ZERO);
        assert!(entity.voxel_phase.grid.is_some());
        assert_eq!(entity.active_phase, Phase::Voxel);
    }

    #[test]
    fn test_voxel_to_particles() {
        let material = MaterialProperties::concrete();
        let mut entity =
            DualPhaseEntity::new(material, IVec3::splat(8), FixedPoint::ONE, FixedVec3::ZERO);
        entity.voxels_to_particles();
        assert!(!entity.particle_phase.particles.is_empty());
    }

    #[test]
    fn test_particles_to_voxels() {
        let material = MaterialProperties::concrete();
        let mut entity =
            DualPhaseEntity::new(material, IVec3::splat(8), FixedPoint::ONE, FixedVec3::ZERO);
        entity.particle_phase.particles.push(DualPhaseParticle {
            position: FixedVec3::from_f32(4.0, 4.0, 4.0),
            mass: FixedPoint::from_f32(10.0),
            active: true,
            ..Default::default()
        });
        entity.particles_to_voxels();
    }

    #[test]
    fn test_dual_phase_manager() {
        let mut manager = DualPhaseManager::new(100);
        let material = MaterialProperties::concrete();
        let entity =
            DualPhaseEntity::new(material, IVec3::splat(4), FixedPoint::ONE, FixedVec3::ZERO);
        let id = manager.add_entity(entity).unwrap();
        manager.step();

        let found = manager.find_at_position(FixedVec3::from_f32(2.0, 2.0, 2.0));
        assert_eq!(found[0], id);
    }
}
