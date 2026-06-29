// Prendergast 机械调控算子
// 决定组织分化方向: ψ = α·γ_oct + β·|v_fluid|
// 来源: Prendergast PJ, Huiskes R, Søballe K (1997)
//       "Biophysical stimuli on cells during tissue differentiation in vivo"

use serde::{Deserialize, Serialize};

/// Prendergast 机械调控模型参数
/// ψ = α·γ_oct + β·|v_fluid|
/// α=0.5, β=0.05 为 Prendergast 1997 原始参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PrendergastModel {
    /// 八面体剪应变权重 α
    pub alpha: f32,
    /// 流体速度权重 β
    pub beta: f32,
}

impl Default for PrendergastModel {
    fn default() -> Self {
        Self {
            alpha: 0.5,  // Prendergast 1997 原始参数
            beta: 0.05,
        }
    }
}

impl PrendergastModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// 计算机械调控参数 ψ = α·γ_oct + β·|v_fluid|
    /// Prendergast 1997, Eq. 1
    pub fn stimulus(&self, octahedral_shear: f32, fluid_velocity: f32) -> f32 {
        self.alpha * octahedral_shear + self.beta * fluid_velocity
    }

    /// 根据机械刺激分类组织分化方向
    /// 阈值来源: Prendergast 1997
    ///   ψ < 0.01        → 软骨分化 (chondrocyte)
    ///   0.01 ≤ ψ ≤ 0.05 → 直接成骨 (direct osteogenesis)
    ///   0.05 < ψ < 0.1  → 纤维组织 (fibrous tissue)
    ///   ψ ≥ 0.1         → 骨吸收 (bone resorption)
    pub fn classify(&self, octahedral_shear: f32, fluid_velocity: f32) -> TissueFate {
        let psi = self.stimulus(octahedral_shear, fluid_velocity);
        if psi < 0.01 {
            TissueFate::Cartilage
        } else if psi <= 0.05 {
            TissueFate::DirectOsteogenesis
        } else if psi < 0.1 {
            TissueFate::Fibrous
        } else {
            TissueFate::Resorption
        }
    }
}

/// 组织分化方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TissueFate {
    /// 软骨分化 (chondrocyte) - ψ < 0.01
    Cartilage,
    /// 直接成骨 (direct osteogenesis) - 0.01 ≤ ψ ≤ 0.05
    DirectOsteogenesis,
    /// 纤维组织 (fibrous tissue) - 0.05 < ψ < 0.1
    Fibrous,
    /// 骨吸收 (bone resorption) - ψ ≥ 0.1
    Resorption,
}

impl TissueFate {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cartilage => "软骨分化",
            Self::DirectOsteogenesis => "直接成骨",
            Self::Fibrous => "纤维组织",
            Self::Resorption => "骨吸收",
        }
    }
}
