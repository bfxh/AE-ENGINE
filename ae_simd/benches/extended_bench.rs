use criterion::{Criterion, black_box, criterion_group};
use ae_simd::batch::*;
use ae_simd::soa::ParticleSoA;

fn bench_simd_physics_step(c: &mut Criterion) {
    let n = 10000;
    let mut x = vec![0.0f32; n];
    let mut y = vec![0.0f32; n];
    let mut z = vec![0.0f32; n];
    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];
    let mut vz = vec![0.0f32; n];
    let fx = vec![0.0f32; n];
    let fy = vec![-9.8f32; n];
    let fz = vec![0.0f32; n];
    let mass = vec![1.0f32; n];
    for i in 0..n {
        x[i] = (i as f32) * 0.1;
        y[i] = (i as f32) * 0.2;
        z[i] = (i as f32) * 0.3;
    }

    c.bench_function("batch_physics_step_10000", |bench| {
        bench.iter(|| {
            batch_physics_step(
                black_box(&mut x),
                black_box(&mut y),
                black_box(&mut z),
                black_box(&mut vx),
                black_box(&mut vy),
                black_box(&mut vz),
                black_box(&fx),
                black_box(&fy),
                black_box(&fz),
                black_box(&mass),
                0.016,
                n,
            );
        });
    });
}

fn bench_simd_scale(c: &mut Criterion) {
    let n = 10000;
    let mut data = vec![0.0f32; n];
    for (i, d) in data.iter_mut().enumerate().take(n) {
        *d = (i as f32) * 0.1;
    }

    c.bench_function("batch_scale_10000", |bench| {
        bench.iter(|| {
            batch_scale(black_box(&mut data), 2.0, n);
        });
    });
}

fn bench_soa_add(c: &mut Criterion) {
    let n = 5000;
    let mut a = ParticleSoA::with_capacity(n);
    let mut b = ParticleSoA::with_capacity(n);
    for i in 0..n {
        a.push(
            ((i as f32) * 0.1, (i as f32) * 0.2, (i as f32) * 0.3),
            ((i as f32) * 0.01, (i as f32) * 0.02, (i as f32) * 0.03),
            1.0,
        );
        b.push(
            ((i as f32) * 0.05, (i as f32) * 0.15, (i as f32) * 0.25),
            ((i as f32) * 0.005, (i as f32) * 0.015, (i as f32) * 0.025),
            1.0,
        );
    }

    c.bench_function("soa_add_5000", |bench| {
        bench.iter(|| {
            let mut result_a = a.clone();
            for i in 0..n {
                let (bx, by, bz) = black_box(&b).positions.get(i);
                result_a.positions.x[i] += bx;
                result_a.positions.y[i] += by;
                result_a.positions.z[i] += bz;
            }
            black_box(result_a);
        });
    });
}

fn bench_soa_dot(c: &mut Criterion) {
    let n = 5000;
    let mut a = ParticleSoA::with_capacity(n);
    let mut b = ParticleSoA::with_capacity(n);
    for i in 0..n {
        a.push(
            ((i as f32) * 0.1, (i as f32) * 0.2, (i as f32) * 0.3),
            ((i as f32) * 0.01, (i as f32) * 0.02, (i as f32) * 0.03),
            1.0,
        );
        b.push(
            ((i as f32) * 0.05, (i as f32) * 0.15, (i as f32) * 0.25),
            ((i as f32) * 0.005, (i as f32) * 0.015, (i as f32) * 0.025),
            1.0,
        );
    }

    c.bench_function("soa_dot_5000", |bench| {
        bench.iter(|| {
            let mut sum = 0.0f32;
            for i in 0..n {
                let (ax, ay, az) = black_box(&a).positions.get(i);
                let (bx, by, bz) = black_box(&b).positions.get(i);
                sum += ax * bx + ay * by + az * bz;
            }
            black_box(sum);
        });
    });
}

criterion_group!(
    extended_benches,
    bench_simd_physics_step,
    bench_simd_scale,
    bench_soa_add,
    bench_soa_dot
);
