// 材料属性 - 骨/金属生物力学材料
// 数据来源:
//   - Cowin SC (2001) Bone Mechanics Handbook
//   - Reilly DT, Burstein AH (1975) J Biomech 8:393-405
//   - Goldstein SA (1987) J Biomech 20:1055-1061
//   - ASM Handbook Vol 2: Properties and Selection

use serde::{Deserialize, Serialize};

/// 通用力学材料属性
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// 杨氏模量 E (GPa)
    pub youngs_modulus: f32,
    /// 屈服强度 σ_yield (MPa)
    pub yield_strength: f32,
    /// 屈服应变 ε_yield (无量纲)
    pub yield_strain: f32,
    /// 泊松比 ν
    pub poisson_ratio: f32,
    /// 密度 ρ (g/cm³)
    pub density: f32,
}

/// 皮质骨 (Cortical Bone)
/// E = 12.2-20.5 GPa, σ_yield = 100-150 MPa, ε_yield ≈ 0.77-0.87%
/// Reilly & Burstein (1975)
pub fn cortical_bone() -> MaterialProperties {
    MaterialProperties {
        youngs_modulus: 17.0,  // GPa, 中位值 (12.2-20.5)
        yield_strength: 125.0, // MPa, 中位值 (100-150)
        yield_strain: 0.0082,  // 0.77-0.87% 中位
        poisson_ratio: 0.3,
        density: 1.85,         // g/cm³
    }
}

/// 松质骨 (Trabecular/Cancellous Bone)
/// E = 0.1-2.0 GPa, σ_yield = 5-15 MPa
/// Goldstein (1987)
pub fn trabecular_bone() -> MaterialProperties {
    MaterialProperties {
        youngs_modulus: 1.0,  // GPa, 中位值 (0.1-2.0)
        yield_strength: 10.0, // MPa, 中位值 (5-15)
        yield_strain: 0.01,   // 屈服应变较高 (~1%)
        poisson_ratio: 0.3,
        density: 0.3,         // g/cm³
    }
}

/// Ti6Al4V 钛合金 (ASTM B348)
/// E = 110 GPa, σ_yield = 880 MPa
pub fn ti6al4v() -> MaterialProperties {
    MaterialProperties {
        youngs_modulus: 110.0, // GPa
        yield_strength: 880.0, // MPa
        yield_strain: 0.008,   // σ/E = 880/110000 ≈ 0.008
        poisson_ratio: 0.34,
        density: 4.43,         // g/cm³
    }
}

/// 316L 不锈钢 (ASTM A240)
/// E = 193 GPa, σ_yield = 170-750 MPa (退火态 170, 冷加工态 750)
pub fn stainless_steel_316l() -> MaterialProperties {
    MaterialProperties {
        youngs_modulus: 193.0, // GPa
        yield_strength: 170.0, // MPa, 退火态下限
        yield_strain: 0.00088, // σ/E = 170/193000 ≈ 0.00088
        poisson_ratio: 0.3,
        density: 8.0,          // g/cm³
    }
}

// ===== 便捷函数 =====

pub fn cortical_bone_E() -> f32 {
    cortical_bone().youngs_modulus
}

pub fn cortical_bone_yield() -> f32 {
    cortical_bone().yield_strength
}

pub fn trabecular_bone_E() -> f32 {
    trabecular_bone().youngs_modulus
}

pub fn trabecular_bone_yield() -> f32 {
    trabecular_bone().yield_strength
}

pub fn ti6al4v_E() -> f32 {
    ti6al4v().youngs_modulus
}

pub fn ti6al4v_yield() -> f32 {
    ti6al4v().yield_strength
}

pub fn stainless_steel_316l_E() -> f32 {
    stainless_steel_316l().youngs_modulus
}

pub fn stainless_steel_316l_yield() -> f32 {
    stainless_steel_316l().yield_strength
}
