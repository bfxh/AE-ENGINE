// v8.0 Constitutive Models for MPM
// ————————————————————————————————
// All stress computations unified in one module.
// Each model produces Cauchy stress σ from deformation gradient F.
//
// References:
//   - Stomakhin et al. 2013 (Snow MPM — Drucker-Prager)
//   - Jiang et al. 2017 (MLS-MPM — Neo-Hookean)
//   - Hu et al. 2018 (MPM fluid — Newtonian)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstitutiveModel {
    /// Neo-Hookean elastic: rubber, soft tissue, gels
    NeoHookean,
    /// Drucker-Prager elastoplastic: sand, soil, snow, granular
    DruckerPrager,
    /// von Mises elastoplastic: metals (iron, steel, copper)
    VonMises,
    /// Newtonian fluid: water, oil, air (via J-dependent pressure)
    NewtonianFluid,
    /// Corotated linear elastic: stiff solids (concrete, stone)
    Corotated,
    /// Fixed corotated (StVK-based): wood, bone
    FixedCorotated,
    /// No deformation: rigid body proxy
    Rigid,
}

/// Material parameters for constitutive models
#[derive(Debug, Clone, Copy)]
pub struct MaterialParams {
    pub young_modulus: f32,  // E (Pa)
    pub poisson_ratio: f32,  // ν
    pub yield_stress: f32,   // σ_y (Pa) — plastic yield
    pub hardening: f32,      // isotropic hardening coefficient
    pub density: f32,        // ρ (kg/m³)
    pub friction_angle: f32, // φ (degrees) — for Drucker-Prager
    pub cohesion: f32,       // c (Pa) — for Drucker-Prager
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            young_modulus: 1e6,
            poisson_ratio: 0.3,
            yield_stress: 1e4,
            hardening: 0.01,
            density: 1000.0,
            friction_angle: 35.0,
            cohesion: 0.0,
        }
    }
}

impl MaterialParams {
    /// Clamp poisson_ratio to physically valid range (-0.999, 0.4999).
    /// At ν=0.5 (incompressible), bulk_modulus and lambda divide by zero.
    /// At ν=-1, mu and lambda divide by zero. We clamp to avoid NaN/Inf
    /// when users set invalid values (e.g., WATER uses 0.499 which is safe,
    /// but a user-set 0.5 would panic in debug / produce Inf in release).
    fn clamped_nu(&self) -> f32 {
        self.poisson_ratio.clamp(-0.999, 0.4999)
    }

    /// Lame first parameter λ
    pub fn lambda(&self) -> f32 {
        let nu = self.clamped_nu();
        self.young_modulus * nu / ((1.0 + nu) * (1.0 - 2.0 * nu))
    }

    /// Shear modulus μ
    pub fn mu(&self) -> f32 {
        let nu = self.clamped_nu();
        self.young_modulus / (2.0 * (1.0 + nu))
    }

    /// Bulk modulus K
    pub fn bulk_modulus(&self) -> f32 {
        let nu = self.clamped_nu();
        self.young_modulus / (3.0 * (1.0 - 2.0 * nu))
    }
}

/// Pre-defined material presets (SI units)
pub mod presets {
    use super::MaterialParams;

    pub const STEEL: MaterialParams = MaterialParams {
        young_modulus: 2.0e11,
        poisson_ratio: 0.3,
        yield_stress: 2.5e8,
        hardening: 0.01,
        density: 7850.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const IRON: MaterialParams = MaterialParams {
        young_modulus: 2.11e11,
        poisson_ratio: 0.29,
        yield_stress: 1.3e8,
        hardening: 0.02,
        density: 7874.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const CONCRETE: MaterialParams = MaterialParams {
        young_modulus: 3.0e10,
        poisson_ratio: 0.2,
        yield_stress: 3.0e7,
        hardening: 0.005,
        density: 2400.0,
        friction_angle: 30.0,
        cohesion: 2.0e6,
    };

    pub const WOOD: MaterialParams = MaterialParams {
        young_modulus: 1.0e10,
        poisson_ratio: 0.3,
        yield_stress: 5.0e7,
        hardening: 0.01,
        density: 600.0,
        friction_angle: 25.0,
        cohesion: 5.0e6,
    };

    pub const SAND: MaterialParams = MaterialParams {
        young_modulus: 5.0e7,
        poisson_ratio: 0.2,
        yield_stress: 1.0e4,
        hardening: 0.005,
        density: 1600.0,
        friction_angle: 35.0,
        cohesion: 0.0,
    };

    pub const WATER: MaterialParams = MaterialParams {
        young_modulus: 2.2e9,
        poisson_ratio: 0.499,
        yield_stress: 0.0,
        hardening: 0.0,
        density: 1000.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const RUBBER: MaterialParams = MaterialParams {
        young_modulus: 1.0e6,
        poisson_ratio: 0.49,
        yield_stress: 1.0e6,
        hardening: 0.1,
        density: 1100.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const GLASS: MaterialParams = MaterialParams {
        young_modulus: 7.0e10,
        poisson_ratio: 0.22,
        yield_stress: 2.5e7,
        hardening: 0.0,
        density: 2500.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const BONE: MaterialParams = MaterialParams {
        young_modulus: 1.8e10,
        poisson_ratio: 0.3,
        yield_stress: 1.2e8,
        hardening: 0.02,
        density: 1900.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const MUSCLE: MaterialParams = MaterialParams {
        young_modulus: 3.0e5,
        poisson_ratio: 0.49,
        yield_stress: 5.0e4,
        hardening: 0.1,
        density: 1060.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const SKIN: MaterialParams = MaterialParams {
        young_modulus: 2.0e6,
        poisson_ratio: 0.45,
        yield_stress: 3.0e5,
        hardening: 0.1,
        density: 1100.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };

    pub const SOIL: MaterialParams = MaterialParams {
        young_modulus: 1.0e7,
        poisson_ratio: 0.25,
        yield_stress: 2.0e4,
        hardening: 0.005,
        density: 1800.0,
        friction_angle: 30.0,
        cohesion: 5.0e3,
    };

    pub const ICE: MaterialParams = MaterialParams {
        young_modulus: 9.0e9,
        poisson_ratio: 0.33,
        yield_stress: 1.0e6,
        hardening: 0.0,
        density: 917.0,
        friction_angle: 20.0,
        cohesion: 1.0e5,
    };

    pub const PLASTIC: MaterialParams = MaterialParams {
        young_modulus: 2.5e9,
        poisson_ratio: 0.38,
        yield_stress: 4.0e7,
        hardening: 0.05,
        density: 1200.0,
        friction_angle: 0.0,
        cohesion: 0.0,
    };
}

/// Compute Cauchy stress σ from deformation gradient F.
/// Returns (stress_3x3_row_major, updated_J, updated_F, plasticity_happened).
///
/// `F` is 9 elements row-major: [F00, F01, F02, F10, F11, F12, F20, F21, F22]
/// Returns stress_3x3 same layout.
pub fn compute_stress(
    model: ConstitutiveModel,
    params: &MaterialParams,
    F: &[f32; 9],
    J: f32, // current volume ratio (det(F))
) -> ([f32; 9], f32, [f32; 9], bool) {
    match model {
        ConstitutiveModel::Rigid => rigid_stress(F, params),
        ConstitutiveModel::NeoHookean => neo_hookean(F, J, params),
        ConstitutiveModel::Corotated => corotated_elastic(F, J, params),
        ConstitutiveModel::FixedCorotated => fixed_corotated(F, J, params),
        ConstitutiveModel::DruckerPrager => drucker_prager(F, J, params),
        ConstitutiveModel::VonMises => von_mises(F, J, params),
        ConstitutiveModel::NewtonianFluid => newtonian_fluid(F, J, params),
    }
}

// ——————————————————————————————————————————————————————————————

/// 3x3 identity
fn identity3() -> [f32; 9] {
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
}

/// Trace of 3x3 matrix
fn trace(m: &[f32; 9]) -> f32 {
    m[0] + m[4] + m[8]
}

/// Multiply 3x3 * 3x3 (row-major)
fn mat_mul(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}

/// Transpose 3x3
fn transpose(m: &[f32; 9]) -> [f32; 9] {
    [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
}

/// SVD-based polar decomposition: F = R * S
fn polar_decomp(F: &[f32; 9]) -> ([f32; 9], [f32; 9]) {
    // Newton iteration: R_{n+1} = 0.5 * (R_n + R_n^{-T})
    // Converges to polar rotation R; then S = R^T * F.
    let mut r = *F;
    for _ in 0..8 {
        let r_inv_t = inv_transpose(&r);
        let mut new_r = [0.0f32; 9];
        let mut diff = 0.0f32;
        for i in 0..9 {
            new_r[i] = 0.5 * (r[i] + r_inv_t[i]);
            diff += (new_r[i] - r[i]).abs();
        }
        r = new_r;
        if diff < 1e-10 {
            break;
        }
    }
    let rt = transpose(&r);
    let s = mat_mul(&rt, F);
    (r, s)
}

fn inv_transpose(m: &[f32; 9]) -> [f32; 9] {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);
    if det.abs() < 1e-15 {
        return identity3();
    }
    let inv_det = 1.0 / det;
    [
        (m[4] * m[8] - m[5] * m[7]) * inv_det,
        -(m[1] * m[8] - m[2] * m[7]) * inv_det,
        (m[1] * m[5] - m[2] * m[4]) * inv_det,
        -(m[3] * m[8] - m[5] * m[6]) * inv_det,
        (m[0] * m[8] - m[2] * m[6]) * inv_det,
        -(m[0] * m[5] - m[2] * m[3]) * inv_det,
        (m[3] * m[7] - m[4] * m[6]) * inv_det,
        -(m[0] * m[7] - m[1] * m[6]) * inv_det,
        (m[0] * m[4] - m[1] * m[3]) * inv_det,
    ]
}

/// Fast symmetric 3x3 eigenvalue via Jacobi (sufficient for MPM)
#[allow(dead_code)]
fn eig_sym_3x3(m: &[f32; 9]) -> [f32; 3] {
    // Eigenvalues of symmetric 3x3 via Cardano formula on the characteristic polynomial.
    // poly: λ³ - I1*λ² + I2*λ - I3 = 0  where I1=trace, I2=sum of principal minors, I3=det
    let a = m[0];
    let b = m[1];
    let c = m[2];
    let d = m[4];
    let e = m[5];
    let f = m[8];

    let i1 = a + d + f;
    let i2 = a * d - b * b + a * f - c * c + d * f - e * e;
    let i3 = a * d * f + 2.0 * b * c * e - a * e * e - d * c * c - f * b * b;

    // Depressed cubic: x³ + p*x + q = 0  where λ = x + I1/3
    let p = i2 - i1 * i1 / 3.0;
    let q = -2.0 * i1 * i1 * i1 / 27.0 + i1 * i2 / 3.0 - i3;
    let shift = i1 / 3.0;

    if p.abs() < 1e-12 {
        // p ≈ 0: x³ + q = 0 → x = -cbrt(q)
        let x = -q.cbrt();
        return [x + shift, x + shift, x + shift];
    }

    let sqrt_neg_p_3 = ((-p) / 3.0).max(0.0).sqrt();
    let arg = if sqrt_neg_p_3 < 1e-12 {
        0.0
    } else {
        ((3.0 * q) / (2.0 * p) * 3.0_f32.sqrt() * sqrt_neg_p_3).clamp(-1.0, 1.0)
    };
    let phi = arg.acos() / 3.0;

    let x1 = 2.0 * sqrt_neg_p_3 * phi.cos();
    let x2 = 2.0 * sqrt_neg_p_3 * (phi + 2.0 * std::f32::consts::PI / 3.0).cos();
    let x3 = 2.0 * sqrt_neg_p_3 * (phi + 4.0 * std::f32::consts::PI / 3.0).cos();

    [x1 + shift, x2 + shift, x3 + shift]
}

// ——————————————————— Constitutive Models ——————————————————————

fn rigid_stress(F: &[f32; 9], _params: &MaterialParams) -> ([f32; 9], f32, [f32; 9], bool) {
    // Rigid: no deformation, return identity
    (identity3(), 1.0, *F, false)
}

fn neo_hookean(F: &[f32; 9], J: f32, params: &MaterialParams) -> ([f32; 9], f32, [f32; 9], bool) {
    let mu = params.mu();
    let lambda = params.lambda();

    let ft = transpose(F);
    let b = mat_mul(F, &ft); // left Cauchy-Green = F*F^T
    let _trace_b = trace(&b);

    // σ = (μ/J)*(b - I) + (λ/J)*ln(J)*I
    let p_term = if J > 0.0 { lambda * J.ln() / J } else { 0.0 };
    let scale = mu / J.max(1e-10);

    let mut stress = identity3();
    for i in 0..9 {
        let dev = if i % 4 == 0 { b[i] - 1.0 + p_term / scale } else { b[i] };
        stress[i] = scale * dev;
    }
    (stress, J, *F, false)
}

fn corotated_elastic(
    F: &[f32; 9],
    J: f32,
    params: &MaterialParams,
) -> ([f32; 9], f32, [f32; 9], bool) {
    let mu = params.mu();
    let lambda = params.lambda();

    let (r, _s) = polar_decomp(F);

    // σ = 2μ*(F-R) + λ*trace(R^T*F - I)*I
    // Simplified: σ = 2μ*(F-R)^T + λ*(J-1)*I  (small strain approximation)
    let mut stress = [0.0f32; 9];
    for i in 0..9 {
        stress[i] = 2.0 * mu * (F[i] - r[i]);
    }
    for i in [0, 4, 8] {
        stress[i] += lambda * (J - 1.0);
    }
    (stress, J, *F, false)
}

fn fixed_corotated(
    F: &[f32; 9],
    J: f32,
    params: &MaterialParams,
) -> ([f32; 9], f32, [f32; 9], bool) {
    let mu = params.mu();
    let lambda = params.lambda();

    let (r, _s) = polar_decomp(F);
    // σ = 2μ*(F-R)*R^T + λ*(J-1)*J*I  (StVK-based)
    let rt = transpose(&r);
    let mut diff = identity3();
    for i in 0..9 {
        diff[i] = F[i] - r[i];
    }
    let diff_rt = mat_mul(&diff, &rt);
    let mut stress = [0.0f32; 9];
    for i in 0..9 {
        stress[i] = 2.0 * mu * diff_rt[i];
    }
    for i in [0, 4, 8] {
        stress[i] += lambda * (J - 1.0) * J;
    }
    (stress, J, *F, false)
}

fn drucker_prager(
    F: &[f32; 9],
    J: f32,
    params: &MaterialParams,
) -> ([f32; 9], f32, [f32; 9], bool) {
    // Elastic trial: compute stress from corotated model
    let (r, s) = polar_decomp(F);
    let mu = params.mu();
    let lambda = params.lambda();

    // Trial strain: ε = log(S) ≈ S - I
    let epsilon_e = [s[0] - 1.0, s[1], s[2], s[3], s[4] - 1.0, s[5], s[6], s[7], s[8] - 1.0];
    let trace_e = epsilon_e[0] + epsilon_e[4] + epsilon_e[8];

    // Deviatoric strain
    let tr3 = trace_e / 3.0;
    let mut eps_dev = epsilon_e;
    eps_dev[0] -= tr3;
    eps_dev[4] -= tr3;
    eps_dev[8] -= tr3;

    // Trial stress (deviatoric)
    let mut s_dev = [0.0f32; 9];
    for i in 0..9 {
        s_dev[i] = 2.0 * mu * eps_dev[i];
    }

    // von Mises equivalent stress
    let s_norm = (0.5
        * (s_dev[0] * s_dev[0]
            + s_dev[4] * s_dev[4]
            + s_dev[8] * s_dev[8]
            + 2.0
                * (s_dev[1] * s_dev[1]
                    + s_dev[2] * s_dev[2]
                    + s_dev[3] * s_dev[3]
                    + s_dev[5] * s_dev[5]
                    + s_dev[6] * s_dev[6]
                    + s_dev[7] * s_dev[7])))
        .sqrt();

    let pressure = -lambda * trace_e; // p = -K * tr(ε)
    let phi = params.friction_angle.to_radians();
    let cohesion = params.cohesion;

    // Drucker-Prager yield: sqrt(J2) + α*p ≤ k
    let alpha = (2.0 * phi.sin()) / (3.0_f32.sqrt() * (3.0 - phi.sin()));
    let k = (6.0 * cohesion * phi.cos()) / (3.0_f32.sqrt() * (3.0 - phi.sin()));

    let yield_val = s_norm + alpha * pressure - k;

    if yield_val > 0.0 {
        // Plastic correction: scale deviatoric stress
        let scale = (k - alpha * pressure).max(0.0) / s_norm.max(1e-10);
        // Project S back: new_S = I + scale*(S-I) + (1-scale)*tr3*I
        let mut new_s = [0.0f32; 9];
        for i in 0..9 {
            let identity = if i % 4 == 0 { 1.0 } else { 0.0 };
            new_s[i] = identity + scale * (s[i] - identity);
        }
        for i in [0, 4, 8] {
            new_s[i] += (1.0 - scale) * tr3;
        }
        let new_F = mat_mul(&r, &new_s);

        for item in s_dev.iter_mut() {
            *item *= scale;
        }

        // Build corrected stress
        let mut stress = [0.0f32; 9];
        stress.copy_from_slice(&s_dev);
        for i in [0, 4, 8] {
            stress[i] -= pressure;
        }

        return (stress, J, new_F, true);
    }

    // Elastic: build full stress tensor
    let mut stress = [0.0f32; 9];
    stress.copy_from_slice(&s_dev);
    for i in [0, 4, 8] {
        stress[i] -= pressure;
    }

    (stress, J, *F, false)
}

fn von_mises(F: &[f32; 9], J: f32, params: &MaterialParams) -> ([f32; 9], f32, [f32; 9], bool) {
    let (r, s) = polar_decomp(F);
    let mu = params.mu();
    let lambda = params.lambda();
    let yield_stress = params.yield_stress;

    // Elastic trial
    let epsilon_e = [s[0] - 1.0, s[1], s[2], s[3], s[4] - 1.0, s[5], s[6], s[7], s[8] - 1.0];
    let trace_e = epsilon_e[0] + epsilon_e[4] + epsilon_e[8];
    let tr3 = trace_e / 3.0;

    let mut eps_dev = epsilon_e;
    eps_dev[0] -= tr3;
    eps_dev[4] -= tr3;
    eps_dev[8] -= tr3;

    let mut s_dev = [0.0f32; 9];
    for i in 0..9 {
        s_dev[i] = 2.0 * mu * eps_dev[i];
    }

    let s_norm = (0.5
        * (s_dev[0] * s_dev[0]
            + s_dev[4] * s_dev[4]
            + s_dev[8] * s_dev[8]
            + 2.0
                * (s_dev[1] * s_dev[1]
                    + s_dev[2] * s_dev[2]
                    + s_dev[3] * s_dev[3]
                    + s_dev[5] * s_dev[5]
                    + s_dev[6] * s_dev[6]
                    + s_dev[7] * s_dev[7])))
        .sqrt();

    let yield_threshold = yield_stress / (3.0_f32.sqrt()); // σ_y/√3 for von Mises

    if s_norm > yield_threshold {
        let scale = yield_threshold / s_norm;
        // Project S back to yield surface: new_S = I + scale*(S-I) + (1-scale)*tr3*I
        // This prevents F from growing unboundedly during plastic flow.
        let mut new_s = [0.0f32; 9];
        for i in 0..9 {
            let identity = if i % 4 == 0 { 1.0 } else { 0.0 };
            new_s[i] = identity + scale * (s[i] - identity);
        }
        for i in [0, 4, 8] {
            new_s[i] += (1.0 - scale) * tr3;
        }
        let new_F = mat_mul(&r, &new_s);
        // Scale deviatoric stress to yield surface
        for item in s_dev.iter_mut() {
            *item *= scale;
        }
        let pressure = -lambda * trace_e;
        let mut stress = [0.0f32; 9];
        stress.copy_from_slice(&s_dev);
        for i in [0, 4, 8] {
            stress[i] -= pressure;
        }
        return (stress, J, new_F, true);
    }

    let pressure = -lambda * trace_e;
    let mut stress = [0.0f32; 9];
    stress.copy_from_slice(&s_dev);
    for i in [0, 4, 8] {
        stress[i] -= pressure;
    }
    (stress, J, *F, false)
}

fn newtonian_fluid(
    F: &[f32; 9],
    J: f32,
    params: &MaterialParams,
) -> ([f32; 9], f32, [f32; 9], bool) {
    let k = params.bulk_modulus();
    // Cauchy stress s = -p*I where p = K*(1-J) is physical pressure (positive compression).
    // Equivalently s_diag = K*(J-1) (negative when compressed).
    let sigma_diag = k * (J - 1.0);
    let mut stress = [0.0f32; 9];
    stress[0] = sigma_diag;
    stress[4] = sigma_diag;
    stress[8] = sigma_diag;
    (stress, J, *F, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neo_hookean_identity() {
        let _F = identity3();
        let (stress, _, _, _) = neo_hookean(&_F, 1.0, &MaterialParams::default());
        // At identity, stress should be zero
        for (i, stress_i) in stress.iter().enumerate() {
            assert!((stress_i).abs() < 1e-4, "stress[{}] = {}", i, stress_i);
        }
    }

    #[test]
    fn test_newtonian_at_rest() {
        let F = identity3();
        let (stress, _, _, _) = newtonian_fluid(&F, 1.0, &MaterialParams::default());
        assert!((stress[0]).abs() < 1e-4);
        assert!((stress[4]).abs() < 1e-4);
        assert!((stress[8]).abs() < 1e-4);
    }

    #[test]
    fn test_newtonian_compression() {
        let F = identity3();
        let (stress, _, _, _) = newtonian_fluid(&F, 0.9, &MaterialParams::default());
        // Compressed: Cauchy stress negative (compression)
        assert!(stress[0] < 0.0);
    }

    #[test]
    fn test_sand_yields_under_stress() {
        let _F = identity3();
        // Apply large shear: off-diagonal component
        let mut shear_F = identity3();
        shear_F[1] = 0.5; // ε_xy = 0.5
        let (_, _, _, plastic) = drucker_prager(&shear_F, 1.0, &presets::SAND);
        assert!(plastic);
    }

    #[test]
    fn test_steel_elastic_small_strain() {
        let F = identity3();
        let (_, _, _, plastic) = von_mises(&F, 1.0, &presets::STEEL);
        assert!(!plastic);
    }
}
