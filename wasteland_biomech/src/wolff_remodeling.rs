// Wolff 定律骨重塑
// daily mechanical stimulus: ψ_daily = Σ(ρ_i · σ_i^m) / n_loads
// 骨密度演化: dρ/dt = k·(ψ_daily - ψ_ref)
// Carter 杨氏模量: E = 3790·ρ^3 (ρ 单位 g/cm³, E 单位 MPa)
// 来源:
//   - Carter DR (1984) "Mechanical loading histories and cortical bone remodeling"
//   - Carter DR, Hayes WC (1977) Science 194:1174-1176
//   - Wolff J (1892) "Das Gesetz der Transformation der Knochen"

use serde::{Deserialize, Serialize};

/// Wolff 定律重塑模型参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WolffRemodeling {
    /// 重塑速率常数 k (day⁻¹), 范围 0.02-0.05
    pub k: f32,
    /// 参考机械刺激 ψ_ref (MPa/day), 健康骨 ≈ 0.004
    pub psi_ref: f32,
    /// Carter 应力指数 m, 典型值 3
    pub m: f32,
}

impl Default for WolffRemodeling {
    fn default() -> Self {
        Self {
            k: 0.035,   // 中位值 (0.02-0.05)
            psi_ref: 0.004,
            m: 3.0,
        }
    }
}

/// 单次载荷记录
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StressEntry {
    /// 应力幅值 σ_i (MPa)
    pub stress: f32,
    /// 该载荷的循环次数 ρ_i (load cycles)
    pub cycles: f32,
}

/// 密度上下限 (g/cm³)
pub const DENSITY_MIN: f32 = 0.05;
pub const DENSITY_MAX: f32 = 2.0;

impl WolffRemodeling {
    pub fn new() -> Self {
        Self::default()
    }

    /// 计算每日机械刺激: ψ_daily = Σ(ρ_i · σ_i^m) / n_loads
    /// Carter 1984, Eq. 2
    pub fn daily_stimulus(&self, stress_history: &[StressEntry]) -> f32 {
        if stress_history.is_empty() {
            return 0.0;
        }
        let total: f32 = stress_history
            .iter()
            .map(|e| e.cycles * e.stress.powf(self.m))
            .sum();
        total / stress_history.len() as f32
    }

    /// 单步重塑: dρ/dt = k·(ψ_daily - ψ_ref)
    /// 对每个体素独立应用, 并钳制到 [DENSITY_MIN, DENSITY_MAX]
    pub fn step(&self, density_field: &mut [f32], stress_history: &[StressEntry], dt: f32) {
        let psi_daily = self.daily_stimulus(stress_history);
        let drho = self.k * (psi_daily - self.psi_ref);
        for rho in density_field.iter_mut() {
            *rho = (*rho + dt * drho).clamp(DENSITY_MIN, DENSITY_MAX);
        }
    }

    /// Carter 杨氏模量公式: E = 3790·ρ^3 (MPa)
    /// 输入 ρ 单位 g/cm³
    /// Carter & Hayes (1977)
    pub fn youngs_modulus(density: f32) -> f32 {
        3790.0 * density.powi(3)
    }
}
