// 义肢 (Prosthetics) - 4 种骨科固定/置换装置
// ExternalFixator  : 外固定器, 刚性钢针, stiffness ~ 1000 N/mm
// LCPPlate         : 锁定加压钢板, 螺钉固定, stiffness ~ 500 N/mm
// IntramedullaryNail: 髓内钉, 中央支撑, stiffness ~ 800 N/mm
// Osseointegration : 骨整合义肢, 钛合金直接接骨, stiffness ~ 2000 N/mm
// 来源:
//   - AO Foundation (2018) Principles of Fracture Fixation
//   - Brånemark PI et al. (2001) Osseointegration: Skeletal Anchors

use serde::{Deserialize, Serialize};

/// 义肢材料
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProstheticMaterial {
    /// 钛合金 Ti6Al4V (ASTM B348)
    Ti6Al4V,
    /// 316L 不锈钢 (ASTM A240)
    StainlessSteel316L,
}

impl ProstheticMaterial {
    /// 杨氏模量 (GPa)
    pub fn youngs_modulus(&self) -> f32 {
        match self {
            Self::Ti6Al4V => 110.0,
            Self::StainlessSteel316L => 193.0,
        }
    }

    /// 屈服强度 (MPa)
    pub fn yield_strength(&self) -> f32 {
        match self {
            Self::Ti6Al4V => 880.0,
            Self::StainlessSteel316L => 170.0,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Ti6Al4V => "Ti6Al4V 钛合金",
            Self::StainlessSteel316L => "316L 不锈钢",
        }
    }
}

/// 义肢类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProstheticType {
    ExternalFixator,
    LCPPlate,
    IntramedullaryNail,
    Osseointegration,
}

/// 义肢通用接口
pub trait Prosthetic: Send + Sync {
    /// 刚度 (N/mm)
    fn stiffness(&self) -> f32;
    /// 失效载荷 (N)
    fn failure_load(&self) -> f32;
    /// 感染风险 (0.0-1.0)
    fn infection_risk(&self) -> f32;
    /// 材料
    fn material(&self) -> ProstheticMaterial;
    /// 类型
    fn prosthetic_type(&self) -> ProstheticType;
}

/// 外固定器: 刚性钢针, stiffness ~ 1000 N/mm
/// 钢针贯穿皮肤, 感染风险较高
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExternalFixator {
    pub material: ProstheticMaterial,
    pub stiffness: f32,
    pub failure_load: f32,
    pub infection_risk: f32,
}

impl Default for ExternalFixator {
    fn default() -> Self {
        Self {
            material: ProstheticMaterial::StainlessSteel316L,
            stiffness: 1000.0,
            failure_load: 4000.0,
            infection_risk: 0.15,
        }
    }
}

impl Prosthetic for ExternalFixator {
    fn stiffness(&self) -> f32 {
        self.stiffness
    }
    fn failure_load(&self) -> f32 {
        self.failure_load
    }
    fn infection_risk(&self) -> f32 {
        self.infection_risk
    }
    fn material(&self) -> ProstheticMaterial {
        self.material
    }
    fn prosthetic_type(&self) -> ProstheticType {
        ProstheticType::ExternalFixator
    }
}

/// 锁定加压钢板 (LCP): 螺钉固定, stiffness ~ 500 N/mm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LCPPlate {
    pub material: ProstheticMaterial,
    pub stiffness: f32,
    pub failure_load: f32,
    pub infection_risk: f32,
}

impl Default for LCPPlate {
    fn default() -> Self {
        Self {
            material: ProstheticMaterial::Ti6Al4V,
            stiffness: 500.0,
            failure_load: 3000.0,
            infection_risk: 0.05,
        }
    }
}

impl Prosthetic for LCPPlate {
    fn stiffness(&self) -> f32 {
        self.stiffness
    }
    fn failure_load(&self) -> f32 {
        self.failure_load
    }
    fn infection_risk(&self) -> f32 {
        self.infection_risk
    }
    fn material(&self) -> ProstheticMaterial {
        self.material
    }
    fn prosthetic_type(&self) -> ProstheticType {
        ProstheticType::LCPPlate
    }
}

/// 髓内钉: 中央支撑, stiffness ~ 800 N/mm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IntramedullaryNail {
    pub material: ProstheticMaterial,
    pub stiffness: f32,
    pub failure_load: f32,
    pub infection_risk: f32,
}

impl Default for IntramedullaryNail {
    fn default() -> Self {
        Self {
            material: ProstheticMaterial::Ti6Al4V,
            stiffness: 800.0,
            failure_load: 3500.0,
            infection_risk: 0.03,
        }
    }
}

impl Prosthetic for IntramedullaryNail {
    fn stiffness(&self) -> f32 {
        self.stiffness
    }
    fn failure_load(&self) -> f32 {
        self.failure_load
    }
    fn infection_risk(&self) -> f32 {
        self.infection_risk
    }
    fn material(&self) -> ProstheticMaterial {
        self.material
    }
    fn prosthetic_type(&self) -> ProstheticType {
        ProstheticType::IntramedullaryNail
    }
}

/// 骨整合义肢: 钛合金直接接骨, stiffness ~ 2000 N/mm
/// 经皮接口存在感染风险
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Osseointegration {
    pub material: ProstheticMaterial,
    pub stiffness: f32,
    pub failure_load: f32,
    pub infection_risk: f32,
}

impl Default for Osseointegration {
    fn default() -> Self {
        Self {
            material: ProstheticMaterial::Ti6Al4V,
            stiffness: 2000.0,
            failure_load: 5000.0,
            infection_risk: 0.08,
        }
    }
}

impl Prosthetic for Osseointegration {
    fn stiffness(&self) -> f32 {
        self.stiffness
    }
    fn failure_load(&self) -> f32 {
        self.failure_load
    }
    fn infection_risk(&self) -> f32 {
        self.infection_risk
    }
    fn material(&self) -> ProstheticMaterial {
        self.material
    }
    fn prosthetic_type(&self) -> ProstheticType {
        ProstheticType::Osseointegration
    }
}

/// 工厂函数: 根据类型创建义肢实例
pub fn create_prosthetic(t: ProstheticType) -> Box<dyn Prosthetic> {
    match t {
        ProstheticType::ExternalFixator => Box::new(ExternalFixator::default()),
        ProstheticType::LCPPlate => Box::new(LCPPlate::default()),
        ProstheticType::IntramedullaryNail => Box::new(IntramedullaryNail::default()),
        ProstheticType::Osseointegration => Box::new(Osseointegration::default()),
    }
}