# Wasteland Project — 废土创世

> 3A 级独立游戏引擎 · 多语言混合架构（Rust + C++ + Java）
> 状态: 活跃开发中

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Language: C++](https://img.shields.io/badge/C++-17-blue.svg)](https://isocpp.org/)
[![Language: Java](https://img.shields.io/badge/Java-17-red.svg)](https://openjdk.org/)

## 项目简介

Wasteland Project 是一个 3A 级独立游戏引擎，采用**多语言混合架构**：
- **Rust** — 游戏逻辑 + 模拟层 + 引擎主体（62 crate workspace）
- **C++** — 性能关键路径（AVX2 SIMD 内核 + 工业 Jolt 物理后端预留）
- **Java** — 后端服务（多人匹配 / 房间 / 排行榜）

核心特色：
- 🧬 **3A 级生物系统** — 47 模块 1128 测试（循环/呼吸/消化/神经/免疫/衰老/癌症/表观/药理/毒理...）
- ⚛️ **完整物理引擎** — GJK/EPA 碰撞 + MPM 物质点法 + XPBD 软体 + BVH 加速 + AVX2 SIMD
- 🧪 **物理化学引擎** — 反应动力学 / 热力学 / 电化学 / 流体力学
- 🌍 **生态系统模拟** — 14 个 wasteland_* crate 联动（物理/化学/生物/天气/地质/水文...）
- 🔒 **确定性模拟** — FixedPoint Q32.32 定点数保证跨平台 lockstep 多人同步

## 架构总览

```
┌─────────────────────────────────────────────────────┐
│  游戏逻辑层 (Rust)                                   │
│  game/ editor/ wasteland_ai/ wasteland_character/   │
├─────────────────────────────────────────────────────┤
│  模拟层 (Rust, 62 crates)                            │
│  wasteland_biology/  chemistry/  physics/  field/   │
│  wasteland_engine/ (LOD 三层 MPM + 频率调度)        │
├─────────────────────────────────────────────────────┤
│  性能层 (C++ via FFI)                                │
│  cpp/simd_kernels.cpp (AVX2 物理积分)               │
│  wasteland_cpp_kernel/ (cc crate + g++ 编译绑定)    │
├─────────────────────────────────────────────────────┤
│  服务层 (Java 17 via HTTP)                           │
│  services/java-backend/ (匹配/房间/排行榜)          │
├─────────────────────────────────────────────────────┤
│  平台层                                              │
│  Windows (Vulkan)  Linux (Vulkan)                   │
└─────────────────────────────────────────────────────┘
```

## 快速开始

### 环境要求

- **Rust** stable 1.95+（GNU 或 MSVC 工具链）
- **g++** 13+（C++ 内核编译，MSYS2 MinGW64 或系统 g++）
- **Java** 17+（后端服务，可选）
- **CMake** 3.20+（仅 Jolt 后端，可选）

### 构建运行

```bash
# 编译整个 workspace（注意: rustc 1.95.0 ICE 绕过）
$env:CARGO_PROFILE_DEV_OPT_LEVEL="0"
$env:RUSTFLAGS="-C codegen-units=1"
$env:CARGO_INCREMENTAL="0"

cargo build --workspace --target-dir target2

# 运行物理守恒验证测试
cargo run --release --bin conservation_test --target-dir target2

# 运行全部测试
cargo test --workspace --target-dir target2
```

### 运行 Java 后端（可选）

```bash
cd services/java-backend
javac -encoding UTF-8 -d out src/*.java
java -cp out WastelandServer
# 监听 http://localhost:8080
```

## 核心模块

### 物理引擎（Rust + C++）

| 模块 | 行数 | 功能 |
|------|------|------|
| wasteland_physics | 8848 | GJK/EPA 碰撞、MPM 物质点法、SVD 雪塑性、双相实体、体素破坏、布娃娃、6 种关节、17 种材料、FixedPoint 确定性 |
| wasteland_xpbd | 1141 | XPBD 求解器 + 10 种约束（距离/接触/角度/体积/形状匹配...） |
| wasteland_bvh | 618 | BVH（中位数分割）+ AABB/射线/视锥查询 + 动态 refit |
| wasteland_simd | 1483 | AVX2/FMA 内联 SIMD + SoA 布局 + 8 路批量物理积分 |
| wasteland_cpp_kernel | 250+ | C++ AVX2 内核（cc crate + g++ 编译，Rust FFI 调用） |
| wasteland_engine/simulation.rs | 980 | LOD 三层网格 + Moving Window MPM + 频率调度 + 14 crate 联动 |

### 生物系统（Rust，47 模块 1128 测试）

按生理系统分类：
- **循环/呼吸/消化/排泄/血液** — circulatory, respiratory, digestive, excretory, blood
- **神经/感觉/节律/电感知** — nervous_system, sensory, circadian, bioelectric, perception
- **免疫/肌骨/力学/体温/稳态** — immune_system, musculoskeletal, biomechanics, thermoregulation, homeostasis
- **衰老/癌症/表观/药理/毒理** — aging, cancer, epigenetics, pharmacology, toxicology
- **营养/应激/发育/群体遗传/生殖** — nutrition, stress_response, developmental, population_genetics, reproduction_advanced

每个模块引用具体论文来源（Guyton & Hall, Young 2018 Nobel, Michaelis-Menten, Henderson-Hasselbalch, Hill 1938...）

### 后端服务（Java）

| 服务 | 端点 | 数据结构 |
|------|------|---------|
| MatchService | /match/join, /match/status | ConcurrentLinkedQueue（4 人成队） |
| RoomService | /room/create, /room/list | ConcurrentHashMap |
| LeaderboardService | /leaderboard/submit, /leaderboard/top | ConcurrentSkipListMap 降序 |

## 完整 Crate 列表（62 个）

物理/模拟: wasteland_physics, wasteland_xpbd, wasteland_bvh, wasteland_simd, wasteland_cpp_kernel, wasteland_chemistry, wasteland_biology, wasteland_biomech, wasteland_field, wasteland_particle, wasteland_thermo, wasteland_fluid, wasteland_acoustics, wasteland_optics, wasteland_geo, wasteland_weather, wasteland_hydro, wasteland_eco, wasteland_electro, wasteland_materials, wasteland_physchem, wasteland_botany

引擎: wasteland_engine, wasteland_render, nova_render, wasteland_audio, wasteland_animation, wasteland_terrain, wasteland_pathfinding, wasteland_factory, wasteland_crafting, wasteland_modding, wasteland_eventbus, wasteland_metaentity, wasteland_timeslice, wasteland_emergence, wasteland_frequency, wasteland_axiom, wasteland_info, wasteland_compute, wasteland_scheduler, wasteland_registry, wasteland_unified_interface

游戏: game, editor, wasteland_game, wasteland_character, wasteland_ai, wasteland_ai_tools, wasteland_ai_bridge, wasteland_memory, wasteland_network, wasteland_storage, wasteland_profiler, wasteland_io, wasteland_asset, wasteland_asset_pipeline, wasteland_serialize, wasteland_save_system

## 文档

- [REARCHITECTURE_PLAN.md](REARCHITECTURE_PLAN.md) — 多语言混合架构规划
- [ARCHITECTURE_V7.md](ARCHITECTURE_V7.md) — V7 架构设计
- [wasteland_biology/AAA_EXPANSION_PLAN.md](wasteland_biology/AAA_EXPANSION_PLAN.md) — 生物系统 3A 扩展计划
- [wasteland_biology/PROGRESS.md](wasteland_biology/PROGRESS.md) — 生物系统进度报告
- [services/java-backend/README.md](services/java-backend/README.md) — Java 后端文档

## 测试

```bash
# 物理守恒验证（质量/动量/能量守恒）
cargo run --release --bin conservation_test --target-dir target2

# 生物系统全部测试（1128 测试）
cargo test -p wasteland_biology --target-dir target2 --lib

# C++ 内核测试（Rust FFI 调用验证）
cargo test -p wasteland_cpp_kernel --target-dir target2

# 全 workspace 测试
cargo test --workspace --target-dir target2
```

## 许可证

MIT License — 见 [LICENSE](LICENSE)

## 贡献

欢迎 Issue 和 PR。请先阅读 [REARCHITECTURE_PLAN.md](REARCHITECTURE_PLAN.md) 了解架构。
