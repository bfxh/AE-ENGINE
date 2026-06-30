#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdArch {
    None,
    Sse2,
    Avx,
    Avx2,
    Avx512,
    Neon,
    WasmSimd128,
}

pub struct SimdDetector;

impl SimdDetector {
    pub fn detect() -> SimdArch {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                return SimdArch::Avx512;
            }
            if is_x86_feature_detected!("avx2") {
                return SimdArch::Avx2;
            }
            if is_x86_feature_detected!("avx") {
                return SimdArch::Avx;
            }
            if is_x86_feature_detected!("sse2") {
                return SimdArch::Sse2;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return SimdArch::Neon;
            }
        }
        SimdArch::None
    }

    pub fn is_supported() -> bool {
        Self::detect() != SimdArch::None
    }

    pub fn simd_width() -> usize {
        match Self::detect() {
            SimdArch::Avx512 => 16,
            SimdArch::Avx | SimdArch::Avx2 => 8,
            SimdArch::Sse2 | SimdArch::Neon | SimdArch::WasmSimd128 => 4,
            SimdArch::None => 1,
        }
    }
}

pub struct SimdOps;

impl SimdOps {
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Performs dot product using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
        let len = a.len().min(b.len());
        let mut sum = _mm256_setzero_ps();
        let chunks = len / 8;
        for i in 0..chunks {
            let va = _mm256_loadu_ps(a.as_ptr().add(i * 8));
            let vb = _mm256_loadu_ps(b.as_ptr().add(i * 8));
            sum = _mm256_fmadd_ps(va, vb, sum);
        }
        let mut result = [0.0f32; 8];
        _mm256_storeu_ps(result.as_mut_ptr(), sum);
        let mut total = result.iter().sum::<f32>();
        let rem = len % 8;
        for i in (len - rem)..len {
            total += a[i] * b[i];
        }
        total
    }

    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    return Self::dot_product_avx2(a, b);
                }
            }
        }
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Performs vector addition using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn vector_add_avx2(a: &[f32], b: &[f32], out: &mut [f32]) {
        let len = a.len().min(b.len()).min(out.len());
        let chunks = len / 8;
        for i in 0..chunks {
            let va = _mm256_loadu_ps(a.as_ptr().add(i * 8));
            let vb = _mm256_loadu_ps(b.as_ptr().add(i * 8));
            let vc = _mm256_add_ps(va, vb);
            _mm256_storeu_ps(out.as_mut_ptr().add(i * 8), vc);
        }
        let rem = len % 8;
        for i in (len - rem)..len {
            out[i] = a[i] + b[i];
        }
    }

    pub fn vector_add(a: &[f32], b: &[f32], out: &mut [f32]) {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    Self::vector_add_avx2(a, b, out);
                    return;
                }
            }
        }
        for i in 0..a.len().min(b.len()).min(out.len()) {
            out[i] = a[i] + b[i];
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Performs 4x4 matrix multiplication using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn matmul_4x4_avx2(a: &[f32; 16], b: &[f32; 16], out: &mut [f32; 16]) {
        for row in 0..4 {
            let va = _mm256_loadu_ps(a.as_ptr().add(row * 4));
            let mut sum = _mm256_setzero_ps();
            for col in 0..4 {
                let _vb = _mm256_set1_ps(b[col]);
                let vb_full = _mm256_set_ps(
                    b[12 + col],
                    b[8 + col],
                    b[4 + col],
                    b[col],
                    b[12 + col],
                    b[8 + col],
                    b[4 + col],
                    b[col],
                );
                sum = _mm256_fmadd_ps(va, vb_full, sum);
            }
            let mut result = [0.0f32; 8];
            _mm256_storeu_ps(result.as_mut_ptr(), sum);
            for col in 0..4 {
                out[row * 4 + col] = result[col];
            }
        }
    }

    pub fn matmul_4x4(a: &[f32; 16], b: &[f32; 16], out: &mut [f32; 16]) {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    Self::matmul_4x4_avx2(a, b, out);
                    return;
                }
            }
        }
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[row * 4 + k] * b[k * 4 + col];
                }
                out[row * 4 + col] = sum;
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Computes softmax using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn softmax_avx2(input: &[f32], output: &mut [f32]) {
        let len = input.len();
        let max_val = input.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let mut sum = _mm256_setzero_ps();
        let mut remaining = len;
        let mut offset = 0;

        while remaining >= 8 {
            let v = _mm256_loadu_ps(input.as_ptr().add(offset));
            let v_max = _mm256_set1_ps(max_val);
            let v_exp = Self::avx2_exp_approx(_mm256_sub_ps(v, v_max));
            sum = _mm256_add_ps(sum, v_exp);
            _mm256_storeu_ps(output.as_mut_ptr().add(offset), v_exp);
            offset += 8;
            remaining -= 8;
        }

        let mut result = [0.0f32; 8];
        _mm256_storeu_ps(result.as_mut_ptr(), sum);
        let total = result.iter().sum::<f32>();

        let inv_total = 1.0 / total;
        for item in output.iter_mut().take(len) {
            *item *= inv_total;
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn avx2_exp_approx(x: __m256) -> __m256 {
        let _one = _mm256_set1_ps(1.0f32);
        let ln2 = _mm256_set1_ps(std::f32::consts::LN_2);
        let inv_ln2 = _mm256_set1_ps(std::f32::consts::LOG2_E);
        let c0 = _mm256_set1_ps(1.0f32);
        let c1 = _mm256_set1_ps(1.0f32);
        let c2 = _mm256_set1_ps(0.5f32);
        let c3 = _mm256_set1_ps(1.0 / 6.0);
        let c4 = _mm256_set1_ps(1.0 / 24.0);

        let x_clamped =
            _mm256_min_ps(_mm256_max_ps(x, _mm256_set1_ps(-87.0)), _mm256_set1_ps(87.0));
        let n = _mm256_cvtps_epi32(_mm256_round_ps(
            _mm256_mul_ps(x_clamped, inv_ln2),
            _MM_FROUND_TO_NEAREST_INT,
        ));
        let n_f = _mm256_cvtepi32_ps(n);
        let r = _mm256_sub_ps(x_clamped, _mm256_mul_ps(n_f, ln2));

        let mut result = c4;
        result = _mm256_fmadd_ps(result, r, c3);
        result = _mm256_fmadd_ps(result, r, c2);
        result = _mm256_fmadd_ps(result, r, c1);
        result = _mm256_fmadd_ps(result, r, c0);

        let pow2 =
            _mm256_castsi256_ps(_mm256_slli_epi32(_mm256_add_epi32(n, _mm256_set1_epi32(127)), 23));
        _mm256_mul_ps(result, pow2)
    }

    pub fn softmax(input: &[f32], output: &mut [f32]) {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    Self::softmax_avx2(input, output);
                    return;
                }
            }
        }
        let max_val = input.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let mut sum = 0.0f32;
        for (i, &val) in input.iter().enumerate() {
            let exp = (val - max_val).exp();
            output[i] = exp;
            sum += exp;
        }
        for val in output.iter_mut() {
            *val /= sum;
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Computes GELU activation using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn gelu_avx2(input: &[f32], output: &mut [f32]) {
        let len = input.len().min(output.len());
        let chunks = len / 8;
        let half = _mm256_set1_ps(0.5f32);
        let _one = _mm256_set1_ps(1.0f32);
        let sqrt_2_over_pi = _mm256_set1_ps(std::f32::consts::FRAC_2_SQRT_PI);
        let coeff = _mm256_set1_ps(0.044715f32);

        for i in 0..chunks {
            let x = _mm256_loadu_ps(input.as_ptr().add(i * 8));
            let x3 = _mm256_mul_ps(_mm256_mul_ps(x, x), x);
            let tanh_in = _mm256_mul_ps(sqrt_2_over_pi, _mm256_fmadd_ps(coeff, x3, x));
            let tanh_val = Self::avx2_tanh_approx(tanh_in);
            let add_one = _mm256_add_ps(_one, tanh_val);
            let result = _mm256_mul_ps(_mm256_mul_ps(half, x), add_one);
            _mm256_storeu_ps(output.as_mut_ptr().add(i * 8), result);
        }

        let rem_start = chunks * 8;
        for i in rem_start..len {
            let x = input[i];
            output[i] = 0.5 * x * (1.0 + (std::f32::consts::FRAC_2_SQRT_PI * (x + 0.044715 * x * x * x)).tanh());
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn avx2_tanh_approx(x: __m256) -> __m256 {
        let x_clamped =
            _mm256_min_ps(_mm256_max_ps(x, _mm256_set1_ps(-10.0)), _mm256_set1_ps(10.0));
        let x2 = _mm256_mul_ps(x_clamped, x_clamped);
        let p0 = _mm256_set1_ps(1.0f32);
        let p1 = _mm256_set1_ps(1.0 / 3.0);
        let p2 = _mm256_set1_ps(2.0 / 15.0);
        let p3 = _mm256_set1_ps(17.0 / 315.0);
        let num = _mm256_fmadd_ps(p3, x2, _mm256_fmadd_ps(p2, x2, _mm256_fmadd_ps(p1, x2, p0)));
        _mm256_div_ps(x_clamped, num)
    }

    pub fn gelu(input: &[f32], output: &mut [f32]) {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    Self::gelu_avx2(input, output);
                    return;
                }
            }
        }
        for (i, &x) in input.iter().enumerate() {
            output[i] = 0.5 * x * (1.0 + (std::f32::consts::FRAC_2_SQRT_PI * (x + 0.044715 * x * x * x)).tanh());
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    /// Performs layer normalization using AVX2 instructions
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the target CPU
    pub unsafe fn layer_norm_avx2(
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        eps: f32,
        output: &mut [f32],
    ) {
        let len = input.len().min(output.len());
        let mean = input.iter().sum::<f32>() / len as f32;
        let var = input.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / len as f32;
        let inv_std = 1.0 / (var + eps).sqrt();
        let v_mean = _mm256_set1_ps(mean);
        let v_inv_std = _mm256_set1_ps(inv_std);

        let chunks = len / 8;
        for i in 0..chunks {
            let v = _mm256_loadu_ps(input.as_ptr().add(i * 8));
            let v_norm = _mm256_mul_ps(_mm256_sub_ps(v, v_mean), v_inv_std);
            let v_gamma = _mm256_loadu_ps(gamma.as_ptr().add(i * 8));
            let v_beta = _mm256_loadu_ps(beta.as_ptr().add(i * 8));
            let v_out = _mm256_fmadd_ps(v_norm, v_gamma, v_beta);
            _mm256_storeu_ps(output.as_mut_ptr().add(i * 8), v_out);
        }

        let rem_start = chunks * 8;
        for i in rem_start..len {
            let norm = (input[i] - mean) * inv_std;
            let gi = gamma.get(i).copied().unwrap_or(1.0);
            let bi = beta.get(i).copied().unwrap_or(0.0);
            output[i] = norm * gi + bi;
        }
    }

    pub fn layer_norm(input: &[f32], gamma: &[f32], beta: &[f32], eps: f32, output: &mut [f32]) {
        #[cfg(target_arch = "x86_64")]
        {
            if SimdDetector::detect() >= SimdArch::Avx2 {
                unsafe {
                    Self::layer_norm_avx2(input, gamma, beta, eps, output);
                    return;
                }
            }
        }
        let len = input.len();
        let mean = input.iter().sum::<f32>() / len as f32;
        let var = input.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / len as f32;
        let inv_std = 1.0 / (var + eps).sqrt();
        for i in 0..len {
            let norm = (input[i] - mean) * inv_std;
            let gi = gamma.get(i).copied().unwrap_or(1.0);
            let bi = beta.get(i).copied().unwrap_or(0.0);
            output[i] = norm * gi + bi;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_detection() {
        let arch = SimdDetector::detect();
        assert!(SimdDetector::is_supported());
        assert!(SimdDetector::simd_width() >= 1);
        let _ = arch;
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let b = vec![1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let result = SimdOps::dot_product(&a, &b);
        let expected: f32 = a.iter().sum();
        assert!((result - expected).abs() < 0.01);
    }

    #[test]
    fn test_vector_add() {
        let a = vec![1.0f32; 16];
        let b = vec![2.0f32; 16];
        let mut out = vec![0.0f32; 16];
        SimdOps::vector_add(&a, &b, &mut out);
        for &val in &out {
            assert!((val - 3.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_softmax() {
        let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut output = vec![0.0f32; 8];
        SimdOps::softmax(&input, &mut output);
        let sum: f32 = output.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        assert!(output[7] > output[0]);
    }

    #[test]
    fn test_gelu() {
        let input = vec![-2.0f32, -1.0, 0.0, 1.0, 2.0, 0.5, -0.5, 3.0];
        let mut output = vec![0.0f32; 8];
        SimdOps::gelu(&input, &mut output);
        assert!(output[2] < 0.1);
        assert!(output[4] > 1.0);
    }

    #[test]
    fn test_layer_norm() {
        let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let gamma = vec![1.0f32; 8];
        let beta = vec![0.0f32; 8];
        let mut output = vec![0.0f32; 8];
        SimdOps::layer_norm(&input, &gamma, &beta, 1e-5, &mut output);
        let mean: f32 = output.iter().sum::<f32>() / 8.0;
        assert!(mean.abs() < 0.1);
    }

    #[test]
    fn test_matmul_4x4() {
        let a: [f32; 16] =
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let b: [f32; 16] =
            [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0];
        let mut out = [0.0f32; 16];
        SimdOps::matmul_4x4(&a, &b, &mut out);
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 2.0 } else { 0.0 };
                assert!((out[i * 4 + j] - expected).abs() < 0.01);
            }
        }
    }
}
