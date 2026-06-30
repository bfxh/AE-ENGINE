#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationDomain {
    #[default]
    Thermal,
    Chemical,
    Mechanical,
    Electromagnetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationState {
    Normal,
    Transitioning,
    Isolated,
    Recovering,
}

#[derive(Debug, Clone, Default)]
pub struct EnergyBundle {
    pub total_energy: f32,
    pub total_momentum: [f32; 3],
    pub total_mass: f32,

    pub fragment_count: u32,
    pub fragment_velocity_mean: [f32; 3],
    pub fragment_velocity_std: f32,

    pub chemical_residue: Vec<(u32, f32)>,
    pub radiation_level: f32,
    pub temperature: f32,

    /// Domain that produced this bundle (for domain-specific recovery effects).
    /// §4.4 extension: enables total_energy → kinetic/charge conversion on recovery.
    pub domain: IsolationDomain,
}

#[derive(Debug, Clone)]
pub struct IsolationZone {
    pub id: u32,
    pub domain: IsolationDomain,
    pub state: IsolationState,
    pub center: [f32; 3],
    pub radius_inner: f32,
    pub radius_outer: f32,
    pub intensity: f32,
    pub energy_bundle: EnergyBundle,
    pub created_tick: u64,
    pub lifetime: f32,
}

impl IsolationZone {
    pub fn weight_at(&self, pos: [f32; 3]) -> f32 {
        let dx = pos[0] - self.center[0];
        let dy = pos[1] - self.center[1];
        let dz = pos[2] - self.center[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist <= self.radius_inner {
            1.0
        } else if dist >= self.radius_outer {
            0.0
        } else {
            let t = (dist - self.radius_inner) / (self.radius_outer - self.radius_inner);
            let t = t * t * (3.0 - 2.0 * t);
            1.0 - t
        }
    }
}

pub struct DomainIsolationManager {
    pub zones: Vec<IsolationZone>,
    pub next_id: u32,

    pub thermal_threshold: f32,
    pub em_field_threshold: f32,

    pub total_zones_created: u32,
    pub total_zones_resolved: u32,
}

impl Default for DomainIsolationManager {
    fn default() -> Self {
        Self {
            zones: Vec::new(),
            next_id: 0,
            thermal_threshold: 5000.0,
            em_field_threshold: 1e8,
            total_zones_created: 0,
            total_zones_resolved: 0,
        }
    }
}

impl DomainIsolationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn detect_and_create(
        &mut self,
        positions: &[[f32; 3]],
        temperatures: &[f32],
        chemical_ids: &[u32],
        strains: &[[f32; 9]],
        charges: &[f32],
        near_indices: &[(usize, f32)],
        tick: u64,
    ) {
        // Chemical domain temperature threshold (Arrhenius approximation).
        // Real reaction_rate would come from chemistry system; temp + chemical_id is a proxy.
        const CHEMICAL_TEMP_THRESHOLD: f32 = 1000.0;
        // Mechanical domain strain intensity threshold.
        // ||F - I||_F > 0.5 means ~50% deformation — extreme mechanical state.
        // True strain_rate (dF/dt) would need F_prev; strain intensity is a proxy.
        const MECHANICAL_STRAIN_THRESHOLD: f32 = 0.5;
        // EM domain uses em_field_threshold (1e8) on |charge| as a proxy.
        // True em_field (V/m) would require per-particle field computation (E = k*q/r²);
        // |charge| is a proxy — extreme charge accumulation implies extreme EM state.
        //
        // §Performance: only scan near_indices (~10k) instead of all particles (~1M).
        // Domain isolation is for extreme local events (explosions, impacts, plasma);
        // far particles (200m+) triggering zones would be invisible to the player and
        // unprocessible by update() which only handles near_indices. 100x speedup.

        for &(i, _) in near_indices {
            if i >= positions.len() || i >= temperatures.len() {
                continue;
            }
            let temp = temperatures[i];
            let pos = positions[i];

            let already_isolated = self.zones.iter().any(|z| {
                let dx = z.center[0] - pos[0];
                let dy = z.center[1] - pos[1];
                let dz = z.center[2] - pos[2];
                dx * dx + dy * dy + dz * dz < z.radius_inner * z.radius_inner
            });
            if already_isolated {
                continue;
            }

            // Thermal domain: extreme heat (>5000K) → plasma equations
            if temp > self.thermal_threshold {
                let zone = IsolationZone {
                    id: self.next_id,
                    domain: IsolationDomain::Thermal,
                    state: IsolationState::Transitioning,
                    center: pos,
                    radius_inner: 2.0,
                    radius_outer: 5.0,
                    intensity: ((temp - self.thermal_threshold) / self.thermal_threshold)
                        .min(1.0),
                    energy_bundle: EnergyBundle {
                        total_energy: temp * 1000.0,
                        temperature: temp,
                        ..Default::default()
                    },
                    created_tick: tick,
                    lifetime: 0.0,
                };
                self.zones.push(zone);
                self.next_id += 1;
                self.total_zones_created += 1;
                continue; // Thermal takes priority, skip Chemical/Mechanical
            }

            // Chemical domain: active reaction (1000K < temp < 5000K + has chemical_id)
            // Approximation: actual reaction_rate field not yet in MpssBuffer;
            // temp + chemical_id presence is a reasonable proxy (Arrhenius k = A*exp(-Ea/RT)).
            if temp > CHEMICAL_TEMP_THRESHOLD
                && i < chemical_ids.len()
                && chemical_ids[i] != 0
            {
                let intensity = ((temp - CHEMICAL_TEMP_THRESHOLD)
                    / (self.thermal_threshold - CHEMICAL_TEMP_THRESHOLD))
                    .min(1.0);
                let zone = IsolationZone {
                    id: self.next_id,
                    domain: IsolationDomain::Chemical,
                    state: IsolationState::Transitioning,
                    center: pos,
                    radius_inner: 1.5,
                    radius_outer: 3.0,
                    intensity,
                    energy_bundle: EnergyBundle {
                        total_energy: temp * 100.0,
                        temperature: temp,
                        chemical_residue: vec![(chemical_ids[i], 1.0)],
                        ..Default::default()
                    },
                    created_tick: tick,
                    lifetime: 0.0,
                };
                self.zones.push(zone);
                self.next_id += 1;
                self.total_zones_created += 1;
                continue; // Chemical triggered, skip Mechanical
            }

            // Mechanical domain: extreme strain (||F - I||_F > 0.5) → shockwave equations
            // F is deformation gradient (3x3 row-major); deviation from identity = deformation.
            // Strain intensity is used as proxy for true dF/dt (which would need F_prev).
            if i < strains.len() {
                let f = strains[i];
                let fi = (
                    (f[0] - 1.0) * (f[0] - 1.0)
                        + f[1] * f[1]
                        + f[2] * f[2]
                        + f[3] * f[3]
                        + (f[4] - 1.0) * (f[4] - 1.0)
                        + f[5] * f[5]
                        + f[6] * f[6]
                        + f[7] * f[7]
                        + (f[8] - 1.0) * (f[8] - 1.0)
                )
                    .sqrt();
                if fi > MECHANICAL_STRAIN_THRESHOLD {
                    let zone = IsolationZone {
                        id: self.next_id,
                        domain: IsolationDomain::Mechanical,
                        state: IsolationState::Transitioning,
                        center: pos,
                        radius_inner: 1.0,
                        radius_outer: 2.5,
                        intensity: (fi / 2.0).min(1.0),
                        energy_bundle: EnergyBundle {
                            total_energy: fi * 500.0,
                            temperature: temp,
                            ..Default::default()
                        },
                        created_tick: tick,
                        lifetime: 0.0,
                    };
                    self.zones.push(zone);
                    self.next_id += 1;
                    self.total_zones_created += 1;
                }
            }

            // Electromagnetic domain: extreme charge accumulation (|q| > em_field_threshold)
            // → Maxwell's equations replace quasi-static approximation.
            // True em_field (V/m) would come from ElectrostaticSolver; |charge| is a proxy
            // (E ~ k*q/r², so extreme |q| implies extreme field at any reasonable r).
            if i < charges.len() {
                let q_abs = charges[i].abs();
                if q_abs > self.em_field_threshold {
                    let intensity = ((q_abs - self.em_field_threshold) / self.em_field_threshold)
                        .min(1.0);
                    let zone = IsolationZone {
                        id: self.next_id,
                        domain: IsolationDomain::Electromagnetic,
                        state: IsolationState::Transitioning,
                        center: pos,
                        radius_inner: 0.8,
                        radius_outer: 2.0,
                        intensity,
                        energy_bundle: EnergyBundle {
                            total_energy: q_abs * 1000.0,
                            temperature: temp,
                            ..Default::default()
                        },
                        created_tick: tick,
                        lifetime: 0.0,
                    };
                    self.zones.push(zone);
                    self.next_id += 1;
                    self.total_zones_created += 1;
                }
            }
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        temperatures: &[f32],
        strains: &[[f32; 9]],
        charges: &[f32],
        positions: &[[f32; 3]],
        velocities: &[[f32; 3]],
        masses: &[f32],
        near_indices: &[(usize, f32)],
    ) {
        for zone in &mut self.zones {
            zone.lifetime += dt;
            // §4.4 Sync domain tag (so recovery code can apply domain-specific effects
            // without looking up the zone). Idempotent — set every tick.
            zone.energy_bundle.domain = zone.domain;

            match zone.state {
                IsolationState::Transitioning => {
                    if zone.lifetime > 0.1 {
                        zone.state = IsolationState::Isolated;
                    }
                }
                IsolationState::Isolated => {
                    // §4.4 Diff snapshot: evolve energy_bundle during isolation.
                    // Instead of a static snapshot frozen at creation time, the bundle
                    // reflects the zone's energy evolution (heat dissipation, radiation
                    // accumulation). On recovery, callers receive the *current* bundle
                    // state — the net result of isolation, not the initial state.
                    //
                    // Temperature decays toward ambient (20s time constant).
                    // Heat dissipates to environment during isolation.
                    zone.energy_bundle.temperature *= 1.0 - dt * 0.05;
                    // §4.4 Sync total_energy with temperature decay (was frozen at creation).
                    // Thermal/Chemical: total_energy tracks temperature (heat content).
                    // Mechanical/EM: total_energy is non-thermal (strain/charge based), unchanged.
                    if matches!(zone.domain, IsolationDomain::Thermal | IsolationDomain::Chemical) {
                        zone.energy_bundle.radiation_level += dt * zone.intensity * 0.1;
                        // Re-derive total_energy from current temperature using domain ratio.
                        // Thermal ratio = 1000.0 (detect_and_create init), Chemical = 100.0.
                        let ratio = match zone.domain {
                            IsolationDomain::Thermal => 1000.0,
                            IsolationDomain::Chemical => 100.0,
                            _ => 1000.0,
                        };
                        zone.energy_bundle.total_energy = zone.energy_bundle.temperature * ratio;
                    }

                    // §4.4 Activate unused EnergyBundle fields:
                    // Aggregate per-particle momentum/mass/velocity stats from near-field
                    // particles within the zone. These are used on recovery to transfer
                    // momentum back to the simulation (e.g., explosion ejecta impulse).
                    //
                    // fragment_count = particles with speed > 5 m/s (ejecta threshold)
                    // fragment_velocity_mean = mass-weighted mean velocity
                    // fragment_velocity_std = mass-weighted std dev (spread of ejecta)
                    let mut total_momentum = [0.0f32; 3];
                    let mut total_mass = 0.0f32;
                    let mut fragment_count = 0u32;
                    const FRAGMENT_SPEED_THRESHOLD: f32 = 5.0;
                    for &(i, _) in near_indices {
                        if i >= positions.len() {
                            continue;
                        }
                        let w = zone.weight_at(positions[i]);
                        if w <= 0.0 {
                            continue;
                        }
                        let m = if i < masses.len() { masses[i] } else { 0.0 };
                        let v = if i < velocities.len() { velocities[i] } else { [0.0; 3] };
                        let wm = w * m;
                        total_momentum[0] += v[0] * wm;
                        total_momentum[1] += v[1] * wm;
                        total_momentum[2] += v[2] * wm;
                        total_mass += wm;
                        let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                        if speed > FRAGMENT_SPEED_THRESHOLD {
                            fragment_count += 1;
                        }
                    }
                    if total_mass > 1e-9 {
                        let mean = [
                            total_momentum[0] / total_mass,
                            total_momentum[1] / total_mass,
                            total_momentum[2] / total_mass,
                        ];
                        // Compute mass-weighted velocity std dev
                        let mut var_sum = 0.0f32;
                        for &(i, _) in near_indices {
                            if i >= positions.len() {
                                continue;
                            }
                            let w = zone.weight_at(positions[i]);
                            if w <= 0.0 {
                                continue;
                            }
                            let m = if i < masses.len() { masses[i] } else { 0.0 };
                            let v = if i < velocities.len() { velocities[i] } else { [0.0; 3] };
                            let wm = w * m;
                            let dv = [
                                v[0] - mean[0],
                                v[1] - mean[1],
                                v[2] - mean[2],
                            ];
                            let speed2 = dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2];
                            var_sum += speed2 * wm;
                        }
                        let std = (var_sum / total_mass).sqrt();
                        zone.energy_bundle.total_momentum = total_momentum;
                        zone.energy_bundle.total_mass = total_mass;
                        zone.energy_bundle.fragment_count = fragment_count;
                        zone.energy_bundle.fragment_velocity_mean = mean;
                        zone.energy_bundle.fragment_velocity_std = std;
                    }
                    match zone.domain {
                        IsolationDomain::Mechanical => {
                            // Recovery when strain intensity drops below 0.1 (10% deformation)
                            let mut max_fi = 0.0f32;
                            for &(i, _) in near_indices {
                                if i >= positions.len() {
                                    continue;
                                }
                                let w = zone.weight_at(positions[i]);
                                if w > 0.5 && i < strains.len() {
                                    let f = strains[i];
                                    let fi = (
                                        (f[0] - 1.0) * (f[0] - 1.0)
                                            + f[1] * f[1]
                                            + f[2] * f[2]
                                            + f[3] * f[3]
                                            + (f[4] - 1.0) * (f[4] - 1.0)
                                            + f[5] * f[5]
                                            + f[6] * f[6]
                                            + f[7] * f[7]
                                            + (f[8] - 1.0) * (f[8] - 1.0)
                                    )
                                        .sqrt();
                                    if fi > max_fi {
                                        max_fi = fi;
                                    }
                                }
                            }
                            if max_fi < 0.1 {
                                zone.state = IsolationState::Recovering;
                            }
                        }
                        IsolationDomain::Electromagnetic => {
                            // Recovery when |charge| drops below 10% of em_field_threshold
                            // (charge has dissipated, field strength returned to quasi-static regime)
                            let mut max_q = 0.0f32;
                            for &(i, _) in near_indices {
                                if i >= positions.len() {
                                    continue;
                                }
                                let w = zone.weight_at(positions[i]);
                                if w > 0.5 && i < charges.len() {
                                    let q_abs = charges[i].abs();
                                    if q_abs > max_q {
                                        max_q = q_abs;
                                    }
                                }
                            }
                            if max_q < self.em_field_threshold * 0.1 {
                                zone.state = IsolationState::Recovering;
                            }
                        }
                        _ => {
                            // Thermal/Chemical recovery based on temperature
                            let mut max_temp = 0.0f32;
                            for &(i, _) in near_indices {
                                if i >= positions.len() || i >= temperatures.len() {
                                    continue;
                                }
                                let w = zone.weight_at(positions[i]);
                                if w > 0.5 && temperatures[i] > max_temp {
                                    max_temp = temperatures[i];
                                }
                            }
                            // Chemical reactions quench below 500K;
                            // Thermal plasma cools below 2500K (half of 5000K threshold).
                            let recovery_threshold = match zone.domain {
                                IsolationDomain::Chemical => 500.0,
                                _ => self.thermal_threshold * 0.5,
                            };
                            if max_temp < recovery_threshold {
                                zone.state = IsolationState::Recovering;
                            }
                        }
                    }
                }
                IsolationState::Recovering => {
                    // §4.4 fix: Don't remove Recovering zones here.
                    // collect_energy_bundles() is the sole path for removing Recovering zones,
                    // ensuring the energy_bundle is applied to particles BEFORE removal.
                    // Previously, update() removed zones with lifetime>0.5 before
                    // collect_energy_bundles() could collect their bundles, causing
                    // total_energy/fragment_velocity_std/total_mass/fragment_count
                    // activation effects to be silently lost.
                    // The lifetime>0.5 check is enforced in collect_energy_bundles().
                }
                _ => {}
            }
        }
    }

    pub fn collect_energy_bundles(&mut self) -> Vec<(EnergyBundle, [f32; 3], f32)> {
        // Phase 6 fix S7: return (bundle, center, radius_outer) so callers can apply
        // energy to local particles instead of dumping into global_temperature.
        let mut bundles = Vec::new();
        let mut to_remove = Vec::new();

        for zone in &self.zones {
            // §4.4 fix: Only collect zones that have been in Recovering state long enough
            // (lifetime > 0.5s). This gives the zone time to evolve its energy_bundle
            // (temperature decay, radiation accumulation) before applying to particles.
            // Without this check, zones would be collected on the first frame they enter
            // Recovering, missing the evolved bundle state.
            if zone.state == IsolationState::Recovering && zone.lifetime > 0.5 {
                bundles.push((zone.energy_bundle.clone(), zone.center, zone.radius_outer));
                to_remove.push(zone.id);
            }
        }

        self.zones.retain(|z| !to_remove.contains(&z.id));
        self.total_zones_resolved += to_remove.len() as u32;

        bundles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §4.4 EnergyBundle aggregation: verify that update() correctly computes
    /// total_momentum, total_mass, fragment_count, fragment_velocity_mean,
    /// fragment_velocity_std from per-particle velocity/mass data.
    #[test]
    fn test_energy_bundle_aggregation() {
        let mut mgr = DomainIsolationManager::new();
        // Particle 0: very hot (triggers Thermal zone), at origin
        let positions = vec![[0.0, 0.0, 0.0]];
        let temperatures = vec![6000.0]; // > 5000K threshold
        let chemical_ids = vec![0u32];
        let strains = vec![[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]]; // identity
        let charges = vec![0.0];
        // Single particle with mass=2, velocity=(10, 0, 0)
        let velocities = vec![[10.0, 0.0, 0.0]];
        let masses = vec![2.0];

        mgr.detect_and_create(
            &positions,
            &temperatures,
            &chemical_ids,
            &strains,
            &charges,
            &[(0, 1.0)],
            1,
        );
        assert_eq!(mgr.zones.len(), 1, "Thermal zone should be created");
        assert_eq!(mgr.zones[0].domain, IsolationDomain::Thermal);

        // Force zone into Isolated state (Transitioning → Isolated after 0.1s)
        let near_indices: Vec<(usize, f32)> = vec![(0, 1.0)];
        mgr.update(
            0.2, // > 0.1 to transition to Isolated
            &temperatures,
            &strains,
            &charges,
            &positions,
            &velocities,
            &masses,
            &near_indices,
        );
        assert_eq!(mgr.zones[0].state, IsolationState::Isolated);

        // Update again to populate EnergyBundle fields
        mgr.update(
            0.1,
            &temperatures,
            &strains,
            &charges,
            &positions,
            &velocities,
            &masses,
            &near_indices,
        );

        let bundle = &mgr.zones[0].energy_bundle;
        // Particle at origin with weight=1.0 (within radius_inner=2.0)
        // total_mass = mass * weight = 2.0 * 1.0 = 2.0
        assert!(
            (bundle.total_mass - 2.0).abs() < 1e-5,
            "total_mass should be 2.0, got {}",
            bundle.total_mass
        );
        // total_momentum = vel * mass * weight = (10,0,0) * 2 * 1 = (20, 0, 0)
        assert!(
            (bundle.total_momentum[0] - 20.0).abs() < 1e-5,
            "total_momentum[0] should be 20.0, got {}",
            bundle.total_momentum[0]
        );
        // fragment_velocity_mean = total_momentum / total_mass = (10, 0, 0)
        assert!(
            (bundle.fragment_velocity_mean[0] - 10.0).abs() < 1e-5,
            "fragment_velocity_mean[0] should be 10.0, got {}",
            bundle.fragment_velocity_mean[0]
        );
        // fragment_count: speed=10 > 5 threshold → 1 fragment
        assert_eq!(
            bundle.fragment_count, 1,
            "fragment_count should be 1 (speed 10 > 5 threshold)"
        );
        // fragment_velocity_std: single particle, std = 0
        assert!(
            bundle.fragment_velocity_std < 1e-5,
            "fragment_velocity_std should be 0 for single particle, got {}",
            bundle.fragment_velocity_std
        );
    }

    /// Verify fragment_velocity_std is non-zero with multiple particles at different velocities
    #[test]
    fn test_energy_bundle_velocity_std() {
        let mut mgr = DomainIsolationManager::new();
        // 2 particles at origin, both in Thermal zone
        let positions = vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]];
        let temperatures = vec![6000.0, 6000.0];
        let chemical_ids = vec![0u32, 0u32];
        let strains = vec![
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        ];
        let charges = vec![0.0, 0.0];
        // Particle 0: vel=(10,0,0) mass=1; Particle 1: vel=(0,0,0) mass=1
        // Mean vel = (5, 0, 0); var = ((10-5)² + (0-5)²) / 2 = 25; std = 5
        let velocities = vec![[10.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let masses = vec![1.0, 1.0];

        mgr.detect_and_create(
            &positions,
            &temperatures,
            &chemical_ids,
            &strains,
            &charges,
            &[(0, 1.0), (1, 1.0)],
            1,
        );
        // Both particles trigger zones but second is within first's radius_inner (2.0)
        // (position 0.5 < 2.0 from origin), so `already_isolated` skips it.
        assert_eq!(mgr.zones.len(), 1);

        let near_indices: Vec<(usize, f32)> = vec![(0, 1.0), (1, 1.0)];
        // Transition to Isolated
        mgr.update(
            0.2,
            &temperatures,
            &strains,
            &charges,
            &positions,
            &velocities,
            &masses,
            &near_indices,
        );
        // Update to aggregate
        mgr.update(
            0.1,
            &temperatures,
            &strains,
            &charges,
            &positions,
            &velocities,
            &masses,
            &near_indices,
        );

        let bundle = &mgr.zones[0].energy_bundle;
        // Both particles have weight 1.0 at distance 0 and 0.5 (both < radius_inner=2.0)
        // total_mass = 1.0 + 1.0 = 2.0
        assert!((bundle.total_mass - 2.0).abs() < 1e-5);
        // fragment_velocity_mean = (10+0)/2 = 5.0
        assert!(
            (bundle.fragment_velocity_mean[0] - 5.0).abs() < 1e-5,
            "mean should be 5.0, got {}",
            bundle.fragment_velocity_mean[0]
        );
        // fragment_velocity_std = sqrt(((10-5)² + (0-5)²)/2) = sqrt(25) = 5.0
        assert!(
            (bundle.fragment_velocity_std - 5.0).abs() < 1e-4,
            "std should be ~5.0, got {}",
            bundle.fragment_velocity_std
        );
        // Both particles have speed 10 and 0, only first > 5 threshold → 1 fragment
        assert_eq!(bundle.fragment_count, 1);
    }
}
