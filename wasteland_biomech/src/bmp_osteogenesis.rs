// BMP 驱动骨生成 (Osteogenesis)
// Hill 方程: k_osteo = k_max·c_BMP^n / (K_d^n + c_BMP^n)
// 骨形成速率: dρ/dt = k_osteo · ρ_max
// 内源性 BMP 衰减: dc_BMP/dt = -k_decay·c_BMP
// 来源:
//   - Urist MR (1965) Science 150:893-899 (BMP 发现)
//   - Wang EA et al. (1990) PNAS 87:2220-2224 (BMP-2 骨诱导)
//   - Hill AV (1910) J Physiol 40:iv-vii (Hill 方程)

use serde::{Deserialize, Serialize};

/// BMP 驱动骨生成模型参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BmpOsteogenesis {
    /// 最大骨形成速率 k_max (day⁻¹), 0.5
    pub k_max: f32,
    /// Hill 系数 n (协同性), 2.0
    pub n: f32,
    /// BMP 解离常数 K_d (nM), 5.0
    pub k_d: f32,
    /// 内源性 BMP 衰减率 k_decay (day⁻¹), 0.1
    pub k_decay: f32,
}

impl Default for BmpOsteogenesis {
    fn default() -> Self {
        Self {
            k_max: 0.5,
            n: 2.0,
            k_d: 5.0,
            k_decay: 0.1,
        }
    }
}

/// 骨密度上下限 (g/cm³)
pub const RHO_MAX: f32 = 2.0;
pub const RHO_MIN: f32 = 0.05;

impl BmpOsteogenesis {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hill 方程: k_osteo = k_max·c_BMP^n / (K_d^n + c_BMP^n)
    /// Hill 1910
    pub fn hill_function(&self, c_bmp: f32) -> f32 {
        let c_n = c_bmp.powf(self.n);
        let k_n = self.k_d.powf(self.n);
        self.k_max * c_n / (k_n + c_n)
    }

    /// 单步推进:
    ///   dρ/dt = k_osteo · ρ_max           (骨形成)
    ///   dc_BMP/dt = -k_decay·c_BMP         (BMP 衰减)
    pub fn step(&self, density_field: &mut [f32], bmp_field: &mut [f32], dt: f32) {
        let rho_max = RHO_MAX;
        for (rho, c_bmp) in density_field.iter_mut().zip(bmp_field.iter_mut()) {
            let k_osteo = self.hill_function(*c_bmp);
            // 骨形成
            *rho = (*rho + dt * k_osteo * rho_max).clamp(RHO_MIN, RHO_MAX);
            // BMP 衰减 (解析解: c(t+dt) = c(t)·exp(-k_decay·dt))
            *c_bmp *= (-self.k_decay * dt).exp();
        }
    }

    /// 在指定位置注入 BMP (局部峰值)
    pub fn inject_bmp(bmp_field: &mut [f32], indices: &[usize], amount: f32) {
        for &idx in indices {
            if idx < bmp_field.len() {
                bmp_field[idx] += amount;
            }
        }
    }
}
