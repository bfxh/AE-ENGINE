# ae_biology

> 废土生物模拟系统 —— 面向游戏引擎的生物学 crate

## 概述

`ae_biology` 提供废土生存游戏所需的生物学模拟功能，覆盖从分子到生态系统的多个尺度：

- **分子层**: 基因组、CRISPR 编辑、基因驱动（`genome`, `sci_fic_biology`）
- **细胞层**: 微生物组、感染反应、毒素代谢（`microbiome`, `infection`, `metabolism`, `venom_toxin`）
- **组织层**: 28 种组织类型、生物矿化、软组织力学（`tissues`, `biomineralization`, `soft_tissue`）
- **器官层**: 32 种器官、器官系统、功能剖面（`organs`, `organs_extended`）
- **个体层**: 有机体、再生、伤口愈合、疾病（`organisms`, `regeneration`, `wound_healing`, `disease`）
- **群体层**: 进化、共生、生态系统（`evolution`, `symbiosis`, `ecosystem`）
- **环境层**: 极端环境、外星生物学（`extreme_environment`, `exotic_biology`）
- **调控层**: 激素、神经（`hormones`, `neural`）

## 模块清单（22 个）

| 模块 | 测试数 | 说明 |
|------|--------|------|
| biomineralization | - | 生物矿化（骨、壳、牙釉质） |
| disease | - | 疾病模型 |
| ecosystem | - | 生态系统 |
| evolution | 16 | 进化与遗传算法 |
| exotic_biology | - | 外星生物学 |
| extreme_environment | - | 极端环境适应 |
| genome | 3 | 基因组 |
| hormones | 7 | 激素调控 |
| infection | 19 | 5 变量炎症反应 ODE |
| metabolism | 22 | 代谢稳态 |
| microbiome | - | 微生物组 |
| neural | 6 | 神经网络 |
| organisms | - | 有机体 |
| organs | 7 | 器官系统 |
| organs_extended | 12 | 器官扩展（32 种器官） |
| regeneration | - | 再生 |
| sci_fic_biology | 8 | 科幻生物学（CRISPR、辐射、基因驱动） |
| soft_tissue | - | 软组织力学 |
| symbiosis | - | 共生关系 |
| tissues | 12 | 28 种组织类型 |
| venom_toxin | - | 毒液与毒素 |
| wound_healing | - | 伤口愈合 |

## 编译与测试

```bash
cargo check -p ae_biology --manifest-path d:\rj\ae_project\Cargo.toml --target-dir d:\rj\ae_project\target2
cargo test  -p ae_biology --manifest-path d:\rj\ae_project\Cargo.toml --target-dir d:\rj\ae_project\target2 --lib
```

## 依赖

`serde`, `glam`, `log`, `uuid`, `slotmap`, `smallvec`, `hashbrown`, `rand`, `rand_distr`, `rayon`

## 参考来源

- Guyton & Hall, *Textbook of Medical Physiology* (14th ed.)
- Tortora & Derrickson, *Principles of Anatomy and Physiology* (15th ed.)
- Junqueira, *Basic Histology* (15th ed.)
- ICRP Publication 89 (2002) — Adult Reference Computational Phantoms
- Bergmann et al. 2009, *Science* — Cardiomyocyte renewal
- Reynolds et al. — Mathematical model of pulmonary infection
- Landau & Lifshitz, *Theory of Elasticity*