//! V8 沙盒：化学反应网络模块
//!
//! 多组分化学反应引擎，支持气体混合物中的燃烧反应和固体燃料（木材/炭）的氧化。
//!
//! 设计要点：
//! - 7 种气体组分（O2/CO2/H2O_vapor/CO/CH4/H2S/N2）在 `ChemicalMixture` 中追踪
//! - 固体燃料（Wood/Charcoal/Ash）通过 `SolidFuel` 枚举单独追踪，不进入气体混合物
//! - Arrhenius 速率方程 k = A·exp(-Ea/(RT)) 控制反应速率
//! - 反应放热按第一反应物质量归一化（J/kg）
//! - 步进采用伪一级动力学：每步消耗分数 = 1 - exp(-k·dt)
//! - 固体产物（灰烬/炭）在简化模型中不追踪，仅气体产物进入混合物

use serde::{Deserialize, Serialize};

use crate::R_GAS;

// ─── 化学物质种类 ──────────────────────────────────────────

/// 化学物质种类枚举
///
/// 气体组分（O2..N2）在 `ChemicalMixture` 中追踪；
/// 固体组分（Ash/Charcoal/Wood）不进入气体混合物，
/// 其中 Wood 由 `SolidFuel` 在 `step()` 中单独处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemicalSpecies {
    O2,       // 氧气
    CO2,      // 二氧化碳
    H2O,      // 水蒸气（区别于 Cell 中的液态水）
    CO,       // 一氧化碳（不完全燃烧）
    CH4,      // 甲烷（腐烂产物）
    H2S,      // 硫化氢（蛋白质腐烂产物）
    N2,       // 氮气（空气主要成分，惰性）
    Ash,      // 灰烬（固体燃烧残余）
    Charcoal, // 炭（木材不完全燃烧残余）
    Wood,     // 木材（固体燃料，仅用于反应定义）
}

/// 各物质的摩尔质量 kg/mol
fn molar_mass_of(species: ChemicalSpecies) -> f32 {
    match species {
        ChemicalSpecies::O2 => 0.032,
        ChemicalSpecies::CO2 => 0.044,
        ChemicalSpecies::H2O => 0.018,
        ChemicalSpecies::CO => 0.028,
        ChemicalSpecies::CH4 => 0.016,
        ChemicalSpecies::H2S => 0.034,
        ChemicalSpecies::N2 => 0.028,
        ChemicalSpecies::Ash => 0.060,    // 矿物混合物近似
        ChemicalSpecies::Charcoal => 0.012, // 碳近似
        ChemicalSpecies::Wood => 0.030,   // 木材平均（C6H9O4 等）
    }
}

// ─── 化学组分混合物 ────────────────────────────────────────

/// 化学组分混合物（用于气体 cell）
///
/// 仅追踪 7 种气体组分的质量（kg）。固体产物（灰烬/炭/木材）不在此结构中，
/// 由 `SolidFuel` 在反应步进时单独管理。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChemicalMixture {
    pub o2: f32,        // kg 氧气
    pub co2: f32,       // kg 二氧化碳
    pub h2o_vapor: f32, // kg 水蒸气
    pub co: f32,        // kg 一氧化碳
    pub ch4: f32,       // kg 甲烷
    pub h2s: f32,       // kg 硫化氢
    pub n2: f32,        // kg 氮气
}

impl ChemicalMixture {
    /// 混合物总质量 kg
    pub fn total_mass(&self) -> f32 {
        self.o2 + self.co2 + self.h2o_vapor + self.co + self.ch4 + self.h2s + self.n2
    }

    /// 某组分质量分数（固体组分返回 0）
    pub fn mass_fraction(&self, species: ChemicalSpecies) -> f32 {
        let total = self.total_mass();
        if total <= 0.0 {
            return 0.0;
        }
        let mass = self.species_mass(species);
        mass / total
    }

    /// 向混合物添加某组分（固体组分为无操作）
    pub fn add(&mut self, species: ChemicalSpecies, mass: f32) {
        if mass <= 0.0 {
            return;
        }
        match species {
            ChemicalSpecies::O2 => self.o2 += mass,
            ChemicalSpecies::CO2 => self.co2 += mass,
            ChemicalSpecies::H2O => self.h2o_vapor += mass,
            ChemicalSpecies::CO => self.co += mass,
            ChemicalSpecies::CH4 => self.ch4 += mass,
            ChemicalSpecies::H2S => self.h2s += mass,
            ChemicalSpecies::N2 => self.n2 += mass,
            // 固体不加入气体混合物
            ChemicalSpecies::Ash | ChemicalSpecies::Charcoal | ChemicalSpecies::Wood => {}
        }
    }

    /// 从混合物移除某组分，返回实际移除量（不超过现有；固体返回 0）
    pub fn remove(&mut self, species: ChemicalSpecies, mass: f32) -> f32 {
        if mass <= 0.0 {
            return 0.0;
        }
        let current = self.species_mass(species);
        let removed = mass.min(current);
        match species {
            ChemicalSpecies::O2 => self.o2 -= removed,
            ChemicalSpecies::CO2 => self.co2 -= removed,
            ChemicalSpecies::H2O => self.h2o_vapor -= removed,
            ChemicalSpecies::CO => self.co -= removed,
            ChemicalSpecies::CH4 => self.ch4 -= removed,
            ChemicalSpecies::H2S => self.h2s -= removed,
            ChemicalSpecies::N2 => self.n2 -= removed,
            ChemicalSpecies::Ash | ChemicalSpecies::Charcoal | ChemicalSpecies::Wood => {}
        }
        removed
    }

    /// 混合物平均摩尔质量 kg/mol（按质量加权）
    ///
    /// M_avg = Σ(m_i) / Σ(m_i / M_i)
    pub fn molar_mass(&self) -> f32 {
        let total = self.total_mass();
        if total <= 0.0 {
            return 0.0;
        }
        let total_moles = self.o2 / molar_mass_of(ChemicalSpecies::O2)
            + self.co2 / molar_mass_of(ChemicalSpecies::CO2)
            + self.h2o_vapor / molar_mass_of(ChemicalSpecies::H2O)
            + self.co / molar_mass_of(ChemicalSpecies::CO)
            + self.ch4 / molar_mass_of(ChemicalSpecies::CH4)
            + self.h2s / molar_mass_of(ChemicalSpecies::H2S)
            + self.n2 / molar_mass_of(ChemicalSpecies::N2);
        if total_moles <= 0.0 {
            return 0.0;
        }
        total / total_moles
    }

    /// 读取某组分的质量（内部用，固体返回 0）
    fn species_mass(&self, species: ChemicalSpecies) -> f32 {
        match species {
            ChemicalSpecies::O2 => self.o2,
            ChemicalSpecies::CO2 => self.co2,
            ChemicalSpecies::H2O => self.h2o_vapor,
            ChemicalSpecies::CO => self.co,
            ChemicalSpecies::CH4 => self.ch4,
            ChemicalSpecies::H2S => self.h2s,
            ChemicalSpecies::N2 => self.n2,
            ChemicalSpecies::Ash | ChemicalSpecies::Charcoal | ChemicalSpecies::Wood => 0.0,
        }
    }
}

// ─── 固体燃料类型 ──────────────────────────────────────────

/// 固体燃料类型（反应步进时标识当前 cell 的固体燃料）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidFuel {
    None,     // 无固体燃料
    Wood,     // 木材
    Charcoal, // 炭
}

// ─── 化学反应定义 ──────────────────────────────────────────

/// 化学反应定义
///
/// `heat_release` 按第一反应物质量归一化（J/kg_first_reactant），
/// 即总放热 = (extent × first_stoich) × heat_release。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalReaction {
    pub name: &'static str,
    pub reactants: Vec<(ChemicalSpecies, f32)>, // 反应物 (种类, 化学计量数 kg)
    pub products: Vec<(ChemicalSpecies, f32)>,  // 产物 (种类, 化学计量数 kg)
    pub activation_energy: f32, // J/mol 活化能
    pub pre_exponential: f32,   // 1/s 指前因子
    pub heat_release: f32,      // J/kg_reaction 放热(正)/吸热(负)，按第一反应物归一
    pub min_temp: f32,          // K 最低反应温度
}

// ─── 反应数据库 ────────────────────────────────────────────

/// 预定义化学反应数据库
///
/// 化学计量数以 kg 表示，相对于 1 kg 第一反应物归一。
/// 质量平衡验证（忽略固体产物不计入气体）：
/// - 完全燃烧：1.0 + 1.42 = 2.42 → 1.83 + 0.55 + 0.04(ash) = 2.42 ✓
/// - 不完全燃烧：1.0 + 0.7 = 1.7 → 1.0 + 0.4 + 0.25(charcoal) + 0.05(ash) = 1.7 ✓
/// - CO 燃烧：1.0 + 0.571 = 1.571 → 1.571 ✓
/// - CH4 燃烧：1.0 + 4.0 = 5.0 → 2.75 + 2.25 = 5.0 ✓
/// - H2S 燃烧：1.0 + 1.41 = 2.41 → 0.53 + (SO2 不追踪) ≈ 0.53（简化）
pub fn reaction_database() -> Vec<ChemicalReaction> {
    vec![
        // 1. 木材完全燃烧：Wood + O2 → CO2 + H2O + Ash
        // 放热 16 MJ/kg_wood，点燃温度 ~590K（木材着火点）
        ChemicalReaction {
            name: "wood_complete_combustion",
            reactants: vec![
                (ChemicalSpecies::Wood, 1.0),
                (ChemicalSpecies::O2, 1.42),
            ],
            products: vec![
                (ChemicalSpecies::CO2, 1.83),
                (ChemicalSpecies::H2O, 0.55),
                (ChemicalSpecies::Ash, 0.04),
            ],
            activation_energy: 80_000.0,
            pre_exponential: 1.0e8,
            heat_release: 1.6e7,
            min_temp: 590.0,
        },
        // 2. 木材不完全燃烧：Wood + O2(少量) → CO + H2O + Charcoal + Ash
        // 放热较低 10 MJ/kg（CO 和炭仍含化学能未释放）
        ChemicalReaction {
            name: "wood_incomplete_combustion",
            reactants: vec![
                (ChemicalSpecies::Wood, 1.0),
                (ChemicalSpecies::O2, 0.7),
            ],
            products: vec![
                (ChemicalSpecies::CO, 1.0),
                (ChemicalSpecies::H2O, 0.4),
                (ChemicalSpecies::Charcoal, 0.25),
                (ChemicalSpecies::Ash, 0.05),
            ],
            activation_energy: 100_000.0,
            pre_exponential: 1.0e7,
            heat_release: 1.0e7,
            min_temp: 590.0,
        },
        // 3. CO 燃烧：CO + O2 → CO2
        // 2CO + O2 → 2CO2，按 kg 归一：1.0 kg CO + 0.571 kg O2 → 1.571 kg CO2
        ChemicalReaction {
            name: "co_combustion",
            reactants: vec![
                (ChemicalSpecies::CO, 1.0),
                (ChemicalSpecies::O2, 0.571),
            ],
            products: vec![
                (ChemicalSpecies::CO2, 1.571),
            ],
            activation_energy: 120_000.0,
            pre_exponential: 1.0e9,
            heat_release: 1.0e7,
            min_temp: 700.0,
        },
        // 4. 甲烷燃烧：CH4 + O2 → CO2 + H2O
        // CH4 + 2O2 → CO2 + 2H2O，按 kg 归一：1.0 kg CH4 + 4.0 kg O2 → 2.75 kg CO2 + 2.25 kg H2O
        ChemicalReaction {
            name: "ch4_combustion",
            reactants: vec![
                (ChemicalSpecies::CH4, 1.0),
                (ChemicalSpecies::O2, 4.0),
            ],
            products: vec![
                (ChemicalSpecies::CO2, 2.75),
                (ChemicalSpecies::H2O, 2.25),
            ],
            activation_energy: 150_000.0,
            pre_exponential: 1.0e10,
            heat_release: 5.0e7,
            min_temp: 800.0,
        },
        // 5. H2S 燃烧：H2S + O2 → H2O_vapor + (SO2 忽略)
        // 2H2S + 3O2 → 2H2O + 2SO2，简化忽略 SO2（模型不追踪硫氧化物）
        ChemicalReaction {
            name: "h2s_combustion",
            reactants: vec![
                (ChemicalSpecies::H2S, 1.0),
                (ChemicalSpecies::O2, 1.41),
            ],
            products: vec![
                (ChemicalSpecies::H2O, 0.53),
            ],
            activation_energy: 130_000.0,
            pre_exponential: 1.0e8,
            heat_release: 2.0e7,
            min_temp: 600.0,
        },
    ]
}

// ─── 化学反应引擎 ──────────────────────────────────────────

/// 化学反应引擎
pub struct ChemistryEngine {
    pub reactions: Vec<ChemicalReaction>,
}

impl ChemistryEngine {
    pub fn new() -> Self {
        Self {
            reactions: reaction_database(),
        }
    }

    /// 计算给定温度下某反应的速率常数 k = A * exp(-Ea/(R*T))
    ///
    /// 温度低于 min_temp 时返回 0（反应不发生）
    pub fn reaction_rate(&self, reaction: &ChemicalReaction, temperature: f32) -> f32 {
        if temperature < reaction.min_temp {
            return 0.0;
        }
        reaction.pre_exponential * (-reaction.activation_energy / (R_GAS * temperature)).exp()
    }

    /// 推进一步化学反应，修改 mixture 和 solid_fuel_mass，返回释放的能量 J
    ///
    /// 速率受限于最小反应物量和 Arrhenius 速率常数。
    /// 多个反应按顺序执行，前一个反应的产物可被后续反应消耗（如 CO 再燃烧）。
    pub fn step(
        &self,
        mixture: &mut ChemicalMixture,
        solid_fuel_mass: &mut f32,
        solid_fuel_type: SolidFuel,
        temperature: f32,
        dt: f32,
    ) -> f32 {
        let mut total_heat = 0.0f32;

        for reaction in &self.reactions {
            // 检查反应是否适用于当前固体燃料类型
            if !reaction_applies(reaction, solid_fuel_type) {
                continue;
            }

            // Arrhenius 速率常数
            let k = self.reaction_rate(reaction, temperature);
            if k <= 0.0 {
                continue;
            }

            // 伪一级动力学：本步消耗分数 = 1 - exp(-k·dt)
            let frac = 1.0 - (-k * dt).exp();
            if frac <= 0.0 {
                continue;
            }

            // 第一反应物的可用量和化学计量数
            let (first_species, first_stoich) = reaction.reactants[0];
            let first_available =
                species_mass_for(mixture, *solid_fuel_mass, solid_fuel_type, first_species);
            if first_available <= 0.0 || first_stoich <= 0.0 {
                continue;
            }

            // 基于速率的反应进度（batch 单位）
            let mut extent = frac * first_available / first_stoich;

            // 检查其他反应物是否限制进度
            for &(species, stoich) in &reaction.reactants[1..] {
                if stoich <= 0.0 {
                    continue;
                }
                let available =
                    species_mass_for(mixture, *solid_fuel_mass, solid_fuel_type, species);
                extent = extent.min(available / stoich);
            }

            if extent <= 0.0 {
                continue;
            }

            // 移除反应物
            for &(species, stoich) in &reaction.reactants {
                let consumed = extent * stoich;
                remove_species_mass(
                    mixture,
                    solid_fuel_mass,
                    solid_fuel_type,
                    species,
                    consumed,
                );
            }

            // 添加产物（气体进入混合物，固体产物在简化模型中丢弃）
            for &(species, stoich) in &reaction.products {
                let produced = extent * stoich;
                mixture.add(species, produced);
            }

            // 累计放热（按第一反应物质量归一）
            let first_consumed = extent * first_stoich;
            total_heat += first_consumed * reaction.heat_release;
        }

        total_heat
    }
}

impl Default for ChemistryEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 内部辅助函数 ──────────────────────────────────────────

/// 检查反应是否适用于当前固体燃料类型
///
/// 含 Wood 反应物的反应仅在 solid_fuel_type == Wood 时适用；
/// 含 Charcoal 反应物的反应仅在 solid_fuel_type == Charcoal 时适用；
/// 纯气体反应始终适用。
fn reaction_applies(reaction: &ChemicalReaction, solid_fuel_type: SolidFuel) -> bool {
    for &(species, _) in &reaction.reactants {
        match species {
            ChemicalSpecies::Wood => {
                if solid_fuel_type != SolidFuel::Wood {
                    return false;
                }
            }
            ChemicalSpecies::Charcoal => {
                if solid_fuel_type != SolidFuel::Charcoal {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// 读取某组分的可用质量（气体从 mixture，固体燃料从 solid_fuel_mass）
fn species_mass_for(
    mixture: &ChemicalMixture,
    solid_fuel_mass: f32,
    solid_fuel_type: SolidFuel,
    species: ChemicalSpecies,
) -> f32 {
    match species {
        ChemicalSpecies::Wood => {
            if solid_fuel_type == SolidFuel::Wood {
                solid_fuel_mass
            } else {
                0.0
            }
        }
        ChemicalSpecies::Charcoal => {
            if solid_fuel_type == SolidFuel::Charcoal {
                solid_fuel_mass
            } else {
                0.0
            }
        }
        ChemicalSpecies::Ash => 0.0, // Ash 从不作为反应物
        // 气体组分从混合物读取
        _ => mixture.species_mass(species),
    }
}

/// 移除反应物质量（气体从 mixture，固体燃料从 solid_fuel_mass）
fn remove_species_mass(
    mixture: &mut ChemicalMixture,
    solid_fuel_mass: &mut f32,
    solid_fuel_type: SolidFuel,
    species: ChemicalSpecies,
    amount: f32,
) {
    if amount <= 0.0 {
        return;
    }
    match species {
        ChemicalSpecies::Wood => {
            if solid_fuel_type == SolidFuel::Wood {
                *solid_fuel_mass = (*solid_fuel_mass - amount).max(0.0);
            }
        }
        ChemicalSpecies::Charcoal => {
            if solid_fuel_type == SolidFuel::Charcoal {
                *solid_fuel_mass = (*solid_fuel_mass - amount).max(0.0);
            }
        }
        ChemicalSpecies::Ash => {} // Ash 不作为反应物移除
        // 气体组分从混合物移除
        _ => {
            mixture.remove(species, amount);
        }
    }
}

// ─── 单元测试 ──────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── ChemicalMixture 基础测试 ──

    #[test]
    fn test_mixture_total_mass() {
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 0.5);
        m.add(ChemicalSpecies::CO2, 0.3);
        m.add(ChemicalSpecies::N2, 0.2);
        assert!((m.total_mass() - 1.0).abs() < 1e-6, "总质量应为 1.0");
    }

    #[test]
    fn test_mixture_mass_fraction() {
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 0.4);
        m.add(ChemicalSpecies::N2, 0.6);
        assert!((m.mass_fraction(ChemicalSpecies::O2) - 0.4).abs() < 1e-6, "O2 质量分数 0.4");
        assert!((m.mass_fraction(ChemicalSpecies::N2) - 0.6).abs() < 1e-6, "N2 质量分数 0.6");
        assert!((m.mass_fraction(ChemicalSpecies::CO2) - 0.0).abs() < 1e-6, "CO2 质量分数 0");
        // 固体组分质量分数为 0
        assert!((m.mass_fraction(ChemicalSpecies::Ash) - 0.0).abs() < 1e-6, "Ash 质量分数 0");
    }

    #[test]
    fn test_mixture_add() {
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 0.3);
        m.add(ChemicalSpecies::H2O, 0.1);
        assert!((m.o2 - 0.3).abs() < 1e-6, "O2 字段应为 0.3");
        assert!((m.h2o_vapor - 0.1).abs() < 1e-6, "H2O_vapor 字段应为 0.1");
        // 固体添加无效果
        m.add(ChemicalSpecies::Ash, 0.5);
        assert!((m.total_mass() - 0.4).abs() < 1e-6, "Ash 不应进入混合物");
        // 负质量添加无效果
        m.add(ChemicalSpecies::O2, -0.1);
        assert!((m.o2 - 0.3).abs() < 1e-6, "负质量添加无效");
    }

    #[test]
    fn test_mixture_remove() {
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 0.5);
        let removed = m.remove(ChemicalSpecies::O2, 0.3);
        assert!((removed - 0.3).abs() < 1e-6, "实际移除 0.3");
        assert!((m.o2 - 0.2).abs() < 1e-6, "剩余 0.2");
    }

    #[test]
    fn test_mixture_remove_more_than_available() {
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::CO, 0.2);
        let removed = m.remove(ChemicalSpecies::CO, 0.5);
        assert!((removed - 0.2).abs() < 1e-6, "移除量不超过现有：实际移除 0.2");
        assert!((m.co - 0.0).abs() < 1e-6, "CO 应为 0");
        // 固体移除返回 0
        let solid_removed = m.remove(ChemicalSpecies::Wood, 0.5);
        assert!((solid_removed - 0.0).abs() < 1e-6, "固体移除返回 0");
    }

    #[test]
    fn test_mixture_molar_mass() {
        // 0.032 kg O2 + 0.044 kg CO2
        // moles = 1.0 + 1.0 = 2.0
        // M_avg = 0.076 / 2.0 = 0.038 kg/mol
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 0.032);
        m.add(ChemicalSpecies::CO2, 0.044);
        let m_avg = m.molar_mass();
        assert!((m_avg - 0.038).abs() < 1e-5, "平均摩尔质量应为 0.038，实际 {}", m_avg);
    }

    #[test]
    fn test_mixture_molar_mass_pure_gas() {
        // 纯 O2：M = 0.032 kg/mol
        let mut m = ChemicalMixture::default();
        m.add(ChemicalSpecies::O2, 1.0);
        assert!((m.molar_mass() - 0.032).abs() < 1e-6, "纯 O2 摩尔质量 0.032");
        // 纯 N2：M = 0.028 kg/mol
        let mut m2 = ChemicalMixture::default();
        m2.add(ChemicalSpecies::N2, 1.0);
        assert!((m2.molar_mass() - 0.028).abs() < 1e-6, "纯 N2 摩尔质量 0.028");
    }

    // ── 反应速率测试 ──

    #[test]
    fn test_reaction_rate_exponential_growth() {
        let engine = ChemistryEngine::new();
        // CO 燃烧反应（min_temp=700K）
        let co_rxn = engine
            .reactions
            .iter()
            .find(|r| r.name == "co_combustion")
            .unwrap();
        let k_800 = engine.reaction_rate(co_rxn, 800.0);
        let k_1000 = engine.reaction_rate(co_rxn, 1000.0);
        assert!(k_1000 > k_800, "速率应随温度升高而增大: k(800)={} k(1000)={}", k_800, k_1000);
        assert!(k_800 > 0.0, "高于 min_temp 时速率 > 0");
    }

    #[test]
    fn test_reaction_rate_below_min_temp() {
        let engine = ChemistryEngine::new();
        let co_rxn = engine
            .reactions
            .iter()
            .find(|r| r.name == "co_combustion")
            .unwrap();
        // 低于 min_temp=700K 时速率应为 0
        let k = engine.reaction_rate(co_rxn, 600.0);
        assert!((k - 0.0).abs() < 1e-6, "低于 min_temp 时速率 = 0");
    }

    // ── 木材燃烧测试 ──

    #[test]
    fn test_wood_combustion_consumes_o2_produces_co2() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::O2, 2.0); // 充足氧气
        let mut wood = 1.0f32;
        let o2_before = mixture.o2;
        let co2_before = mixture.co2;

        // T=600K 略高于 min_temp=590K，dt=0.01s
        let heat = engine.step(&mut mixture, &mut wood, SolidFuel::Wood, 600.0, 0.01);

        assert!(mixture.o2 < o2_before, "O2 应减少: {} -> {}", o2_before, mixture.o2);
        assert!(mixture.co2 > co2_before, "CO2 应增加: {} -> {}", co2_before, mixture.co2);
        assert!(wood < 1.0, "木材应减少: {}", wood);
        assert!(heat > 0.0, "燃烧放热 > 0: heat={}", heat);
    }

    #[test]
    fn test_combustion_releases_energy() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::O2, 5.0);
        let mut wood = 2.0f32;

        let heat = engine.step(&mut mixture, &mut wood, SolidFuel::Wood, 800.0, 0.01);

        // 800K 时完全燃烧反应：1 kg wood → 1.6e7 J
        // 至少应释放显著能量
        assert!(heat > 1.0e6, "应释放大量能量: heat={}", heat);
    }

    #[test]
    fn test_no_combustion_without_o2() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default(); // 无 O2
        let mut wood = 1.0f32;

        let heat = engine.step(&mut mixture, &mut wood, SolidFuel::Wood, 800.0, 0.01);

        assert!((heat - 0.0).abs() < 1e-3, "缺氧时不应放热: heat={}", heat);
        assert!((wood - 1.0).abs() < 1e-6, "缺氧时木材不消耗: wood={}", wood);
    }

    #[test]
    fn test_no_combustion_below_ignition_temp() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::O2, 2.0);
        let mut wood = 1.0f32;

        // 低于点燃温度 590K
        let heat = engine.step(&mut mixture, &mut wood, SolidFuel::Wood, 400.0, 0.01);

        assert!((heat - 0.0).abs() < 1e-3, "低于点燃温度不反应: heat={}", heat);
        assert!((wood - 1.0).abs() < 1e-6, "木材不消耗: wood={}", wood);
    }

    // ── CO 燃烧测试 ──

    #[test]
    fn test_co_combustion() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::CO, 0.1);
        mixture.add(ChemicalSpecies::O2, 0.1);
        let co_before = mixture.co;
        let co2_before = mixture.co2;

        let heat = engine.step(&mut mixture, &mut f32::default(), SolidFuel::None, 800.0, 0.01);

        assert!(mixture.co < co_before, "CO 应减少: {} -> {}", co_before, mixture.co);
        assert!(mixture.co2 > co2_before, "CO2 应增加: {} -> {}", co2_before, mixture.co2);
        assert!(heat > 0.0, "CO 燃烧放热 > 0: heat={}", heat);
    }

    // ── 甲烷燃烧测试 ──

    #[test]
    fn test_ch4_combustion() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::CH4, 0.1);
        mixture.add(ChemicalSpecies::O2, 0.5);
        let ch4_before = mixture.ch4;
        let h2o_before = mixture.h2o_vapor;

        let heat = engine.step(&mut mixture, &mut f32::default(), SolidFuel::None, 900.0, 0.01);

        assert!(mixture.ch4 < ch4_before, "CH4 应减少: {} -> {}", ch4_before, mixture.ch4);
        assert!(mixture.h2o_vapor > h2o_before, "H2O 应增加: {} -> {}", h2o_before, mixture.h2o_vapor);
        assert!(mixture.co2 > 0.0, "应产生 CO2");
        assert!(heat > 0.0, "CH4 燃烧放热 > 0: heat={}", heat);
    }

    // ── 多反应共存测试 ──

    #[test]
    fn test_multiple_reactions_coexist() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::CO, 0.1);
        mixture.add(ChemicalSpecies::CH4, 0.1);
        mixture.add(ChemicalSpecies::O2, 1.0); // 充足氧气
        let co_before = mixture.co;
        let ch4_before = mixture.ch4;

        let heat = engine.step(&mut mixture, &mut f32::default(), SolidFuel::None, 900.0, 0.01);

        assert!(mixture.co < co_before, "CO 应减少（被燃烧）");
        assert!(mixture.ch4 < ch4_before, "CH4 应减少（被燃烧）");
        assert!(mixture.co2 > 0.0, "两种燃烧都产生 CO2");
        assert!(heat > 0.0, "总放热 > 0: heat={}", heat);
    }

    // ── H2S 燃烧测试 ──

    #[test]
    fn test_h2s_combustion() {
        let engine = ChemistryEngine::new();
        let mut mixture = ChemicalMixture::default();
        mixture.add(ChemicalSpecies::H2S, 0.1);
        mixture.add(ChemicalSpecies::O2, 0.5);
        let h2s_before = mixture.h2s;
        let h2o_before = mixture.h2o_vapor;

        let heat = engine.step(&mut mixture, &mut f32::default(), SolidFuel::None, 700.0, 0.01);

        assert!(mixture.h2s < h2s_before, "H2S 应减少");
        assert!(mixture.h2o_vapor > h2o_before, "应产生 H2O");
        assert!(heat > 0.0, "H2S 燃烧放热 > 0: heat={}", heat);
    }
}
