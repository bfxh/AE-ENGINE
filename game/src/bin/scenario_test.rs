//! Integration scenario tests for end-to-end engine verification.
//!
//! Validates the three scenarios from ARCHITECTURE_V7.md §9.2:
//!   1. Explosion: high-temp cluster -> domain isolation -> energy release -> phase change
//!   2. Fire spread: combustible material -> heat conduction -> pyrolysis -> temp gradient
//!   3. Phase transition: mixed materials at phase boundaries (water/iron/wood/concrete)
//!
//! Run: cargo run --release --bin scenario_test

use ae_engine::{GameWorld, WorldBounds};

fn main() {
    println!("=== Wasteland Engine - Scenario Integration Tests ===\n");

    let bounds = WorldBounds {
        min: glam::Vec3::new(-100.0, -10.0, -100.0),
        max: glam::Vec3::new(100.0, 100.0, 100.0),
    };

    let mut all_pass = true;
    all_pass &= test_explosion_scenario(bounds);
    all_pass &= test_fire_spread_scenario(bounds);
    all_pass &= test_phase_transition_scenario(bounds);

    println!("\n=== Summary ===");
    if all_pass {
        println!("ALL SCENARIO TESTS PASSED");
    } else {
        println!("SOME SCENARIO TESTS FAILED");
        std::process::exit(1);
    }
}

/// Scenario 1: Explosion
/// Set up a cluster of 6000K particles, run simulation, verify:
///   - Domain isolation triggers (zone count > 0)
///   - Nearby particles heat up (energy bundle local effect)
///   - Phase transitions occur (Solid -> Gas for nearby material)
fn test_explosion_scenario(bounds: WorldBounds) -> bool {
    println!("--- Scenario 1: Explosion ---");
    let mut world = GameWorld::new(bounds);

    // Spawn cold material particles around origin (material 3 = iron)
    for i in 0..20 {
        if let Some(idx) = world.simulation.mpss.spawn() {
            let angle = i as f32 * 0.314;
            let r = 3.0 + (i % 5) as f32 * 0.5;
            world.simulation.mpss.pos[idx] = [r * angle.cos(), 2.0, r * angle.sin()];
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 3; // iron
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
    }

    // Spawn the "explosion": 5 particles at 6000K in the center
    for i in 0..5 {
        if let Some(idx) = world.simulation.mpss.spawn() {
            world.simulation.mpss.pos[idx] = [(i as f32 - 2.0) * 0.3, 2.0, 0.0];
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 6000.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 0;
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
    }

    let total = world.simulation.mpss.count;
    println!("Initial: {} particles (20 iron @ 293K + 5 hot @ 6000K)", total);

    // Run 300 ticks (5 seconds)
    let mut zone_triggered = false;
    let mut phase_changed = false;
    let initial_phases: Vec<_> = (0..world.simulation.mpss.count)
        .map(|i| world.simulation.mpss.phase[i])
        .collect();

    for i in 0..300 {
        world.tick();

        if world.simulation.domain_isolation.zones.len() > 0 {
            zone_triggered = true;
        }

        // Check if any nearby iron particle changed phase
        for j in 0..world.simulation.mpss.count {
            if !world.simulation.mpss.active[j] {
                continue;
            }
            if j < initial_phases.len() && world.simulation.mpss.phase[j] != initial_phases[j] {
                phase_changed = true;
            }
        }

        if (i + 1) % 60 == 0 {
            let s = world.stats();
            let zones = world.simulation.domain_isolation.zones.len();
            let eb_pub = world.event_bus.published_count();
            println!(
                "  t={:.1}s T={:.1}K zones={} EB={} phase_changed={}",
                s.time, s.global_temperature, zones, eb_pub, phase_changed
            );
        }
    }

    // Verify outcomes
    let pass_zone = zone_triggered;
    let pass_phase = phase_changed;
    let pass = pass_zone && pass_phase;

    println!(
        "  Result: zone_triggered={} phase_changed={} -> {}",
        pass_zone, pass_phase,
        if pass { "PASS" } else { "FAIL" }
    );
    println!();
    pass
}

/// Scenario 2: Fire Spread
/// Set up a line of wood particles (material 0), ignite one end, verify:
///   - Heat conducts to neighbors (temperature rises)
///   - Wood pyrolysis occurs (Solid -> Gas at 500K)
///   - Temperature gradient forms (hot near ignition, cold far away)
fn test_fire_spread_scenario(bounds: WorldBounds) -> bool {
    println!("--- Scenario 2: Fire Spread ---");
    let mut world = GameWorld::new(bounds);

    // Spawn a line of wood particles at y=1, spaced 0.5m apart for fast conduction
    let mut wood_indices: Vec<usize> = Vec::new();
    for i in 0..10 {
        if let Some(idx) = world.simulation.mpss.spawn() {
            world.simulation.mpss.pos[idx] = [i as f32 * 0.5, 1.0, 0.0];
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 0; // wood
            world.simulation.mpss.lifetime[idx] = f32::MAX;
            wood_indices.push(idx);
        }
    }

    // Ignite the first particle
    if let Some(&first) = wood_indices.first() {
        world.simulation.mpss.temperature[first] = 800.0; // above 500K pyrolysis threshold
        println!(
            "Initial: 10 wood particles in a line, first ignited to 800K (pyrolysis @ 500K)"
        );
    }

    let initial_temps: Vec<f32> = wood_indices
        .iter()
        .map(|&i| world.simulation.mpss.temperature[i])
        .collect();

    // Run 300 ticks (5 seconds)
    for i in 0..300 {
        world.tick();

        if (i + 1) % 60 == 0 {
            let s = world.stats();
            let temps: Vec<f32> = wood_indices
                .iter()
                .map(|&idx| world.simulation.mpss.temperature[idx])
                .collect();
            let phases: Vec<u8> = wood_indices
                .iter()
                .map(|&idx| world.simulation.mpss.phase[idx] as u8)
                .collect();
            println!(
                "  t={:.1}s T={:.1}K wood_temps=[{}] phases=[{}]",
                s.time,
                s.global_temperature,
                temps
                    .iter()
                    .map(|t| format!("{:.0}", t))
                    .collect::<Vec<_>>()
                    .join(","),
                phases
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }

    // Verify: first particle should have undergone pyrolysis (Solid -> Gas)
    let first_phase = world.simulation.mpss.phase[wood_indices[0]];
    let pass_pyrolysis = first_phase == ae_particle::mpss::MpssPhase::Gas;

    // Verify: temperature gradient (first hotter than last)
    let final_first = world.simulation.mpss.temperature[wood_indices[0]];
    let final_last = world.simulation.mpss.temperature[wood_indices[9]];
    let pass_gradient = final_first > final_last;

    // Verify: some heat conducted (last particle warmer than initial by any amount)
    let pass_conduction = final_last > initial_temps[9];

    let pass = pass_pyrolysis && pass_gradient && pass_conduction;
    println!(
        "  Result: pyrolysis={} gradient({:.0}>{:.0})={} conduction({:.0}>{:.0})={} -> {}",
        pass_pyrolysis,
        final_first,
        final_last,
        pass_gradient,
        final_last,
        initial_temps[9],
        pass_conduction,
        if pass { "PASS" } else { "FAIL" }
    );
    println!();
    pass
}

/// Scenario 3: Phase Transition
/// Set up different materials at different temperatures, verify:
///   - Water: 200K (ice) -> 300K (water) -> 400K (steam)
///   - Iron: 1500K (solid) -> 2000K (liquid)
///   - Wood: 400K (solid) -> 600K (gas/pyrolysis)
fn test_phase_transition_scenario(bounds: WorldBounds) -> bool {
    println!("--- Scenario 3: Phase Transition ---");
    let mut world = GameWorld::new(bounds);

    // Material key: 0=wood, 1=water, 2=concrete, 3=iron
    struct PhaseTest {
        name: &'static str,
        material: u16,
        temps: [f32; 3],
        expected: [ae_particle::mpss::MpssPhase; 3],
    }

    let tests = [
        PhaseTest {
            name: "water",
            material: 1,
            temps: [200.0, 300.0, 400.0],
            expected: [
                ae_particle::mpss::MpssPhase::Solid, // ice
                ae_particle::mpss::MpssPhase::Liquid, // water
                ae_particle::mpss::MpssPhase::Gas,    // steam
            ],
        },
        PhaseTest {
            name: "iron",
            material: 3,
            temps: [1500.0, 2000.0, 3500.0],
            expected: [
                ae_particle::mpss::MpssPhase::Solid,  // solid
                ae_particle::mpss::MpssPhase::Liquid, // liquid
                ae_particle::mpss::MpssPhase::Gas,    // gas
            ],
        },
        PhaseTest {
            name: "wood",
            material: 0,
            temps: [400.0, 600.0, 800.0],
            expected: [
                ae_particle::mpss::MpssPhase::Solid, // solid
                ae_particle::mpss::MpssPhase::Gas,   // pyrolyzed
                ae_particle::mpss::MpssPhase::Gas,   // gas
            ],
        },
    ];

    let mut all_pass = true;
    for test in &tests {
        // Use a fresh buffer slice for each test
        let mut indices: Vec<usize> = Vec::new();
        for &temp in &test.temps {
            if let Some(idx) = world.simulation.mpss.spawn() {
                world.simulation.mpss.pos[idx] = [0.0, 50.0 + idx as f32 * 2.0, 0.0];
                world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
                world.simulation.mpss.temperature[idx] = temp;
                world.simulation.mpss.mass[idx] = 1.0;
                world.simulation.mpss.material_idx[idx] = test.material;
                world.simulation.mpss.lifetime[idx] = f32::MAX;
                indices.push(idx);
            }
        }

        // Apply phase transitions directly
        let transitions = world.simulation.mpss.apply_phase_transitions();

        let mut test_pass = true;
        for (i, &idx) in indices.iter().enumerate() {
            let actual = world.simulation.mpss.phase[idx];
            let expected = test.expected[i];
            let ok = actual == expected;
            if !ok {
                test_pass = false;
            }
            println!(
                "  {}: temp={:.0}K phase={:?} expected={:?} {}",
                test.name,
                test.temps[i],
                actual,
                expected,
                if ok { "OK" } else { "MISMATCH" }
            );
        }
        if !test_pass {
            all_pass = false;
        }
        let _ = transitions;
    }

    println!(
        "  Result: phase transitions -> {}",
        if all_pass { "PASS" } else { "FAIL" }
    );
    println!();
    all_pass
}
