//! Conservation verification tests (ARCHITECTURE_V7 §9.4)
//!
//! Validates three fundamental conservation laws:
//!   1. Mass conservation:   Σm_before = Σm_after  (no particle creation/destruction)
//!   2. Momentum conservation: Σp_xz_before = Σp_xz_after  (gravity only affects y-axis)
//!   3. Energy conservation: ΣE_thermal change < 5%  (no phase change, no domain isolation)
//!
//! Run: cargo run --release --bin conservation_test

use wasteland_engine::{GameWorld, WorldBounds};

fn total_mass(world: &GameWorld) -> f64 {
    let mpss = &world.simulation.mpss;
    let mut sum = 0.0f64;
    for i in 0..mpss.count {
        if mpss.active[i] {
            sum += mpss.mass[i] as f64;
        }
    }
    sum
}

fn total_momentum(world: &GameWorld) -> [f64; 3] {
    let mpss = &world.simulation.mpss;
    let mut p = [0.0f64; 3];
    for i in 0..mpss.count {
        if mpss.active[i] {
            let m = mpss.mass[i] as f64;
            p[0] += m * mpss.vel[i][0] as f64;
            p[1] += m * mpss.vel[i][1] as f64;
            p[2] += m * mpss.vel[i][2] as f64;
        }
    }
    p
}

fn total_kinetic_energy(world: &GameWorld) -> f64 {
    let mpss = &world.simulation.mpss;
    let mut sum = 0.0f64;
    for i in 0..mpss.count {
        if mpss.active[i] {
            let m = mpss.mass[i] as f64;
            let v = &mpss.vel[i];
            sum += 0.5 * m * (v[0] as f64 * v[0] as f64
                + v[1] as f64 * v[1] as f64
                + v[2] as f64 * v[2] as f64);
        }
    }
    sum
}

fn total_thermal_energy(world: &GameWorld) -> f64 {
    let mpss = &world.simulation.mpss;
    let mut sum = 0.0f64;
    for i in 0..mpss.count {
        if mpss.active[i] {
            let m = mpss.mass[i] as f64;
            let t = mpss.temperature[i] as f64;
            sum += m * t;
        }
    }
    sum
}

fn main() {
    println!("=== Wasteland Engine - Conservation Verification (§9.4) ===\n");

    let bounds = WorldBounds {
        min: glam::Vec3::new(-1000.0, -100.0, -1000.0),
        max: glam::Vec3::new(1000.0, 500.0, 1000.0),
    };

    let mut all_pass = true;
    all_pass &= test_mass_conservation(bounds);
    all_pass &= test_momentum_conservation(bounds);
    all_pass &= test_energy_conservation(bounds);

    println!("\n=== Summary ===");
    if all_pass {
        println!("ALL CONSERVATION TESTS PASSED");
    } else {
        println!("SOME CONSERVATION TESTS FAILED");
        std::process::exit(1);
    }
}

/// Test 1: Mass conservation
/// Spawn 1000 particles, run 60 ticks, verify total mass unchanged.
/// Uses material 2 (concrete) with >2m spacing to avoid cross_domain reactions.
fn test_mass_conservation(bounds: WorldBounds) -> bool {
    println!("--- Test 1: Mass Conservation ---");
    let mut world = GameWorld::new(bounds);

    // Kill default test particles from GameWorld::new (avoid high-temp interference)
    let initial_count = world.simulation.mpss.count;
    for i in 0..initial_count {
        if world.simulation.mpss.active[i] {
            world.simulation.mpss.kill(i);
        }
    }

    // Spawn 1000 particles at >3m spacing (avoids reaction_radius=0.5m cross_domain)
    // Use material 2 (concrete) — no combustion, no corrosion, no oxidation
    for i in 0..1000 {
        if let Some(idx) = world.simulation.mpss.spawn() {
            // Grid layout: 10x10x10 with 5m spacing = 50x50x50 cube
            let x = (i % 10) as f32 * 5.0 - 25.0;
            let y = ((i / 10) % 10) as f32 * 5.0 + 100.0;
            let z = (i / 100) as f32 * 5.0 - 25.0;
            world.simulation.mpss.pos[idx] = [x, y, z];
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0 + (i as f32 % 3.0) * 0.5;
            world.simulation.mpss.material_idx[idx] = 2; // concrete
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
    }

    let m_before = total_mass(&world);
    let count_before = world.simulation.mpss.count;
    println!("  Initial mass: {:.4} ({} particles)", m_before, count_before);

    for tick in 0..60 {
        let before_count = world.simulation.mpss.count;
        world.tick();
        let after_count = world.simulation.mpss.count;
        if after_count != before_count {
            println!(
                "  [tick {}] particle count changed: {} -> {} (delta {})",
                tick + 1,
                before_count,
                after_count,
                after_count as i64 - before_count as i64
            );
        }
    }

    let m_after = total_mass(&world);
    let count_after = world.simulation.mpss.count;
    let dm = (m_after - m_before).abs();
    let pass = dm < 1e-3;
    println!(
        "  Final mass:   {:.4} ({} particles, delta count {})",
        m_after,
        count_after,
        count_after as i64 - count_before as i64
    );
    println!("  Delta:         {:.6}", dm);
    println!("  Result: {}", if pass { "PASS" } else { "FAIL" });
    pass
}

/// Test 2: Momentum conservation (x/z axes)
/// Spawn particles with symmetric velocities (Σp_xz = 0), run 60 ticks,
/// verify x/z momentum remains near zero. Gravity only affects y-axis.
fn test_momentum_conservation(bounds: WorldBounds) -> bool {
    println!("\n--- Test 2: Momentum Conservation (x/z, gravity-free axes) ---");
    let mut world = GameWorld::new(bounds);

    // Kill default test particles
    let initial_count = world.simulation.mpss.count;
    for i in 0..initial_count {
        if world.simulation.mpss.active[i] {
            world.simulation.mpss.kill(i);
        }
    }

    // Spawn 100 particles in pairs with opposite x/z velocities
    // Use material 2 (concrete) with 5m spacing to avoid cross_domain reactions
    for i in 0..50 {
        let vx = (i as f32 + 1.0) * 0.1;
        let vz = (i as f32 + 1.0) * 0.05;
        let base_x = 100.0 + (i as f32) * 5.0;
        // Pair 1: +x, +z velocity
        if let Some(idx) = world.simulation.mpss.spawn() {
            world.simulation.mpss.pos[idx] = [base_x, 100.0, 100.0];
            world.simulation.mpss.vel[idx] = [vx, 0.0, vz];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 2;
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
        // Pair 2: -x, -z velocity (cancels pair 1)
        if let Some(idx) = world.simulation.mpss.spawn() {
            world.simulation.mpss.pos[idx] = [base_x, 100.0, 100.0];
            world.simulation.mpss.vel[idx] = [-vx, 0.0, -vz];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 2;
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
    }

    let p_before = total_momentum(&world);
    println!(
        "  Initial momentum: px={:.4}, py={:.4}, pz={:.4}",
        p_before[0], p_before[1], p_before[2]
    );

    for _ in 0..60 {
        world.tick();
    }

    let p_after = total_momentum(&world);
    let dpx = (p_after[0] - p_before[0]).abs();
    let dpz = (p_after[2] - p_before[2]).abs();
    // x/z momentum should be conserved (gravity only affects y)
    // Allow tolerance for numerical drift and mid/far field update rounding
    let pass = dpx < 1.0 && dpz < 1.0;
    println!(
        "  Final momentum:   px={:.4}, py={:.4}, pz={:.4}",
        p_after[0], p_after[1], p_after[2]
    );
    println!("  Delta px: {:.6}, Delta pz: {:.6}", dpx, dpz);
    println!("  (py not conserved due to gravity impulse)");
    println!("  Result: {}", if pass { "PASS" } else { "FAIL" });
    pass
}

/// Test 3: Energy conservation
/// Spawn isothermal particles at 293K (no phase change), run 60 ticks,
/// verify total thermal energy change < 5%. Kinetic energy may change due
/// to gravity, but thermal energy should be conserved (heat conduction only
/// redistributes, doesn't create/destroy heat).
fn test_energy_conservation(bounds: WorldBounds) -> bool {
    println!("\n--- Test 3: Thermal Energy Conservation ---");
    let mut world = GameWorld::new(bounds);

    // Kill default test particles (avoid high-temp 6000K interference)
    let initial_count = world.simulation.mpss.count;
    for i in 0..initial_count {
        if world.simulation.mpss.active[i] {
            world.simulation.mpss.kill(i);
        }
    }

    // Spawn 500 particles at uniform 293K, >3m spacing
    // Use material 2 (concrete) — no phase change, no reactions at 293K
    for i in 0..500 {
        if let Some(idx) = world.simulation.mpss.spawn() {
            let x = (i % 10) as f32 * 5.0 - 25.0;
            let y = ((i / 10) % 50) as f32 * 5.0 + 100.0;
            world.simulation.mpss.pos[idx] = [x, y, 100.0];
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = 2; // concrete
            world.simulation.mpss.lifetime[idx] = f32::MAX;
        }
    }

    let e_thermal_before = total_thermal_energy(&world);
    let e_kinetic_before = total_kinetic_energy(&world);
    println!(
        "  Initial thermal energy: {:.2}, kinetic: {:.2}",
        e_thermal_before, e_kinetic_before
    );

    for _ in 0..60 {
        world.tick();
    }

    let e_thermal_after = total_thermal_energy(&world);
    let e_kinetic_after = total_kinetic_energy(&world);
    let d_thermal = (e_thermal_after - e_thermal_before).abs();
    let rel_thermal = d_thermal / e_thermal_before;
    println!(
        "  Final thermal energy:   {:.2}, kinetic: {:.2}",
        e_thermal_after, e_kinetic_after
    );
    println!("  Delta thermal: {:.2} ({:.3}%)", d_thermal, rel_thermal * 100.0);
    // Thermal energy should be conserved (no phase change, no domain isolation at 293K)
    // Allow 5% tolerance for atmospheric coupling and numerical drift
    let pass = rel_thermal < 0.05;
    println!("  Result: {}", if pass { "PASS" } else { "FAIL" });
    pass
}
