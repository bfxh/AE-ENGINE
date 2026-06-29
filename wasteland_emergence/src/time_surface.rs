use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSurface {
    pub id: Uuid,
    pub name: String,
    pub base_surface: BaseSurface,
    pub history: Vec<SurfaceEvent>,
    pub cumulative_damage: f32,
    pub surface_age: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseSurface {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub material_hardness: f32,
    pub material_brittleness: f32,
    pub chemical_resistance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEvent {
    pub event_type: SurfaceEventType,
    pub position: Vec3,
    pub time: f32,
    pub intensity: f32,
    pub radius: f32,
    pub direction: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceEventType {
    Impact,
    Scratch,
    Corrosion,
    Erosion,
    HeatDamage,
    RadiationBurn,
    BiologicalColonization,
    FreezeThaw,
    ChemicalEtch,
    Wear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceModification {
    pub deformation: Vec<SurfaceDeformation>,
    pub color_change: Option<[f32; 3]>,
    pub roughness_change: f32,
    pub chemical_alteration: Vec<ChemicalAlteration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceDeformation {
    pub vertex_index: usize,
    pub displacement: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalAlteration {
    pub chemical_id: u32,
    pub concentration_change: f32,
}

impl TimeSurface {
    pub fn new(name: &str, base: BaseSurface) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            base_surface: base,
            history: Vec::new(),
            cumulative_damage: 0.0,
            surface_age: 0.0,
        }
    }

    pub fn record_event(&mut self, event: SurfaceEvent) {
        self.history.push(event);
    }

    pub fn apply_impact(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        impactor_hardness: f32,
        impactor_mass: f32,
        time: f32,
    ) -> SurfaceModification {
        let speed = velocity.length();
        let kinetic_energy = 0.5 * impactor_mass * speed * speed;
        let hardness_ratio = impactor_hardness / self.base_surface.material_hardness;

        let deformation_radius = (kinetic_energy * hardness_ratio / 10.0).sqrt().min(5.0);
        let max_depth = (kinetic_energy * hardness_ratio / 100.0).min(2.0);

        let direction = if speed > 0.001 { velocity.normalize() } else { -Vec3::Y };

        let event = SurfaceEvent {
            event_type: SurfaceEventType::Impact,
            position,
            time,
            intensity: kinetic_energy,
            radius: deformation_radius,
            direction,
        };
        self.history.push(event);

        let mut modifications = Vec::new();
        for (i, vertex) in self.base_surface.vertices.iter().enumerate() {
            let dist = (*vertex - position).length();
            if dist < deformation_radius {
                let falloff = (1.0 - dist / deformation_radius).powi(2);
                let displacement = direction * max_depth * falloff;
                modifications.push(SurfaceDeformation { vertex_index: i, displacement });
            }
        }

        self.cumulative_damage += kinetic_energy * 0.001;
        self.surface_age += time;

        SurfaceModification {
            deformation: modifications,
            color_change: None,
            roughness_change: (max_depth / 2.0).min(0.5),
            chemical_alteration: Vec::new(),
        }
    }

    pub fn apply_corrosion(
        &mut self,
        position: Vec3,
        agent_intensity: f32,
        duration: f32,
        time: f32,
    ) -> SurfaceModification {
        let effective_depth = agent_intensity * duration * self.base_surface.chemical_resistance;
        let radius = 2.0 + agent_intensity;

        let event = SurfaceEvent {
            event_type: SurfaceEventType::Corrosion,
            position,
            time,
            intensity: agent_intensity,
            radius,
            direction: Vec3::ZERO,
        };
        self.history.push(event);

        let mut modifications = Vec::new();
        for (i, vertex) in self.base_surface.vertices.iter().enumerate() {
            let dist = (*vertex - position).length();
            if dist < radius {
                let falloff = (1.0 - dist / radius).powi(3);
                let depth = effective_depth * falloff;
                modifications
                    .push(SurfaceDeformation { vertex_index: i, displacement: -Vec3::Y * depth });
            }
        }

        self.surface_age += time;

        SurfaceModification {
            deformation: modifications,
            color_change: Some([0.6 + agent_intensity * 0.4, 0.3 + agent_intensity * 0.2, 0.1]),
            roughness_change: (agent_intensity * duration * 0.2).min(0.5),
            chemical_alteration: vec![ChemicalAlteration {
                chemical_id: 0,
                concentration_change: agent_intensity * duration,
            }],
        }
    }

    pub fn apply_scratch(
        &mut self,
        start: Vec3,
        end: Vec3,
        depth: f32,
        width: f32,
        time: f32,
    ) -> SurfaceModification {
        let direction = (end - start).normalize();
        let length = (end - start).length();

        let event = SurfaceEvent {
            event_type: SurfaceEventType::Scratch,
            position: (start + end) * 0.5,
            time,
            intensity: depth,
            radius: width,
            direction,
        };
        self.history.push(event);

        let mut modifications = Vec::new();
        for (i, vertex) in self.base_surface.vertices.iter().enumerate() {
            let to_vertex = *vertex - start;
            let t = to_vertex.dot(direction).clamp(0.0, length);
            let closest_point = start + direction * t;
            let dist = (*vertex - closest_point).length();

            if dist < width {
                let falloff = 1.0 - dist / width;
                let scratch_depth = depth * falloff * falloff;
                modifications.push(SurfaceDeformation {
                    vertex_index: i,
                    displacement: -Vec3::Y * scratch_depth,
                });
            }
        }

        self.surface_age += time;

        SurfaceModification {
            deformation: modifications,
            color_change: None,
            roughness_change: 0.1,
            chemical_alteration: Vec::new(),
        }
    }

    pub fn apply_erosion(
        &mut self,
        positions: &[Vec3],
        particle_size: f32,
        particle_hardness: f32,
        time: f32,
    ) -> SurfaceModification {
        let mut modifications = Vec::new();
        let radius = particle_size * 3.0;

        for pos in positions {
            let event = SurfaceEvent {
                event_type: SurfaceEventType::Erosion,
                position: *pos,
                time,
                intensity: particle_hardness,
                radius,
                direction: Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0,
                    -0.5 - rand::random::<f32>(),
                    (rand::random::<f32>() - 0.5) * 2.0,
                )
                .normalize(),
            };
            self.history.push(event);
        }

        for (i, vertex) in self.base_surface.vertices.iter().enumerate() {
            let mut total_erosion = 0.0f32;

            for pos in positions {
                let dist = (*vertex - *pos).length();
                if dist < radius {
                    let falloff = (1.0 - dist / radius).powi(2);
                    total_erosion += particle_size * particle_hardness * falloff;
                }
            }

            if total_erosion > 0.0 {
                modifications.push(SurfaceDeformation {
                    vertex_index: i,
                    displacement: -Vec3::Y * total_erosion * 0.01,
                });
            }
        }

        self.surface_age += time;

        SurfaceModification {
            deformation: modifications,
            color_change: None,
            roughness_change: (particle_size * positions.len() as f32 * 0.01).min(0.3),
            chemical_alteration: Vec::new(),
        }
    }

    pub fn reconstruct_surface(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut vertices = self.base_surface.vertices.clone();
        let mut total_deformations = vec![Vec3::ZERO; vertices.len()];

        for event in &self.history {
            match event.event_type {
                SurfaceEventType::Impact
                | SurfaceEventType::Corrosion
                | SurfaceEventType::Scratch
                | SurfaceEventType::Erosion => {
                    for (i, vertex) in vertices.iter().enumerate() {
                        let dist = (*vertex - event.position).length();
                        if dist < event.radius {
                            let falloff = (1.0 - dist / event.radius).powi(2);
                            if event.event_type == SurfaceEventType::Impact {
                                total_deformations[i] -=
                                    event.direction * event.intensity * falloff * 0.001;
                            } else {
                                total_deformations[i] -= Vec3::Y * event.intensity * falloff * 0.01;
                            }
                        }
                    }
                },
                _ => {},
            }
        }

        for (i, deformation) in total_deformations.iter().enumerate() {
            vertices[i] += *deformation;
        }

        (vertices, self.base_surface.normals.clone())
    }

    pub fn get_damage_heatmap(&self) -> Vec<(Vec3, f32)> {
        self.history.iter().map(|event| (event.position, event.intensity)).collect()
    }

    pub fn total_events(&self) -> usize {
        self.history.len()
    }
}
