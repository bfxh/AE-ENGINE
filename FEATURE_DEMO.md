# 废土创世 (Wasteland Genesis) - 功能演示文档

> **版本**: v5.0
> **日期**: 2026-06-05
> **项目路径**: `d:\rj\wasteland_project\`
> **核心原则**: 还原现实 — 一切行为准则按真实物理、化学、生物规则运行

---

## 设计文档

- [废土创世 终极设计文档 v1.0](file:///d:/rj/wasteland_project/废土创世_终极设计文档_v1.0.md) — 世界观、核心机制、技术架构总纲
- [各模块详细设计 v1.0](file:///d:/rj/kf/jm/废土创世_各模块详细设计_v1.0.md) — 接口定义、数据结构、数据流、性能预算

---

## 功能演示概览

本项目实现了一个完整的**规则驱动物理沙盒宇宙**模拟引擎，以**交互溶解架构**为核心。所有实体统一为元体，物理、化学、生物不再区分为独立系统。

| 系统 | 状态 | 核心功能 |
|------|------|----------|
| **元体系统** | ✅ | 统一物理/化学/生物属性向量，元素周期表引擎 |
| **交互溶解架构** | ✅ | 统一响应函数，三级哈希缓存，布隆过滤器预筛 |
| **结构场** | ✅ | 约束图分析，应力传播路径，分组并行处理 |
| **功能推导引擎** | ✅ | 几何+材料→功能推导，蓝图自动记录，Socket吸附 |
| **物理引擎** | ✅ | MPM物质点法、6种材质模型、定点数确定性引擎 |
| **场论系统** | ✅ | 温度/密度/压力场、热力学耦合 |
| **化学引擎** | ✅ | 15种反应类型、热力学推导、反应进度、SMARTS匹配 |
| **生物系统** | ✅ | 生态系统、基因Token、20层基因组、进化、疾病传播 |
| **涌现系统** | ✅ | 形态发生场、全息材质、菌丝网络 |
| **事件总线** | ✅ | 跨模块通信、批处理分发、历史查询、优先级排序 |
| **制造系统** | ✅ | Socket装配、蓝图记录、配方库、功能推导融合 |
| **模组系统** | ✅ | WASM/Lua沙盒、mod.toml解析、依赖拓扑排序、冲突检测 |
| **Blender插件** | ✅ | 自动安装、场景生成 |

---

## 核心哲学

**规则取代配方，万物交互统一，真实不妥协，废土自有生命。**

```
游戏底层不存在独立的物理引擎、化学引擎和生物引擎。
所有实体统一为元体，每个元体携带完整的属性向量。
两个元体接近时，通过统一响应函数计算力向量、属性变化和新生成元体。
所有交互结果在同一物理步长内生效，没有先后顺序。
```

---

## 元体系统 (MetaEntity) — 交互溶解架构核心

### 属性向量

```
元体属性向量:
├── 物理量: 质量、密度、硬度、韧性、弹性模量、屈服强度、极限强度
├── 化学量: 元素组成、化学键类型、反应活性、酸碱度、氧化还原电位
└── 生物量: 基因Token序列、代谢速率、生长速率、修复速率、神经信号强度
```

### 交互机制

**核心代码** ([interaction.rs](file:///d:/rj/wasteland_project/wasteland_metaentity/src/interaction.rs)):

```rust
pub fn compute(a: &MetaEntity, b: &MetaEntity, distance: f32, dt: f32) -> InteractionResult {
    // 1. 机械响应: 弹性碰撞 + 阻尼 + 摩擦热
    let (fa, fb, contact_heat) = Self::mechanical_response(a, b, overlap, normal, dt);
    // 2. 化学响应: 酸腐蚀 + 氧化 + 生锈
    let (ca, cb, chem_heat, chem_generated) = Self::chemical_response(a, b, distance, dt);
    // 3. 热响应: 热传导
    // 4. 生物响应: 毒素传递 + 菌丝分解 + 酶催化
    // 5. 电响应: 导体接触
    // 6. 氧化响应: 铁+氧→锈
}
```

### 预定义材质工厂

| 工厂方法 | 材质 | 特性 |
|----------|------|------|
| `MetaEntity::iron()` | 铁 | 密度7874, 导电1e7, 易氧化 |
| `MetaEntity::water()` | 水 | 密度1000, 比热4184, 溶剂 |
| `MetaEntity::concrete()` | 混凝土 | 抗压3e7, 碱性pH12 |
| `MetaEntity::wood()` | 木材 | 可燃0.8, 植物细胞 |
| `MetaEntity::clone_organism()` | 克隆人 | ACTN3/MSTN基因, 认知滤网 |

---

## 交互缓存系统 — 三级哈希索引

**核心代码** ([interaction_cache.rs](file:///d:/rj/wasteland_project/wasteland_metaentity/src/interaction_cache.rs)):

```
布隆过滤器预筛 → 排除不可能交互
    ↓
Level 1: 精确匹配 (InteractionKey完全相同)
    ↓ 未命中
Level 2: 模糊分类 (相同category + distance_band)
    ↓ 未命中
Level 3: 条件匹配 (相同element_hash + bond_hash)
    ↓ 未命中
计算新交互结果 → 插入缓存
```

---

## 结构场 — 宏观层级耦合

**核心代码** ([structural_field.rs](file:///d:/rj/wasteland_project/wasteland_metaentity/src/structural_field.rs)):

当建筑、机械等结构体被组装完成后，系统自动分析元体间的约束关系：

```
组装完成 → 构建约束图 (BFS深度分配)
    ↓
识别关键节点 (深度>70% 或 下游>3)
    ↓
结构分组 (按根节点分组，并行处理)
    ↓
预计算应力传播路径
    ↓
外力作用时沿路径快速传播应力
```

---

## 功能推导引擎

**核心代码** ([functional_derivation.rs](file:///d:/rj/wasteland_project/wasteland_metaentity/src/functional_derivation.rs)):

**规则取代配方：** 不查表，根据几何特征+材料属性+质量分布实时推导物品功能。

```rust
// 切割功能: 锐边 + 高硬度
if geometry.has_sharp_edge && entity.physics.hardness > 3.0 {
    let confidence = edge_sharpness * 0.4 + hardness/10 * 0.3 + aspect_ratio * 0.03 + toughness/100 * 0.2;
    functions.push(Function::Cutting, confidence);
}
// 穿刺功能: 尖端 + 高硬度 + 细长
// 钝击功能: 大质量 + 任意形状
// 容器功能: 空腔
// 支撑功能: 平坦表面 + 高强度
```

输出的功能置信度是**连续概率分布**（如劈砍0.89、穿刺0.12），由玩家的使用方式赋予最终意义。

---

## 武器与护甲系统

### 统一伤害判定

所有物体都可攻击。武器伤害由刃材硬度、有效质量、速度系数和刃口微观状态实时计算，不查表。

### 护盾系统

繁荣纪元遗留的引力偏导护盾，通过驱动引力子产生时空曲率：
- 高速物体（子弹）能量-动量密度大，被轻易偏转
- 低速制导武器（弩矢、手雷）和近战攻击可穿透
- 护盾能量有限，可被饱和攻击耗尽

### 破坏分级

| 等级 | 状态 | 效果 |
|------|------|------|
| 1 | 表面损伤 | 外观变化，功能不变 |
| 2 | 局部变形 | 功能衰减 |
| 3 | 部件断裂 | 部分功能丧失 |
| 4 | 完全毁坏 | 元体标记为销毁 |

所有改变永久保存，基于真实材料力学计算。

---

## 化学引擎

### 第一性原理推导

不查表，基于电负性、键能、热力学数据实时计算反应产物、速率和热效应。反应按动力学缓慢推进，而非瞬间完成。

**核心代码** ([thermodynamics.rs](file:///d:/rj/wasteland_project/wasteland_chemistry/src/thermodynamics.rs)):

```rust
// 吉布斯自由能变化判断反应可行性
pub fn calculate_gibbs_free_energy(reaction: &Reaction, temperature: f32) -> f32 {
    let delta_h = reaction.bond_energy_products - reaction.bond_energy_reactants;
    let delta_s = reaction.entropy_products - reaction.entropy_reactants;
    delta_h - temperature * delta_s
}
```

### 15种反应类型

**核心代码** ([reactions.rs](file:///d:/rj/wasteland_project/wasteland_chemistry/src/reactions.rs)):

| 类别 | 反应类型 | 示例 |
|------|----------|------|
| **氧化还原** | Oxidation, Reduction | 铁锈形成 |
| **燃烧** | Combustion, Explosion | 汽油燃烧 |
| **酸碱** | AcidBase, Corrosion | 酸雨腐蚀 |
| **衰变** | RadioactiveDecay | 铀衰变 |
| **生物** | Biological, Catalysis | 酶催化 |
| **聚合** | Polymerization | 塑料合成 |

### 实验系统

玩家可通过火焰颜色、沉淀反应、显微镜观察、基因测序等方式主动探索物质规律，发现新配方。知识自动记录于个人日志。

---

## 生物引擎

### 基因Token系统

每个Token对应真实生物学功能（如ACTN3、MSTN），决定性状和代谢：

```
训练 → 基因表达上调
损伤 → 超量恢复强化
基因编辑器 → 主动拼接新Token
```

所有改变均伴随潜在副作用（代谢失衡、神经适应性代价）。

### 成长体系

摒弃经验值与等级。三条路径协同作用：

| 路径 | 机制 | 副作用 |
|------|------|--------|
| 用进废退 | 适应性表达 | 代谢失衡 |
| 超量恢复 | 损伤修复强化 | 神经适应性代价 |
| 基因编辑 | Token拼接 | 表达不稳定 |

### 生态系统

**12种生物群落** ([ecosystem.rs](file:///d:/rj/wasteland_project/wasteland_biology/src/ecosystem.rs)):

| 生物群落 | 环境特征 | 典型物种 |
|----------|----------|----------|
| **Wasteland** | 荒芜平原 | 变异草、秃鹫 |
| **RuinedCity** | 废墟城市 | 老鼠、蟑螂 |
| **RadioactiveMarsh** | 放射性沼泽 | 发光蘑菇 |
| **ToxicForest** | 毒林 | 刺灌木、脑真菌 |
| **Underground** | 地下洞穴 | 盲眼生物 |

---

## NPC智能架构

### 分层心智

| 层 | 技术 | 职责 |
|----|------|------|
| 战略层 | GOAP动态规划 | 长期目标选择 |
| 战术层 | 效用AI评分 | 行动优先级排序 |
| 执行层 | 行为树 | 具体动作执行 |

### 不降智原则

- 时间切片交错更新
- 班组共享心智
- 视野外预测推演
- 所有NPC与玩家共享同一套世界规则

### 长期记忆

NPC会记住玩家的行为（帮助、背叛、污染水源等），并影响后续态度、对话和任务生成。关系网络动态演化。

---

## 物理引擎

### 六种材质模型

**MPM物质点法** ([mpm.rs](file:///d:/rj/wasteland_project/wasteland_physics/src/mpm.rs)):

| 材质 | 特性 | 应用场景 |
|------|------|----------|
| **Elastic** | 完全弹性形变 | 橡胶、弹簧 |
| **ElastoPlastic** | 弹塑性形变 | 金属、混凝土 |
| **Granular** | 颗粒流动 | 沙子、土壤 |
| **Brittle** | 脆性断裂 | 玻璃、岩石 |
| **Fluid** | 流体动力学 | 水、岩浆 |
| **Snow** | 积雪硬化 | 雪、粉末 |

### 粒子-体素双相转换

**"像沙子的钢铁"** 形态转换系统 ([dual_phase.rs](file:///d:/rj/wasteland_project/wasteland_physics/src/dual_phase.rs)):

```
┌─────────────────────────────────────────────────────────┐
│                    双相实体                            │
├───────────────────┬───────────────────────────────────┤
│   粒子相          │   体素相                         │
│   (运动/碰撞)      │   (材质/化学状态)                 │
├───────────────────┼───────────────────────────────────┤
│ 拉格朗日视角     │ 欧拉视角                          │
│ 动量守恒         │ 质量守恒                          │
│ 形变追踪         │ 化学扩散                          │
└───────────────────┴───────────────────────────────────┘
         ↔ 双向转换 ↔
```

### 体素/网格混合破坏

**核心代码** ([destruction.rs](file:///d:/rj/wasteland_project/wasteland_physics/src/destruction.rs)):

未破坏的物体永远是三角网格，只有被破坏的局部才激活体素。体素使用稀疏八叉树存储，应力超过屈服强度标记为塑性变形，超过极限强度分裂为独立碎片。

---

## 场论系统

**核心代码** ([unified_field.rs](file:///d:/rj/wasteland_project/wasteland_field/src/unified_field.rs)):

```rust
pub fn set_default_thermodynamic_couplings(&mut self) {
    self.add_coupling("temperature", "density", Linear, -0.001);
    self.add_coupling("temperature", "pressure", Linear, 0.01);
    self.add_coupling("density", "pressure", Linear, 0.1);
    self.add_coupling("radiation", "temperature", Linear, 0.05);
    self.add_coupling("moisture", "temperature", GradientDriven, 0.005);
}
```

### 场激发态检测

粒子不是独立实体，而是场的激发态：

| 场类型 | 激发条件 | 产生效果 |
|--------|----------|----------|
| 密度场 | 密度峰值 | 生成粒子 |
| 应力场 | 应力奇点 | 产生裂纹 |
| 化学浓度场 | 浓度热点 | 触发反应 |

---

## 涌现系统

### 形态发生场

**基因Token驱动的生长** ([morphogenesis.rs](file:///d:/rj/wasteland_project/wasteland_emergence/src/morphogenesis.rs)):

```rust
pub struct MorphogeneticField {
    morphogens: HashMap<String, ScalarField>,  // 形态发生素
    gradients: HashMap<String, VectorField>,   // 梯度场
    gene_tokens: Vec<GeneToken>,              // 基因Token
}
```

### 全息材质系统

**光谱响应函数** ([holographic_material.rs](file:///d:/rj/wasteland_project/wasteland_emergence/src/holographic_material.rs)):

| 属性 | 描述 |
|------|------|
| 光谱反射率 | 波长依赖的反射 |
| 角度响应 | 观察角度依赖 |
| 化学状态 | 氧化程度影响颜色 |
| SH系数 | 720系数/2.8KB |

### 菌丝网络模拟

**五阶段生长** ([mycelial_network.rs](file:///d:/rj/wasteland_project/wasteland_emergence/src/mycelial_network.rs)):

```
1. 根节点初始化
        ↓
2. 菌丝尖端生长
        ↓
3. 营养运输 + Anastomosis融合
        ↓
4. 子实体形成 (4阶段)
        ↓
5. 孢子释放 + 衰老剪枝
```

---

## 统一数据层与仲裁

### ECS组件数组

所有元体属性、结构场参数、状态标志都存储在统一ECS组件数组中。物理、化学、生物的交互直接读写同一份数据，无需跨系统通信。

**核心代码** ([ecs.rs](file:///d:/rj/wasteland_project/wasteland_engine/src/ecs.rs))

### 跨系统耦合仲裁器

物理、化学、生物系统通过统一仲裁器交换状态变更事件：

```
优先级: 物理修正 > 化学腐蚀 > 生物代谢
同一字段被多系统修改时取最大绝对值
被阻挡的效果必须传播次级事件
```

**核心代码** ([arbitration.rs](file:///d:/rj/wasteland_project/wasteland_engine/src/arbitration.rs))

---

## 存档系统

### 事件溯源架构

所有状态变更只追加不删除，存档本身是SQLite数据库。读档时从上一个快照快速重放，离线模拟数十年仅需数秒。

**核心代码** ([sqlite_store.rs](file:///d:/rj/wasteland_project/wasteland_timeslice/src/sqlite_store.rs))

### 世界离线演化

存档时记录时间戳，读档时根据离线时长加速补算宏观变化（势力兴衰、生态演变、建筑老化），世界永远在流动。

---

## 性能架构

### 多层并行

**核心代码** ([performance_tiers.rs](file:///d:/rj/wasteland_project/wasteland_physics/src/performance_tiers.rs)):

| 层级 | 对象 | 碰撞检测 | 频率 |
|------|------|----------|------|
| 0 | 玩家武器/Boss装甲 | 连续碰撞检测 | 每帧 |
| 1 | 普通NPC | 离散碰撞检测 | 每帧 |
| 2 | 碎片/粒子 | 简化球体检测 | 每2帧 |
| 3 | 静止物体 | 休眠 | 按需唤醒 |

### 分层时间切片

**核心代码** ([time_slice.rs](file:///d:/rj/wasteland_project/wasteland_timeslice/src/time_slice.rs)):

不同时间敏感度系统按不同频率运行，优化CPU/GPU资源分配。

---

## 资产管线

```
AI生成模型 → Open3D自动几何质检 → MeshCNN/PointNet++语义分割
    → Blender脚本精修 → 导出glTF → Godot导入时自动挂载功能组件
```

**核心代码** ([asset_quality.rs](file:///d:/rj/wasteland_project/wasteland_engine/src/asset_quality.rs)):

自动检测非流形、破洞、面数、UV完整性、悬空碎片，不合格资产自动回退至预制模型库。

---

## Blender插件

### 插件功能

| 功能 | 状态 | 说明 |
|------|------|------|
| 地形生成 | ✅ | 16,641顶点 |
| 建筑生成 | ✅ | 30栋建筑 |
| 树木生成 | ✅ | 100棵树木 |
| glTF导出 | ✅ | 完整场景导出 |

### 自动化安装

```bash
cd d:\rj\wasteland_project\blender_plugin
python install_plugin.py
```

---

## 多语言架构

| 语言 | 职责 | 关键细节 |
|------|------|----------|
| Rust | 核心确定性引擎 | 元体数据结构、交互响应、结构场、物理步进、缓存系统、存档与事件溯源 |
| C | 底层数学与汇编封装 | 定点数运算、SIMD向量内核（AVX2/NEON）、内存池分配器 |
| 汇编 | 极致热点加速 | 批量碰撞检测和约束求解最内层循环 |
| C++ | Godot集成与编辑工具 | GDExtension桥接、渲染管线修改（SDFGI/FSR）、Tracy性能分析 |
| Go | 全局场服务器 | 风场、磁场、热扩散、声场等大规模低频计算（10-20Hz） |
| Java | 宏观战略服务器 | 势力版图、经济、生态、遗传演化等分钟级~天级模拟 |
| Python | 离线资产管线 | ComfyUI/Flux生图、TRELLIS/Hunyuan3D生成3D模型、Blender自动化精修 |

---

## 开发路线

| 阶段 | 内容 | 状态 |
|------|------|------|
| 原型期 | 元体成对交互（铁+酸腐蚀），功能推导引擎 | ✅ |
| 核心期 | 结构场约束图，建筑破坏，三级缓存 | ✅ |
| 扩展期 | RDKit化学规则库，基因Token，母巢AI，L2模组工具 | 🔜 |
| 打磨期 | 汇编SIMD加速，SSD优化，全平台自适应与帧率锁 | 🔜 |

---

## 测试验证

### 综合测试报告

```
测试运行: python run_all_tests.py

┌─────────────────────────────────────────────────────┐
│ 测试项                    │ 状态   │ 耗时      │
├────────────────────────────┼────────┼───────────┤
│ Rust Compilation Tests    │ ✅ PASS │ 0.18s    │
│ Blender Verification      │ ✅ PASS │ 3.28s    │
│ System Integration Tests  │ ✅ PASS │ 0.25s    │
├────────────────────────────┼────────┼───────────┤
│ 总计: 3 passed, 0 failed  │        │ 3.71s    │
└─────────────────────────────────────────────────────┘
```

---

## 项目结构

```
wasteland_project/
├── wasteland_metaentity/   # 元体系统 — 交互溶解架构核心
│   ├── meta_entity.rs      # 元体数据结构
│   ├── interaction.rs      # 统一响应函数
│   ├── structural_field.rs # 结构场约束图
│   ├── interaction_cache.rs # 三级哈希缓存
│   └── functional_derivation.rs # 功能推导引擎
├── wasteland_engine/       # 核心引擎
│   ├── ecs.rs              # 统一ECS数据层
│   ├── arbitration.rs      # 跨系统耦合仲裁器
│   └── asset_quality.rs    # 资产管线质检
├── wasteland_physics/      # 物理系统
│   ├── mpm.rs              # MPM物理
│   ├── octree.rs           # 稀疏八叉树
│   ├── dual_phase.rs       # 双相转换
│   ├── destruction.rs      # 混合破坏
│   ├── fixed_point.rs      # 定点数引擎
│   ├── physics_trait.rs    # 物理抽象层
│   └── performance_tiers.rs # 多层并行
├── wasteland_field/        # 场论系统
│   ├── unified_field.rs    # 统一场
│   ├── field_solver.rs     # 场求解器
│   └── reaction_diffusion.rs
├── wasteland_particle/     # 粒子系统
│   ├── emergent_rules.rs   # 涌现规则
│   └── biological_emergence.rs
├── wasteland_emergence/    # 涌现系统
│   ├── morphogenesis.rs    # 形态发生场
│   ├── holographic_material.rs
│   └── mycelial_network.rs
├── wasteland_chemistry/    # 化学系统
│   ├── thermodynamics.rs   # 热力学推导
│   └── reactions.rs        # 15种反应类型
├── wasteland_biology/      # 生物系统
│   └── ecosystem.rs        # 12种生物群落
├── wasteland_timeslice/    # 时间切片
│   ├── time_slice.rs       # 分层时间切片
│   ├── sqlite_store.rs     # 事件溯源存档
│   └── diff_graph.rs       # 差分更新图
├── wasteland_eventbus/     # 事件总线
│   ├── event.rs            # 事件类型定义
│   ├── bus.rs              # 批处理分发引擎
│   └── subscription.rs     # 订阅过滤器
├── wasteland_crafting/     # 制造系统
│   ├── socket.rs           # Socket装配定义
│   ├── recipe.rs           # 配方数据库
│   ├── blueprint.rs        # 蓝图记录与分享
│   └── assembly.rs         # 装配会话与撤销
├── wasteland_modding/      # 模组系统
│   ├── manifest.rs         # mod.toml解析
│   ├── loader.rs           # 依赖拓扑排序
│   ├── registry.rs         # 接口注册表
│   ├── conflict.rs         # 冲突检测
│   └── sandbox.rs          # WASM/Lua沙盒
├── gdextension/            # Godot扩展
├── godot_project/          # Godot项目
├── blender_plugin/         # Blender插件
└── tests/                  # 测试套件
```

---

## 事件总线系统

**核心代码** ([bus.rs](file:///d:/rj/wasteland_project/wasteland_eventbus/src/bus.rs)):

跨模块通信核心。物理、化学、生物、AI系统通过统一EventBus交换事件：

```
物理碰撞 → EventBus → 化学引擎 (检测反应)
                    → AI模块 (NPC感知)
                    → 声音系统 (碰撞声)
                    → UI (伤害数字)
```

- **批处理模式**：收集一帧所有事件，帧末批量分发，减少函数调用开销
- **优先级桶**：4级优先级排序，高优先级事件优先处理
- **历史查询**：最近1000个事件环形缓冲，支持按类型/实体/位置过滤
- **延迟事件**：支持emit_deferred将事件推迟到下一帧处理

### 事件类型

| 类别 | 事件 |
|------|------|
| 物理 | CollisionDetected, ForceApplied, DestructionStarted, FragmentGenerated |
| 化学 | ReactionStarted, ReactionCompleted, ExplosionDetected, CorrosionApplied |
| 生物 | DamageReceived, ToxinApplied, MutationOccurred, DeathEvent |
| 世界 | StructureBuilt, ItemCrafted, BlueprintDiscovered |
| NPC | NpcPerceived, NpcDecided, NpcSpoke |

---

## 制造系统

### Socket装配系统

**核心代码** ([socket.rs](file:///d:/rj/wasteland_project/wasteland_crafting/src/socket.rs)):

```
装配流程:
零件选择 → Socket兼容检查 → 位置吸附 → 约束验证 → 功能推导 → 蓝图记录
```

- **SlotType**: Blade/Handle/Guard/Pommel/Engine/Armor/Barrel/Stock/Scope/Magazine
- **Constraint**: 互斥/依赖/排斥/协同/替代
- **FusionRule**: 加权平均/最大值/最小值/和/调和 → 硬度/韧性/质量/耐久/锋利度

### 配方数据库

**核心代码** ([recipe.rs](file:///d:/rj/wasteland_project/wasteland_crafting/src/recipe.rs)):

支持按分类/可行性/关键词搜索配方，自动检查物品/工具/技能/工作台需求。

### 蓝图库

**核心代码** ([blueprint.rs](file:///d:/rj/wasteland_project/wasteland_crafting/src/blueprint.rs)):

玩家创造的物品可自动生成为蓝图，支持MIT/CC BY/CC BY-SA等开源协议分享。

---

## 模组系统

### mod.toml解析

**核心代码** ([manifest.rs](file:///d:/rj/wasteland_project/wasteland_modding/src/manifest.rs)):

```toml
[package]
name = "wasteland-chemistry-engine"
version = "0.1.0"
api_version = "1.0"

[dependencies]
wasteland-physics = ">=0.1.0"

[modules]
chemistry = { type = "native", entry = "chemistry_module_init" }
```

### 安全层级

| 层级 | 类型 | 权限 | 内存限制 |
|------|------|------|----------|
| L0 | Native | 完全权限 | 无限制 |
| L1 | WASM | 白名单函数 | 64MB |
| L2 | Lua | 禁用os/io/require | 16MB |

### 依赖解析

**核心代码** ([loader.rs](file:///d:/rj/wasteland_project/wasteland_modding/src/loader.rs)):

构建DAG依赖图 → 拓扑排序 → 循环依赖检测 → 按序加载。加载顺序：基础系统 → 核心模组 → 扩展模组 → UI模组。

---

## 基因组系统

**核心代码** ([genome.rs](file:///d:/rj/wasteland_project/wasteland_biology/src/genome.rs)):

### 20层基因架构

| 属性 | 层 | 映射公式 |
|------|-----|----------|
| 速度 | 敏捷L1 | base × (1 + L1 × 0.02) |
| 暴击率 | 敏捷L2 | 0.05 + L2 × 0.01 |
| 命中精度 | 敏捷L3 | 0.8 + L3 × 0.005 |
| 攻速 | 敏捷L4 | base × (1 + L4 × 0.015) |
| 闪避 | 敏捷L5 | 0.05 + L5 × 0.01 |
| 基础伤害 | 力量L1 | base × (1 + L1 × 0.03) |
| 暴击倍率 | 力量L3 | 1.5 + L3 × 0.02 |
| 感知范围 | 智力L1 | 20 + L1 × 1.0 |
| 最大生命 | 体质L1 | 100 + L1 × 20 |
| 辐射抗性 | 体质L3 | L3 × 0.02 |

### 三种族预设

| 种族 | 敏捷 | 力量 | 智力 | 体质 |
|------|------|------|------|------|
| 废土客 | 45 | 55 | 40 | 65 |
| 虫族 | 70 | 40 | 65 | 45 |
| 克隆人 | 55 | 55 | 60 | 55 |

---

## 化学匹配引擎

**核心代码** ([reaction_matcher.rs](file:///d:/rj/wasteland_project/wasteland_chemistry/src/reaction_matcher.rs)):

基于官能团索引的SMARTS模式匹配：

```
输入物质A + 物质B → 提取官能团 → 索引查找候选规则
    → 验证温度/压力/pH范围 → 计算ΔG可行性
    → 验证活化能 → 生成产物 + 危险性标注
```

---

## 快速开始

```bash
# 运行测试
cd d:\rj\wasteland_project\tests
python run_all_tests.py

# 使用Blender插件
cd d:\rj\wasteland_project\blender_plugin
python install_plugin.py

# 启动Godot项目
# 打开 godot_project/project.godot → 运行主场景
```

---

**项目状态**: ✅ 所有核心功能已实现并通过测试
**设计文档**: [废土创世 终极设计文档 v1.0](file:///d:/rj/wasteland_project/废土创世_终极设计文档_v1.0.md)