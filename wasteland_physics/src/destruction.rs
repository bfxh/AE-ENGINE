use glam::IVec3;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::material::MaterialProperties;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelGrid {
    pub resolution: IVec3,
    pub voxel_size: FixedPoint,
    pub origin: FixedVec3,
    pub voxels: Vec<Voxel>,
    pub material: MaterialProperties,
    pub total_mass: FixedPoint,
    pub integrity: FixedPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Voxel {
    pub health: FixedPoint,
    pub max_health: FixedPoint,
    pub density: FixedPoint,
    pub flags: VoxelFlags,
    pub temperature: FixedPoint,
    pub radiation_level: FixedPoint,
    pub chemical_stain: u8,
    pub stress: FixedPoint,
    pub strain: FixedPoint,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VoxelFlags: u8 {
        const ACTIVE = 0b00000001;
        const DAMAGED = 0b00000010;
        const DESTROYED = 0b00000100;
        const ON_FIRE = 0b00001000;
        const IRRADIATED = 0b00010000;
        const CORRODED = 0b00100000;
        const MELTED = 0b01000000;
        const LOAD_BEARING = 0b10000000;
    }
}

impl Serialize for VoxelFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VoxelFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u8::deserialize(deserializer)?;
        Ok(VoxelFlags::from_bits_retain(bits))
    }
}

impl Default for Voxel {
    fn default() -> Self {
        Self {
            health: FixedPoint::from_f32(100.0),
            max_health: FixedPoint::from_f32(100.0),
            density: FixedPoint::ONE,
            flags: VoxelFlags::ACTIVE,
            temperature: FixedPoint::from_f32(293.0),
            radiation_level: FixedPoint::ZERO,
            chemical_stain: 0,
            stress: FixedPoint::ZERO,
            strain: FixedPoint::ZERO,
        }
    }
}

impl VoxelGrid {
    pub fn new(
        resolution: IVec3,
        voxel_size: FixedPoint,
        origin: FixedVec3,
        material: MaterialProperties,
    ) -> Self {
        let total = (resolution.x * resolution.y * resolution.z) as usize;
        let mut voxels = Vec::with_capacity(total);
        for _ in 0..total {
            voxels.push(Voxel {
                max_health: material.tensile_strength * voxel_size,
                health: material.tensile_strength * voxel_size,
                density: material.density / FixedPoint::from_f32(1000.0),
                stress: FixedPoint::ZERO,
                strain: FixedPoint::ZERO,
                ..Default::default()
            });
        }
        let total_volume =
            FixedPoint::from_i32(resolution.x * resolution.y * resolution.z) * voxel_size.powi(3);
        let total_mass = total_volume * material.density;

        Self {
            resolution,
            voxel_size,
            origin,
            voxels,
            material,
            total_mass,
            integrity: FixedPoint::ONE,
        }
    }

    pub fn world_to_voxel(&self, world_pos: FixedVec3) -> Option<IVec3> {
        let local = world_pos - self.origin;
        let vx = local.x / self.voxel_size;
        let vy = local.y / self.voxel_size;
        let vz = local.z / self.voxel_size;
        let v = IVec3::new(
            vx.floor().to_f32() as i32,
            vy.floor().to_f32() as i32,
            vz.floor().to_f32() as i32,
        );
        if v.x >= 0
            && v.x < self.resolution.x
            && v.y >= 0
            && v.y < self.resolution.y
            && v.z >= 0
            && v.z < self.resolution.z
        {
            Some(v)
        } else {
            None
        }
    }

    pub fn voxel_to_world(&self, voxel: IVec3) -> FixedVec3 {
        let half = self.voxel_size * FixedPoint::from_f32(0.5);
        FixedVec3::new(
            FixedPoint::from_i32(voxel.x) * self.voxel_size + half,
            FixedPoint::from_i32(voxel.y) * self.voxel_size + half,
            FixedPoint::from_i32(voxel.z) * self.voxel_size + half,
        ) + self.origin
    }

    fn index(&self, pos: IVec3) -> usize {
        (pos.z * self.resolution.y * self.resolution.x + pos.y * self.resolution.x + pos.x) as usize
    }

    pub fn get_voxel(&self, pos: IVec3) -> Option<&Voxel> {
        if pos.x < 0
            || pos.x >= self.resolution.x
            || pos.y < 0
            || pos.y >= self.resolution.y
            || pos.z < 0
            || pos.z >= self.resolution.z
        {
            return None;
        }
        Some(&self.voxels[self.index(pos)])
    }

    pub fn get_voxel_mut(&mut self, pos: IVec3) -> Option<&mut Voxel> {
        if pos.x < 0
            || pos.x >= self.resolution.x
            || pos.y < 0
            || pos.y >= self.resolution.y
            || pos.z < 0
            || pos.z >= self.resolution.z
        {
            return None;
        }
        let idx = self.index(pos);
        Some(&mut self.voxels[idx])
    }

    pub fn damage_sphere(
        &mut self,
        center: FixedVec3,
        radius: FixedPoint,
        damage: FixedPoint,
    ) -> Vec<IVec3> {
        let center_voxel = match self.world_to_voxel(center) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let ratio = radius / self.voxel_size;
        let frac = ratio.fract();
        let ceil = if frac > FixedPoint::ZERO { ratio.floor() + FixedPoint::ONE } else { ratio };
        let voxel_radius = ceil.to_f32() as i32 + 1;
        let mut destroyed = Vec::new();

        for x in (center_voxel.x - voxel_radius)..=(center_voxel.x + voxel_radius) {
            for y in (center_voxel.y - voxel_radius)..=(center_voxel.y + voxel_radius) {
                for z in (center_voxel.z - voxel_radius)..=(center_voxel.z + voxel_radius) {
                    let pos = IVec3::new(x, y, z);
                    let world_pos = self.voxel_to_world(pos);
                    let dist = (world_pos - center).length();
                    if dist > radius {
                        continue;
                    }
                    let falloff = FixedPoint::ONE - (dist / radius);
                    let effective_damage = damage * falloff * falloff;

                    if let Some(voxel) = self.get_voxel_mut(pos) {
                        if voxel.flags.contains(VoxelFlags::DESTROYED) {
                            continue;
                        }
                        voxel.health -= effective_damage;
                        voxel.flags.insert(VoxelFlags::DAMAGED);
                        if voxel.health <= FixedPoint::ZERO {
                            voxel.health = FixedPoint::ZERO;
                            voxel.flags.remove(VoxelFlags::ACTIVE);
                            voxel.flags.insert(VoxelFlags::DESTROYED);
                            destroyed.push(pos);
                        }
                    }
                }
            }
        }
        self.recalculate_integrity();
        destroyed
    }

    pub fn damage_line(
        &mut self,
        start: FixedVec3,
        end: FixedVec3,
        radius: FixedPoint,
        damage: FixedPoint,
    ) -> Vec<IVec3> {
        let dir = end - start;
        let length = dir.length();
        if length < FixedPoint::EPSILON {
            return self.damage_sphere(start, radius, damage);
        }
        let dir = dir / length;
        let ratio = length / (self.voxel_size * FixedPoint::from_f32(0.5));
        let frac = ratio.fract();
        let ceil = if frac > FixedPoint::ZERO { ratio.floor() + FixedPoint::ONE } else { ratio };
        let steps = ceil.to_f32() as usize;
        let mut destroyed = Vec::new();

        for i in 0..=steps {
            let t = FixedPoint::from_f32(i as f32) / FixedPoint::from_f32(steps as f32);
            let point = start + dir * length * t;
            let step_damage = damage / FixedPoint::from_f32(steps as f32);
            let new_destroyed = self.damage_sphere(point, radius, step_damage);
            destroyed.extend(new_destroyed);
        }
        destroyed.sort_by_key(|v| (v.x, v.y, v.z));
        destroyed.dedup();
        destroyed
    }

    pub fn structural_collapse(&mut self) -> Vec<IVec3> {
        let mut collapsed = Vec::new();
        let _gravity = FixedVec3::new(FixedPoint::ZERO, -FixedPoint::ONE, FixedPoint::ZERO);

        for x in 0..self.resolution.x {
            for z in 0..self.resolution.z {
                let mut has_support = false;
                for y in 0..self.resolution.y {
                    let pos = IVec3::new(x, y, z);
                    let voxel = match self.get_voxel(pos) {
                        Some(v) => v,
                        None => continue,
                    };
                    if voxel.flags.contains(VoxelFlags::DESTROYED) {
                        has_support = false;
                        continue;
                    }
                    if y == 0 || has_support {
                        has_support = true;
                        continue;
                    }
                    let below = self.get_voxel(IVec3::new(x, y - 1, z));
                    let has_below = below.is_none_or(|v| !v.flags.contains(VoxelFlags::DESTROYED));
                    if !has_below {
                        if let Some(v) = self.get_voxel_mut(pos) {
                            v.flags.insert(VoxelFlags::DESTROYED);
                            v.flags.remove(VoxelFlags::ACTIVE);
                            collapsed.push(pos);
                        }
                    }
                }
            }
        }
        self.recalculate_integrity();
        collapsed
    }

    fn recalculate_integrity(&mut self) {
        let active_count =
            self.voxels.iter().filter(|v| v.flags.contains(VoxelFlags::ACTIVE)).count();
        let total = self.voxels.len();
        self.integrity = if total > 0 {
            FixedPoint::from_f32(active_count as f32 / total as f32)
        } else {
            FixedPoint::ZERO
        };
    }

    pub fn apply_heat(
        &mut self,
        position: FixedVec3,
        radius: FixedPoint,
        temperature: FixedPoint,
        duration: FixedPoint,
    ) {
        let center_voxel = match self.world_to_voxel(position) {
            Some(v) => v,
            None => return,
        };
        let ratio = radius / self.voxel_size;
        let frac = ratio.fract();
        let ceil = if frac > FixedPoint::ZERO { ratio.floor() + FixedPoint::ONE } else { ratio };
        let voxel_radius = ceil.to_f32() as i32 + 1;
        let melting_point = self.material.melting_point;
        let flammability = self.material.flammability;

        for x in (center_voxel.x - voxel_radius)..=(center_voxel.x + voxel_radius) {
            for y in (center_voxel.y - voxel_radius)..=(center_voxel.y + voxel_radius) {
                for z in (center_voxel.z - voxel_radius)..=(center_voxel.z + voxel_radius) {
                    let pos = IVec3::new(x, y, z);
                    let world_pos = self.voxel_to_world(pos);
                    let dist = (world_pos - position).length();
                    if dist > radius {
                        continue;
                    }
                    let falloff = FixedPoint::ONE - (dist / radius);
                    if let Some(voxel) = self.get_voxel_mut(pos) {
                        voxel.temperature += (temperature - voxel.temperature)
                            * falloff
                            * duration
                            * FixedPoint::from_f32(0.1);
                        if voxel.temperature > melting_point {
                            voxel.flags.insert(VoxelFlags::MELTED);
                        }
                        if flammability > FixedPoint::ZERO
                            && voxel.temperature > FixedPoint::from_f32(500.0)
                        {
                            voxel.flags.insert(VoxelFlags::ON_FIRE);
                        }
                    }
                }
            }
        }
    }

    pub fn apply_radiation(
        &mut self,
        position: FixedVec3,
        radius: FixedPoint,
        rads_per_second: FixedPoint,
        duration: FixedPoint,
    ) {
        let center_voxel = match self.world_to_voxel(position) {
            Some(v) => v,
            None => return,
        };
        let ratio = radius / self.voxel_size;
        let frac = ratio.fract();
        let ceil = if frac > FixedPoint::ZERO { ratio.floor() + FixedPoint::ONE } else { ratio };
        let voxel_radius = ceil.to_f32() as i32 + 1;
        let total_dose = rads_per_second * duration;

        for x in (center_voxel.x - voxel_radius)..=(center_voxel.x + voxel_radius) {
            for y in (center_voxel.y - voxel_radius)..=(center_voxel.y + voxel_radius) {
                for z in (center_voxel.z - voxel_radius)..=(center_voxel.z + voxel_radius) {
                    let pos = IVec3::new(x, y, z);
                    let world_pos = self.voxel_to_world(pos);
                    let dist = (world_pos - position).length();
                    if dist > radius {
                        continue;
                    }
                    let falloff = FixedPoint::ONE - (dist / radius);
                    let absorbed = total_dose
                        * falloff
                        * (FixedPoint::ONE - self.material.radiation_resistance);
                    if let Some(voxel) = self.get_voxel_mut(pos) {
                        voxel.radiation_level += absorbed;
                        voxel.flags.insert(VoxelFlags::IRRADIATED);
                        voxel.health -= absorbed * FixedPoint::from_f32(0.01);
                    }
                }
            }
        }
    }

    pub fn active_voxel_count(&self) -> usize {
        self.voxels.iter().filter(|v| v.flags.contains(VoxelFlags::ACTIVE)).count()
    }

    pub fn destroyed_voxel_count(&self) -> usize {
        self.voxels.iter().filter(|v| v.flags.contains(VoxelFlags::DESTROYED)).count()
    }

    pub fn fracture_propagate(&mut self, origin: IVec3, force: FixedPoint) -> Vec<IVec3> {
        let mut newly_destroyed = Vec::new();
        let toughness = self.material.toughness;
        let shear = self.material.shear_strength;
        let max_radius = (force / shear).to_f32().ceil() as i32;

        let mut queue = VecDeque::new();
        let mut visited = vec![false; self.voxels.len()];
        let origin_idx = self.index(origin);
        visited[origin_idx] = true;
        queue.push_back((origin, force));

        let neighbors = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
            IVec3::new(1, 1, 0),
            IVec3::new(1, -1, 0),
            IVec3::new(-1, 1, 0),
            IVec3::new(-1, -1, 0),
            IVec3::new(1, 0, 1),
            IVec3::new(1, 0, -1),
            IVec3::new(-1, 0, 1),
            IVec3::new(-1, 0, -1),
            IVec3::new(0, 1, 1),
            IVec3::new(0, 1, -1),
            IVec3::new(0, -1, 1),
            IVec3::new(0, -1, -1),
        ];

        let shear_tenth = shear * FixedPoint::from_f32(0.1);
        let toughness_factor = toughness * FixedPoint::from_f32(0.01);
        let elasticity_min = (self.material.elasticity).max(FixedPoint::from_f32(0.01));

        while let Some((pos, remaining_force)) = queue.pop_front() {
            if remaining_force < shear_tenth {
                continue;
            }

            for dir in &neighbors {
                let next = pos + *dir;
                if next.x < 0
                    || next.x >= self.resolution.x
                    || next.y < 0
                    || next.y >= self.resolution.y
                    || next.z < 0
                    || next.z >= self.resolution.z
                {
                    continue;
                }
                let idx = self.index(next);
                if visited[idx] {
                    continue;
                }
                visited[idx] = true;

                let voxel = &mut self.voxels[idx];
                if voxel.flags.contains(VoxelFlags::DESTROYED) {
                    continue;
                }

                let dx = FixedPoint::from_i32((next.x - origin.x).abs());
                let dy = FixedPoint::from_i32((next.y - origin.y).abs());
                let dz = FixedPoint::from_i32((next.z - origin.z).abs());
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                let max_radius_fp = FixedPoint::from_i32(max_radius);
                let falloff = (FixedPoint::ONE - (dist / max_radius_fp)).max(FixedPoint::ZERO);
                let force_falloff =
                    (FixedPoint::ONE - toughness_factor).max(FixedPoint::from_f32(0.1));
                let transmitted_force = remaining_force * falloff * force_falloff;

                voxel.stress += transmitted_force;
                voxel.strain = voxel.stress / elasticity_min;
                let damage = transmitted_force * voxel.strain * FixedPoint::from_f32(0.01);

                voxel.health -= damage;
                voxel.flags.insert(VoxelFlags::DAMAGED);

                if voxel.health <= FixedPoint::ZERO {
                    voxel.health = FixedPoint::ZERO;
                    voxel.flags.remove(VoxelFlags::ACTIVE);
                    voxel.flags.insert(VoxelFlags::DESTROYED);
                    newly_destroyed.push(next);
                    queue.push_back((next, transmitted_force * FixedPoint::from_f32(0.7)));
                } else if transmitted_force > shear {
                    queue.push_back((next, transmitted_force * FixedPoint::from_f32(0.5)));
                }
            }
        }
        self.recalculate_integrity();
        newly_destroyed
    }

    pub fn thermal_conduction_step(&mut self, dt: FixedPoint) {
        let conductivity = self.material.thermal_conductivity;
        if conductivity < FixedPoint::from_f32(1e-6) {
            return;
        }

        let mut new_temps = vec![FixedPoint::ZERO; self.voxels.len()];

        for z in 0..self.resolution.z {
            for y in 0..self.resolution.y {
                for x in 0..self.resolution.x {
                    let idx = self.index(IVec3::new(x, y, z));
                    let current = self.voxels[idx];
                    if current.flags.contains(VoxelFlags::DESTROYED) {
                        new_temps[idx] = current.temperature;
                        continue;
                    }

                    let mut sum_temp = FixedPoint::ZERO;
                    let mut count = 0i32;

                    let neighbor_offsets =
                        [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];

                    for (dx, dy, dz) in &neighbor_offsets {
                        let nx = x + dx;
                        let ny = y + dy;
                        let nz = z + dz;
                        if nx >= 0
                            && nx < self.resolution.x
                            && ny >= 0
                            && ny < self.resolution.y
                            && nz >= 0
                            && nz < self.resolution.z
                        {
                            let n_idx = self.index(IVec3::new(nx, ny, nz));
                            sum_temp += self.voxels[n_idx].temperature;
                            count += 1;
                        }
                    }

                    let avg_neighbor = if count > 0 {
                        sum_temp / FixedPoint::from_i32(count)
                    } else {
                        current.temperature
                    };
                    let diffusion = conductivity * (avg_neighbor - current.temperature) * dt;
                    new_temps[idx] = current.temperature + diffusion;
                }
            }
        }

        for (i, temp) in new_temps.iter().enumerate() {
            self.voxels[i].temperature = *temp;
            if self.voxels[i].temperature > self.material.melting_point {
                self.voxels[i].flags.insert(VoxelFlags::MELTED);
            }
        }
    }

    pub fn structural_analysis(&mut self) -> StructuralReport {
        let mut report = StructuralReport::default();
        let mut visited = vec![false; self.voxels.len()];

        for z in 0..self.resolution.z {
            for y in 0..self.resolution.y {
                for x in 0..self.resolution.x {
                    let idx = self.index(IVec3::new(x, y, z));
                    if visited[idx] || self.voxels[idx].flags.contains(VoxelFlags::DESTROYED) {
                        continue;
                    }
                    let cluster_size = self.flood_fill_cluster(IVec3::new(x, y, z), &mut visited);
                    report
                        .clusters
                        .push(StructuralCluster { start: IVec3::new(x, y, z), size: cluster_size });
                    if cluster_size > report.largest_cluster {
                        report.largest_cluster = cluster_size;
                    }
                }
            }
        }

        report.total_voxels = self.voxels.len();
        report.active_voxels = self.active_voxel_count();
        report.destroyed_voxels = self.destroyed_voxel_count();
        report.integrity = self.integrity;
        report.is_collapsing = self.integrity < FixedPoint::from_f32(0.3);

        report
    }

    fn flood_fill_cluster(&self, start: IVec3, visited: &mut [bool]) -> usize {
        let mut queue = VecDeque::new();
        let start_idx = self.index(start);
        let mut size = 0;

        if visited[start_idx] {
            return 0;
        }
        visited[start_idx] = true;
        queue.push_back(start);
        size += 1;

        let dirs = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        while let Some(pos) = queue.pop_front() {
            for dir in &dirs {
                let next = pos + *dir;
                if next.x < 0
                    || next.x >= self.resolution.x
                    || next.y < 0
                    || next.y >= self.resolution.y
                    || next.z < 0
                    || next.z >= self.resolution.z
                {
                    continue;
                }
                let idx = self.index(next);
                if visited[idx] || self.voxels[idx].flags.contains(VoxelFlags::DESTROYED) {
                    continue;
                }
                visited[idx] = true;
                queue.push_back(next);
                size += 1;
            }
        }
        size
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralReport {
    pub total_voxels: usize,
    pub active_voxels: usize,
    pub destroyed_voxels: usize,
    pub integrity: FixedPoint,
    pub clusters: Vec<StructuralCluster>,
    pub largest_cluster: usize,
    pub is_collapsing: bool,
}

impl Default for StructuralReport {
    fn default() -> Self {
        Self {
            total_voxels: 0,
            active_voxels: 0,
            destroyed_voxels: 0,
            integrity: FixedPoint::ONE,
            clusters: Vec::new(),
            largest_cluster: 0,
            is_collapsing: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralCluster {
    pub start: IVec3,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeformationState {
    Intact,
    PlasticDeformation,
    Fractured,
    Fragmented,
    Pulverized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridDestructionState {
    pub mesh_vertices: Vec<FixedVec3>,
    pub mesh_indices: Vec<[u32; 3]>,
    pub active_voxel_regions: Vec<VoxelActivationRegion>,
    pub deformation_states: Vec<DeformationState>,
    pub yield_strength: FixedPoint,
    pub ultimate_strength: FixedPoint,
    pub fragmented_entities: Vec<FragmentEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelActivationRegion {
    pub origin: IVec3,
    pub resolution: IVec3,
    pub voxel_size: FixedPoint,
    pub world_origin: FixedVec3,
    pub grid: Option<VoxelGrid>,
    pub activation_cause: ActivationCause,
    pub activation_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ActivationCause {
    Impact(FixedPoint),
    ThermalStress,
    Corrosion,
    Explosion,
    StressFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentEntity {
    pub fragment_id: Uuid,
    pub position: FixedVec3,
    pub velocity: FixedVec3,
    pub angular_velocity: FixedVec3,
    pub mass: FixedPoint,
    pub bounding_sphere_radius: FixedPoint,
    pub voxel_region: VoxelActivationRegion,
    pub parent_entity_id: Uuid,
    pub separation_tick: u64,
}

impl HybridDestructionState {
    pub fn new(
        vertices: Vec<FixedVec3>,
        indices: Vec<[u32; 3]>,
        yield_strength: FixedPoint,
        ultimate_strength: FixedPoint,
    ) -> Self {
        Self {
            mesh_vertices: vertices,
            mesh_indices: indices,
            active_voxel_regions: Vec::new(),
            deformation_states: Vec::new(),
            yield_strength,
            ultimate_strength,
            fragmented_entities: Vec::new(),
        }
    }

    pub fn activate_voxel_region(
        &mut self,
        world_center: FixedVec3,
        radius: FixedPoint,
        voxel_size: FixedPoint,
        material: MaterialProperties,
        cause: ActivationCause,
        tick: u64,
    ) -> &VoxelActivationRegion {
        let ratio = (radius * FixedPoint::from_f32(2.0)) / voxel_size;
        let frac = ratio.fract();
        let ceil = if frac > FixedPoint::ZERO { ratio.floor() + FixedPoint::ONE } else { ratio };
        let resolution_dim = ceil.to_f32() as i32 + 2;
        let resolution = IVec3::new(resolution_dim, resolution_dim, resolution_dim);
        let padded = radius + voxel_size;
        let world_origin = FixedVec3::new(
            world_center.x - padded,
            world_center.y - padded,
            world_center.z - padded,
        );

        let mut grid = VoxelGrid::new(resolution, voxel_size, world_origin, material);

        for vertex in &self.mesh_vertices {
            let dist = (FixedVec3::new(vertex.x, vertex.y, vertex.z) - world_center).length();
            if dist < radius {
                if let Some(voxel_pos) = grid.world_to_voxel(*vertex) {
                    if let Some(voxel) = grid.get_voxel_mut(voxel_pos) {
                        voxel.flags.insert(VoxelFlags::ACTIVE);
                    }
                }
            }
        }

        let region = VoxelActivationRegion {
            origin: IVec3::ZERO,
            resolution,
            voxel_size,
            world_origin,
            grid: Some(grid),
            activation_cause: cause,
            activation_tick: tick,
        };

        self.active_voxel_regions.push(region);
        self.deformation_states.push(DeformationState::Intact);
        self.active_voxel_regions.last().unwrap()
    }

    pub fn update_deformation_states(&mut self, tick: u64) -> Vec<FragmentEntity> {
        let mut new_fragments = Vec::new();
        let mut indices_to_split = Vec::new();

        for (i, region) in self.active_voxel_regions.iter_mut().enumerate() {
            if let Some(ref grid) = region.grid {
                let max_stress = grid
                    .voxels
                    .iter()
                    .filter(|v| v.flags.contains(VoxelFlags::ACTIVE))
                    .map(|v| v.stress)
                    .fold(FixedPoint::ZERO, |a, b| if b > a { b } else { a });

                let new_state = if max_stress > self.ultimate_strength {
                    DeformationState::Fragmented
                } else if max_stress > self.yield_strength * FixedPoint::from_f32(1.5) {
                    DeformationState::Fractured
                } else if max_stress > self.yield_strength {
                    DeformationState::PlasticDeformation
                } else {
                    DeformationState::Intact
                };

                if i < self.deformation_states.len() {
                    if self.deformation_states[i] != new_state
                        && new_state == DeformationState::Fragmented
                    {
                        indices_to_split.push(i);
                    }
                    self.deformation_states[i] = new_state;
                }
            }
        }

        for i in indices_to_split {
            let fragment = self.split_fragment(i, tick);
            new_fragments.push(fragment);
        }

        self.fragmented_entities.extend(new_fragments.clone());
        new_fragments
    }

    fn split_fragment(&mut self, region_index: usize, tick: u64) -> FragmentEntity {
        let region = &self.active_voxel_regions[region_index];
        let half = region.voxel_size * FixedPoint::from_f32(0.5);
        let center = region.world_origin
            + FixedVec3::new(
                FixedPoint::from_i32(region.resolution.x) * half,
                FixedPoint::from_i32(region.resolution.y) * half,
                FixedPoint::from_i32(region.resolution.z) * half,
            );

        let mass = if let Some(ref grid) = region.grid {
            FixedPoint::from_f32(grid.active_voxel_count() as f32)
                * region.voxel_size.powi(3)
                * grid.material.density
        } else {
            FixedPoint::ONE
        };

        FragmentEntity {
            fragment_id: Uuid::new_v4(),
            position: center,
            velocity: FixedVec3::new(
                FixedPoint::from_f32((rand::random::<f32>() - 0.5) * 5.0),
                FixedPoint::from_f32(rand::random::<f32>() * 3.0),
                FixedPoint::from_f32((rand::random::<f32>() - 0.5) * 5.0),
            ),
            angular_velocity: FixedVec3::new(
                FixedPoint::from_f32((rand::random::<f32>() - 0.5) * 10.0),
                FixedPoint::from_f32((rand::random::<f32>() - 0.5) * 10.0),
                FixedPoint::from_f32((rand::random::<f32>() - 0.5) * 10.0),
            ),
            mass,
            bounding_sphere_radius: FixedPoint::from_f32(region.resolution.x as f32)
                * region.voxel_size
                * FixedPoint::from_f32(0.5),
            voxel_region: region.clone(),
            parent_entity_id: Uuid::nil(),
            separation_tick: tick,
        }
    }

    pub fn is_voxel_active_at(&self, world_pos: FixedVec3) -> bool {
        for region in &self.active_voxel_regions {
            let local = world_pos - region.world_origin;
            if local.x >= FixedPoint::ZERO
                && local.y >= FixedPoint::ZERO
                && local.z >= FixedPoint::ZERO
                && local.x < FixedPoint::from_i32(region.resolution.x) * region.voxel_size
                && local.y < FixedPoint::from_i32(region.resolution.y) * region.voxel_size
                && local.z < FixedPoint::from_i32(region.resolution.z) * region.voxel_size
            {
                return true;
            }
        }
        false
    }

    pub fn voxel_overhead_ratio(&self) -> FixedPoint {
        if self.mesh_vertices.is_empty() {
            return FixedPoint::ZERO;
        }
        let total_voxel_volume: FixedPoint = self
            .active_voxel_regions
            .iter()
            .map(|r| {
                FixedPoint::from_i32(r.resolution.x * r.resolution.y * r.resolution.z)
                    * r.voxel_size.powi(3)
            })
            .fold(FixedPoint::ZERO, |a, b| a + b);

        let mesh_bounds = if self.mesh_vertices.len() > 1 {
            let first = self.mesh_vertices[0];
            let mut min = FixedVec3::new(first.x, first.y, first.z);
            let mut max = FixedVec3::new(first.x, first.y, first.z);
            for v in &self.mesh_vertices[1..] {
                min = FixedVec3::new(
                    if v.x < min.x { v.x } else { min.x },
                    if v.y < min.y { v.y } else { min.y },
                    if v.z < min.z { v.z } else { min.z },
                );
                max = FixedVec3::new(
                    if v.x > max.x { v.x } else { max.x },
                    if v.y > max.y { v.y } else { max.y },
                    if v.z > max.z { v.z } else { max.z },
                );
            }
            let diff = max - min;
            diff.x * diff.y * diff.z
        } else {
            FixedPoint::ONE
        };

        total_voxel_volume
            / if mesh_bounds > FixedPoint::from_f32(0.001) {
                mesh_bounds
            } else {
                FixedPoint::from_f32(0.001)
            }
    }

    pub fn stats(&self) -> HybridDestructionStats {
        HybridDestructionStats {
            vertex_count: self.mesh_vertices.len(),
            index_count: self.mesh_indices.len(),
            active_voxel_regions: self.active_voxel_regions.len(),
            fragmented_entity_count: self.fragmented_entities.len(),
            intact_regions: self
                .deformation_states
                .iter()
                .filter(|s| **s == DeformationState::Intact)
                .count(),
            plastic_regions: self
                .deformation_states
                .iter()
                .filter(|s| **s == DeformationState::PlasticDeformation)
                .count(),
            fractured_regions: self
                .deformation_states
                .iter()
                .filter(|s| **s == DeformationState::Fractured)
                .count(),
            fragmented_regions: self
                .deformation_states
                .iter()
                .filter(|s| **s == DeformationState::Fragmented)
                .count(),
            voxel_overhead: self.voxel_overhead_ratio(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HybridDestructionStats {
    pub vertex_count: usize,
    pub index_count: usize,
    pub active_voxel_regions: usize,
    pub fragmented_entity_count: usize,
    pub intact_regions: usize,
    pub plastic_regions: usize,
    pub fractured_regions: usize,
    pub fragmented_regions: usize,
    pub voxel_overhead: FixedPoint,
}
