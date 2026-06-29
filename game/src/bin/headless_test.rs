//! 无图形界面的引擎测试程序
//! 验证 LOD 分层、域隔离检测、粒子温度更新

use wasteland_engine::{GameWorld, WorldBounds};

fn main() {
    println!("=== Wasteland Engine - Headless Test ===");

    let bounds = WorldBounds {
        min: glam::Vec3::new(-500.0, -100.0, -500.0),
        max: glam::Vec3::new(500.0, 300.0, 500.0),
    };

    let mut world = GameWorld::new(bounds);

    println!("World created");
    println!(
        "Initial mpss particles: {} (capacity {})",
        world.simulation.mpss.count,
        world.simulation.mpss.capacity
    );

    // 验证初始粒子分布
    let mpss = &world.simulation.mpss;
    let mut near = 0;
    let mut mid = 0;
    let mut far = 0;
    let mut high_temp = 0;
    let mut low_temp = 0;
    let player_pos = glam::Vec3::ZERO;
    for i in 0..mpss.count {
        if !mpss.active[i] {
            continue;
        }
        let dx = mpss.pos[i][0] - player_pos.x;
        let dy = mpss.pos[i][1] - player_pos.y;
        let dz = mpss.pos[i][2] - player_pos.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq < 50.0 * 50.0 {
            near += 1;
        } else if dist_sq < 200.0 * 200.0 {
            mid += 1;
        } else {
            far += 1;
        }
        if mpss.temperature[i] > 5000.0 {
            high_temp += 1;
        }
        if mpss.temperature[i] < 50.0 {
            low_temp += 1;
        }
    }
    println!(
        "Initial distribution: near={} mid={} far={} high_temp(>5000K)={} low_temp(<50K)={}",
        near, mid, far, high_temp, low_temp
    );

    // 运行 600 ticks (10秒 @ 60Hz)
    println!("\nRunning 600 ticks (10 seconds @ 60Hz)...");
    for i in 0..600 {
        world.tick();

        // 每 60 ticks (1秒) 报告一次
        if (i + 1) % 60 == 0 {
            let s = world.stats();
            let lod = &world.simulation.lod_stats;
            let eb_pub = world.event_bus.published_count();
            let eb_proc = world.event_bus.processed_count();
            let zones = world.simulation.domain_isolation.zones.len();
            println!(
                "t={:.1}s tick={} T={:.1}K rad={:.4} voxels={} meta={} eco={} LOD(n/m/f)={}/{}/{} EB(p/r)={}/{} zones={}",
                s.time,
                s.tick_count,
                s.global_temperature,
                s.global_radiation,
                s.total_voxels,
                s.meta_entity_count,
                s.ecosystem_count,
                lod.near,
                lod.mid,
                lod.far,
                eb_pub,
                eb_proc,
                zones
            );
        }
    }

    // 最终粒子状态分析
    println!("\n=== Final Particle Analysis ===");
    let mpss = &world.simulation.mpss;
    let mut near = 0;
    let mut mid = 0;
    let mut far = 0;
    let mut high_temp = 0;
    let mut low_temp = 0;
    let mut total_temp: f32 = 0.0;
    let mut count = 0;
    for i in 0..mpss.count {
        if !mpss.active[i] {
            continue;
        }
        let dx = mpss.pos[i][0] - player_pos.x;
        let dy = mpss.pos[i][1] - player_pos.y;
        let dz = mpss.pos[i][2] - player_pos.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq < 50.0 * 50.0 {
            near += 1;
        } else if dist_sq < 200.0 * 200.0 {
            mid += 1;
        } else {
            far += 1;
        }
        if mpss.temperature[i] > 5000.0 {
            high_temp += 1;
        }
        if mpss.temperature[i] < 50.0 {
            low_temp += 1;
        }
        total_temp += mpss.temperature[i];
        count += 1;
    }
    println!(
        "Final distribution: near={} mid={} far={} high_temp(>5000K)={} low_temp(<50K)={}",
        near, mid, far, high_temp, low_temp
    );
    if count > 0 {
        println!(
            "Average temperature: {:.1}K (global: {:.1}K)",
            total_temp / count as f32,
            world.global_temperature
        );
    }

    // Phase distribution (Solid/Liquid/Gas/Plasma/...)
    let mpss = &world.simulation.mpss;
    let mut phase_solid = 0usize;
    let mut phase_liquid = 0usize;
    let mut phase_gas = 0usize;
    let mut phase_plasma = 0usize;
    let mut phase_other = 0usize;
    for i in 0..mpss.count {
        if !mpss.active[i] {
            continue;
        }
        // MpssPhase is #[repr(u8)]: Solid=0, Liquid=1, Gas=2, Plasma=3, ...
        match mpss.phase[i] as u8 {
            0 => phase_solid += 1,
            1 => phase_liquid += 1,
            2 => phase_gas += 1,
            3 => phase_plasma += 1,
            _ => phase_other += 1,
        }
    }
    println!(
        "Phase distribution: Solid={} Liquid={} Gas={} Plasma={} Other={}",
        phase_solid, phase_liquid, phase_gas, phase_plasma, phase_other
    );

    // 域隔离区域详情
    let zones = &world.simulation.domain_isolation.zones;
    println!("\n=== Domain Isolation Zones ({}) ===", zones.len());
    for z in zones {
        println!(
            "  Zone {}: {:?} state={:?} center={:?} temp={:.1}K lifetime={:.2}s",
            z.id, z.domain, z.state, z.center, z.energy_bundle.temperature, z.lifetime
        );
    }

    println!("\n=== Test Complete ===");
}
