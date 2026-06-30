//! Nonlinear Schroedinger Equation Solver (NLS)
//!
//! i * d psi / dt + (1/2) * d^2 psi / dx^2 + |psi|^2 * psi = 0
//!
//! The canonical integrable nonlinear wave equation (Zakharov-Shabat 1972).
//! Describes envelope dynamics of weakly nonlinear dispersive waves.
//!
//! Focus case (kappa > 0, sign convention here): bright solitons
//!   psi(x,t) = A * sech(A*(x - v*t - x0)) * exp(i*(v*x - (v^2/2 - A^2/2)*t))
//!   amplitude A, velocity v, phase velocity (v^2 - A^2)/2
//!
//! Conservation laws (infinite hierarchy, first 3):
//!   N  = integral |psi|^2 dx        (particle number / power / mass)
//!   P  = integral Im(psi* psi_x) dx (momentum)
//!   H  = integral [ 1/2 |psi_x|^2 - 1/2 |psi|^4 ] dx  (Hamiltonian)
//!
//! Split-step Fourier method (Strang splitting, unconditionally stable,
//! spectrally accurate in space, second-order in time):
//!   The NLS Hamiltonian splits as H = H_lin + H_nl where
//!     H_lin  = -(1/2) d^2/dx^2     (linear dispersion, diagonal in k-space)
//!     H_nl   = |psi|^2             (nonlinear phase, local in x-space)
//!   One step:
//!     1. half linear:    psi <- exp(i*dt/4 * k^2) * psi_k      (FFT)
//!     2. full nonlinear: psi <- exp(i*dt * |psi|^2) * psi      (pointwise)
//!     3. half linear:    psi <- exp(i*dt/4 * k^2) * psi_k      (FFT)
//!   The linear step applies the exact propagator in Fourier space:
//!     psi_k(t+dt) = exp(-i * (-k^2/2) * dt) * psi_k(t)
//!                 = exp(i * k^2 * dt / 2) * psi_k(t)
//!
//! Applications:
//!   - Fiber optic soliton transmission (Hasegawa-Tappert 1973)
//!   - Bose-Einstein condensation (Gross-Pitaevskii eqn)
//!   - Plasma Langmuir wave envelopes
//!   - Deep water wave packets (Zakharov 1968)
//!   - Self-focusing / wave collapse in nonlinear optics
//!
//! Based on Zakharov & Shabat 1972 (inverse scattering soliton),
//! Hasegawa & Tappert 1973 (fiber soliton),
//! Feit, Fleck & Ward 1982 (split-step Fourier).

use serde::{Deserialize, Serialize};

/// NLS config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlsConfig {
    pub nx: usize,
    pub dx: f32,
    pub dt: f32,
    /// Nonlinearity sign: +1 focusing (bright solitons), -1 defocusing (dark solitons)
    pub kappa: f32,
}

impl Default for NlsConfig {
    fn default() -> Self {
        NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 }
    }
}

impl NlsConfig {
    pub fn length(&self) -> f32 {
        (self.nx as f32) * self.dx
    }
    pub fn n_cells(&self) -> usize {
        self.nx
    }
    /// Maximum resolved wavenumber (Nyquist): pi / dx
    pub fn k_nyquist(&self) -> f32 {
        std::f32::consts::PI / self.dx
    }
}

/// Minimal complex type (re, im) to avoid external crate dependency
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub fn new(re: f32, im: f32) -> Self { Complex { re, im } }
    pub fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    pub fn one() -> Self { Complex { re: 1.0, im: 0.0 } }
    pub fn scale(self, s: f32) -> Self { Complex { re: self.re * s, im: self.im * s } }
    pub fn norm2(self) -> f32 { self.re * self.re + self.im * self.im }
    pub fn norm(self) -> f32 { self.norm2().sqrt() }
    pub fn conj(self) -> Self { Complex { re: self.re, im: -self.im } }
    pub fn from_polar(r: f32, theta: f32) -> Self {
        Complex { re: r * theta.cos(), im: r * theta.sin() }
    }
}

impl std::ops::Add for Complex {
    type Output = Complex;
    fn add(self, o: Complex) -> Complex {
        Complex { re: self.re + o.re, im: self.im + o.im }
    }
}

impl std::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, o: Complex) -> Complex {
        Complex { re: self.re - o.re, im: self.im - o.im }
    }
}

impl std::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, o: Complex) -> Complex {
        Complex {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
}

pub struct NlsSolver {
    pub config: NlsConfig,
    /// Wavefunction psi(x) — complex valued
    pub psi: Vec<Complex>,
    /// Fourier workspace (reused each step)
    psi_k: Vec<Complex>,
    /// Wavenumber grid kx (shifted for FFT ordering)
    kx: Vec<f32>,
    /// Linear propagator exp(i * k^2 * dt / 4) precomputed for half step
    lin_half: Vec<Complex>,
    pub time: f32,
    pub steps: usize,
}

impl NlsSolver {
    pub fn new(config: NlsConfig) -> Self {
        let n = config.n_cells();
        assert!(n.is_power_of_two(), "split-step Fourier requires nx = power of 2");
        let dt = config.dt;
        let dx = config.dx;
        let nx = config.nx;

        // FFT-shifted wavenumber grid: k = 2*pi * [0, 1, ..., n/2-1, -n/2, ..., -1] / L
        let lx = (nx as f32) * dx;
        let two_pi_over_l = 2.0 * std::f32::consts::PI / lx;
        let mut kx = vec![0.0f32; nx];
        for i in 0..nx {
            let ki = if i < nx / 2 { i as i32 } else { i as i32 - nx as i32 };
            kx[i] = (ki as f32) * two_pi_over_l;
        }

        // Linear half-step propagator: exp(i * k^2 * dt / 4)
        let mut lin_half = vec![Complex::zero(); nx];
        for i in 0..nx {
            let phase = -0.25 * kx[i] * kx[i] * dt;
            lin_half[i] = Complex::from_polar(1.0, phase);
        }

        NlsSolver {
            config,
            psi: vec![Complex::zero(); n],
            psi_k: vec![Complex::zero(); n],
            kx,
            lin_half,
            time: 0.0,
            steps: 0,
        }
    }

    pub fn n_cells(&self) -> usize { self.config.n_cells() }

    /// Initialize a bright soliton: psi = A * sech(A*(x-x0)) * exp(i*v*x)
    /// (at t=0, no phase offset)
    pub fn initialize_bright_soliton(&mut self, amplitude: f32, center: f32, velocity: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        for i in 0..nx {
            let x = (i as f32) * dx;
            let s = amplitude * (x - center);
            let sech = 1.0 / s.cosh();
            let r = amplitude * sech;
            let phase = velocity * x;
            self.psi[i] = Complex::from_polar(r, phase);
        }
    }

    /// Initialize two bright solitons (superposition at t=0; not an exact two-soliton
    /// but a good approximation when they are well separated)
    pub fn initialize_two_solitons(&mut self, a1: f32, c1: f32, v1: f32, a2: f32, c2: f32, v2: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        for i in 0..nx {
            let x = (i as f32) * dx;
            let s1 = a1 * (x - c1);
            let r1 = a1 / s1.cosh();
            let s2 = a2 * (x - c2);
            let r2 = a2 / s2.cosh();
            let p1 = Complex::from_polar(r1, v1 * x);
            let p2 = Complex::from_polar(r2, v2 * x);
            self.psi[i] = p1 + p2;
        }
    }

    /// Initialize a Gaussian wave packet: psi = A * exp(-((x-x0)/w)^2) * exp(i*v*x)
    pub fn initialize_gaussian(&mut self, amplitude: f32, center: f32, width: f32, velocity: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let inv_w2 = 1.0 / (width * width);
        for i in 0..nx {
            let x = (i as f32) * dx;
            let dx_ = x - center;
            let r = amplitude * (-dx_ * dx_ * inv_w2).exp();
            self.psi[i] = Complex::from_polar(r, velocity * x);
        }
    }

    /// Radix-2 iterative FFT (Cooley-Tukey). inverse=true -> IFFT with 1/N normalization.
    fn fft_in_place(data: &mut [Complex], inverse: bool) {
        let n = data.len();
        assert!(n.is_power_of_two());
        if n == 1 { return; }
        let mut j = 0usize;
        for i in 1..n {
            let mut bit = n >> 1;
            while (j & bit) != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j { data.swap(i, j); }
        }
        let two_pi = 2.0 * std::f32::consts::PI;
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut len = 2;
        while len <= n {
            let ang = sign * two_pi / (len as f32);
            let wlen = Complex::from_polar(1.0, ang);
            for i in (0..n).step_by(len) {
                let mut w = Complex::one();
                for k in 0..(len / 2) {
                    let u = data[i + k];
                    let v = data[i + k + len / 2] * w;
                    data[i + k] = u + v;
                    data[i + k + len / 2] = u - v;
                    w = w * wlen;
                }
            }
            len <<= 1;
        }
        if inverse {
            let inv_n = 1.0 / (n as f32);
            for c in data.iter_mut() {
                *c = c.scale(inv_n);
            }
        }
    }

    /// Apply linear half-step: psi <- FFT^-1( exp(i*k^2*dt/4) * FFT(psi) )
    fn linear_half_step(&mut self) {
        let n = self.psi.len();
        // FFT forward
        self.psi_k[..n].copy_from_slice(&self.psi);
        Self::fft_in_place(&mut self.psi_k, false);
        // Apply propagator
        for i in 0..n {
            self.psi_k[i] = self.psi_k[i] * self.lin_half[i];
        }
        // IFFT back
        Self::fft_in_place(&mut self.psi_k, true);
        self.psi.copy_from_slice(&self.psi_k);
    }

    /// Apply nonlinear full step: psi <- exp(i * kappa * |psi|^2 * dt) * psi
    fn nonlinear_full_step(&mut self) {
        let kappa = self.config.kappa;
        let dt = self.config.dt;
        for i in 0..self.psi.len() {
            let rho2 = self.psi[i].norm2();
            let phase = kappa * rho2 * dt;
            let rot = Complex::from_polar(1.0, phase);
            self.psi[i] = self.psi[i] * rot;
        }
    }

    /// One Strang split-step (half-lin, full-nl, half-lin)
    pub fn step(&mut self) {
        self.linear_half_step();
        self.nonlinear_full_step();
        self.linear_half_step();
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n { self.step(); }
    }

    /// Particle number / power: N = integral |psi|^2 dx
    pub fn particle_number(&self) -> f32 {
        let dx = self.config.dx;
        self.psi.iter().map(|c| c.norm2()).sum::<f32>() * dx
    }

    /// Momentum: P = integral Im(psi* psi_x) dx
    /// Discretized with central differences (periodic).
    pub fn momentum(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let inv_2dx = 1.0 / (2.0 * dx);
        let mut p = 0.0f32;
        for i in 0..nx {
            let ip = (i + 1) % nx;
            let im = (i + nx - 1) % nx;
            // d psi / dx ~ (psi[ip] - psi[im]) / (2 dx)
            let dpsi_re = (self.psi[ip].re - self.psi[im].re) * inv_2dx;
            let dpsi_im = (self.psi[ip].im - self.psi[im].im) * inv_2dx;
            // Im(psi* psi_x) = psi.re * dpsi_im - psi.im * dpsi_re
            let im_part = self.psi[i].re * dpsi_im - self.psi[i].im * dpsi_re;
            p += im_part;
        }
        p * dx
    }

    /// Hamiltonian: H = integral [ (1/2)|psi_x|^2 - (kappa/2)|psi|^4 ] dx
    pub fn hamiltonian(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let kappa = self.config.kappa;
        let inv_2dx = 1.0 / (2.0 * dx);
        let mut h = 0.0f32;
        for i in 0..nx {
            let ip = (i + 1) % nx;
            let im = (i + nx - 1) % nx;
            let dpsi_re = (self.psi[ip].re - self.psi[im].re) * inv_2dx;
            let dpsi_im = (self.psi[ip].im - self.psi[im].im) * inv_2dx;
            let grad2 = dpsi_re * dpsi_re + dpsi_im * dpsi_im;
            let rho2 = self.psi[i].norm2();
            h += 0.5 * grad2 - 0.5 * kappa * rho2 * rho2;
        }
        h * dx
    }

    /// Maximum |psi|
    pub fn max_amplitude(&self) -> f32 {
        self.psi.iter().map(|c| c.norm()).fold(0.0f32, f32::max)
    }

    /// Find peak position (grid index of max |psi|)
    pub fn find_peak(&self) -> usize {
        let mut peak = 0usize;
        let mut max_rho = 0.0f32;
        for (i, c) in self.psi.iter().enumerate() {
            let r = c.norm2();
            if r > max_rho {
                max_rho = r;
                peak = i;
            }
        }
        peak
    }

    pub fn reset(&mut self) {
        for c in self.psi.iter_mut() { *c = Complex::zero(); }
        self.time = 0.0;
        self.steps = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_config_default() {
        let c = NlsConfig::default();
        assert_eq!(c.nx, 256);
        assert_eq!(c.dx, 0.1);
        assert_eq!(c.dt, 0.005);
        assert_eq!(c.kappa, 1.0);
    }

    #[test]
    fn test_config_length() {
        let c = NlsConfig { nx: 128, dx: 0.2, dt: 0.01, kappa: 1.0 };
        assert!(approx_eq(c.length(), 25.6, 1e-6));
    }

    #[test]
    fn test_config_n_cells() {
        let c = NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 };
        assert_eq!(c.n_cells(), 64);
    }

    #[test]
    fn test_k_nyquist() {
        let c = NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 };
        // Nyquist wavenumber = pi / dx
        assert!(approx_eq(c.k_nyquist(), std::f32::consts::PI / 0.1, 1e-5));
    }

    #[test]
    fn test_solver_new() {
        let s = NlsSolver::new(NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 });
        assert_eq!(s.psi.len(), 64);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for c in s.psi.iter() {
            assert_eq!(c.re, 0.0);
            assert_eq!(c.im, 0.0);
        }
    }

    #[test]
    fn test_complex_arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        let sum = a + b;
        assert!(approx_eq(sum.re, 4.0, 1e-6));
        assert!(approx_eq(sum.im, 6.0, 1e-6));
        let prod = a * b;
        // (1+2i)(3+4i) = 3 + 4i + 6i + 8i^2 = -5 + 10i
        assert!(approx_eq(prod.re, -5.0, 1e-6));
        assert!(approx_eq(prod.im, 10.0, 1e-6));
        assert!(approx_eq(a.norm2(), 5.0, 1e-6));
    }

    #[test]
    fn test_initialize_bright_soliton() {
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 0.0);
        // Peak at center i=128 (x=12.8)
        let peak = s.find_peak();
        assert!((peak as i32 - 128).abs() <= 1, "peak should be near center, got {}", peak);
        // Amplitude at peak should be ~1.0
        let amp = s.psi[peak].norm();
        assert!(approx_eq(amp, 1.0, 1e-5), "peak amplitude {}, expected 1.0", amp);
        // Far away should be near 0
        assert!(s.psi[0].norm() < 0.01);
        assert!(s.psi[255].norm() < 0.01);
    }

    #[test]
    fn test_initialize_two_solitons() {
        let mut s = NlsSolver::new(NlsConfig { nx: 512, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_two_solitons(1.0, 12.8, 0.0, 1.0, 38.4, 0.0);
        // Should have two peaks
        let mut peaks = 0;
        let mut prev = 0.0f32;
        let mut going_up = false;
        for i in 0..512 {
            let r = s.psi[i].norm();
            if r > prev && !going_up {
                going_up = true;
            } else if r < prev && going_up {
                peaks += 1;
                going_up = false;
            }
            prev = r;
        }
        assert!(peaks >= 2, "expected at least 2 peaks, got {}", peaks);
    }

    #[test]
    fn test_initialize_gaussian() {
        let mut s = NlsSolver::new(NlsConfig { nx: 128, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_gaussian(1.0, 6.4, 1.0, 0.0);
        // Peak at center
        assert!(approx_eq(s.psi[64].norm(), 1.0, 1e-5));
        // Decays
        assert!(s.psi[0].norm() < 0.01);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = NlsSolver::new(NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 });
        s.initialize_gaussian(0.5, 3.2, 1.0, 0.0);
        assert_eq!(s.time, 0.0);
        s.step();
        assert!(approx_eq(s.time, 0.01, 1e-9));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.02, 1e-9));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = NlsSolver::new(NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 });
        s.initialize_gaussian(0.5, 3.2, 1.0, 0.0);
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 1.0, 1e-6));
    }

    #[test]
    fn test_particle_number_conservation() {
        // N = integral |psi|^2 dx should be conserved by split-step Fourier
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 0.0);
        let n0 = s.particle_number();
        s.step_n(1000);
        let n1 = s.particle_number();
        let drift = (n1 - n0).abs() / n0.abs().max(1e-6);
        assert!(drift < 5e-3,
            "particle number drift too large: {} -> {} ({:.6}%)", n0, n1, drift * 100.0);
    }

    #[test]
    fn test_hamiltonian_conservation() {
        // H should be approximately conserved (Strang splitting is second-order)
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.002, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 0.0);
        let h0 = s.hamiltonian();
        s.step_n(500);
        let h1 = s.hamiltonian();
        let drift = (h1 - h0).abs() / h0.abs().max(1e-6);
        assert!(drift < 0.05,
            "hamiltonian drift too large: {} -> {} ({:.4}%)", h0, h1, drift * 100.0);
    }

    #[test]
    fn test_soliton_amplitude_preserved() {
        // A bright soliton should preserve its amplitude over time (exact soliton solution)
        let mut s = NlsSolver::new(NlsConfig { nx: 512, dx: 0.05, dt: 0.002, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 0.0);
        let a0 = s.max_amplitude();
        s.step_n(500);
        let a1 = s.max_amplitude();
        // Should be within a few percent
        assert!((a1 - a0).abs() / a0 < 0.05,
            "soliton amplitude not preserved: {} -> {}", a0, a1);
    }

    #[test]
    fn test_soliton_propagates() {
        // Soliton with velocity v should move
        let mut s = NlsSolver::new(NlsConfig { nx: 512, dx: 0.05, dt: 0.002, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 2.0);
        let p0 = s.find_peak() as f32 * s.config.dx;
        s.step_n(500);
        let p1 = s.find_peak() as f32 * s.config.dx;
        let dx_moved = p1 - p0;
        // Should have moved (direction: positive v -> rightward, modulo periodic wrap)
        assert!(dx_moved.abs() > 0.1,
            "soliton should move, dx_moved={}", dx_moved);
    }

    #[test]
    fn test_no_nan() {
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_bright_soliton(1.5, 12.8, 1.0);
        s.step_n(500);
        for c in s.psi.iter() {
            assert!(!c.re.is_nan() && !c.im.is_nan(), "NaN in psi");
            assert!(c.re.is_finite() && c.im.is_finite(), "Inf in psi");
        }
    }

    #[test]
    fn test_defocusing_no_blowup() {
        // Defocusing NLS (kappa=-1) with gaussian initial should not blow up
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: -1.0 });
        s.initialize_gaussian(1.0, 12.8, 2.0, 0.0);
        let a0 = s.max_amplitude();
        s.step_n(500);
        let a1 = s.max_amplitude();
        assert!(a1 < 2.0 * a0 + 1e-3, "defocusing should not blow up: {} -> {}", a0, a1);
        for c in s.psi.iter() {
            assert!(!c.re.is_nan() && !c.im.is_nan(), "NaN in defocusing psi");
        }
    }

    #[test]
    fn test_focusing_soliton_breathes() {
        // Focusing NLS with a perturbed initial condition: amplitude should stay bounded
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_gaussian(1.2, 12.8, 2.0, 0.0);
        s.step_n(500);
        let a = s.max_amplitude();
        assert!(a < 3.0 && a > 0.1, "focusing amplitude out of range: {}", a);
    }

    #[test]
    fn test_momentum_conservation() {
        // Momentum P = integral Im(psi* psi_x) dx should be conserved
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.002, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 1.0);
        let p0 = s.momentum();
        s.step_n(500);
        let p1 = s.momentum();
        let drift = (p1 - p0).abs() / p0.abs().max(1e-3);
        assert!(drift < 0.1,
            "momentum drift too large: {} -> {} ({:.4}%)", p0, p1, drift * 100.0);
    }

    #[test]
    fn test_reset() {
        let mut s = NlsSolver::new(NlsConfig { nx: 64, dx: 0.1, dt: 0.01, kappa: 1.0 });
        s.initialize_gaussian(1.0, 3.2, 1.0, 0.0);
        s.step_n(10);
        assert!(s.steps > 0);
        assert!(s.max_amplitude() > 0.0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        for c in s.psi.iter() {
            assert_eq!(c.re, 0.0);
            assert_eq!(c.im, 0.0);
        }
    }

    #[test]
    fn test_find_peak() {
        let mut s = NlsSolver::new(NlsConfig { nx: 256, dx: 0.1, dt: 0.005, kappa: 1.0 });
        s.initialize_bright_soliton(1.0, 12.8, 0.0);
        let peak = s.find_peak();
        // Peak should be near x=12.8 -> i=128
        assert!((peak as i32 - 128).abs() <= 1, "peak at {}, expected 128", peak);
    }

    #[test]
    fn test_two_soliton_collision() {
        // Two solitons colliding: after collision, peaks should re-emerge
        // (Zakharov-Shabat soliton collision property)
        let mut s = NlsSolver::new(NlsConfig { nx: 1024, dx: 0.05, dt: 0.002, kappa: 1.0 });
        // Two solitons moving toward each other
        s.initialize_two_solitons(1.0, 12.8, 2.0, 1.0, 38.4, -2.0);
        let a0 = s.max_amplitude();
        s.step_n(1000);
        let a1 = s.max_amplitude();
        // After collision, solitons should re-emerge with similar amplitude
        assert!((a1 - a0).abs() / a0 < 0.3,
            "soliton amplitude after collision should be similar: {} -> {}", a0, a1);
        assert!(!s.psi.iter().any(|c| c.re.is_nan() || c.im.is_nan()),
            "NaN after collision");
    }
}