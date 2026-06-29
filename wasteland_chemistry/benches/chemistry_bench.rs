use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::Vec3;
use wasteland_chemistry::kinetics::{KineticModel, RateConstant, ReactionOrder};
use wasteland_chemistry::reactions::{ChemicalReaction, ReactionSystem};
use wasteland_chemistry::thermodynamics::ThermodynamicsEngine;

fn bench_single_reaction(c: &mut Criterion) {
    let mut system = ReactionSystem::new();
    let reaction = ChemicalReaction::combustion_organic();
    system.trigger_reaction(reaction, Vec3::ZERO, 1.0);

    c.bench_function("chemistry_single_reaction", |bench| {
        bench.iter(|| {
            system.update(black_box(0.016), black_box(0.0));
        });
    });
}

fn bench_many_reactions(c: &mut Criterion) {
    let mut system = ReactionSystem::new();
    for i in 0..100 {
        let reaction = if i % 3 == 0 {
            ChemicalReaction::combustion_organic()
        } else if i % 3 == 1 {
            ChemicalReaction::rust_formation()
        } else {
            ChemicalReaction::acid_rain_formation()
        };
        system.trigger_reaction(reaction, Vec3::new(i as f32, 0.0, 0.0), 1.0);
    }

    c.bench_function("chemistry_100_reactions", |bench| {
        bench.iter(|| {
            system.update(black_box(0.016), black_box(0.0));
        });
    });
}

fn bench_thermo_calculation(c: &mut Criterion) {
    let engine = ThermodynamicsEngine::new(1024);

    c.bench_function("chemistry_gibbs_free_energy", |bench| {
        bench.iter(|| {
            for t in 0..100 {
                let temp = 273.0 + (t as f32) * 5.0;
                let _ = engine.reaction_rate_estimate(black_box(50.0), black_box(temp));
            }
        });
    });
}

fn bench_kinetics_step(c: &mut Criterion) {
    let model =
        KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0));

    c.bench_function("chemistry_kinetics_1000_steps", |bench| {
        bench.iter(|| {
            for _ in 0..1000 {
                black_box(model.calculate_rate(black_box(&[1.0]), black_box(300.0)));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_single_reaction,
    bench_many_reactions,
    bench_thermo_calculation,
    bench_kinetics_step
);
criterion_main!(benches);
