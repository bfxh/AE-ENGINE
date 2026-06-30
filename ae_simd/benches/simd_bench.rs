use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ae_simd::batch::*;
use ae_simd::soa::ParticleSoA;

fn bench_simd_dot3(c: &mut Criterion) {
    let n = 10000;
    let mut x0 = vec![0.0f32; n];
    let mut y0 = vec![0.0f32; n];
    let mut z0 = vec![0.0f32; n];
    let mut x1 = vec![0.0f32; n];
    let mut y1 = vec![0.0f32; n];
    let mut z1 = vec![0.0f32; n];
    let mut results = vec![0.0f32; n];
    for i in 0..n {
        x0[i] = (i as f32) * 0.1;
        y0[i] = (i as f32) * 0.2;
        z0[i] = (i as f32) * 0.3;
        x1[i] = ((i + 1) % n) as f32 * 0.1;
        y1[i] = ((i + 1) % n) as f32 * 0.2;
        z1[i] = ((i + 1) % n) as f32 * 0.3;
    }

    c.bench_function("simd_dot3_10000", |bench| {
        bench.iter(|| {
            batch_dot3(
                black_box(&x0),
                black_box(&y0),
                black_box(&z0),
                black_box(&x1),
                black_box(&y1),
                black_box(&z1),
                black_box(&mut results),
                n,
            );
        });
    });
}

fn bench_simd_cross3(c: &mut Criterion) {
    let n = 10000;
    let mut x0 = vec![0.0f32; n];
    let mut y0 = vec![0.0f32; n];
    let mut z0 = vec![0.0f32; n];
    let mut x1 = vec![0.0f32; n];
    let mut y1 = vec![0.0f32; n];
    let mut z1 = vec![0.0f32; n];
    let mut rx = vec![0.0f32; n];
    let mut ry = vec![0.0f32; n];
    let mut rz = vec![0.0f32; n];
    for i in 0..n {
        x0[i] = (i as f32) * 0.1;
        y0[i] = (i as f32) * 0.2;
        z0[i] = (i as f32) * 0.3;
        x1[i] = ((i + 1) % n) as f32 * 0.1;
        y1[i] = ((i + 1) % n) as f32 * 0.2;
        z1[i] = ((i + 1) % n) as f32 * 0.3;
    }

    c.bench_function("simd_cross3_10000", |bench| {
        bench.iter(|| {
            batch_cross3(
                black_box(&x0),
                black_box(&y0),
                black_box(&z0),
                black_box(&x1),
                black_box(&y1),
                black_box(&z1),
                black_box(&mut rx),
                black_box(&mut ry),
                black_box(&mut rz),
                n,
            );
        });
    });
}

fn bench_simd_length3(c: &mut Criterion) {
    let n = 10000;
    let mut x = vec![0.0f32; n];
    let mut y = vec![0.0f32; n];
    let mut z = vec![0.0f32; n];
    let mut results = vec![0.0f32; n];
    for i in 0..n {
        x[i] = (i as f32) * 0.1;
        y[i] = (i as f32) * 0.2;
        z[i] = (i as f32) * 0.3;
    }

    c.bench_function("simd_length3_10000", |bench| {
        bench.iter(|| {
            batch_length3(black_box(&x), black_box(&y), black_box(&z), black_box(&mut results), n);
        });
    });
}

fn bench_simd_sincos(c: &mut Criterion) {
    let n = 10000;
    let mut angles = vec![0.0f32; n];
    let mut sin_out = vec![0.0f32; n];
    let mut cos_out = vec![0.0f32; n];
    for (i, angle) in angles.iter_mut().enumerate().take(n) {
        *angle = (i as f32) * 0.001;
    }

    c.bench_function("simd_sincos_10000", |bench| {
        bench.iter(|| {
            batch_sin_cos(black_box(&angles), black_box(&mut sin_out), black_box(&mut cos_out), n);
        });
    });
}

fn bench_simd_soa_push(c: &mut Criterion) {
    c.bench_function("simd_soa_push_10000", |bench| {
        bench.iter(|| {
            let mut soa = ParticleSoA::with_capacity(10000);
            for i in 0..10000 {
                soa.push(
                    ((i as f32) * 0.1, (i as f32) * 0.2, (i as f32) * 0.3),
                    ((i as f32) * 0.01, (i as f32) * 0.02, (i as f32) * 0.03),
                    1.0,
                );
            }
            black_box(soa);
        });
    });
}

criterion_group!(
    benches,
    bench_simd_dot3,
    bench_simd_cross3,
    bench_simd_length3,
    bench_simd_sincos,
    bench_simd_soa_push
);
criterion_main!(benches);
