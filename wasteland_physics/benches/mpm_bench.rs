use criterion::{Criterion, black_box, criterion_group, criterion_main};
use wasteland_physics::fixed_point::{FixedPoint, FixedVec3};
use wasteland_physics::mpm::{MpmConfig, MpmMaterialModel, MpmSimulation};

fn bench_mpm_simulation_step_small(c: &mut Criterion) {
    let config = MpmConfig {
        grid_resolution: [8, 8, 8],
        cell_size: FixedPoint::from_f32(0.1),
        particle_count: 10,
        youngs_modulus: FixedPoint::from_f32(1e3),
        poissons_ratio: FixedPoint::from_f32(0.3),
        yield_stress: FixedPoint::from_f32(1e4),
        hardening: FixedPoint::from_f32(0.1),
        density: FixedPoint::from_f32(1000.0),
        gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
        dt: FixedPoint::from_f32(0.016),
        substeps: 1,
        material_model: MpmMaterialModel::Elastic,
        enable_fracture: false,
        fracture_strain: FixedPoint::from_f32(0.15),
    };

    let mut sim = MpmSimulation::new(config);

    c.bench_function("mpm_step_100_particles", |bench| {
        bench.iter(|| {
            sim.step();
            black_box(sim.active_particles());
        });
    });
}

fn bench_mpm_simulation_step_medium(c: &mut Criterion) {
    let config = MpmConfig {
        grid_resolution: [8, 8, 8],
        cell_size: FixedPoint::from_f32(0.1),
        particle_count: 50,
        youngs_modulus: FixedPoint::from_f32(1e2),
        poissons_ratio: FixedPoint::from_f32(0.3),
        yield_stress: FixedPoint::from_f32(1e3),
        hardening: FixedPoint::from_f32(0.05),
        density: FixedPoint::from_f32(500.0),
        gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
        dt: FixedPoint::from_f32(0.016),
        substeps: 1,
        material_model: MpmMaterialModel::Elastic,
        enable_fracture: false,
        fracture_strain: FixedPoint::from_f32(0.05),
    };

    let mut sim = MpmSimulation::new(config);

    c.bench_function("mpm_step_500_particles", |bench| {
        bench.iter(|| {
            sim.step();
            black_box(sim.kinetic_energy());
        });
    });
}

fn bench_mpm_particle_creation(c: &mut Criterion) {
    let config = MpmConfig {
        grid_resolution: [8, 8, 8],
        cell_size: FixedPoint::from_f32(0.1),
        particle_count: 0,
        youngs_modulus: FixedPoint::from_f32(1e3),
        poissons_ratio: FixedPoint::from_f32(0.3),
        yield_stress: FixedPoint::from_f32(1e4),
        hardening: FixedPoint::from_f32(0.1),
        density: FixedPoint::from_f32(1000.0),
        gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
        dt: FixedPoint::from_f32(0.016),
        substeps: 1,
        material_model: MpmMaterialModel::Elastic,
        enable_fracture: false,
        fracture_strain: FixedPoint::from_f32(0.15),
    };

    let mut sim = MpmSimulation::new(config);

    c.bench_function("mpm_add_1000_particles", |bench| {
        bench.iter(|| {
            for i in 0..1000 {
                let x = (i % 10) as f32 * 0.1;
                let y = (i / 10) as f32 * 0.1;
                sim.add_particle(
                    FixedVec3::from_f32(x, y, 0.0),
                    FixedVec3::from_f32(0.0, 0.0, 0.0),
                    FixedPoint::from_f32(0.01),
                    [1.0, 0.5, 0.2, 1.0],
                );
            }
            black_box(sim.active_particles());
        });
    });
}

criterion_group!(
    benches,
    bench_mpm_simulation_step_small,
    bench_mpm_simulation_step_medium,
    bench_mpm_particle_creation,
);
criterion_main!(benches);
