// 相场法断裂 (Phase-Field Fracture)
// 基于 Bourdin/Francfort-Marigo 变分断裂模型 + Griffith 准则
// 自由能泛函: Ψ(ε, φ) = (1-φ)²·Ψ_elastic(ε) + G_c·(φ²/(2l) + l|∇φ|²/2)
// 来源:
//   - Bourdin B, Francfort GA, Marigo JJ (2008) "The Variational Approach to Fracture"
//   - Francfort GA, Marigo JJ (1998) J Mech Phys Solids 46:1319-1342

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// 相场法断裂模型参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseFieldFracture {
    /// 临界能量释放率 G_c (J/m²)
    /// 皮质骨 G_c ≈ 1500-2200 J/m²
    /// 松质骨 G_c ≈ 500-800 J/m²
    pub g_c: f32,
    /// 长度尺度参数 l (m), 控制裂纹扩散宽度, 建议 0.5-2mm
    pub length_scale: f32,
    /// 迁移率 M, 控制相场演化速度
    pub mobility: f32,
}

impl PhaseFieldFracture {
    /// 皮质骨典型参数 (G_c 中位值 1800 J/m², l = 1mm)
    pub fn cortical_bone() -> Self {
        Self {
            g_c: 1800.0,
            length_scale: 0.001,
            mobility: 1.0,
        }
    }

    /// 松质骨典型参数 (G_c 中位值 650 J/m², l = 1.5mm)
    pub fn trabecular_bone() -> Self {
        Self {
            g_c: 650.0,
            length_scale: 0.0015,
            mobility: 1.0,
        }
    }

    /// 断裂能密度: G_c·(φ²/(2l) + l|∇φ|²/2)
    /// Bourdin 2008, Eq. 7
    #[inline]
    pub fn fracture_energy_density(&self, phi: f32, grad_phi: Vec3) -> f32 {
        let bulk = phi * phi / (2.0 * self.length_scale);
        let gradient = self.length_scale * grad_phi.dot(grad_phi) / 2.0;
        self.g_c * (bulk + gradient)
    }

    /// 退化弹性能密度: (1-φ)²·Ψ_elastic
    /// 退化函数 g(φ) = (1-φ)²
    #[inline]
    pub fn elastic_energy_density(&self, phi: f32, psi_elastic: f32) -> f32 {
        let degradation = (1.0 - phi).powi(2);
        degradation * psi_elastic
    }
}

/// 相场网格: 标量场 φ 与其梯度
/// φ ∈ [0,1]: 1=完整, 0=完全断裂
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseFieldGrid {
    /// 相场值 φ
    pub phi: Vec<f32>,
    /// ∇φ 梯度场
    pub grad_phi: Vec<Vec3>,
    /// 网格维度 (nx, ny, nz)
    pub dim: (usize, usize, usize),
    /// 网格间距 (dx, dy, dz) 单位 m
    pub cell_size: (f32, f32, f32),
}

impl PhaseFieldGrid {
    pub fn new(dim: (usize, usize, usize), cell_size: (f32, f32, f32)) -> Self {
        let n = dim.0 * dim.1 * dim.2;
        Self {
            phi: vec![1.0; n], // 初始完整
            grad_phi: vec![Vec3::ZERO; n],
            dim,
            cell_size,
        }
    }

    /// 网格总单元数
    pub fn cell_count(&self) -> usize {
        self.dim.0 * self.dim.1 * self.dim.2
    }

    /// 三维索引 → 线性索引
    #[inline]
    fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        i * self.dim.1 * self.dim.2 + j * self.dim.2 + k
    }

    /// 计算梯度场 (中心差分, 边界采用单侧差分)
    pub fn compute_gradients(&mut self) {
        let (nx, ny, nz) = self.dim;
        let (dx, dy, dz) = self.cell_size;
        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let idx = self.idx(i, j, k);
                    let ip = (i + 1).min(nx - 1);
                    let im = i.saturating_sub(1);
                    let jp = (j + 1).min(ny - 1);
                    let jm = j.saturating_sub(1);
                    let kp = (k + 1).min(nz - 1);
                    let km = k.saturating_sub(1);

                    let dphi_dx = (self.phi[self.idx(ip, j, k)] - self.phi[self.idx(im, j, k)])
                        / (2.0 * dx);
                    let dphi_dy = (self.phi[self.idx(i, jp, k)] - self.phi[self.idx(i, jm, k)])
                        / (2.0 * dy);
                    let dphi_dz = (self.phi[self.idx(i, j, kp)] - self.phi[self.idx(i, j, km)])
                        / (2.0 * dz);

                    self.grad_phi[idx] = Vec3::new(dphi_dx, dphi_dy, dphi_dz);
                }
            }
        }
    }

    /// 计算单元 (i,j,k) 处的拉普拉斯算子 ∇²φ (中心差分)
    #[inline]
    fn laplacian(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, ny, nz) = self.dim;
        let (dx, dy, dz) = self.cell_size;
        let idx = self.idx(i, j, k);
        let center = self.phi[idx];

        let ip = (i + 1).min(nx - 1);
        let im = i.saturating_sub(1);
        let jp = (j + 1).min(ny - 1);
        let jm = j.saturating_sub(1);
        let kp = (k + 1).min(nz - 1);
        let km = k.saturating_sub(1);

        let d2x = (self.phi[self.idx(ip, j, k)] - 2.0 * center + self.phi[self.idx(im, j, k)])
            / (dx * dx);
        let d2y = (self.phi[self.idx(i, jp, k)] - 2.0 * center + self.phi[self.idx(i, jm, k)])
            / (dy * dy);
        let d2z = (self.phi[self.idx(i, j, kp)] - 2.0 * center + self.phi[self.idx(i, j, km)])
            / (dz * dz);

        d2x + d2y + d2z
    }

    /// 显式 Euler 推进相场演化
    /// 演化方程: ∂φ/∂t = -M·∂Ψ/∂φ
    ///         = M·(2(1-φ)·Ψ_elastic - G_c·φ/l + G_c·l·∇²φ)
    /// 稳定性约束: dt < dx² / (2·M·G_c·l) (CFL-like)
    pub fn step(&mut self, model: &PhaseFieldFracture, psi_elastic: &[f32], dt: f32) {
        let (nx, ny, nz) = self.dim;
        let mut new_phi = self.phi.clone();
        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let idx = self.idx(i, j, k);
                    let phi = self.phi[idx];
                    let lap = self.laplacian(i, j, k);
                    let psi = psi_elastic.get(idx).copied().unwrap_or(0.0);
                    // ∂φ/∂t = M·(2(1-φ)·Ψ_elastic - G_c·φ/l + G_c·l·∇²φ)
                    let dphi_dt = model.mobility
                        * (2.0 * (1.0 - phi) * psi
                            - model.g_c * phi / model.length_scale
                            + model.g_c * model.length_scale * lap);
                    new_phi[idx] = (phi + dt * dphi_dt).clamp(0.0, 1.0);
                }
            }
        }
        self.phi = new_phi;
        self.compute_gradients();
    }

    /// 应用 Dirichlet 边界条件: 裂纹处 φ = 0
    pub fn apply_crack_bc(&mut self, crack_indices: &[usize]) {
        for &idx in crack_indices {
            if idx < self.phi.len() {
                self.phi[idx] = 0.0;
            }
        }
    }

    /// 应用外部载荷 (应力场驱动相场演化)
    /// stress_field 这里作为弹性能密度场 Ψ_elastic 输入 (J/m³)
    pub fn apply_external_load(&mut self, stress_field: &[f32], model: &PhaseFieldFracture, dt: f32) {
        self.step(model, stress_field, dt);
    }

    /// 计算总退化弹性能: Σ (1-φ)²·Ψ_elastic
    pub fn total_elastic_energy(&self, psi_elastic: &[f32]) -> f32 {
        self.phi
            .iter()
            .zip(psi_elastic.iter())
            .map(|(&phi, &psi)| (1.0 - phi).powi(2) * psi)
            .sum()
    }

    /// 计算总断裂能: Σ G_c·(φ²/(2l) + l|∇φ|²/2)
    pub fn total_fracture_energy(&self, model: &PhaseFieldFracture) -> f32 {
        self.phi
            .iter()
            .zip(self.grad_phi.iter())
            .map(|(&phi, &g)| model.fracture_energy_density(phi, g))
            .sum()
    }
}
