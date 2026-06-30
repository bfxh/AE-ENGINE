//! Performance test: 1 million particles (ARCHITECTURE_V7 §9.3 target)
//!
//! Spawns 1M particles and runs 60 ticks (1 second @ 60Hz), measuring:
//!   - Total elapsed time
//!   - Per-tick average
//!   - FPS equivalent
//!   - LOD distribution
//!   - Memory usage
//!
//! Run: cargo run --release --bin perf_test_1m

use std::time::Instant;
use ae_engine::{GameWorld, WorldBounds};

fn main() {
    println!("=== Wasteland Engine - 1M Particle Performance Test ===\n");

    let bounds = WorldBounds {
        min: glam::Vec3::new(-500.0, -100.0, -500.0),
        max: glam::Vec3::new(500.0, 300.0, 500.0),
    };

    let mut world = GameWorld::new(bounds);
    println!(
        "Initial particles: {} (test particles from GameWorld::new)",
        world.simulation.mpss.count
    );

    // Spawn 1 million particles distributed across the world
    println!("Spawning 1,000,000 particles...");
    let spawn_start = Instant::now();

    let target = 1_000_000;
    let mut spawned = 0;
    // Distribute in a 500x500x200 volume around origin
    // Most particles in mid-field (50-200m) to stress LOD system
    for i in 0..target {
        if let Some(idx) = world.simulation.mpss.spawn() {
            // Distribute: 10% near, 40% mid, 50% far
            let r = if i % 10 == 0 {
                // Near-field: 0-50m
                let angle = (i as f32) * 0.1;
                let dist = (i % 500) as f32 * 0.1;
                [dist * angle.cos(), 5.0, dist * angle.sin()]
            } else if i % 10 < 5 {
                // Mid-field: 50-200m
                let angle = (i as f32) * 0.01;
                let dist = 50.0 + (i % 1500) as f32 * 0.1;
                [dist * angle.cos(), 5.0, dist * angle.sin()]
            } else {
                // Far-field: 200-500m
                let angle = (i as f32) * 0.001;
                let dist = 200.0 + (i % 3000) as f32 * 0.1;
                [dist * angle.cos(), 5.0, dist * angle.sin()]
            };
            world.simulation.mpss.pos[idx] = r;
            world.simulation.mpss.vel[idx] = [0.0, 0.0, 0.0];
            world.simulation.mpss.temperature[idx] = 293.0;
            world.simulation.mpss.mass[idx] = 1.0;
            world.simulation.mpss.material_idx[idx] = (i % 4) as u16;
            world.simulation.mpss.lifetime[idx] = f32::MAX;
            spawned += 1;
        } else {
            println!("WARNING: MpssBuffer full at {} particles (capacity {})", i, world.simulation.mpss.capacity);
            break;
        }
    }

    let spawn_elapsed = spawn_start.elapsed();
    println!(
        "Spawned {} particles in {:.2}s ({:.0} particles/sec)",
        spawned,
        spawn_elapsed.as_secs_f32(),
        spawned as f64 / spawn_elapsed.as_secs_f64()
    );

    let mem = world.simulation.mpss.memory_usage();
    println!(
        "MpssBuffer memory usage: {:.1} MB ({:.0} bytes/particle)",
        mem as f64 / 1_048_576.0,
        mem as f64 / spawned as f64
    );

    // Run 60 ticks (1 second @ 60Hz) and measure
    println!("\nRunning 60 ticks (1 second @ 60Hz)...");
    let run_start = Instant::now();

    let mut tick_times: Vec<f64> = Vec::with_capacity(60);
    for i in 0..60 {
        let tick_start = Instant::now();
        world.tick();
        let tick_elapsed = tick_start.elapsed().as_secs_f64() * 1000.0; // ms
        tick_times.push(tick_elapsed);

        if (i + 1) % 10 == 0 {
            let s = world.stats();
            let lod = &world.simulation.lod_stats;
            println!(
                "  t={:.2}s tick={} T={:.1}K LOD(n/m/f)={}/{}/{} tick_time={:.1}ms",
                s.time, s.tick_count, s.global_temperature, lod.near, lod.mid, lod.far, tick_elapsed
            );
        }
    }

    let run_elapsed = run_start.elapsed();
    let avg_tick = tick_times.iter().sum::<f64>() / tick_times.len() as f64;
    let max_tick = tick_times.iter().cloned().fold(0.0f64, f64::max);
    let min_tick = tick_times.iter().cloned().fold(f64::MAX, f64::min);
    let fps = 1000.0 / avg_tick;

    println!("\n=== Performance Results ===");
    println!("Total elapsed:       {:.2}s", run_elapsed.as_secs_f32());
    println!("Average tick time:   {:.2} ms", avg_tick);
    println!("Min tick time:       {:.2} ms", min_tick);
    println!("Max tick time:       {:.2} ms", max_tick);
    println!("Effective FPS:       {:.1}", fps);
    println!("Realtime ratio:      {:.2}x ({:.0}%)", run_elapsed.as_secs_f32(), 100.0 / fps);

    // Final stats
    let s = world.stats();
    let lod = &world.simulation.lod_stats;
    let eb_pub = world.event_bus.published_count();
    let eb_proc = world.event_bus.processed_count();
    println!("\n=== Final State ===");
    println!(
        "Particles: {} (active: {})",
        world.simulation.mpss.count,
        world.simulation.mpss.active_indices().count()
    );
    println!("Global temp: {:.1}K", s.global_temperature);
    println!("LOD(n/m/f):  {}/{}/{}", lod.near, lod.mid, lod.far);
    println!("EB(p/r):     {}/{}", eb_pub, eb_proc);
    println!(
        "Zones:       {}",
        world.simulation.domain_isolation.zones.len()
    );

    // Phase distribution
    let mut solid = 0;
    let mut liquid = 0;
    let mut gas = 0;
    let mut other = 0;
    for i in 0..world.simulation.mpss.count {
        if !world.simulation.mpss.active[i] {
            continue;
        }
        use ae_particle::mpss::MpssPhase;
        match world.simulation.mpss.phase[i] {
            MpssPhase::Solid => solid += 1,
            MpssPhase::Liquid => liquid += 1,
            MpssPhase::Gas => gas += 1,
            _ => other += 1,
        }
    }
    println!(
        "Phase:       Solid={} Liquid={} Gas={} Other={}",
        solid, liquid, gas, other
    );

    // Performance verdict
    println!("\n=== Verdict ===");
    if fps >= 60.0 {
        println!("✓ TARGET MET: 60 FPS with 1M particles");
    } else if fps >= 30.0 {
        println!("~ PARTIAL: {:.0} FPS (target 60), needs optimization", fps);
    } else {
        println!("✗ BELOW TARGET: {:.0} FPS (target 60), significant optimization needed", fps);
    }
}
