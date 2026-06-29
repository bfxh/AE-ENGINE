//! V8 物理沙盒：建筑材料属性库
//!
//! 提供建筑/结构材料的完整物理属性向量，用于热力学耦合与结构损伤模拟。
//! 涵盖密度、比热、热导率、燃点、燃烧参数、结构强度、熔点、腐蚀抗性等。
//!
//! 设计要点：
//! - 不可燃材料的 `ignition_temp` 设为 `f32::INFINITY`，`burn_rate`/`burn_energy` 为 0
//! - 不可熔材料的 `melt_temp` 设为 `f32::INFINITY`（如 Wood 碳化而非熔化、Dirt 不熔）
//! - 腐蚀抗性 `corrosion_resistance` ∈ [0, 1]，1 表示完全不腐蚀
//! - 所有数值取自工程材料手册常用估值，精度满足游戏级物理模拟

use serde::{Deserialize, Serialize};

// ─── 材料种类枚举 ─────────────────────────────────────────
/// 建筑/结构材料种类
///
/// 区别于 `CellKind`（沙盒内的物理相态：铁/水/气），`MaterialKind` 描述
/// 建筑构件的材质（墙、柱、地板等），两者在 V8 中分别承载结构属性和热力学属性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialKind {
    /// 木材：可燃，热导率低，结构强度中等
    Wood,
    /// 混凝土：不可燃，热导率低，结构强度高
    Concrete,
    /// 砖：不可燃，热导率中等，结构强度高
    Brick,
    /// 玻璃：不可燃，热导率中等，结构强度低（易碎）
    Glass,
    /// 金属（区别于 Iron）：高热导率，高结构强度
    Metal,
    /// 泥土：不可燃，热导率低，结构强度低
    Dirt,
    /// 石材：不可燃，热导率中等，结构强度高
    Stone,
}

// ─── 材料属性向量 ─────────────────────────────────────────
/// 单种材料的完整物理属性
///
/// 所有字段采用 SI 单位，便于直接接入热传导/结构/燃烧子模块。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// 密度 kg/m³
    pub density: f32,
    /// 比热容 J/(kg·K)
    pub specific_heat: f32,
    /// 热导率 W/(m·K)
    pub thermal_conductivity: f32,
    /// 燃点 K（不可燃材料设为 `f32::INFINITY`）
    pub ignition_temp: f32,
    /// 燃烧速率 kg/s（不可燃为 0）
    pub burn_rate: f32,
    /// 燃烧释放能量 J/kg（不可燃为 0）
    pub burn_energy: f32,
    /// 承压强度 Pa
    pub structural_strength: f32,
    /// 熔点 K（不熔材料设为 `f32::INFINITY`）
    pub melt_temp: f32,
    /// 腐蚀抗性 0..1（1 = 完全不腐蚀）
    pub corrosion_resistance: f32,
}

impl MaterialKind {
    /// 返回该材料的完整属性向量
    ///
    /// 数值来源：工程材料手册常用估值，精度满足游戏级物理模拟。
    pub fn properties(self) -> MaterialProperties {
        match self {
            // 木材：橡木典型值，燃点 ~590K（约 317°C），燃烧热 ~16 MJ/kg
            MaterialKind::Wood => MaterialProperties {
                density: 700.0,
                specific_heat: 1700.0,
                thermal_conductivity: 0.15,
                ignition_temp: 590.0,
                burn_rate: 0.05,
                burn_energy: 1.6e7,
                structural_strength: 40.0e6,
                melt_temp: f32::INFINITY, // 木材碳化而非熔化
                corrosion_resistance: 0.3,
            },
            // 混凝土：密度 2400，k≈1.4，承压 30MPa，熔点 ~1900K
            MaterialKind::Concrete => MaterialProperties {
                density: 2400.0,
                specific_heat: 880.0,
                thermal_conductivity: 1.4,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 30.0e6,
                melt_temp: 1900.0,
                corrosion_resistance: 0.9,
            },
            // 砖：黏土砖典型值，承压 50MPa
            MaterialKind::Brick => MaterialProperties {
                density: 1800.0,
                specific_heat: 900.0,
                thermal_conductivity: 0.8,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 50.0e6,
                melt_temp: 1800.0,
                corrosion_resistance: 0.95,
            },
            // 玻璃：钠钙玻璃，承压低（~10MPa），易碎
            MaterialKind::Glass => MaterialProperties {
                density: 2500.0,
                specific_heat: 840.0,
                thermal_conductivity: 1.0,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 10.0e6,
                melt_temp: 1700.0,
                corrosion_resistance: 0.98,
            },
            // 金属（结构钢类）：高 k、高强度、熔点 ~1800K
            MaterialKind::Metal => MaterialProperties {
                density: 7800.0,
                specific_heat: 490.0,
                thermal_conductivity: 50.0,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 250.0e6,
                melt_temp: 1800.0,
                corrosion_resistance: 0.6,
            },
            // 泥土：低强度、低 k，不熔（视为松散颗粒集合）
            MaterialKind::Dirt => MaterialProperties {
                density: 1600.0,
                specific_heat: 800.0,
                thermal_conductivity: 0.6,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 5.0e6,
                melt_temp: f32::INFINITY,
                corrosion_resistance: 0.7,
            },
            // 石材：花岗岩类，承压 80MPa，熔点 ~1500K
            MaterialKind::Stone => MaterialProperties {
                density: 2700.0,
                specific_heat: 790.0,
                thermal_conductivity: 2.5,
                ignition_temp: f32::INFINITY,
                burn_rate: 0.0,
                burn_energy: 0.0,
                structural_strength: 80.0e6,
                melt_temp: 1500.0,
                corrosion_resistance: 0.92,
            },
        }
    }

    /// 是否可燃（燃点有限即为可燃）
    pub fn is_combustible(self) -> bool {
        self.properties().ignition_temp.is_finite()
    }

    /// 考虑温度和腐蚀的有效热导率 W/(m·K)
    ///
    /// 物理模型：
    /// - 高温下材料热导率基本不变（固体热导率对温度不敏感，简化处理）
    /// - 腐蚀降低热导率：腐蚀产物（铁锈、碳化层等）热导率显著低于基体
    /// - `corrosion` 参数为外部传入的有效腐蚀度 ∈ [0, 1]，已被
    ///   `corrosion_resistance` 折算过（调用方负责：`effective_corrosion = (1 - resistance) * env_corrosion`）
    pub fn thermal_conductivity_at(self, _temperature: f32, corrosion: f32) -> f32 {
        let base = self.properties();
        let c = corrosion.clamp(0.0, 1.0);
        // 腐蚀产物通用热导率下限（铁锈 ~0.6，碳化层 ~0.1，取保守值 0.5）
        const K_CORRODED: f32 = 0.5;
        // 线性混合：基体 ↔ 腐蚀产物
        base.thermal_conductivity * (1.0 - c) + K_CORRODED * c
    }

    /// 结构完整性 0..1（高温 + 损伤降低完整性）
    ///
    /// 物理模型：
    /// - 接近熔点时强度急剧下降（用线性近似代替真实的非线性软化曲线）
    /// - `damage` 为外部累积损伤 ∈ [0, 1]（机械/热冲击造成）
    /// - 返回 0 表示完全失效，1 表示完好
    pub fn structural_integrity(self, temperature: f32, damage: f32) -> f32 {
        let base = self.properties();
        let d = damage.clamp(0.0, 1.0);

        // 温度因子：T < 0.5 * melt 时无影响，T → melt 时趋近 0
        // 不熔材料（melt_temp = INFINITY）温度因子恒为 1
        let temp_factor = if base.melt_temp.is_infinite() || base.melt_temp <= 0.0 {
            1.0
        } else {
            let ratio = (temperature / base.melt_temp).clamp(0.0, 1.0);
            // 0.5 以下无衰减，0.5..1 线性衰减到 0
            if ratio <= 0.5 {
                1.0
            } else {
                2.0 * (1.0 - ratio)
            }
        };

        // 完整性 = 温度因子 * (1 - 损伤)，下限 0
        (temp_factor * (1.0 - d)).clamp(0.0, 1.0)
    }
}

impl MaterialProperties {
    /// 便捷访问：是否可燃
    pub fn is_combustible(&self) -> bool {
        self.ignition_temp.is_finite()
    }
}

// ─── 单元测试 ─────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ─── 各材料关键属性验证 ─────────────────────────────────

    #[test]
    fn test_wood_properties() {
        let p = MaterialKind::Wood.properties();
        assert!((p.density - 700.0).abs() < 1e-3, "Wood 密度应为 700");
        assert!((p.thermal_conductivity - 0.15).abs() < 1e-3, "Wood 热导率应为 0.15");
        assert!((p.ignition_temp - 590.0).abs() < 1e-3, "Wood 燃点应为 590K");
        assert!((p.burn_energy - 1.6e7).abs() < 1.0, "Wood 燃烧热应为 1.6e7 J/kg");
        assert!(p.melt_temp.is_infinite(), "Wood 不熔（碳化）");
    }

    #[test]
    fn test_concrete_properties() {
        let p = MaterialKind::Concrete.properties();
        assert!((p.density - 2400.0).abs() < 1e-3, "Concrete 密度应为 2400");
        assert!((p.specific_heat - 880.0).abs() < 1e-3, "Concrete 比热应为 880");
        assert!(p.ignition_temp.is_infinite(), "Concrete 不可燃");
        assert!((p.structural_strength - 30.0e6).abs() < 1.0, "Concrete 承压 30MPa");
        assert!((p.melt_temp - 1900.0).abs() < 1e-3, "Concrete 熔点 1900K");
    }

    #[test]
    fn test_brick_properties() {
        let p = MaterialKind::Brick.properties();
        assert!((p.density - 1800.0).abs() < 1e-3, "Brick 密度应为 1800");
        assert!((p.thermal_conductivity - 0.8).abs() < 1e-3, "Brick 热导率应为 0.8");
        assert!((p.structural_strength - 50.0e6).abs() < 1.0, "Brick 承压 50MPa");
        assert!(p.burn_rate == 0.0, "Brick 不可燃，burn_rate=0");
    }

    #[test]
    fn test_glass_properties() {
        let p = MaterialKind::Glass.properties();
        assert!((p.density - 2500.0).abs() < 1e-3, "Glass 密度应为 2500");
        assert!((p.structural_strength - 10.0e6).abs() < 1.0, "Glass 承压 10MPa（易碎）");
        assert!((p.melt_temp - 1700.0).abs() < 1e-3, "Glass 熔点 1700K");
        assert!((p.corrosion_resistance - 0.98).abs() < 1e-3, "Glass 高腐蚀抗性");
    }

    #[test]
    fn test_metal_properties() {
        let p = MaterialKind::Metal.properties();
        assert!((p.density - 7800.0).abs() < 1e-3, "Metal 密度应为 7800");
        assert!((p.thermal_conductivity - 50.0).abs() < 1e-3, "Metal 高热导率 50");
        assert!((p.structural_strength - 250.0e6).abs() < 1.0, "Metal 高强度 250MPa");
        assert!((p.melt_temp - 1800.0).abs() < 1e-3, "Metal 熔点 1800K");
    }

    #[test]
    fn test_dirt_properties() {
        let p = MaterialKind::Dirt.properties();
        assert!((p.density - 1600.0).abs() < 1e-3, "Dirt 密度应为 1600");
        assert!((p.structural_strength - 5.0e6).abs() < 1.0, "Dirt 低强度 5MPa");
        assert!(p.melt_temp.is_infinite(), "Dirt 不熔（松散颗粒）");
        assert!(p.burn_energy == 0.0, "Dirt 不可燃");
    }

    #[test]
    fn test_stone_properties() {
        let p = MaterialKind::Stone.properties();
        assert!((p.density - 2700.0).abs() < 1e-3, "Stone 密度应为 2700");
        assert!((p.thermal_conductivity - 2.5).abs() < 1e-3, "Stone 热导率 2.5");
        assert!((p.structural_strength - 80.0e6).abs() < 1.0, "Stone 承压 80MPa");
        assert!((p.melt_temp - 1500.0).abs() < 1e-3, "Stone 熔点 1500K");
    }

    // ─── 可燃性测试 ─────────────────────────────────────────

    #[test]
    fn test_is_combustible() {
        assert!(MaterialKind::Wood.is_combustible(), "Wood 应可燃");
        assert!(!MaterialKind::Concrete.is_combustible(), "Concrete 不可燃");
        assert!(!MaterialKind::Brick.is_combustible(), "Brick 不可燃");
        assert!(!MaterialKind::Glass.is_combustible(), "Glass 不可燃");
        assert!(!MaterialKind::Metal.is_combustible(), "Metal 不可燃");
        assert!(!MaterialKind::Dirt.is_combustible(), "Dirt 不可燃");
        assert!(!MaterialKind::Stone.is_combustible(), "Stone 不可燃");
    }

    // ─── 热导率随腐蚀变化测试 ───────────────────────────────

    #[test]
    fn test_thermal_conductivity_decreases_with_corrosion() {
        let base = MaterialKind::Metal.properties().thermal_conductivity;
        // 无腐蚀：等于基体热导率
        let k0 = MaterialKind::Metal.thermal_conductivity_at(300.0, 0.0);
        assert!((k0 - base).abs() < 1e-3, "corrosion=0 应为基体热导率");

        // 完全腐蚀：等于腐蚀产物热导率（0.5）
        let k1 = MaterialKind::Metal.thermal_conductivity_at(300.0, 1.0);
        assert!((k1 - 0.5).abs() < 1e-3, "corrosion=1 应为腐蚀产物热导率 0.5");

        // 中间值应在 [0.5, base] 之间且单调下降
        let k_half = MaterialKind::Metal.thermal_conductivity_at(300.0, 0.5);
        assert!(k_half > 0.5 && k_half < base, "corrosion=0.5 应介于两者之间");
        let expected = base * 0.5 + 0.5 * 0.5;
        assert!((k_half - expected).abs() < 1e-3, "线性混合值匹配");
    }

    #[test]
    fn test_thermal_conductivity_temperature_invariant() {
        // 高温下热导率基本不变（设计简化）
        let k_cold = MaterialKind::Metal.thermal_conductivity_at(300.0, 0.0);
        let k_hot = MaterialKind::Metal.thermal_conductivity_at(1500.0, 0.0);
        assert!((k_cold - k_hot).abs() < 1e-3, "无腐蚀时热导率不随温度变化");
    }

    // ─── 结构完整性测试 ─────────────────────────────────────

    #[test]
    fn test_structural_integrity_temperature_effect() {
        // 常温、无损伤：完整性 = 1
        let i_cold = MaterialKind::Metal.structural_integrity(300.0, 0.0);
        assert!((i_cold - 1.0).abs() < 1e-3, "常温无损伤完整性=1");

        // 接近熔点（melt=1800K）：完整性趋于 0
        let i_near_melt = MaterialKind::Metal.structural_integrity(1790.0, 0.0);
        assert!(i_near_melt < 0.05, "接近熔点完整性应趋于 0: {}", i_near_melt);

        // 中间温度（0.5..1 区间线性衰减）：T=0.75*melt → temp_factor=0.5
        let i_mid = MaterialKind::Metal.structural_integrity(0.75 * 1800.0, 0.0);
        assert!((i_mid - 0.5).abs() < 1e-3, "T=0.75*melt 时完整性=0.5: {}", i_mid);
    }

    #[test]
    fn test_structural_integrity_damage_effect() {
        // 常温下损伤降低完整性
        let i0 = MaterialKind::Concrete.structural_integrity(300.0, 0.0);
        let i_half = MaterialKind::Concrete.structural_integrity(300.0, 0.5);
        let i_full = MaterialKind::Concrete.structural_integrity(300.0, 1.0);
        assert!((i0 - 1.0).abs() < 1e-3, "无损伤完整性=1");
        assert!((i_half - 0.5).abs() < 1e-3, "损伤 0.5 完整性=0.5");
        assert!((i_full - 0.0).abs() < 1e-3, "损伤 1.0 完整性=0");
    }

    #[test]
    fn test_structural_integrity_non_meltable() {
        // 不熔材料（Wood/Dirt）温度因子恒为 1，完整性只受损伤影响
        let i_hot_wood = MaterialKind::Wood.structural_integrity(5000.0, 0.0);
        assert!((i_hot_wood - 1.0).abs() < 1e-3,
            "Wood 不熔，高温无损伤时完整性仍为 1: {}", i_hot_wood);
        let i_hot_dirt = MaterialKind::Dirt.structural_integrity(5000.0, 0.3);
        assert!((i_hot_dirt - 0.7).abs() < 1e-3,
            "Dirt 不熔，仅损伤影响完整性: {}", i_hot_dirt);
    }

    // ─── 燃烧属性测试 ───────────────────────────────────────

    #[test]
    fn test_combustion_properties_wood_vs_brick() {
        let wood = MaterialKind::Wood.properties();
        let brick = MaterialKind::Brick.properties();
        // Wood 有燃烧能量和速率
        assert!(wood.burn_energy > 0.0, "Wood 有燃烧能量");
        assert!(wood.burn_rate > 0.0, "Wood 有燃烧速率");
        assert!(wood.ignition_temp.is_finite(), "Wood 燃点有限");
        // Brick 无燃烧属性
        assert!(brick.burn_energy == 0.0, "Brick 无燃烧能量");
        assert!(brick.burn_rate == 0.0, "Brick 无燃烧速率");
        assert!(brick.ignition_temp.is_infinite(), "Brick 燃点无限");
    }

    #[test]
    fn test_all_non_combustible_have_zero_burn_params() {
        // 所有不可燃材料的 burn_rate 和 burn_energy 必须为 0
        for kind in [
            MaterialKind::Concrete,
            MaterialKind::Brick,
            MaterialKind::Glass,
            MaterialKind::Metal,
            MaterialKind::Dirt,
            MaterialKind::Stone,
        ] {
            let p = kind.properties();
            assert!(!kind.is_combustible(), "{:?} 应不可燃", kind);
            assert!(p.burn_rate == 0.0, "{:?} burn_rate 应为 0", kind);
            assert!(p.burn_energy == 0.0, "{:?} burn_energy 应为 0", kind);
        }
    }

    // ─── 边界与单调性 ───────────────────────────────────────

    #[test]
    fn test_corrosion_clamp_out_of_range() {
        // 超出 [0,1] 范围应被钳制
        let base = MaterialKind::Stone.properties().thermal_conductivity;
        let k_neg = MaterialKind::Stone.thermal_conductivity_at(300.0, -1.0);
        let k_over = MaterialKind::Stone.thermal_conductivity_at(300.0, 2.0);
        assert!((k_neg - base).abs() < 1e-3, "corrosion<0 钳制为 0");
        assert!((k_over - 0.5).abs() < 1e-3, "corrosion>1 钳制为 1");
    }

    #[test]
    fn test_integrity_bounded_zero_to_one() {
        // 极端温度 + 极端损伤下，完整性仍应 ∈ [0, 1]
        for kind in [
            MaterialKind::Wood,
            MaterialKind::Concrete,
            MaterialKind::Brick,
            MaterialKind::Glass,
            MaterialKind::Metal,
            MaterialKind::Dirt,
            MaterialKind::Stone,
        ] {
            let i_low = kind.structural_integrity(0.0, 0.0);
            let i_high = kind.structural_integrity(1.0e6, 1.0);
            assert!(i_low >= 0.0 && i_low <= 1.0, "{:?} 完整性超出 [0,1]: {}", kind, i_low);
            assert!(i_high >= 0.0 && i_high <= 1.0, "{:?} 完整性超出 [0,1]: {}", kind, i_high);
        }
    }
}
