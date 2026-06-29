#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
use std::arch::x86_64::*;

pub fn batch_dot3(
    x0: &[f32],
    y0: &[f32],
    z0: &[f32],
    x1: &[f32],
    y1: &[f32],
    z1: &[f32],
    results: &mut [f32],
    count: usize,
) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_dot3_avx2(x0, y0, z0, x1, y1, z1, results, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            results[i] = x0[i] * x1[i] + y0[i] * y1[i] + z0[i] * z1[i];
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn batch_dot3_avx2(
    x0: &[f32],
    y0: &[f32],
    z0: &[f32],
    x1: &[f32],
    y1: &[f32],
    z1: &[f32],
    results: &mut [f32],
    count: usize,
) {
    let mut i = 0;
    while i + 8 <= count {
        let vx0 = _mm256_loadu_ps(x0.as_ptr().add(i));
        let vy0 = _mm256_loadu_ps(y0.as_ptr().add(i));
        let vz0 = _mm256_loadu_ps(z0.as_ptr().add(i));
        let vx1 = _mm256_loadu_ps(x1.as_ptr().add(i));
        let vy1 = _mm256_loadu_ps(y1.as_ptr().add(i));
        let vz1 = _mm256_loadu_ps(z1.as_ptr().add(i));

        let dot = _mm256_fmadd_ps(_mm256_fmadd_ps(_mm256_mul_ps(vx0, vx1), vy0, vy1), vz0, vz1);

        _mm256_storeu_ps(results.as_mut_ptr().add(i), dot);
        i += 8;
    }
    for j in i..count {
        results[j] = x0[j] * x1[j] + y0[j] * y1[j] + z0[j] * z1[j];
    }
}

#[allow(clippy::too_many_arguments)]
pub fn batch_cross3(
    x0: &[f32],
    y0: &[f32],
    z0: &[f32],
    x1: &[f32],
    y1: &[f32],
    z1: &[f32],
    rx: &mut [f32],
    ry: &mut [f32],
    rz: &mut [f32],
    count: usize,
) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_cross3_avx2(x0, y0, z0, x1, y1, z1, rx, ry, rz, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            rx[i] = y0[i] * z1[i] - z0[i] * y1[i];
            ry[i] = z0[i] * x1[i] - x0[i] * z1[i];
            rz[i] = x0[i] * y1[i] - y0[i] * x1[i];
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
#[allow(clippy::too_many_arguments)]
unsafe fn batch_cross3_avx2(
    x0: &[f32],
    y0: &[f32],
    z0: &[f32],
    x1: &[f32],
    y1: &[f32],
    z1: &[f32],
    rx: &mut [f32],
    ry: &mut [f32],
    rz: &mut [f32],
    count: usize,
) {
    let mut i = 0;
    while i + 8 <= count {
        let vx0 = _mm256_loadu_ps(x0.as_ptr().add(i));
        let vy0 = _mm256_loadu_ps(y0.as_ptr().add(i));
        let vz0 = _mm256_loadu_ps(z0.as_ptr().add(i));
        let vx1 = _mm256_loadu_ps(x1.as_ptr().add(i));
        let vy1 = _mm256_loadu_ps(y1.as_ptr().add(i));
        let vz1 = _mm256_loadu_ps(z1.as_ptr().add(i));

        let vy0z1 = _mm256_mul_ps(vy0, vz1);
        let vz0y1 = _mm256_mul_ps(vz0, vy1);
        let vz0x1 = _mm256_mul_ps(vz0, vx1);
        let vx0z1 = _mm256_mul_ps(vx0, vz1);
        let vx0y1 = _mm256_mul_ps(vx0, vy1);
        let vy0x1 = _mm256_mul_ps(vy0, vx1);

        _mm256_storeu_ps(rx.as_mut_ptr().add(i), _mm256_sub_ps(vy0z1, vz0y1));
        _mm256_storeu_ps(ry.as_mut_ptr().add(i), _mm256_sub_ps(vz0x1, vx0z1));
        _mm256_storeu_ps(rz.as_mut_ptr().add(i), _mm256_sub_ps(vx0y1, vy0x1));
        i += 8;
    }
    for j in i..count {
        rx[j] = y0[j] * z1[j] - z0[j] * y1[j];
        ry[j] = z0[j] * x1[j] - x0[j] * z1[j];
        rz[j] = x0[j] * y1[j] - y0[j] * x1[j];
    }
}

pub fn batch_length3(x: &[f32], y: &[f32], z: &[f32], results: &mut [f32], count: usize) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_length3_avx2(x, y, z, results, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            results[i] = (x[i] * x[i] + y[i] * y[i] + z[i] * z[i]).sqrt();
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn batch_length3_avx2(x: &[f32], y: &[f32], z: &[f32], results: &mut [f32], count: usize) {
    let mut i = 0;
    while i + 8 <= count {
        let vx = _mm256_loadu_ps(x.as_ptr().add(i));
        let vy = _mm256_loadu_ps(y.as_ptr().add(i));
        let vz = _mm256_loadu_ps(z.as_ptr().add(i));

        let sq = _mm256_fmadd_ps(_mm256_fmadd_ps(_mm256_mul_ps(vx, vx), vy, vy), vz, vz);
        let lengths = _mm256_sqrt_ps(sq);
        _mm256_storeu_ps(results.as_mut_ptr().add(i), lengths);
        i += 8;
    }
    for j in i..count {
        results[j] = (x[j] * x[j] + y[j] * y[j] + z[j] * z[j]).sqrt();
    }
}

pub fn batch_normalize3(x: &mut [f32], y: &mut [f32], z: &mut [f32], count: usize) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_normalize3_avx2(x, y, z, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            let len = (x[i] * x[i] + y[i] * y[i] + z[i] * z[i]).sqrt();
            if len > 1e-10 {
                let inv = 1.0 / len;
                x[i] *= inv;
                y[i] *= inv;
                z[i] *= inv;
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn batch_normalize3_avx2(x: &mut [f32], y: &mut [f32], z: &mut [f32], count: usize) {
    let mut i = 0;
    let eps = _mm256_set1_ps(1e-10);
    while i + 8 <= count {
        let vx = _mm256_loadu_ps(x.as_ptr().add(i));
        let vy = _mm256_loadu_ps(y.as_ptr().add(i));
        let vz = _mm256_loadu_ps(z.as_ptr().add(i));

        let sq = _mm256_fmadd_ps(_mm256_fmadd_ps(_mm256_mul_ps(vx, vx), vy, vy), vz, vz);
        let len = _mm256_sqrt_ps(sq);
        let mask = _mm256_cmp_ps(len, eps, _CMP_GT_OQ);
        let inv = _mm256_div_ps(_mm256_set1_ps(1.0), len);
        let inv_safe = _mm256_blendv_ps(_mm256_setzero_ps(), inv, mask);

        _mm256_storeu_ps(x.as_mut_ptr().add(i), _mm256_mul_ps(vx, inv_safe));
        _mm256_storeu_ps(y.as_mut_ptr().add(i), _mm256_mul_ps(vy, inv_safe));
        _mm256_storeu_ps(z.as_mut_ptr().add(i), _mm256_mul_ps(vz, inv_safe));
        i += 8;
    }
    for j in i..count {
        let len = (x[j] * x[j] + y[j] * y[j] + z[j] * z[j]).sqrt();
        if len > 1e-10 {
            let inv = 1.0 / len;
            x[j] *= inv;
            y[j] *= inv;
            z[j] *= inv;
        }
    }
}

pub fn batch_scale(x: &mut [f32], factor: f32, count: usize) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_scale_avx2(x, factor, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for (i, xi) in x.iter_mut().enumerate().take(count) {
            *xi *= factor;
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn batch_scale_avx2(x: &mut [f32], factor: f32, count: usize) {
    let vf = _mm256_set1_ps(factor);
    let mut i = 0;
    while i + 8 <= count {
        let v = _mm256_loadu_ps(x.as_ptr().add(i));
        _mm256_storeu_ps(x.as_mut_ptr().add(i), _mm256_mul_ps(v, vf));
        i += 8;
    }
    for xi in x.iter_mut().skip(i).take(count - i) {
        *xi *= factor;
    }
}

pub fn batch_sqrt(input: &[f32], output: &mut [f32], count: usize) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_sqrt_avx2(input, output, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            output[i] = input[i].sqrt();
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2")]
unsafe fn batch_sqrt_avx2(input: &[f32], output: &mut [f32], count: usize) {
    let mut i = 0;
    while i + 8 <= count {
        let v = _mm256_loadu_ps(input.as_ptr().add(i));
        _mm256_storeu_ps(output.as_mut_ptr().add(i), _mm256_sqrt_ps(v));
        i += 8;
    }
    for j in i..count {
        output[j] = input[j].sqrt();
    }
}

pub fn batch_sin_cos(angles: &[f32], sin_out: &mut [f32], cos_out: &mut [f32], count: usize) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        if count >= 8 {
            batch_sin_cos_avx2(angles, sin_out, cos_out, count);
            return;
        }
    }
    for i in 0..count {
        sin_out[i] = angles[i].sin();
        cos_out[i] = angles[i].cos();
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2")]
unsafe fn batch_sin_cos_avx2(
    angles: &[f32],
    sin_out: &mut [f32],
    cos_out: &mut [f32],
    count: usize,
) {
    let half_pi = _mm256_set1_ps(std::f32::consts::FRAC_PI_2);
    let two_over_pi = _mm256_set1_ps(2.0 / std::f32::consts::PI);

    let mut i = 0;
    while i + 8 <= count {
        let mut x = _mm256_loadu_ps(angles.as_ptr().add(i));

        let k = _mm256_cvttps_epi32(_mm256_mul_ps(x, two_over_pi));
        let kf = _mm256_cvtepi32_ps(k);
        x = _mm256_sub_ps(x, _mm256_mul_ps(kf, half_pi));

        let x2 = _mm256_mul_ps(x, x);
        let x3 = _mm256_mul_ps(x2, x);
        let x4 = _mm256_mul_ps(x2, x2);
        let x5 = _mm256_mul_ps(x3, x2);
        let x6 = _mm256_mul_ps(x4, x2);
        let x7 = _mm256_mul_ps(x5, x2);

        let inv6 = _mm256_set1_ps(1.0 / 6.0);
        let inv120 = _mm256_set1_ps(1.0 / 120.0);
        let inv5040 = _mm256_set1_ps(1.0 / 5040.0);
        let inv2 = _mm256_set1_ps(0.5);
        let inv24 = _mm256_set1_ps(1.0 / 24.0);
        let inv720 = _mm256_set1_ps(1.0 / 720.0);

        let sin = _mm256_fmadd_ps(
            _mm256_fnmadd_ps(
                _mm256_mul_ps(x7, inv5040),
                _mm256_mul_ps(x5, inv120),
                _mm256_setzero_ps(),
            ),
            _mm256_set1_ps(1.0),
            _mm256_mul_ps(x3, inv6),
        );
        let sin_approx = _mm256_fnmadd_ps(x, sin, _mm256_setzero_ps());

        let cos = _mm256_fnmadd_ps(
            _mm256_fnmadd_ps(
                _mm256_mul_ps(x6, inv720),
                _mm256_mul_ps(x4, inv24),
                _mm256_setzero_ps(),
            ),
            _mm256_set1_ps(1.0),
            _mm256_mul_ps(x2, inv2),
        );

        let quad_mask_sin = _mm256_and_si256(k, _mm256_set1_epi32(1));
        let quad_mask_cos =
            _mm256_and_si256(_mm256_add_epi32(k, _mm256_set1_epi32(1)), _mm256_set1_epi32(1));
        let quad_f32_sin = _mm256_cvtepi32_ps(quad_mask_sin);
        let quad_f32_cos = _mm256_cvtepi32_ps(quad_mask_cos);

        let neg_one = _mm256_set1_ps(-1.0);
        let sign_mask_sin = _mm256_fmadd_ps(quad_f32_sin, neg_one, _mm256_set1_ps(1.0));
        let sign_mask_cos = _mm256_fmadd_ps(quad_f32_cos, neg_one, _mm256_set1_ps(1.0));

        _mm256_storeu_ps(sin_out.as_mut_ptr().add(i), _mm256_mul_ps(sin_approx, sign_mask_sin));
        _mm256_storeu_ps(cos_out.as_mut_ptr().add(i), _mm256_mul_ps(cos, sign_mask_cos));
        i += 8;
    }
    for j in i..count {
        sin_out[j] = angles[j].sin();
        cos_out[j] = angles[j].cos();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn batch_physics_step(
    px: &mut [f32],
    py: &mut [f32],
    pz: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    vz: &mut [f32],
    fx: &[f32],
    fy: &[f32],
    fz: &[f32],
    inv_mass: &[f32],
    dt: f32,
    count: usize,
) {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        batch_physics_step_avx2(px, py, pz, vx, vy, vz, fx, fy, fz, inv_mass, dt, count);
        return;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..count {
            let im = inv_mass[i];
            vx[i] += fx[i] * im * dt;
            vy[i] += fy[i] * im * dt;
            vz[i] += fz[i] * im * dt;
            px[i] += vx[i] * dt;
            py[i] += vy[i] * dt;
            pz[i] += vz[i] * dt;
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[target_feature(enable = "avx2,fma")]
#[allow(clippy::too_many_arguments)]
unsafe fn batch_physics_step_avx2(
    px: &mut [f32],
    py: &mut [f32],
    pz: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    vz: &mut [f32],
    fx: &[f32],
    fy: &[f32],
    fz: &[f32],
    inv_mass: &[f32],
    dt: f32,
    count: usize,
) {
    let vdt = _mm256_set1_ps(dt);
    let mut i = 0;
    while i + 8 <= count {
        let v_im = _mm256_loadu_ps(inv_mass.as_ptr().add(i));
        let v_fx = _mm256_loadu_ps(fx.as_ptr().add(i));
        let v_fy = _mm256_loadu_ps(fy.as_ptr().add(i));
        let v_fz = _mm256_loadu_ps(fz.as_ptr().add(i));
        let mut v_vx = _mm256_loadu_ps(vx.as_ptr().add(i));
        let mut v_vy = _mm256_loadu_ps(vy.as_ptr().add(i));
        let mut v_vz = _mm256_loadu_ps(vz.as_ptr().add(i));
        let mut v_px = _mm256_loadu_ps(px.as_ptr().add(i));
        let mut v_py = _mm256_loadu_ps(py.as_ptr().add(i));
        let mut v_pz = _mm256_loadu_ps(pz.as_ptr().add(i));

        let acc_x = _mm256_mul_ps(_mm256_mul_ps(v_fx, v_im), vdt);
        let acc_y = _mm256_mul_ps(_mm256_mul_ps(v_fy, v_im), vdt);
        let acc_z = _mm256_mul_ps(_mm256_mul_ps(v_fz, v_im), vdt);

        v_vx = _mm256_add_ps(v_vx, acc_x);
        v_vy = _mm256_add_ps(v_vy, acc_y);
        v_vz = _mm256_add_ps(v_vz, acc_z);

        v_px = _mm256_fmadd_ps(v_vx, vdt, v_px);
        v_py = _mm256_fmadd_ps(v_vy, vdt, v_py);
        v_pz = _mm256_fmadd_ps(v_vz, vdt, v_pz);

        _mm256_storeu_ps(vx.as_mut_ptr().add(i), v_vx);
        _mm256_storeu_ps(vy.as_mut_ptr().add(i), v_vy);
        _mm256_storeu_ps(vz.as_mut_ptr().add(i), v_vz);
        _mm256_storeu_ps(px.as_mut_ptr().add(i), v_px);
        _mm256_storeu_ps(py.as_mut_ptr().add(i), v_py);
        _mm256_storeu_ps(pz.as_mut_ptr().add(i), v_pz);
        i += 8;
    }
    for j in i..count {
        let im = inv_mass[j];
        vx[j] += fx[j] * im * dt;
        vy[j] += fy[j] * im * dt;
        vz[j] += fz[j] * im * dt;
        px[j] += vx[j] * dt;
        py[j] += vy[j] * dt;
        pz[j] += vz[j] * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_dot3() {
        let x0 = [1.0, 2.0, 3.0, 4.0];
        let y0 = [0.0, 0.0, 0.0, 0.0];
        let z0 = [0.0, 0.0, 0.0, 0.0];
        let x1 = [1.0, 1.0, 1.0, 1.0];
        let y1 = [0.0, 0.0, 0.0, 0.0];
        let z1 = [0.0, 0.0, 0.0, 0.0];
        let mut results = [0.0; 4];
        batch_dot3(&x0, &y0, &z0, &x1, &y1, &z1, &mut results, 4);
        assert!((results[0] - 1.0).abs() < 0.001);
        assert!((results[1] - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_batch_cross3() {
        let x0 = [1.0, 0.0, 0.0];
        let y0 = [0.0, 1.0, 0.0];
        let z0 = [0.0, 0.0, 1.0];
        let x1 = [0.0, 0.0, 1.0];
        let y1 = [1.0, 0.0, 0.0];
        let z1 = [0.0, 1.0, 0.0];
        let mut rx = [0.0; 3];
        let mut ry = [0.0; 3];
        let mut rz = [0.0; 3];
        batch_cross3(&x0, &y0, &z0, &x1, &y1, &z1, &mut rx, &mut ry, &mut rz, 3);
        assert!((rx[0] - 0.0).abs() < 0.01);
        assert!((ry[0] - 0.0).abs() < 0.01);
        assert!((rz[0] - 1.0).abs() < 0.01);
        assert!((rx[1] - 1.0).abs() < 0.01);
        assert!((ry[1] - 0.0).abs() < 0.01);
        assert!((rz[1] - 0.0).abs() < 0.01);
        assert!((rx[2] - 0.0).abs() < 0.01);
        assert!((ry[2] - 1.0).abs() < 0.01);
        assert!((rz[2] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_batch_length3() {
        let x = [3.0, 0.0];
        let y = [4.0, 0.0];
        let z = [0.0, 5.0];
        let mut results = [0.0; 2];
        batch_length3(&x, &y, &z, &mut results, 2);
        assert!((results[0] - 5.0).abs() < 0.001);
        assert!((results[1] - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_batch_sin_cos() {
        let angles = [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI];
        let mut sin_out = [0.0; 3];
        let mut cos_out = [0.0; 3];
        batch_sin_cos(&angles, &mut sin_out, &mut cos_out, 3);
        assert!(sin_out[0].abs() < 0.01);
        assert!((cos_out[0] - 1.0).abs() < 0.01);
        assert!((sin_out[1] - 1.0).abs() < 0.01);
        assert!(cos_out[1].abs() < 0.01);
    }

    #[test]
    fn test_batch_physics_step() {
        let mut px = [0.0f32; 4];
        let mut py = [0.0f32; 4];
        let mut pz = [0.0f32; 4];
        let mut vx = [0.0f32; 4];
        let mut vy = [0.0f32; 4];
        let mut vz = [0.0f32; 4];
        let fx = [1.0, 0.0, 0.0, 0.0];
        let fy = [0.0, 2.0, 0.0, 0.0];
        let fz = [0.0, 0.0, 3.0, 0.0];
        let inv_mass = [1.0, 0.5, 0.25, 1.0];
        let dt = 0.5;

        batch_physics_step(
            &mut px, &mut py, &mut pz, &mut vx, &mut vy, &mut vz, &fx, &fy, &fz, &inv_mass, dt, 4,
        );

        assert!((vx[0] - 0.5).abs() < 0.001);
        assert!((vy[0] - 0.0).abs() < 0.001);
        assert!((vz[0] - 0.0).abs() < 0.001);
        assert!((px[0] - 0.25).abs() < 0.001);

        assert!((vx[1] - 0.0).abs() < 0.001);
        assert!((vy[1] - 0.5).abs() < 0.001);
        assert!((vz[1] - 0.0).abs() < 0.001);
        assert!((py[1] - 0.25).abs() < 0.001);

        assert!((vx[2] - 0.0).abs() < 0.001);
        assert!((vy[2] - 0.0).abs() < 0.001);
        assert!((vz[2] - 0.375).abs() < 0.001);
        assert!((pz[2] - 0.1875).abs() < 0.001);

        assert!((vx[3] - 0.0).abs() < 0.001);
        assert!((pz[3] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_batch_normalize3() {
        let mut x = [3.0, 0.0, 1.0];
        let mut y = [0.0, 4.0, 1.0];
        let mut z = [0.0, 0.0, 1.0];
        batch_normalize3(&mut x, &mut y, &mut z, 3);
        assert!((x[0] - 1.0).abs() < 0.001);
        assert!((y[0] - 0.0).abs() < 0.001);
        assert!((z[0] - 0.0).abs() < 0.001);
        assert!((x[1] - 0.0).abs() < 0.001);
        assert!((y[1] - 1.0).abs() < 0.001);
        let len = (x[2] * x[2] + y[2] * y[2] + z[2] * z[2]).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_batch_scale() {
        let mut x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let count = 9;
        batch_scale(&mut x, 2.0, count);
        for (i, &xi) in x.iter().enumerate().take(count) {
            assert!((xi - ((i + 1) as f32 * 2.0)).abs() < 0.001);
        }
    }

    #[test]
    fn test_batch_sqrt() {
        let input = [1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0, 81.0, 100.0];
        let mut output = [0.0f32; 10];
        batch_sqrt(&input, &mut output, 10);
        for (i, &output_i) in output.iter().enumerate() {
            let expected = ((i + 1) as f32).powi(2).sqrt();
            assert!((output_i - expected).abs() < 0.001);
        }
    }

    #[test]
    fn test_batch_dot3_all_ones() {
        let x0 = [1.0; 16];
        let y0 = [1.0; 16];
        let z0 = [1.0; 16];
        let x1 = [1.0; 16];
        let y1 = [1.0; 16];
        let z1 = [1.0; 16];
        let mut results = [0.0; 16];
        batch_dot3(&x0, &y0, &z0, &x1, &y1, &z1, &mut results, 16);
        for r in &results {
            assert!((r - 3.0).abs() < 0.001);
        }
    }
}
