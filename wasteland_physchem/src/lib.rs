//! wasteland_physchem - 物理化学守恒底层 + 原子级反应推导
//!
//! 核心理念：
//! 1. 绝对守恒 - 质量/能量/动量/电荷/角动量/原子计数/重子数/轻子数
//! 2. 原子级推导 - 从原子性质+热力学+动力学预测未知化学反应，不依赖预设数据库
//! 3. 不阉割性能 - f64 全精度，完整物理模型，不用近似偷工减料
//! 4. 可扩展性 - 支持未来所有极端情况
//!
//! 架构层次：
//! - conservation:    守恒律执行器（每个物理操作前后强制验证）
//! - elements:        118 元素完整性质数据库（NIST/IUPAC）
//! - molecules:       分子图结构（原子节点+化学键边）+ BDE 键能
//! - functional_groups: 官能团识别 + 反应位点标注
//! - thermodynamics:  Benson 基团贡献法 + 键能加和 + 相平衡
//! - kinetics:        Arrhenius + 过渡态理论 + Marcus 电子转移 + Evans-Polanyi
//! - quantum_approx:  扩展 Huckel 半经验量子 + 前线轨道理论
//! - reaction_prediction: 反应预测引擎（从原子推导未知反应）

#![allow(dead_code)]

pub mod conservation;
pub mod elements;
pub mod molecules;
pub mod functional_groups;
pub mod thermodynamics;
pub mod kinetics;
pub mod quantum_approx;
pub mod reaction_prediction;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::conservation::*;
    pub use crate::elements::*;
    pub use crate::molecules::*;
    pub use crate::functional_groups::*;
    pub use crate::thermodynamics::*;
    pub use crate::kinetics::*;
    pub use crate::quantum_approx::*;
    pub use crate::reaction_prediction::*;
}