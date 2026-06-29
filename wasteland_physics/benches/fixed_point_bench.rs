use criterion::{Criterion, black_box, criterion_group, criterion_main};
use wasteland_physics::fixed_point::{FixedPoint, FixedVec3};

fn bench_fixed_add(c: &mut Criterion) {
    let a = FixedPoint::from_f32(1.5);
    let b = FixedPoint::from_f32(2.75);
    c.bench_function("fixed_point_add", |bench| {
        bench.iter(|| {
            let _ = black_box(a) + black_box(b);
        });
    });
}

fn bench_fixed_mul(c: &mut Criterion) {
    let a = FixedPoint::from_f32(std::f32::consts::PI);
    let b = FixedPoint::from_f32(std::f32::consts::E);
    c.bench_function("fixed_point_mul", |bench| {
        bench.iter(|| {
            let _ = black_box(a) * black_box(b);
        });
    });
}

fn bench_fixed_div(c: &mut Criterion) {
    let a = FixedPoint::from_f32(100.0);
    let b = FixedPoint::from_f32(3.0);
    c.bench_function("fixed_point_div", |bench| {
        bench.iter(|| {
            let _ = black_box(a) / black_box(b);
        });
    });
}

fn bench_fixed_sqrt(c: &mut Criterion) {
    let v = FixedPoint::from_f32(2.0);
    c.bench_function("fixed_point_sqrt", |bench| {
        bench.iter(|| {
            let _ = black_box(v).sqrt();
        });
    });
}

fn bench_fixed_sin(c: &mut Criterion) {
    let v = FixedPoint::from_f32(1.0);
    c.bench_function("fixed_point_sin", |bench| {
        bench.iter(|| {
            let _ = black_box(v).sin();
        });
    });
}

fn bench_fixed_exp(c: &mut Criterion) {
    let v = FixedPoint::from_f32(0.5);
    c.bench_function("fixed_point_exp", |bench| {
        bench.iter(|| {
            let _ = black_box(v).exp();
        });
    });
}

fn bench_fixed_ln(c: &mut Criterion) {
    let v = FixedPoint::from_f32(2.0);
    c.bench_function("fixed_point_ln", |bench| {
        bench.iter(|| {
            let _ = black_box(v).ln();
        });
    });
}

fn bench_vec3_dot(c: &mut Criterion) {
    let a = FixedVec3::from_f32(1.0, 2.0, 3.0);
    let b = FixedVec3::from_f32(4.0, 5.0, 6.0);
    c.bench_function("fixed_vec3_dot", |bench| {
        bench.iter(|| {
            let _ = black_box(a).dot(black_box(b));
        });
    });
}

fn bench_vec3_cross(c: &mut Criterion) {
    let a = FixedVec3::from_f32(1.0, 2.0, 3.0);
    let b = FixedVec3::from_f32(4.0, 5.0, 6.0);
    c.bench_function("fixed_vec3_cross", |bench| {
        bench.iter(|| {
            let _ = black_box(a).cross(black_box(b));
        });
    });
}

fn bench_vec3_length(c: &mut Criterion) {
    let v = FixedVec3::from_f32(1.0, 2.0, 3.0);
    c.bench_function("fixed_vec3_length", |bench| {
        bench.iter(|| {
            let _ = black_box(v).length();
        });
    });
}

fn bench_fixed_mat3_det(c: &mut Criterion) {
    use wasteland_physics::fixed_point::FixedMat3;
    let m = FixedMat3 {
        x_axis: FixedVec3::from_f32(1.0, 0.0, 0.0),
        y_axis: FixedVec3::from_f32(0.0, 2.0, 0.0),
        z_axis: FixedVec3::from_f32(0.0, 0.0, 3.0),
    };
    c.bench_function("fixed_mat3_determinant", |bench| {
        bench.iter(|| {
            let _ = black_box(m).determinant();
        });
    });
}

fn bench_fixed_from_f32(c: &mut Criterion) {
    c.bench_function("fixed_point_from_f32", |bench| {
        bench.iter(|| {
            let _ = FixedPoint::from_f32(black_box(std::f32::consts::PI));
        });
    });
}

fn bench_fixed_to_f32(c: &mut Criterion) {
    let v = FixedPoint::from_f32(std::f32::consts::PI);
    c.bench_function("fixed_point_to_f32", |bench| {
        bench.iter(|| {
            let _ = black_box(v).to_f32();
        });
    });
}

criterion_group!(
    benches,
    bench_fixed_add,
    bench_fixed_mul,
    bench_fixed_div,
    bench_fixed_sqrt,
    bench_fixed_sin,
    bench_fixed_exp,
    bench_fixed_ln,
    bench_vec3_dot,
    bench_vec3_cross,
    bench_vec3_length,
    bench_fixed_mat3_det,
    bench_fixed_from_f32,
    bench_fixed_to_f32,
);
criterion_main!(benches);
