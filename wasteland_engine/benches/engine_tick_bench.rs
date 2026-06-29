use criterion::{Criterion, criterion_group, criterion_main};
use glam::Vec3;
use wasteland_game::npc::system::NpcSpecies;
use wasteland_engine::{GameWorld, WorldBounds};

fn bench_engine_tick_empty(c: &mut Criterion) {
    let bounds =
        WorldBounds { min: Vec3::new(-100.0, -100.0, -100.0), max: Vec3::new(100.0, 100.0, 100.0) };
    let mut world = GameWorld::new(bounds);

    c.bench_function("engine_tick_empty", |bench| {
        bench.iter(|| {
            world.tick();
        });
    });
}

fn bench_engine_tick_with_entities(c: &mut Criterion) {
    let bounds =
        WorldBounds { min: Vec3::new(-100.0, -100.0, -100.0), max: Vec3::new(100.0, 100.0, 100.0) };
    let mut world = GameWorld::new(bounds);

    for i in 0..100 {
        let x = (i % 10) as f32 * 5.0 - 25.0;
        let y = ((i / 10) % 10) as f32 * 5.0 - 25.0;
        let z = (i / 100) as f32 * 5.0 - 25.0;
        world.spawn_meta_entity_iron(Vec3::new(x, y, z));
    }

    c.bench_function("engine_tick_100_entities", |bench| {
        bench.iter(|| {
            world.tick();
        });
    });
}

fn bench_engine_tick_heavy(c: &mut Criterion) {
    let bounds =
        WorldBounds { min: Vec3::new(-200.0, -200.0, -200.0), max: Vec3::new(200.0, 200.0, 200.0) };
    let mut world = GameWorld::new(bounds);

    for i in 0..500 {
        let x = (i % 10) as f32 * 10.0 - 50.0;
        let y = ((i / 10) % 10) as f32 * 10.0 - 50.0;
        let z = ((i / 100) % 10) as f32 * 10.0 - 50.0;
        world.spawn_meta_entity_iron(Vec3::new(x, y, z));
    }

    for i in 0..10 {
        world.add_point_charge(
            (i as f32) * 5.0 - 25.0,
            0.0,
            0.0,
            if i % 2 == 0 { 1.0 } else { -1.0 },
        );
    }

    c.bench_function("engine_tick_500_entities_10_charges", |bench| {
        bench.iter(|| {
            world.tick();
        });
    });
}

fn bench_engine_tick_with_npc(c: &mut Criterion) {
    let bounds =
        WorldBounds { min: Vec3::new(-100.0, -100.0, -100.0), max: Vec3::new(100.0, 100.0, 100.0) };
    let mut world = GameWorld::new(bounds);

    for i in 0..50 {
        world.spawn_npc(
            &format!("npc_{}", i),
            Vec3::new((i % 10) as f32 * 3.0 - 15.0, 0.0, (i / 10) as f32 * 3.0 - 15.0),
            NpcSpecies::Human,
            "test_faction",
        );
    }

    c.bench_function("engine_tick_50_npcs", |bench| {
        bench.iter(|| {
            world.tick();
        });
    });
}

criterion_group!(
    benches,
    bench_engine_tick_empty,
    bench_engine_tick_with_entities,
    bench_engine_tick_heavy,
    bench_engine_tick_with_npc
);
criterion_main!(benches);
