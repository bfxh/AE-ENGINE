use criterion::{Criterion, black_box, criterion_group, criterion_main};
use uuid::Uuid;
use wasteland_physics::collision::{CollisionEvent, CollisionId, CollisionShape, CollisionType};
use wasteland_physics::fixed_point::{FixedPoint, FixedVec3};
use wasteland_physics::material::MaterialProperties;
use wasteland_physics::octree::SparseOctree;

fn bench_octree_insert(c: &mut Criterion) {
    let mut octree = SparseOctree::new(
        FixedPoint::from_f32(200.0),
        8,
        FixedVec3::from_f32(-100.0, -100.0, -100.0),
        MaterialProperties::steel(),
    );

    c.bench_function("octree_insert_1000", |bench| {
        let mut positions: Vec<FixedVec3> = Vec::with_capacity(1000);
        for i in 0..1000 {
            let x = (i % 10) as f32 * 2.0 - 10.0;
            let y = (i / 10) as f32 * 2.0 - 10.0;
            let z = ((i / 100) % 10) as f32 * 2.0 - 10.0;
            positions.push(FixedVec3::from_f32(x, y, z));
        }
        bench.iter(|| {
            for pos in &positions {
                octree.activate_voxel(black_box(*pos));
            }
        });
    });
}

fn bench_octree_query(c: &mut Criterion) {
    let mut octree = SparseOctree::new(
        FixedPoint::from_f32(200.0),
        8,
        FixedVec3::from_f32(-100.0, -100.0, -100.0),
        MaterialProperties::steel(),
    );

    for i in 0..500 {
        let x = (i % 10) as f32 * 2.0 - 10.0;
        let y = (i / 10) as f32 * 2.0 - 10.0;
        octree.activate_voxel(FixedVec3::from_f32(x, y, 0.0));
    }

    c.bench_function("octree_is_active_100", |bench| {
        let queries: Vec<FixedVec3> = (0..100)
            .map(|i| {
                let x = (i % 10) as f32 * 2.0 - 10.0;
                let y = (i / 10) as f32 * 2.0 - 10.0;
                FixedVec3::from_f32(x, y, 0.0)
            })
            .collect();
        bench.iter(|| {
            for q in &queries {
                black_box(octree.is_active(black_box(*q)));
            }
        });
    });
}

fn bench_octree_spatial_query(c: &mut Criterion) {
    let mut octree = SparseOctree::new(
        FixedPoint::from_f32(200.0),
        8,
        FixedVec3::from_f32(-100.0, -100.0, -100.0),
        MaterialProperties::steel(),
    );

    for i in 0..500 {
        let x = (i % 10) as f32 * 2.0 - 10.0;
        let y = (i / 10) as f32 * 2.0 - 10.0;
        octree.activate_voxel(FixedVec3::from_f32(x, y, 0.0));
    }

    c.bench_function("octree_active_in_sphere", |bench| {
        let center = FixedVec3::from_f32(0.0, 0.0, 0.0);
        let radius = FixedPoint::from_f32(10.0);
        bench.iter(|| {
            black_box(octree.active_in_sphere(black_box(center), black_box(radius)));
        });
    });
}

fn bench_collision_shape_volume(c: &mut Criterion) {
    let shapes: Vec<CollisionShape> = vec![
        CollisionShape::Sphere { radius: FixedPoint::from_f32(1.0) },
        CollisionShape::Box { half_extents: FixedVec3::from_f32(0.5, 0.5, 0.5) },
        CollisionShape::Capsule {
            radius: FixedPoint::from_f32(0.3),
            half_height: FixedPoint::from_f32(1.0),
        },
        CollisionShape::Cylinder {
            radius: FixedPoint::from_f32(0.5),
            half_height: FixedPoint::from_f32(1.0),
        },
    ];

    c.bench_function("collision_shape_volume", |bench| {
        bench.iter(|| {
            for shape in &shapes {
                black_box(shape.volume());
            }
        });
    });
}

fn bench_material_damage(c: &mut Criterion) {
    let materials = vec![
        MaterialProperties::steel(),
        MaterialProperties::wood(),
        MaterialProperties::stone(),
        MaterialProperties::glass(),
        MaterialProperties::rubber(),
    ];

    let impact_force = FixedPoint::from_f32(1000.0);
    let impact_velocity = FixedVec3::from_f32(0.0, -10.0, 0.0);

    c.bench_function("material_damage_at_point", |bench| {
        bench.iter(|| {
            for mat in &materials {
                black_box(mat.damage_at_point(black_box(impact_force), black_box(impact_velocity)));
            }
        });
    });
}

fn bench_collision_damage_calculation(c: &mut Criterion) {
    let material = MaterialProperties::steel();
    let event = CollisionEvent {
        id: CollisionId(Uuid::new_v4()),
        entity_a: Uuid::new_v4(),
        entity_b: Uuid::new_v4(),
        point: FixedVec3::from_f32(1.0, 2.0, 3.0),
        normal: FixedVec3::from_f32(0.0, 1.0, 0.0),
        impulse: FixedPoint::from_f32(500.0),
        relative_velocity: FixedVec3::from_f32(0.0, -20.0, 0.0),
        material_a: material,
        material_b: MaterialProperties::stone(),
        timestamp: 1.0,
        collision_type: CollisionType::Impact,
    };

    c.bench_function("collision_damage_calculation", |bench| {
        bench.iter(|| {
            black_box(event.calculate_damage());
        });
    });
}

fn bench_octree_compression_ratio(c: &mut Criterion) {
    let mut octree = SparseOctree::new(
        FixedPoint::from_f32(200.0),
        8,
        FixedVec3::from_f32(-100.0, -100.0, -100.0),
        MaterialProperties::steel(),
    );

    for i in 0..500 {
        let x = (i % 10) as f32 * 2.0 - 10.0;
        let y = (i / 10) as f32 * 2.0 - 10.0;
        octree.activate_voxel(FixedVec3::from_f32(x, y, 0.0));
    }

    c.bench_function("octree_compression_ratio", |bench| {
        bench.iter(|| {
            black_box(octree.compression_ratio());
        });
    });
}

criterion_group!(
    benches,
    bench_octree_insert,
    bench_octree_query,
    bench_octree_spatial_query,
    bench_collision_shape_volume,
    bench_material_damage,
    bench_collision_damage_calculation,
    bench_octree_compression_ratio,
);
criterion_main!(benches);
