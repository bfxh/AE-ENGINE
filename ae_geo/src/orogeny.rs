use crate::rocks::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubductionZone {
    pub position: Vec3,
    pub dip_angle: f32,
    pub convergence_rate: f32,
    pub slab_age_ma: f32,
    pub slab_temperature: f32,
    pub sediment_thickness: f32,
    pub locked: bool,
    pub accumulated_strain: f32,
}

impl SubductionZone {
    pub fn new(position: Vec3, dip_angle: f32, convergence_rate: f32) -> Self {
        Self {
            position,
            dip_angle: dip_angle.clamp(10.0, 90.0),
            convergence_rate,
            slab_age_ma: 50.0,
            slab_temperature: 500.0,
            sediment_thickness: 1.0,
            locked: false,
            accumulated_strain: 0.0,
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.slab_age_ma += dt / (365.25 * 24.0 * 3600.0 * 1e6);
        let depth = self.slab_temperature / 25.0;
        let heating_rate = 0.033 * depth;
        self.slab_temperature += heating_rate * dt / (365.25 * 24.0 * 3600.0);

        self.accumulated_strain += self.convergence_rate * dt;
        if self.accumulated_strain > 5.0 && !self.locked {
            self.locked = true;
        }
    }

    pub fn release_earthquake(&mut self) -> Option<SubductionQuake> {
        if self.locked && self.accumulated_strain > 3.0 {
            let magnitude = 6.0 + (self.accumulated_strain / 5.0).log10() * 2.0;
            let quake = SubductionQuake {
                magnitude: magnitude.min(9.5),
                depth: self.dip_angle.sin() * 100.0,
                position: self.position,
                slip: self.accumulated_strain,
            };
            self.accumulated_strain = 0.0;
            self.locked = false;
            Some(quake)
        } else {
            None
        }
    }

    pub fn melt_fraction_at_depth(&self, depth: f32) -> f32 {
        let pressure = depth * 3.3e7;
        let solidus = 1100.0 + pressure * 1.2e-7;
        let liquidus = solidus + 400.0;
        if self.slab_temperature < solidus {
            return 0.0;
        }
        if self.slab_temperature > liquidus {
            return 1.0;
        }
        (self.slab_temperature - solidus) / (liquidus - solidus)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubductionQuake {
    pub magnitude: f32,
    pub depth: f32,
    pub position: Vec3,
    pub slip: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcanicSystem {
    pub position: Vec3,
    pub magma_chamber_depth: f32,
    pub magma_volume: f32,
    pub magma_viscosity: f32,
    pub silica_content: f32,
    pub gas_content: f32,
    pub pressure: f32,
    pub eruption_countdown: f32,
    pub is_active: bool,
    pub vent_diameter: f32,
    pub conduit_blocked: bool,
}

impl VolcanicSystem {
    pub fn new(position: Vec3, magma_chamber_depth: f32, silica_content: f32) -> Self {
        let viscosity = 1e3 * 10.0f32.powf(silica_content * 4.0);
        Self {
            position,
            magma_chamber_depth,
            magma_volume: 1e9,
            magma_viscosity: viscosity,
            silica_content,
            gas_content: 0.03,
            pressure: 1e8,
            eruption_countdown: 1000.0,
            is_active: false,
            vent_diameter: 100.0,
            conduit_blocked: false,
        }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.is_active {
            return;
        }

        let lithostatic_pressure = self.magma_chamber_depth * 2700.0 * 9.81;
        let gas_pressure = self.gas_content * self.pressure * 10.0;
        let overpressure = gas_pressure - lithostatic_pressure;

        if overpressure > 0.0 && !self.conduit_blocked {
            self.eruption_countdown -= dt;
            self.pressure += overpressure * dt * 0.001;
        } else {
            self.pressure += (self.magma_volume / 1e9) * dt * 1e5;
        }

        if self.eruption_countdown <= 0.0 && !self.conduit_blocked {
            self.erupt();
        }
    }

    pub fn erupt(&mut self) -> Eruption {
        let vei = (self.magma_volume / 1e4).log10().clamp(0.0, 8.0);
        let eruption_type = if self.silica_content > 0.65 {
            EruptionType::Plinian
        } else if self.silica_content > 0.55 {
            EruptionType::Vulcanian
        } else if self.silica_content > 0.48 {
            EruptionType::Strombolian
        } else {
            EruptionType::Hawaiian
        };

        let ejecta_volume = 10.0f32.powf(vei) * 1e4;
        self.magma_volume = (self.magma_volume - ejecta_volume).max(0.0);
        self.pressure *= 0.1;
        self.eruption_countdown = 1000.0;
        self.gas_content *= 0.5;

        Eruption {
            vei,
            eruption_type,
            ejecta_volume,
            ash_column_height: 2.0 * vei * 1000.0,
            position: self.position,
            lava_composition: if self.silica_content > 0.63 {
                LavaType::Rhyolitic
            } else if self.silica_content > 0.52 {
                LavaType::Andesitic
            } else {
                LavaType::Basaltic
            },
        }
    }

    pub fn magma_supply(&mut self, amount: f32) {
        self.magma_volume += amount;
        self.pressure += amount / 1e6 * 1e8;
    }

    pub fn crystallize(&mut self, fraction: f32) {
        let crystallized = self.magma_volume * fraction;
        self.magma_volume -= crystallized;
        self.silica_content = (self.silica_content + fraction * 0.1).min(0.75);
        self.magma_viscosity = 1e3 * 10.0f32.powf(self.silica_content * 4.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EruptionType {
    Hawaiian,
    Strombolian,
    Vulcanian,
    Plinian,
    Phreatomagmatic,
    Fissure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LavaType {
    Basaltic,
    Andesitic,
    Rhyolitic,
    Komatiitic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eruption {
    pub vei: f32,
    pub eruption_type: EruptionType,
    pub ejecta_volume: f32,
    pub ash_column_height: f32,
    pub position: Vec3,
    pub lava_composition: LavaType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountainBuilder {
    pub position: Vec3,
    pub height: f32,
    pub width: f32,
    pub uplift_rate: f32,
    pub erosion_rate: f32,
    pub age_ma: f32,
    pub rock_type: RockType,
    pub fold_amplitude: f32,
    pub fold_wavelength: f32,
    pub is_folding: bool,
    pub is_faulting: bool,
    pub fault_displacement: f32,
}

impl MountainBuilder {
    pub fn new(position: Vec3, uplift_rate: f32, rock_type: RockType) -> Self {
        Self {
            position,
            height: 0.0,
            width: 10000.0,
            uplift_rate,
            erosion_rate: 0.001,
            age_ma: 0.0,
            rock_type,
            fold_amplitude: 0.0,
            fold_wavelength: 5000.0,
            is_folding: false,
            is_faulting: false,
            fault_displacement: 0.0,
        }
    }

    pub fn step(&mut self, dt: f32) {
        let dt_ma = dt / (365.25 * 24.0 * 3600.0 * 1e6);
        self.age_ma += dt_ma;

        let effective_uplift = self.uplift_rate * (1.0 - (self.age_ma / 100.0).min(0.95));
        self.height += effective_uplift * dt;

        let erosion = self.height * self.erosion_rate * self.rock_type.erosion_resistance() * dt;
        self.height = (self.height - erosion).max(0.0);

        if self.is_folding {
            self.fold_amplitude += effective_uplift * dt * 0.1;
            self.fold_amplitude = self.fold_amplitude.min(self.height * 0.5);
        }

        if self.is_faulting {
            self.fault_displacement += effective_uplift * dt * 0.5;
        }

        if self.height > 5000.0 {
            self.is_faulting = true;
            self.is_folding = true;
        }
    }

    pub fn height_at(&self, lateral_dist: f32) -> f32 {
        if lateral_dist > self.width * 0.5 {
            return 0.0;
        }

        let base_height = self.height * (1.0 - (lateral_dist / (self.width * 0.5)).powi(2));
        let fold = if self.is_folding {
            self.fold_amplitude
                * (2.0 * std::f32::consts::PI * lateral_dist / self.fold_wavelength).sin()
        } else {
            0.0
        };

        let fault = if self.is_faulting {
            if lateral_dist < self.fault_displacement {
                self.fault_displacement - lateral_dist
            } else {
                0.0
            }
        } else {
            0.0
        };

        (base_height + fold + fault).max(0.0)
    }

    pub fn isostatic_rebound(&mut self, crustal_thickness: f32, dt: f32) {
        let reference_thickness = 35.0;
        let excess = crustal_thickness - reference_thickness;
        if excess > 0.0 {
            let rebound = excess * 0.1 * dt;
            self.height += rebound;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrogenySystem {
    pub subduction_zones: Vec<SubductionZone>,
    pub volcanoes: Vec<VolcanicSystem>,
    pub mountains: Vec<MountainBuilder>,
    pub crustal_thickness: f32,
    pub mantle_heat_flow: f32,
}

impl OrogenySystem {
    pub fn new() -> Self {
        Self {
            subduction_zones: Vec::new(),
            volcanoes: Vec::new(),
            mountains: Vec::new(),
            crustal_thickness: 35.0,
            mantle_heat_flow: 0.065,
        }
    }

    pub fn add_subduction_zone(&mut self, zone: SubductionZone) {
        self.subduction_zones.push(zone);
    }

    pub fn add_volcano(&mut self, volcano: VolcanicSystem) {
        self.volcanoes.push(volcano);
    }

    pub fn add_mountain(&mut self, mountain: MountainBuilder) {
        self.mountains.push(mountain);
    }

    pub fn step(&mut self, dt: f32) -> Vec<Eruption> {
        let mut eruptions = Vec::new();

        for zone in &mut self.subduction_zones {
            zone.step(dt);
            if let Some(quake) = zone.release_earthquake() {
                self.crustal_thickness += quake.slip * 0.01;
            }

            let melt = zone.melt_fraction_at_depth(100.0);
            for volcano in &mut self.volcanoes {
                if (volcano.position - zone.position).length() < 50000.0 && melt > 0.1 {
                    volcano.magma_supply(melt * 1e6 * dt);
                    volcano.is_active = true;
                }
            }
        }

        for volcano in &mut self.volcanoes {
            volcano.step(dt);
            if volcano.eruption_countdown <= 0.0 {
                let eruption = volcano.erupt();
                eruptions.push(eruption);
            }
        }

        for mountain in &mut self.mountains {
            mountain.step(dt);
            mountain.isostatic_rebound(self.crustal_thickness, dt);
        }

        eruptions
    }

    pub fn terrain_height_at(&self, position: Vec3) -> f32 {
        let mut height = 0.0f32;
        for mountain in &self.mountains {
            let dist = (position - mountain.position).length();
            height = height.max(mountain.height_at(dist));
        }
        height
    }

    pub fn volcanic_rock_at(&self, position: Vec3) -> Option<RockType> {
        for volcano in &self.volcanoes {
            if (position - volcano.position).length() < 10000.0 && volcano.is_active {
                return match volcano.silica_content {
                    s if s > 0.63 => Some(RockType::Obsidian),
                    s if s > 0.52 => Some(RockType::Basalt),
                    _ => Some(RockType::Basalt),
                };
            }
        }
        None
    }
}

impl Default for OrogenySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subduction_zone_creation() {
        let zone = SubductionZone::new(Vec3::new(0.0, 0.0, 0.0), 45.0, 0.05);
        assert_eq!(zone.dip_angle, 45.0);
        assert_eq!(zone.convergence_rate, 0.05);
        assert!(!zone.locked);
    }

    #[test]
    fn test_subduction_earthquake() {
        let mut zone = SubductionZone::new(Vec3::ZERO, 30.0, 0.1);
        zone.accumulated_strain = 6.0;
        zone.locked = true;
        let quake = zone.release_earthquake();
        assert!(quake.is_some());
        assert!(quake.unwrap().magnitude >= 6.0);
        assert!(!zone.locked);
        assert_eq!(zone.accumulated_strain, 0.0);
    }

    #[test]
    fn test_volcanic_system() {
        let mut volcano = VolcanicSystem::new(Vec3::new(1000.0, 0.0, 0.0), 5000.0, 0.56);
        assert_eq!(volcano.silica_content, 0.56);
        assert!(volcano.magma_viscosity > 0.0);

        volcano.is_active = true;
        volcano.pressure = 1e10;
        volcano.eruption_countdown = 0.0;
        let eruption = volcano.erupt();
        assert!(eruption.vei >= 0.0);
        assert_eq!(eruption.eruption_type, EruptionType::Vulcanian);
    }

    #[test]
    fn test_mountain_builder() {
        let mut mountain = MountainBuilder::new(Vec3::ZERO, 0.01, RockType::Granite);
        mountain.step(1000.0);
        assert!(mountain.height > 0.0);
        assert!(mountain.age_ma > 0.0);

        let h = mountain.height_at(0.0);
        assert!(h > 0.0);
        let h_edge = mountain.height_at(6000.0);
        assert_eq!(h_edge, 0.0);
    }

    #[test]
    fn test_orogeny_system() {
        let mut orogeny = OrogenySystem::new();
        orogeny.add_subduction_zone(SubductionZone::new(Vec3::ZERO, 45.0, 0.05));
        orogeny.add_volcano(VolcanicSystem::new(Vec3::new(5000.0, 0.0, 0.0), 5000.0, 0.55));
        orogeny.add_mountain(MountainBuilder::new(
            Vec3::new(-5000.0, 0.0, 0.0),
            0.01,
            RockType::Granite,
        ));

        let _eruptions = orogeny.step(30.0);
        assert!(orogeny.subduction_zones[0].accumulated_strain > 0.0);

        let height = orogeny.terrain_height_at(Vec3::ZERO);
        assert!(height >= 0.0);
    }

    #[test]
    fn test_melt_fraction() {
        let mut zone = SubductionZone::new(Vec3::ZERO, 45.0, 0.05);
        zone.slab_temperature = 1600.0;
        let melt = zone.melt_fraction_at_depth(50.0);
        assert!(melt > 0.0);
    }
}
