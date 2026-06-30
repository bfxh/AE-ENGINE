//! Phase 6 Step 2 验证: Dynamic rigid body -> MpssBuffer 同步测试
//!
//! 测试流程:
//! 1. 创建 GameWorld
//! 2. 在 MpssBuffer 中生成一个粒子 (位置 y=10.0)
//! 3. 在 PhysicsWorld 中创建一个 Dynamic rigid body, mpss_index 指向该粒子
//! 4. 运行 ticks, 验证 MpssBuffer 粒子位置跟随 rigid body 下落

use ae_engine::{GameWorld, WorldBounds};
use ae_engine::{FixedPoint, FixedQuat, FixedVec3};
use ae_engine::{BodyType, RigidBody};
use ae_engine::MaterialProperties;

fn main() {
    println!("=== Phase 6 Step 2: Dynamic Body Sync Test ===\n");

    let bounds = WorldBounds {
        min: glam::Vec3::new(-100.0, -10.0, -100.0),
        max: glam::Vec3::new(100.0, 100.0, 100.0),
    };
    let mut world = GameWorld::new(bounds);

    // 1. Spawn an MPSS particle at y=20.0
    let mpss_idx = world.simulation.mpss.spawn().expect("mpss spawn");
    world.simulation.mpss.pos[mpss_idx] = [0.0, 20.0, 0.0];
    world.simulation.mpss.vel[mpss_idx] = [0.0, 0.0, 0.0];
    world.simulation.mpss.temperature[mpss_idx] = 293.0;
    world.simulation.mpss.mass[mpss_idx] = 1.0;
    world.simulation.mpss.material_idx[mpss_idx] = 3; // iron

    println!("Initial MPSS particle idx={} pos=[0, 20, 0]", mpss_idx);

    // 2. Create a Dynamic rigid body linked to this MPSS particle
    let body = RigidBody {
        id: ae_engine::EngineUuid::new_v4(),
        position: FixedVec3::from_f32(0.0, 20.0, 0.0),
        rotation: FixedQuat::IDENTITY,
        velocity: FixedVec3::from_f32(0.0, 0.0, 0.0),
        angular_velocity: FixedVec3::ZERO,
        mass: FixedPoint::from_f32(1.0),
        material: MaterialProperties::default(),
        body_type: BodyType::Dynamic,
        is_sleeping: false,
        sleep_timer: FixedPoint::ZERO,
        forces: FixedVec3::ZERO,
        torque: FixedVec3::ZERO,
        linear_damping: FixedPoint::ZERO,
        angular_damping: FixedPoint::ZERO,
        mpss_index: Some(mpss_idx),
    };
    world.simulation.physics.add_rigid_body(body);
    println!("Added Dynamic rigid body at y=20.0 with mpss_index={}", mpss_idx);

    // 3. Run 60 ticks (1 second) and report positions
    println!("\nRunning 60 ticks (1 second @ 60Hz)...");
    println!("tick | rigid_body.y | mpss.y | drift");
    println!("-----|-------------|--------|------");

    let mut max_drift: f32 = 0.0;
    for i in 0..60 {
        world.tick();

        if (i + 1) % 10 == 0 {
            let rb = world.simulation.physics.rigid_bodies.iter()
                .find(|b| b.mpss_index == Some(mpss_idx))
                .expect("rigid body not found");
            let rb_pos = rb.position.to_glam();
            let mpss_pos = world.simulation.mpss.pos[mpss_idx];
            let drift = (rb_pos.y - mpss_pos[1]).abs();
            if drift > max_drift {
                max_drift = drift;
            }
            println!(
                "{:4} | {:11.4} | {:6.4} | {:.6}",
                i + 1, rb_pos.y, mpss_pos[1], drift
            );
        }
    }

    // 4. Final verification
    let rb = world.simulation.physics.rigid_bodies.iter()
        .find(|b| b.mpss_index == Some(mpss_idx))
        .expect("rigid body not found");
    let rb_pos = rb.position.to_glam();
    let mpss_pos = world.simulation.mpss.pos[mpss_idx];
    let final_drift = (rb_pos.y - mpss_pos[1]).abs();

    println!("\n=== Results ===");
    println!("Rigid body final pos: [{:.4}, {:.4}, {:.4}]", rb_pos.x, rb_pos.y, rb_pos.z);
    println!("MPSS particle final pos: [{:.4}, {:.4}, {:.4}]", mpss_pos[0], mpss_pos[1], mpss_pos[2]);
    println!("Final drift: {:.6} m", final_drift);
    println!("Max drift during simulation: {:.6} m", max_drift);

    // 5. Verdict
    println!("\n=== Verdict ===");
    if final_drift < 0.001 {
        println!("PASS: Phase 6 Step 2 sync working (drift < 1mm)");
    } else if final_drift < 0.1 {
        println!("WARN: Minor sync drift ({}m) - check interpolation", final_drift);
    } else {
        println!("FAIL: Sync drift too large ({}m) - rigid body and MPSS particle diverged", final_drift);
    }

    // 6. Verify gravity effect (both should have fallen)
    let fell_distance = 20.0 - rb_pos.y;
    println!("\nGravity test: body fell {:.4}m in 1s (expected ~4.9m for g=9.8)", fell_distance);
    if fell_distance > 1.0 {
        println!("PASS: Gravity is acting on the rigid body");
    } else {
        println!("WARN: Body did not fall significantly - check physics step");
    }
}