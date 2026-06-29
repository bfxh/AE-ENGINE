//! LOD 漂移调试：每 10 ticks 打印 player_position 和粒子位置范围
use wasteland_engine::{GameWorld, WorldBounds};

fn main() {
    println!("=== LOD Drift Debug ===");
    let bounds = WorldBounds {
        min: glam::Vec3::new(-500.0, -100.0, -500.0),
        max: glam::Vec3::new(500.0, 300.0, 500.0),
    };
    let mut world = GameWorld::new(bounds);

    let player_initial = world.simulation.player_position;
    println!("Initial player_position: {:?}", player_initial);
    println!(
        "Initial mpss count: {} (near 0..5,5 mid 100,5 far 300,5 high 0,5,15 low 0,5,-15)",
        world.simulation.mpss.count
    );

    // 打印初始 5 个粒子的位置
    println!("\n--- First 5 particles initial pos ---");
    for i in 0..5 {
        println!(
            "  [{}] pos={:?} temp={:.1} mat={} active={}",
            i,
            world.simulation.mpss.pos[i],
            world.simulation.mpss.temperature[i],
            world.simulation.mpss.material_idx[i],
            world.simulation.mpss.active[i]
        );
    }
    // 打印 20-25, 25-30, 45-55 的粒子
    println!("\n--- Particles 20-24 (mid), 30-34 (high temp), 50-54 (low temp) ---");
    for i in [20, 21, 30, 31, 50, 51] {
        if i < world.simulation.mpss.count {
            println!(
                "  [{}] pos={:?} temp={:.1} mat={}",
                i,
                world.simulation.mpss.pos[i],
                world.simulation.mpss.temperature[i],
                world.simulation.mpss.material_idx[i]
            );
        }
    }

    println!("\n--- Running 600 ticks, printing every 30 ticks ---");
    for i in 0..600 {
        world.tick();
        if (i + 1) % 30 == 0 {
            let lod = &world.simulation.lod_stats;
            let pp = world.simulation.player_position;
            // 计算粒子分布范围
            let mut min_pos = [f32::MAX; 3];
            let mut max_pos = [f32::MIN; 3];
            let mut near_count_at_zero = 0usize;
            for j in 0..world.simulation.mpss.count {
                if !world.simulation.mpss.active[j] {
                    continue;
                }
                for d in 0..3 {
                    min_pos[d] = min_pos[d].min(world.simulation.mpss.pos[j][d]);
                    max_pos[d] = max_pos[d].max(world.simulation.mpss.pos[j][d]);
                }
                let dx = world.simulation.mpss.pos[j][0];
                let dy = world.simulation.mpss.pos[j][1];
                let dz = world.simulation.mpss.pos[j][2];
                let dist_sq = dx * dx + dy * dy + dz * dz;
                if dist_sq < 50.0 * 50.0 {
                    near_count_at_zero += 1;
                }
            }
            println!(
                "t={:.1}s tick={} T={:.1}K LOD(n/m/f)={}/{}/{} player_pos={:?} pos_range=[{:.1},{:.1}][{:.1},{:.1}][{:.1},{:.1}] near_at_zero={}",
                world.time,
                world.tick_count,
                world.global_temperature,
                lod.near,
                lod.mid,
                lod.far,
                pp,
                min_pos[0], max_pos[0],
                min_pos[1], max_pos[1],
                min_pos[2], max_pos[2],
                near_count_at_zero
            );
        }
    }
}
