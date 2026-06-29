# Wasteland Engine v7.0 架构方案

> 经4轮批判性思考（提出→找缺点→打破→重建）后的最终设计

---

## 0. 当前引擎的根本问题

| 问题 | 严重度 | 现状 |
|------|--------|------|
| 4套独立系统数据冗余 | CRITICAL | PhysicsWorld + XpbdSolver + MpssBuffer + MetaEntity 各自维护位置/速度/质量 |
| 所有子系统每帧全更新 | HIGH | 物理/化学/生物/流体/声学/地质/电磁全部60Hz，无分层 |
| 热力学/XPBD/EventBus 全是死代码 | HIGH | 300行thermal_update从未调用，XPBD从未step()，EventBus无订阅者 |
| 域隔离不存在 | HIGH | 300行硬编码propagate_cross_domain_effects，无边界检查 |
| O(n²) 元体交互 | MEDIUM | 双重循环遍历所有meta_entities，仅距离剪枝 |
| PhysicsLod/FrequencyScheduler 定义但未用 | MEDIUM | 定义了5tier但只调度meta_entities的"是否更新" |

---

## 1. 核心设计：统一粒子场（Unified Particle Field）

### 1.1 设计理念

**所有物质都是粒子。** 不再有独立的 PhysicsWorld / XpbdSolver / MetaEntity，统一到 MpssBuffer。

### 1.2 粒子属性向量（固定长度，SoA布局）

```
每粒子 192 bytes（比当前168多24 bytes用于新字段）

位置/运动 (40B):
  pos[3], vel[3], force[3], mass, radius          // 28B
  c[9] (APIC仿射矩阵)                               // 36B → 已有

物质属性 (48B):
  material_idx(u16), density, hardness, elastic_modulus  // 14B
  thermal_conductivity, specific_heat, temperature       // 12B
  electrical_conductivity, magnetic_permeability         // 8B
  chemical_id(u32), reactivity, ph, toxicity             // 14B

生物属性 (24B):
  biomass, metabolic_rate, health, gene_tokens[4]        // 24B

状态标记 (8B):
  thermal_state(u8), chemical_state(u8), biological_state(u8)  // 3B
  flags(u32), lifetime, age                                  // 5B+2B → 已有

形变 (48B):
  strain[9], jacobian, subcell_strain[3]               // 已有

电磁 (4B):
  charge                                                // 已有

层级 (8B):
  parent_id, lod_level(u8)                             // 4B+1B+padding
```

### 1.3 正交状态轴（3轴独立，效果叠加）

```
热力态: Frozen(0) → Cold(1) → Normal(2) → Hot(3) → Plasma(4) → Extreme(5)
化学态: Inert(0) → Reactive(1) → Burning(2) → Corroding(3) → Radioactive(4)
生物态: Dead(0) → Dormant(1) → Active(2) → Growing(3) → Mutating(4)
```

**不枚举组合**（6×5×5=150种），每个轴独立处理，效果叠加。
轴间耦合通过**耦合系数矩阵**处理（如温度升高→化学反应加速→Arrhenius方程）。

### 1.4 LOD层级粒子

```
LOD 0 (近场 <10m):   1粒子 = 0.01m³  MPM精网格
LOD 1 (中场 10-50m): 1粒子 = 0.1m³   MPM中网格
LOD 2 (远场 50-200m):1粒子 = 1m³     背景场查表
LOD 3 (极远 >200m):  1粒子 = 10m³    背景场插值
```

玩家移动时，LOD边界移动，边界粒子在重叠区双份表示，用权重函数混合。

---

## 2. 多尺度时间步（Multi-Scale Timestep）

### 2.1 分层频率

| 层级 | 频率 | 子系统 | 算法 |
|------|------|--------|------|
| L0 物理 | 60Hz | 碰撞、运动、力学 | XPBD 8子步 / Rapier |
| L1 粒子 | 60Hz | MPM (P2G→solve→G2P) | APIC 4子步 |
| L2 化学 | 6Hz | 化学反应、腐蚀、燃烧 | Arrhenius + 反应表 |
| L3 热力 | 6Hz | 传导、对流、辐射、相变 | FTCS + Stefan-Boltzmann |
| L4 生物 | 1Hz | 代谢、生长、突变 | 基因表达 + 代谢方程 |
| L5 流体 | 6Hz | Navier-Stokes、声学 | 稳定流体 + 声波方程 |
| L6 地质 | 0.1Hz | 侵蚀、构造、径流 | 地质时间步 |
| L7 背景 | 0.1Hz | 全局温度场、污染扩散 | 粗网格扩散 |

### 2.2 同步/异步事件

```
同步事件（当帧处理）:
  - 碰撞 → 触发化学反应（爆炸即时）
  - 相变 → 切换热力态
  - 结构破坏 → 生成碎片粒子

异步事件（下一帧处理）:
  - 锈蚀 → 化学态标记
  - 代谢 → 生物属性更新
  - 辐射累积 → 剂量叠加
```

### 2.3 子步插值

L0物理每帧执行，L2化学每10帧执行。但物理碰撞触发的化学反应需要即时反馈。
解决方案：**碰撞事件在物理子步中同步触发化学检查**，但完整化学反应在L2频率执行。

---

## 3. 兴趣中心LOD（Interest-Centered LOD）

### 3.1 移动窗口MPM

```
精网格窗口: 以玩家为中心，32x32x32 网格，dx=0.1m
  → 覆盖 3.2m³ 区域
  → 只在此窗口内执行完整 MPM (P2G→solve→G2P)

中网格窗口: 64x64x64 网格，dx=0.5m
  → 覆盖 32m³ 区域
  → 简化 MPM（无形变，只有速度/温度）

粗网格背景: 128x128x128 网格，dx=5m
  → 覆盖 640m³ 区域
  → 只存温度/污染/辐射标量场
```

### 3.2 窗口移动

玩家移动时，网格窗口跟随移动。边界粒子用**双线性权重**在两个网格间插值。

### 3.3 FrequencyScheduler 扩展

当前 FrequencyScheduler 只调度 meta_entities。扩展为调度**所有子系统**：

```
Critical(60Hz): 玩家附近10m内的粒子物理+碰撞
High(30Hz):     10-50m内的粒子运动
Medium(10Hz):   50-200m内的粒子简化更新
Low(1Hz):       200m外的背景场更新
Background(0.1Hz): 全局温度/污染/地质
```

---

## 4. 域隔离与能量总包（Domain Isolation + Energy Bundle）

### 4.1 域隔离触发条件

```
热力域隔离: 局部温度 > 5000K → 切换到等离子体方程
化学域隔离: 反应速率 > 阈值 → 切换到爆轰方程
力学域隔离: 应变率 > 阈值 → 切换到冲击波方程
```

### 4.2 渐变耦合区（不是硬边界）

```
隔离区核心: 极端物理方程
隔离区边界(渐变带): w(r) = 1 - smoothstep(r_inner, r_outer, r)
  - w=1: 纯极端方程
  - w=0: 纯经典方程
  - 0<w<1: 两者加权混合
```

### 4.3 分层能量总包

```
宏观层: 总能量E、总动量p、总质量m → 注入经典物理
中观层: 碎片分布(数量、速度分布) → 生成粒子
微观层: 化学残留标记(物质ID、浓度) → 标记粒子属性
```

### 4.4 降维与可逆性

```
降维: 极端区域的特殊属性(磁感应强度B、等离子体频率ωp)
      → 转化为基础属性加权: force += B×v×charge, temperature += ωp²×factor

可逆: 降维时保存差异快照 Δ = (微观状态) - (宏观约束重建)
      恢复时: 微观状态 = 宏观约束重建 + Δ
```

---

## 5. 跨域交互重构

### 5.1 激活EventBus

当前EventBus无订阅者。重构为：

```
EventBus 订阅关系:
  CollisionEvent → 化学系统(同步) + 热力系统(同步) + 破坏系统(同步)
  ReactionEvent → 热力系统(同步) + 生物系统(异步) + 辐射系统(异步)
  ThermalEvent → 化学系统(异步) + 生物系统(异步)
  RadiationEvent → 生物系统(异步) + 化学系统(异步)
  PhaseEvent → 物理系统(同步) + 化学系统(异步)
```

### 5.2 删除硬编码的 propagate_cross_domain_effects

300行硬编码替换为EventBus订阅者注册。

---

## 6. 空间分区加速

### 6.1 HashGrid for 粒子交互

```
格子大小: 2m (交互阈值10m的1/5)
每帧重建HashGrid
交互检测: 只检查同格+相邻26格的粒子
O(n²) → O(n)
```

### 6.2 BVH for 射线/区域查询

用于：射线投射、区域伤害、视野裁剪。

---

## 7. 死代码清理

| 系统 | 操作 |
|------|------|
| XpbdSolver | 接入主循环（替换Rapier for 软体）或移除 |
| thermal_update() | 取消`#[allow(dead_code)]`，接入tick |
| ConductionSolver/ConvectionSolver/RadiationSolver | 在thermal_update中调用 |
| PhaseSolver + phase_states | 在thermal_update中填充 |
| EventBus | 激活订阅者 |
| PhysicsLod | 实现LOD分层更新 |
| UnifiedEngine | 统一到GameWorld或移除 |

---

## 8. 实现优先级（渐进式）

### Phase 1: 清理死代码（1-2天）
1. 接入热力学求解器到tick
2. 激活EventBus订阅者
3. 删除或接入XPBD
4. 删除propagate_cross_domain_effects硬编码

### Phase 2: 多尺度时间步（2-3天）
1. 化学/热力降频到6Hz
2. 生物降频到1Hz
3. 地质降频到0.1Hz
4. 同步/异步事件分离

### Phase 3: 兴趣中心LOD（3-5天）
1. 移动窗口MPM
2. 3层网格动态激活
3. FrequencyScheduler扩展到所有子系统

### Phase 4: 域隔离（3-5天）
1. 域隔离触发条件
2. 渐变耦合区
3. 分层能量总包
4. 降维+差异快照

### Phase 5: 空间分区（1-2天）
1. HashGrid粒子交互
2. BVH射线查询

### Phase 6: 统一粒子场（5-10天）
1. MetaEntity → MpssBuffer 粒子
2. PhysicsWorld → MpssBuffer 粒子
3. 统一属性向量

---

## 9. 验证计划

### 9.1 单元测试
- 每个算法独立测试（热传导、化学反应、生物代谢）
- 状态轴切换测试（Frozen→Hot→Plasma→Normal）
- 域隔离触发/恢复测试

### 9.2 集成测试
- 爆炸场景：碰撞→化学反应→热力→碎片→残留
- 火焰传播：燃烧→热传导→对流→辐射→生物影响
- 辐射场景：放射性衰变→生物损伤→突变

### 9.3 性能测试
- 100万粒子 60fps
- 玩家移动时LOD切换无卡顿
- 域隔离触发时帧时间 < 33ms

### 9.4 守恒验证
- 能量守恒：ΣE_before = ΣE_after + ΔE_辐射
- 质量守恒：Σm_before = Σm_after
- 动量守恒：Σp_before = Σp_after

---

## 10. 方案自批判（已知风险）

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 统一粒子场重构工作量过大 | 高 | 延期 | Phase 6放最后，前5阶段已可独立交付 |
| 多尺度时间步引入非确定性 | 中 | 联机问题 | 固定种子+确定性调度 |
| 域隔离渐变区计算复杂 | 中 | 性能 | 限制同时存在的隔离区数量 |
| LOD切换artifacts | 中 | 视觉跳变 | 重叠区双份表示+权重混合 |
| EventBus延迟 | 低 | 响应慢 | 同步事件绕过队列 |

---

**版本**: v7.0
**日期**: 2026-06-23
**状态**: 方案设计完成，待实现
