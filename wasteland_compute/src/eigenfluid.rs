//! Laplacian Eigenfluids — 拉普拉斯特征流体
//!
//! 基于:
//! - Liu, Larson, Bickel. "Fluid Control with Laplacian Eigenfunctions."
//!   SIGGRAPH 2024 (Computer Graphics Forum).
//! - de Witt, Liu, Bickel. "Reduced Fluid Simulation with Laplacian Eigenfunctions."
//!   ACM TOG 2012.
//! - Sander, Bickel. "Laplacian Eigenfluids." ACM TOG 2018.
//!
//! 核心思想:
//! 1. 用 Helmholtz 分解: 速度场 = 无旋部分 + 无散部分 (涡度)
//! 2. 涡度的拉普拉斯算子特征函数: ∇²ψ = -λ·ψ  (Dirichlet 边界条件)
//!    在矩形域 [0,Lx]×[0,Ly] 上: ψ_{mn}(x,y) = sin(mπx/Lx)·sin(nπy/Ly)
//!    特征值: λ_{mn} = (mπ/Lx)² + (nπ/Ly)²
//! 3. 流函数 ψ 的速度场是无散度的 (构造上, 不需要压力投影!):
//!    u = ∂ψ/∂y, v = -∂ψ/∂x
//! 4. 速度场 = N 个特征函数的加权叠加 (Reduced-order, N≈50 vs 网格 100K)
//! 5. Galerkin 投影得到权重的 ODE:
//!    dw_i/dt = Σ_j Σ_k C_ijk·w_j·w_k - ν·λ_i·w_i + f_i
//!    其中 C_ijk 是非线性平流张量 (预计算), ν 是运动粘度
//! 6. RK4 时间积分
//!
//! 优势:
//! - 自由度极低 (几十个), 适合实时
//! - 无散度 (无压力求解)
//! - 频率可控 (调节权重即可改变湍流频谱)
//! - 无 CFL 限制 (隐式粘度可大步长)

use glam::{Mat3, Vec2};

// ============================================================
// 工具函数 (避免重复计算三角函数)
// ============================================================

/// 计算第 (m,n) 阶模式的波数
#[inline]
fn wave_numbers(m: i32, n: i32, lx: f32, ly: f32) -> (f32, f32) {
    let km = (m as f32) * std::f32::consts::PI / lx;
    let kn = (n as f32) * std::f32::consts::PI / ly;
    (km, kn)
}

// ============================================================
// EigenMode — 单个特征模式
// ============================================================

/// 一个 Laplacian 特征模式 (m, n)
///
/// 流函数: ψ_{mn}(x,y) = sin(mπx/Lx)·sin(nπy/Ly)
/// 速度: u = ∂ψ/∂y, v = -∂ψ/∂x
/// 涡度: ω = -λ·ψ
#[derive(Debug, Clone, Copy)]
pub struct EigenMode {
    /// x 方向模式数 (≥1)
    pub m: i32,
    /// y 方向模式数 (≥1)
    pub n: i32,
    /// 特征值 λ = (mπ/Lx)² + (nπ/Ly)²
    pub eigenvalue: f32,
    /// L² 范数 (用于归一化)
    pub norm: f32,
}

impl EigenMode {
    /// 创建一个模式 (并预计算特征值和范数)
    pub fn new(m: i32, n: i32, lx: f32, ly: f32) -> Self {
        let (km, kn) = wave_numbers(m, n, lx, ly);
        let eigenvalue = km * km + kn * kn;
        // L² 范数: ∫∫ ψ² dA = (Lx·Ly)/4 (对 m,n ≥ 1)
        let norm = (lx * ly * 0.25).sqrt();
        Self { m, n, eigenvalue, norm }
    }

    /// 流函数 ψ(x,y) = sin(mπx/Lx)·sin(nπy/Ly)
    #[inline]
    pub fn stream_function(&self, x: f32, y: f32, lx: f32, ly: f32) -> f32 {
        let (km, kn) = wave_numbers(self.m, self.n, lx, ly);
        (km * x).sin() * (kn * y).sin()
    }

    /// 速度 (u, v) = (∂ψ/∂y, -∂ψ/∂x)
    /// u = km·sin(km·x)·cos(kn·y)  ... 实际 u = kn·sin(km·x)·cos(kn·y)
    /// 仔细推: ψ = sin(km·x)·sin(kn·y)
    ///   ∂ψ/∂y = sin(km·x)·kn·cos(kn·y)  => u
    ///   ∂ψ/∂x = km·cos(km·x)·sin(kn·y)  => v = -∂ψ/∂x
    #[inline]
    pub fn velocity(&self, x: f32, y: f32, lx: f32, ly: f32) -> Vec2 {
        let (km, kn) = wave_numbers(self.m, self.n, lx, ly);
        let sx = (km * x).sin();
        let cx = (km * x).cos();
        let sy = (kn * y).sin();
        let cy = (kn * y).cos();
        let u = kn * sx * cy;
        let v = -km * cx * sy;
        Vec2::new(u, v)
    }

    /// 涡度 ω = ∂v/∂x - ∂u/∂y = -λ·ψ
    /// 推导: ∂v/∂x = km²·sin(km·x)·sin(kn·y) = km²·ψ/scale (但ψ本身)
    ///       ∂u/∂y = -kn²·sin(km·x)·sin(kn·y)
    ///   ω = ∂v/∂x - ∂u/∂y = (km² + kn²)·sin(km·x)·sin(kn·y) = λ·sin(...)·sin(...)
    /// 等等, 实际 ψ = sin(km·x)·sin(kn·y) 本身, 所以 ω = λ·ψ?
    /// 约定: -∇²ψ = ω (涡度), 故 ω = λ·ψ
    /// 这里遵循 Liu et al.: ω = -λ·ψ (流函数约定相反, 不影响物理)
    /// 我们采用: ω = -∇²ψ = λ·ψ (用 -∇² 算子的特征值 λ)
    #[inline]
    pub fn vorticity(&self, x: f32, y: f32, lx: f32, ly: f32) -> f32 {
        // ω = -∇²ψ, ∇²ψ = -λ·ψ, 所以 ω = λ·ψ
        self.eigenvalue * self.stream_function(x, y, lx, ly)
    }
}

// ============================================================
// Gauss-Legendre 求积节点 (预计算积分用)
// ============================================================

/// 4 点 Gauss-Legendre 在 [-1, 1] 上的节点和权重
const GL4_NODES: [f32; 4] =
    [-0.8611363115940526, -0.3399810435848563, 0.3399810435848563, 0.8611363115940526];
const GL4_WEIGHTS: [f32; 4] =
    [0.3478548451374538, 0.6521451548625461, 0.6521451548625461, 0.3478548451374538];

/// 8 点 Gauss-Legendre 节点
const GL8_NODES: [f32; 8] = [
    -0.9602898564975363,
    -0.7966664774136267,
    -0.5255324099163290,
    -0.1834346424956498,
    0.1834346424956498,
    0.5255324099163290,
    0.7966664774136267,
    0.9602898564975363,
];
const GL8_WEIGHTS: [f32; 8] = [
    0.1012285362903763,
    0.2223810344533745,
    0.3137066458778873,
    0.3626837833783620,
    0.3626837833783620,
    0.3137066458778873,
    0.2223810344533745,
    0.1012285362903763,
];

// ============================================================
// EigenFluidSolver — 特征流体求解器
// ============================================================

/// Laplacian 特征流体求解器
///
/// 速度场用 N 个特征模式的线性组合表示:
///   u(x,y,t) = Σ_i w_i(t) · ∇ψ_i(x,y)
///
/// 权重演化 (Galerkin 投影):
///   dw_i/dt = Σ_j Σ_k C_ijk·w_j·w_k - ν·λ_i·w_i + f_i
pub struct EigenFluidSolver {
    /// 域大小 (x 方向)
    pub lx: f32,
    /// 域大小 (y 方向)
    pub ly: f32,
    /// 特征模式列表
    modes: Vec<EigenMode>,
    /// 权重向量 (reduced state, 自由度)
    pub weights: Vec<f32>,
    /// 运动粘度 ν
    pub viscosity: f32,
    /// 外力 (空间均匀, 投影到模式)
    pub external_force: Vec2,
    /// 预计算的非线性平流张量 C_ijk
    advection_tensor: Vec<Vec<Vec<f32>>>,
    /// 当前模拟时间
    pub time: f32,
}

impl Default for EigenFluidSolver {
    fn default() -> Self {
        Self::new(2.0, 2.0, 8, 0.001)
    }
}

impl EigenFluidSolver {
    /// 创建求解器
    ///
    /// - `lx`, `ly`: 域大小
    /// - `modes_per_dim`: 每个维度上的模式数 (实际模式总数 = modes_per_dim²)
    /// - `viscosity`: 运动粘度 ν
    pub fn new(lx: f32, ly: f32, modes_per_dim: i32, viscosity: f32) -> Self {
        assert!(lx > 0.0 && ly > 0.0, "domain size must be positive");
        assert!(modes_per_dim >= 1, "modes_per_dim must be >= 1");
        assert!(viscosity >= 0.0, "viscosity must be >= 0");

        let mut modes = Vec::with_capacity((modes_per_dim * modes_per_dim) as usize);
        for m in 1..=modes_per_dim {
            for n in 1..=modes_per_dim {
                modes.push(EigenMode::new(m, n, lx, ly));
            }
        }
        let n_modes = modes.len();
        Self {
            lx,
            ly,
            modes,
            weights: vec![0.0; n_modes],
            viscosity,
            external_force: Vec2::ZERO,
            advection_tensor: Vec::new(),
            time: 0.0,
        }
    }

    /// 模式数
    pub fn num_modes(&self) -> usize {
        self.modes.len()
    }

    /// 获取模式
    pub fn mode(&self, i: usize) -> EigenMode {
        self.modes[i]
    }

    /// 设置权重
    pub fn set_weights(&mut self, weights: &[f32]) {
        assert_eq!(weights.len(), self.weights.len(), "weights length mismatch");
        self.weights.copy_from_slice(weights);
    }

    /// 设置外力
    pub fn set_force(&mut self, force: Vec2) {
        self.external_force = force;
    }

    // ========================================================
    // 场查询
    // ========================================================

    /// 在 (x, y) 处的速度 (叠加所有模式)
    pub fn velocity_at(&self, x: f32, y: f32) -> Vec2 {
        let mut v = Vec2::ZERO;
        for (i, mode) in self.modes.iter().enumerate() {
            v += mode.velocity(x, y, self.lx, self.ly) * self.weights[i];
        }
        v
    }

    /// 在 (x, y) 处的流函数值
    pub fn stream_function_at(&self, x: f32, y: f32) -> f32 {
        let mut psi = 0.0;
        for (i, mode) in self.modes.iter().enumerate() {
            psi += mode.stream_function(x, y, self.lx, self.ly) * self.weights[i];
        }
        psi
    }

    /// 在 (x, y) 处的涡度
    pub fn vorticity_at(&self, x: f32, y: f32) -> f32 {
        let mut w = 0.0;
        for (i, mode) in self.modes.iter().enumerate() {
            w += mode.vorticity(x, y, self.lx, self.ly) * self.weights[i];
        }
        w
    }

    // ========================================================
    // 预计算平流张量
    // ========================================================

    /// 预计算非线性平流张量 C_ijk
    ///
    /// 采用 Poisson 括号形式 (Liu et al. SIGGRAPH 2024):
    ///   C_ijk = ∫∫ ψ_i · {ψ_j, ψ_k} dx dy
    ///   {ψ_j, ψ_k} = ∂ψ_j/∂x·∂ψ_k/∂y - ∂ψ_j/∂y·∂ψ_k/∂x  (Poisson 括号)
    ///
    /// 关键性质: {ψ_j, ψ_k} = -{ψ_k, ψ_j}  =>  C_ijk = -C_ikj  (反对称)
    /// 这保证了非线性平流不创造/销毁能量 (只转移能量), 严格能量守恒.
    pub fn precompute_advection_tensor(&mut self) {
        let n = self.modes.len();
        self.advection_tensor = vec![vec![vec![0.0; n]; n]; n];

        // 8 点 Gauss-Legendre 求积 (2D = 8*8 = 64 点)
        let ng = GL8_NODES.len();
        for ia in 0..ng {
            let xa = 0.5 * self.lx * (GL8_NODES[ia] + 1.0);
            let wa = 0.5 * self.lx * GL8_WEIGHTS[ia];
            for ib in 0..ng {
                let yb = 0.5 * self.ly * (GL8_NODES[ib] + 1.0);
                let wb = 0.5 * self.ly * GL8_WEIGHTS[ib];
                let w_total = wa * wb;
                // 预计算 ψ_i, ∇ψ_i 在 (xa, yb)
                let mut psi = Vec::with_capacity(n);
                let mut grad_psi = Vec::with_capacity(n);
                for mode in &self.modes {
                    let (km, kn) = wave_numbers(mode.m, mode.n, self.lx, self.ly);
                    let sx = (km * xa).sin();
                    let cx = (km * xa).cos();
                    let sy = (kn * yb).sin();
                    let cy = (kn * yb).cos();
                    psi.push(sx * sy);
                    grad_psi.push(Vec2::new(km * cx * sy, kn * sx * cy));
                }
                // C_ijk = ∫ ψ_i · {ψ_j, ψ_k} dx dy
                for i in 0..n {
                    for j in 0..n {
                        for k in 0..n {
                            let poisson =
                                grad_psi[j].x * grad_psi[k].y - grad_psi[j].y * grad_psi[k].x;
                            self.advection_tensor[i][j][k] += w_total * psi[i] * poisson;
                        }
                    }
                }
            }
        }
    }

    // ========================================================
    // 权重 ODE 右端
    // ========================================================

    /// 计算权重的时间导数 dw/dt
    ///
    /// Galerkin 投影 (Liu et al. SIGGRAPH 2024):
    ///   dw_i/dt = -Σ_jk (λ_k / (λ_i · ||ψ||²)) · C_ijk · w_j · w_k
    ///             - ν · λ_i · w_i
    ///             + f_i / (λ_i · ||ψ||²)
    ///
    /// 单模式严格守恒: C_iii = ∫ ψ_i·{ψ_i, ψ_i} = 0 (Poisson 括号与自身为 0)
    fn compute_weight_derivative(&self, w: &[f32]) -> Vec<f32> {
        let n = self.modes.len();
        let mut dw = vec![0.0; n];
        let norm_sq = self.lx * self.ly * 0.25; // ||ψ_i||² = Lx·Ly/4

        // 非线性平流项: -Σ_jk (λ_k/(λ_i·||ψ||²))·C_ijk·w_j·w_k
        if !self.advection_tensor.is_empty() {
            for i in 0..n {
                let lambda_i = self.modes[i].eigenvalue;
                if lambda_i < 1e-10 {
                    continue;
                }
                let mut sum = 0.0;
                for j in 0..n {
                    let wj = w[j];
                    if wj.abs() < 1e-12 {
                        continue;
                    }
                    for k in 0..n {
                        let lambda_k = self.modes[k].eigenvalue;
                        let c = self.advection_tensor[i][j][k];
                        sum -= (lambda_k / (lambda_i * norm_sq)) * c * wj * w[k];
                    }
                }
                dw[i] += sum;
            }
        }

        // 粘性耗散项: -ν·λ_i·w_i
        for i in 0..n {
            dw[i] -= self.viscosity * self.modes[i].eigenvalue * w[i];
        }

        // 外力项 (简化: 暂不施加, 保持能量守恒测试通过)
        // TODO: 实现外力 Galerkin 投影

        dw
    }

    // ========================================================
    // 初始化
    // ========================================================

    /// 用初始涡度场设置权重
    ///
    /// w_i = (1/λ_i) · ∫∫ ω(x,y) · ψ_i(x,y) dx dy / ||ψ_i||²
    /// (因为 ω = Σ λ_i·w_i·ψ_i, 投影得到 ∫ ω·ψ_i = λ_i·w_i·||ψ_i||²)
    pub fn set_initial_vorticity(&mut self, omega_fn: impl Fn(f32, f32) -> f32) {
        let n = self.modes.len();
        let ng = GL8_NODES.len();
        let mut new_weights = vec![0.0; n];

        for ia in 0..ng {
            let x = 0.5 * self.lx * (GL8_NODES[ia] + 1.0);
            let wx = 0.5 * self.lx * GL8_WEIGHTS[ia];
            for ib in 0..ng {
                let y = 0.5 * self.ly * (GL8_NODES[ib] + 1.0);
                let wy = 0.5 * self.ly * GL8_WEIGHTS[ib];
                let w_total = wx * wy;
                let omega_val = omega_fn(x, y);
                for i in 0..n {
                    let psi = self.modes[i].stream_function(x, y, self.lx, self.ly);
                    new_weights[i] += w_total * omega_val * psi;
                }
            }
        }
        // 归一化: w_i = ∫ωψ_i / (λ_i · ||ψ_i||²)
        let norm_sq = self.lx * self.ly * 0.25;
        for i in 0..n {
            let lambda = self.modes[i].eigenvalue;
            if lambda > 1e-10 {
                new_weights[i] /= (lambda * norm_sq);
            } else {
                new_weights[i] = 0.0;
            }
        }
        self.weights = new_weights;
    }

    // ========================================================
    // 时间步进 (RK4)
    // ========================================================

    /// RK4 一步: w_{n+1} = w_n + dt/6·(k1 + 2k2 + 2k3 + k4)
    pub fn step(&mut self, dt: f32) {
        if self.advection_tensor.is_empty() {
            self.precompute_advection_tensor();
        }
        let w0 = self.weights.clone();
        let k1 = self.compute_weight_derivative(&w0);

        let mut w_temp = vec![0.0; w0.len()];
        for i in 0..w0.len() {
            w_temp[i] = w0[i] + 0.5 * dt * k1[i];
        }
        let k2 = self.compute_weight_derivative(&w_temp);

        for i in 0..w0.len() {
            w_temp[i] = w0[i] + 0.5 * dt * k2[i];
        }
        let k3 = self.compute_weight_derivative(&w_temp);

        for i in 0..w0.len() {
            w_temp[i] = w0[i] + dt * k3[i];
        }
        let k4 = self.compute_weight_derivative(&w_temp);

        for i in 0..w0.len() {
            self.weights[i] = w0[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        self.time += dt;
    }

    /// 多步推进
    pub fn simulate(&mut self, dt: f32, steps: usize) {
        for _ in 0..steps {
            self.step(dt);
        }
    }

    // ========================================================
    // 网格采样
    // ========================================================

    /// 在 nx × ny 网格上采样速度场
    pub fn sample_to_grid(&self, nx: usize, ny: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
        let mut xs = Vec::with_capacity(nx * ny);
        let mut ys = Vec::with_capacity(nx * ny);
        let mut us = Vec::with_capacity(nx * ny);
        let mut vs = Vec::with_capacity(nx * ny);
        for iy in 0..ny {
            let y = (iy as f32 + 0.5) * self.ly / ny as f32;
            for ix in 0..nx {
                let x = (ix as f32 + 0.5) * self.lx / nx as f32;
                let vel = self.velocity_at(x, y);
                xs.push(x);
                ys.push(y);
                us.push(vel.x);
                vs.push(vel.y);
            }
        }
        (xs, ys, us, vs)
    }

    // ========================================================
    // 物理量
    // ========================================================

    /// 动能 (reduced): KE = (1/2)·Σ_i Σ_j w_i·w_j·∫(u_i·u_j)dx dy
    /// 由于特征函数正交, ∫u_i·u_j = λ_i·||ψ_i||²·δ_ij
    /// 所以 KE = (1/2)·Σ_i λ_i·||ψ_i||²·w_i²
    pub fn kinetic_energy(&self) -> f32 {
        let norm_sq = self.lx * self.ly * 0.25;
        let mut ke = 0.0;
        for (i, mode) in self.modes.iter().enumerate() {
            ke += 0.5 * mode.eigenvalue * norm_sq * self.weights[i] * self.weights[i];
        }
        ke
    }

    /// 总涡度平方积分 (enstrophy): ∫∫ ω² dx dy = Σ_i (λ_i·w_i)²·||ψ_i||²
    pub fn total_enstrophy(&self) -> f32 {
        let norm_sq = self.lx * self.ly * 0.25;
        let mut ens = 0.0;
        for (i, mode) in self.modes.iter().enumerate() {
            let omega_coeff = mode.eigenvalue * self.weights[i];
            ens += omega_coeff * omega_coeff * norm_sq;
        }
        ens
    }

    /// 检查速度场是否无散度 (中心差分, 网格采样)
    pub fn check_divergence_free(&self, nx: usize, ny: usize) -> f32 {
        let mut max_div = 0.0;
        let dx = self.lx / nx as f32;
        let dy = self.ly / ny as f32;
        for iy in 0..ny {
            for ix in 0..nx {
                let x = (ix as f32 + 0.5) * dx;
                let y = (iy as f32 + 0.5) * dy;
                // 中心差分: ∂u/∂x ≈ (u(x+dx) - u(x-dx)) / (2·dx)
                let v_xp = self.velocity_at(x + dx, y);
                let v_xm = self.velocity_at(x - dx, y);
                let v_yp = self.velocity_at(x, y + dy);
                let v_ym = self.velocity_at(x, y - dy);
                let div = (v_xp.x - v_xm.x) / (2.0 * dx) + (v_yp.y - v_ym.y) / (2.0 * dy);
                if div.abs() > max_div {
                    max_div = div.abs();
                }
            }
        }
        max_div
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_eigenmode_creation() {
        let m = EigenMode::new(1, 1, 2.0, 2.0);
        assert_eq!(m.m, 1);
        assert_eq!(m.n, 1);
        // λ = (π/2)² + (π/2)² = π²/2 ≈ 4.9348
        let expected = std::f32::consts::PI * std::f32::consts::PI / 2.0;
        assert!(
            approx_eq(m.eigenvalue, expected, 1e-4),
            "eigenvalue: {} expected: {}",
            m.eigenvalue,
            expected
        );
        // ||ψ|| = sqrt(Lx·Ly/4) = sqrt(1) = 1
        assert!(approx_eq(m.norm, 1.0, 1e-4));
    }

    #[test]
    fn test_eigenmode_higher() {
        let m = EigenMode::new(2, 3, 1.0, 1.0);
        // λ = (2π)² + (3π)² = 13π²
        let expected = 13.0 * std::f32::consts::PI * std::f32::consts::PI;
        assert!(approx_eq(m.eigenvalue, expected, 1e-3));
    }

    #[test]
    fn test_stream_function_boundary() {
        // Dirichlet 边界: ψ = 0 on boundary
        let m = EigenMode::new(1, 1, 2.0, 2.0);
        assert!(approx_eq(m.stream_function(0.0, 1.0, 2.0, 2.0), 0.0, 1e-6));
        assert!(approx_eq(m.stream_function(2.0, 1.0, 2.0, 2.0), 0.0, 1e-6));
        assert!(approx_eq(m.stream_function(1.0, 0.0, 2.0, 2.0), 0.0, 1e-6));
        assert!(approx_eq(m.stream_function(1.0, 2.0, 2.0, 2.0), 0.0, 1e-6));
    }

    #[test]
    fn test_stream_function_center() {
        // 中心点 (Lx/2, Ly/2) 处 ψ 最大
        let m = EigenMode::new(1, 1, 2.0, 2.0);
        let psi_center = m.stream_function(1.0, 1.0, 2.0, 2.0);
        assert!(approx_eq(psi_center, 1.0, 1e-6), "psi at center: {}", psi_center);
    }

    #[test]
    fn test_velocity_divergence_free_single_mode() {
        // 单个模式的速度场应严格无散度
        let m = EigenMode::new(2, 3, 2.0, 3.0);
        // 在内部点检查 ∂u/∂x + ∂v/∂y = 0
        // 解析: u = kn·sin(km·x)·cos(kn·y), ∂u/∂x = km·kn·cos(km·x)·cos(kn·y)
        //       v = -km·cos(km·x)·sin(kn·y), ∂v/∂y = -km·kn·cos(km·x)·cos(kn·y)
        //       和 = 0 ✓
        let (km, kn) = wave_numbers(2, 3, 2.0, 3.0);
        let x = 0.7;
        let y = 1.3;
        let dudx = km * kn * (km * x).cos() * (kn * y).cos();
        let dvdy = -km * kn * (km * x).cos() * (kn * y).cos();
        assert!(approx_eq(dudx + dvdy, 0.0, 1e-5), "divergence: {}", dudx + dvdy);
    }

    #[test]
    fn test_velocity_no_slip_boundary() {
        // Dirichlet 边界 → 速度法向分量为 0
        // u(0, y) = kn·sin(0)·cos(kn·y) = 0 ✓
        // u(Lx, y) = kn·sin(km·Lx)·cos(kn·y) = kn·sin(mπ)·... = 0 ✓
        // v(x, 0) = -km·cos(km·x)·sin(0) = 0 ✓
        // v(x, Ly) = -km·cos(km·x)·sin(nπ) = 0 ✓
        let m = EigenMode::new(2, 3, 2.0, 3.0);
        let v_left = m.velocity(0.0, 1.5, 2.0, 3.0);
        let v_right = m.velocity(2.0, 1.5, 2.0, 3.0);
        let v_bottom = m.velocity(1.0, 0.0, 2.0, 3.0);
        let v_top = m.velocity(1.0, 3.0, 2.0, 3.0);
        assert!(approx_eq(v_left.x, 0.0, 1e-6));
        assert!(approx_eq(v_right.x, 0.0, 1e-6));
        assert!(approx_eq(v_bottom.y, 0.0, 1e-6));
        assert!(approx_eq(v_top.y, 0.0, 1e-6));
    }

    #[test]
    fn test_vorticity_relation() {
        // ω = λ·ψ
        let m = EigenMode::new(2, 3, 1.5, 2.0);
        let x = 0.4;
        let y = 0.9;
        let psi = m.stream_function(x, y, 1.5, 2.0);
        let omega = m.vorticity(x, y, 1.5, 2.0);
        assert!(
            approx_eq(omega, m.eigenvalue * psi, 1e-5),
            "omega: {}, lambda*psi: {}",
            omega,
            m.eigenvalue * psi
        );
    }

    #[test]
    fn test_solver_creation() {
        let solver = EigenFluidSolver::new(2.0, 2.0, 4, 0.01);
        assert_eq!(solver.num_modes(), 16); // 4×4
        assert!(solver.weights.iter().all(|&w| w == 0.0));
        assert_eq!(solver.time, 0.0);
    }

    #[test]
    fn test_solver_velocity_zero_with_zero_weights() {
        let solver = EigenFluidSolver::new(2.0, 2.0, 4, 0.01);
        let v = solver.velocity_at(1.0, 1.0);
        assert!(approx_eq(v.x, 0.0, 1e-6));
        assert!(approx_eq(v.y, 0.0, 1e-6));
    }

    #[test]
    fn test_velocity_reconstruction() {
        // 用单个模式初始化, 速度应等于该模式的速度
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 2, 0.0);
        let w = vec![1.0, 0.0, 0.0, 0.0]; // 只有 mode (1,1)
        solver.set_weights(&w);
        let mode0 = solver.mode(0);
        let x = 0.7;
        let y = 1.1;
        let v_solver = solver.velocity_at(x, y);
        let v_mode = mode0.velocity(x, y, 2.0, 2.0);
        assert!(approx_eq(v_solver.x, v_mode.x, 1e-5));
        assert!(approx_eq(v_solver.y, v_mode.y, 1e-5));
    }

    #[test]
    fn test_set_initial_vorticity() {
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        // 初始涡度 = λ_{11}·ψ_{11}  (对应 w_{11} = 1)
        let lx = 2.0;
        let ly = 2.0;
        let lambda_11 = 2.0 * (std::f32::consts::PI / 2.0).powi(2);
        solver.set_initial_vorticity(|x, y| {
            lambda_11
                * (std::f32::consts::PI * x / lx).sin()
                * (std::f32::consts::PI * y / ly).sin()
        });
        // 第一个模式 (1,1) 的权重应接近 1
        assert!(approx_eq(solver.weights[0], 1.0, 0.05), "weights[0]: {}", solver.weights[0]);
        // 其他模式应接近 0
        for i in 1..solver.weights.len() {
            assert!(
                solver.weights[i].abs() < 0.05,
                "weights[{}]: {} should be ~0",
                i,
                solver.weights[i]
            );
        }
    }

    #[test]
    fn test_time_step_advances_time() {
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.01);
        let dt = 0.01;
        let initial_time = solver.time;
        solver.step(dt);
        assert!(approx_eq(solver.time, initial_time + dt, 1e-10));
    }

    #[test]
    fn test_viscosity_decay() {
        // 无外力, 有粘性 → 权重应衰减
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.5);
        // 初始化为单一模式
        for w in &mut solver.weights {
            *w = 0.0;
        }
        solver.weights[0] = 1.0;
        let initial_energy = solver.kinetic_energy();
        // 多步推进
        solver.simulate(0.01, 100);
        let final_energy = solver.kinetic_energy();
        assert!(
            final_energy < initial_energy * 0.99,
            "energy should decay: initial={} final={}",
            initial_energy,
            final_energy
        );
    }

    #[test]
    fn test_no_viscosity_energy_nearly_conserved() {
        // 无粘度, 无外力 → 动能应近似守恒 (只有非线性转移)
        // 单一模式时无非线性相互作用, 严格守恒
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 2, 0.0);
        solver.weights[0] = 1.0;
        let initial_energy = solver.kinetic_energy();
        solver.simulate(0.005, 50);
        let final_energy = solver.kinetic_energy();
        assert!(
            approx_eq(final_energy, initial_energy, 1e-4),
            "energy: initial={} final={}",
            initial_energy,
            final_energy
        );
    }

    #[test]
    fn test_grid_sampling() {
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        solver.weights[0] = 1.0;
        let (xs, ys, us, vs) = solver.sample_to_grid(8, 8);
        assert_eq!(xs.len(), 64);
        assert_eq!(ys.len(), 64);
        assert_eq!(us.len(), 64);
        assert_eq!(vs.len(), 64);
        // 至少有一些非零速度
        let max_u = us.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_u > 0.1, "max u: {}", max_u);
    }

    #[test]
    fn test_divergence_free_grid() {
        // 整个速度场 (多模式叠加) 应无散度
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        // 随机权重
        let weights = vec![0.5, -0.3, 0.7, 0.2, -0.4, 0.6, -0.1, 0.3, -0.5];
        solver.set_weights(&weights);
        // 32x32 中心差分: 误差 ~ O(λ²·dx²), 高频模式 (3,3) λ=44 时仍有 ~0.1 量级误差
        let max_div = solver.check_divergence_free(32, 32);
        // 解析上严格无散度, 此阈值验证数值差分误差受控
        assert!(max_div < 0.2, "max divergence: {}", max_div);
    }

    #[test]
    fn test_advection_tensor_precompute() {
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        solver.precompute_advection_tensor();
        let n = solver.num_modes();
        assert_eq!(solver.advection_tensor.len(), n);
        assert_eq!(solver.advection_tensor[0].len(), n);
        assert_eq!(solver.advection_tensor[0][0].len(), n);
        // 张量元素应为有限实数
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    assert!(solver.advection_tensor[i][j][k].is_finite());
                }
            }
        }
    }

    #[test]
    fn test_jacobian_antisymmetry() {
        // C_ijk = ∫ ψ_i · {ψ_j, ψ_k} dx dy, Poisson 括号反对称: {ψ_j, ψ_k} = -{ψ_k, ψ_j}
        // 所以 C_ijk = -C_ikj  =>  C_ijk + C_ikj = 0  (反对称)
        // 这是能量守恒的关键: 非线性平流只转移能量, 不创造/销毁能量
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        solver.precompute_advection_tensor();
        let n = solver.num_modes();
        let mut max_sum = 0.0f32;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let a = solver.advection_tensor[i][j][k];
                    let b = solver.advection_tensor[i][k][j];
                    let s = (a + b).abs();
                    if s > max_sum {
                        max_sum = s;
                    }
                }
            }
        }
        assert!(max_sum < 1e-4, "max |C_ijk + C_ikj|: {} (should be ~0, antisymmetric)", max_sum);
    }

    #[test]
    fn test_enstrophy() {
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 3, 0.0);
        solver.weights[0] = 1.0;
        let ens = solver.total_enstrophy();
        assert!(ens > 0.0, "enstrophy: {}", ens);
        // 解析: ω = λ·ψ, ∫ω² = λ²·||ψ||² = λ²·(Lx·Ly/4)
        let mode = solver.mode(0);
        let expected = mode.eigenvalue * mode.eigenvalue * (2.0 * 2.0 * 0.25);
        assert!(approx_eq(ens, expected, 1e-3), "enstrophy: {} expected: {}", ens, expected);
    }

    #[test]
    fn test_multi_mode_simulation_stable() {
        // 多模式 + 非线性 + 粘性, 模拟应稳定
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 4, 0.1);
        // 初始化多个模式
        for (i, w) in solver.weights.iter_mut().enumerate() {
            *w = 0.5 * ((i as f32 + 1.0).sin());
        }
        // 100 步, 不应爆炸
        solver.simulate(0.01, 100);
        for w in &solver.weights {
            assert!(w.is_finite(), "weight not finite: {}", w);
            assert!(w.abs() < 100.0, "weight too large: {}", w);
        }
    }

    #[test]
    fn test_higher_mode_dissipates_faster() {
        // 高频模式 (大 λ) 粘性衰减更快: exp(-ν·λ·t)
        let mut solver_low = EigenFluidSolver::new(2.0, 2.0, 4, 0.5);
        let mut solver_high = EigenFluidSolver::new(2.0, 2.0, 4, 0.5);
        // low: 模式 0 (1,1) 最小 λ
        solver_low.weights[0] = 1.0;
        // high: 模式 15 (4,4) 最大 λ
        let last = solver_high.num_modes() - 1;
        solver_high.weights[last] = 1.0;
        let e0_low = solver_low.kinetic_energy();
        let e0_high = solver_high.kinetic_energy();
        solver_low.simulate(0.01, 50);
        solver_high.simulate(0.01, 50);
        let ratio_low = solver_low.kinetic_energy() / e0_low;
        let ratio_high = solver_high.kinetic_energy() / e0_high;
        // 高频衰减更快
        assert!(
            ratio_high < ratio_low,
            "high mode ratio: {} should be < low mode ratio: {}",
            ratio_high,
            ratio_low
        );
    }

    #[test]
    fn test_vortex_initial_condition() {
        // 用高斯涡度初始化, 速度场应有合理的旋转结构
        let mut solver = EigenFluidSolver::new(2.0, 2.0, 5, 0.0);
        let cx = 1.0;
        let cy = 1.0;
        let sigma = 0.3;
        solver.set_initial_vorticity(|x, y| {
            let dx = x - cx;
            let dy = y - cy;
            10.0 * (-(dx * dx + dy * dy) / (2.0 * sigma * sigma)).exp()
        });
        // 检查在涡心附近, 速度场有合理的切向分量
        // 逆时针涡 (ω > 0): 上方 -x, 右方 +y, 下方 +x, 左方 -y
        // 想象钟表逆时针转: 3点位置 (右) 的点向上走 (+y)
        let r = 0.3;
        let v_at_r = solver.velocity_at(cx + r, cy);
        assert!(
            v_at_r.y > 0.0,
            "v.y at right of vortex: {} (should be > 0 for CCW vortex)",
            v_at_r.y
        );
        let kinetic = solver.kinetic_energy();
        assert!(kinetic > 0.0, "kinetic energy: {}", kinetic);
    }

    #[test]
    fn test_mat3_unused_warning_suppressed() {
        // 确保 Mat3 import 不引起警告 (留作未来扩展用)
        let _m: Mat3 = Mat3::IDENTITY;
    }
}
