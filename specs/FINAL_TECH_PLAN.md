# 废土创世：最终技术方案

> **版本**: v1.0
> **日期**: 2026-06-08
> **状态**: 执行中

---

## 核心架构：软件统一建模 + 专属引擎 + 确定性并行调度

---

## 一、核心理念

不追求用一个引擎统一所有计算。物理碰撞、化学推导、生物代谢使用各自领域最成熟的专属引擎，榨取极致性能。所有数据格式和接口统一，让不同引擎能够无摩擦协同。计算任务通过确定性并行调度器分发到多核CPU，每一帧完成海量交互。

---

## 二、统一建模语言

废土创世中所有实体——无论是一块石头、一瓶酸液、还是一个变异生物——都用同一套统一建模语言来描述。这套语言的核心是元体属性向量，它不区分物理、化学、生物，只是把所有属性都列在同一个数据结构里。每个元体都有一组属性，包括物理量（质量、密度、硬度）、化学量（元素组成、反应活性）、生物量（基因Token序列、代谢速率）。属性值可以为零，但字段永远存在。

所有工具——物理引擎、化学引擎、生物引擎、资产管线、存档系统——都使用这同一套语言来描述实体。物理引擎读取密度和硬度来计算碰撞响应，化学引擎读取元素组成和反应活性来推导反应产物，生物引擎读取基因Token和代谢速率来模拟生长。它们读写的是同一个数据结构，不需要任何格式转换。

### 属性向量字段定义

```rust
struct MetaEntity {
    // 标识
    id: Uuid,
    version: u64,           // 乐观锁版本号，调度器用

    // 空间
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    angular_velocity: Vec3,

    // 物理属性
    mass: f32,
    density: f32,
    hardness: f32,
    toughness: f32,
    elastic_modulus: f32,
    yield_strength: f32,
    ultimate_strength: f32,
    poisson_ratio: f32,
    friction_coefficient: f32,
    restitution: f32,

    // 热/电/磁
    temperature: f32,
    thermal_conductivity: f32,
    specific_heat_capacity: f32,
    electrical_conductivity: f32,
    magnetic_permeability: f32,

    // 化学属性
    elemental_composition: Vec<ElementFraction>,
    bond_types: Vec<ChemicalBond>,
    reactivity: f32,
    ph: f32,
    redox_potential: f32,
    oxidation_state: f32,
    corrosion_depth: f32,
    solubility: f32,
    flammability: f32,
    toxicity: f32,

    // 生物属性
    gene_tokens: Vec<GeneToken>,
    metabolic_rate: f32,
    growth_rate: f32,
    repair_rate: f32,
    neural_signal_strength: f32,
    health: f32,
    max_health: f32,
    radiation_dose: f32,
    toxin_level: f32,
    nutrient_level: f32,
    hydration: f32,
    cell_type: CellType,
    tissue_density: f32,

    // 状态
    state: MetaEntityState,
    spawn_tick: u64,
    parent_id: Option<Uuid>,
    children: SmallVec<[Uuid; 4]>,

    // 扩展槽（运行时动态添加）
    extensions: HashMap<String, ExtensionValue>,
}
```

---

## 三、统一接口

所有规则引擎——物理、化学、生物——都通过同一个接口来访问世界状态。接口的核心是两个函数：给定元体ID返回完整属性向量；给定元体ID和变更列表写回属性变化。所有引擎都通过这个接口获取数据，通过这个接口写回数据。接口本身不关心调用者是物理、化学还是生物——它只是忠实地读写元体属性向量。

在接口之下，元体属性向量可以被存储为多种物理格式——内存中的ECS组件数组用于热数据，磁盘上的列式存储用于冷数据，网络传输中的序列化格式用于联机同步。这些存储格式的差异被接口完全屏蔽，调用者不需要知道数据当前在哪里、以什么格式存储。

```rust
trait UnifiedWorld {
    fn read_entity(&self, id: Uuid) -> Option<&MetaEntity>;
    fn write_entity(&mut self, id: Uuid, changes: EntityChanges) -> Result<u64, WriteError>;
    fn query_entities(&self, predicate: &dyn Fn(&MetaEntity) -> bool) -> Vec<&MetaEntity>;
    fn spawn_entity(&mut self, entity: MetaEntity) -> Uuid;
    fn despawn_entity(&mut self, id: Uuid);
}
```

---

## 四、专属引擎协同

### 4.1 物理引擎（Rapier 确定性模式 + 定点数）

Rapier负责刚体运动、碰撞检测、约束求解。手写SIMD加速批量碰撞检测和约束求解内循环。物理引擎读取元体的密度、硬度、弹性等属性，计算碰撞响应，输出力、速度变化和断裂事件。

### 4.2 化学引擎（第一性原理推导）

基于第一性原理（电负性、键能、热力学）实时推导反应产物和速率。常见反应结果使用三级哈希缓存，避免重复计算。推导结果输出物质属性变化（氧化进度、腐蚀深度）和生成物（气体、沉淀）。

### 4.3 生物引擎（ECS批量更新）

基因表达、微生物组变化、神经信号传导使用低频批量处理，每游戏分钟更新一次。批量更新利用ECS的连续内存布局，一次遍历处理所有活跃生物实体。

### 4.4 引擎协同模式

三个专属引擎通过统一接口读写同一份ECS数据。物理引擎写入速度变化，化学引擎读取速度变化判断是否发生碰撞，碰撞触发化学引擎推导反应，反应产物修改材料属性，物理引擎在下一帧使用被修改后的材料属性。这不是事件总线，不是异步回调，是同一份数据的持续读写，只是由不同的专属引擎在不同时刻执行。

---

## 五、确定性并行调度器

每一帧开始时，调度器收集所有待处理的交互事件——碰撞事件、反应事件、代谢更新事件。调度器分析这些事件之间的依赖关系，构建依赖图。如果事件A和事件B作用于不同的元体、修改不同的属性，它们互不依赖，可以并行执行。如果事件C和事件D修改同一个元体的同一个属性，它们互相依赖，必须串行执行。

依赖图构建完成后，子任务被分发到多个工作线程。每个工作线程拥有自己的属性向量缓存，减少共享内存的竞争。当工作线程需要修改共享属性时，通过原子操作或细粒度锁同步。每个元体属性带有版本号，修改时检查版本号是否被其他线程改变。如果被改变，基于最新版本重新计算。

任务分配顺序按照固定规则——元体ID排序、区域哈希排序、事件时间戳排序。任务分配顺序在给定输入下是唯一确定的。工作线程数量固定，只执行分配给它们的任务，不进行动态负载均衡。结果由任务的分配顺序唯一决定，与线程执行速度无关。相同输入在任意平台上产生相同输出。

```rust
struct Scheduler {
    worker_count: usize,
    pending_events: Vec<ScheduledEvent>,
    dependency_graph: DiGraph,
    thread_pool: rayon::ThreadPool,
}

struct ScheduledEvent {
    event_id: u64,
    entity_ids: Vec<Uuid>,
    modified_fields: Vec<FieldId>,
    event_type: EventType,
    priority: u8,
    timestamp: u64,
}
```

---

## 六、统一资产管线

AI生成的模型、手工制作的零件、从社区下载的蓝图——在进入游戏运行时之前，全部被翻译为统一建模语言。几何数据被转换为元体的空间参数，材质数据被转换为元体的物理和化学属性，功能标签被转换为元体的交互注册信息。翻译过程由资产管线的转换层完成，转换层输出的不是特定引擎的格式，而是统一建模语言的序列化文件。

---

## 七、统一存档与同步

存档时，当前世界状态中所有活跃元体的属性向量被序列化为统一格式写入磁盘。读档时，从磁盘加载统一格式，重建元体实体。联机同步时，只有发生变化的属性向量被序列化并发送给其他玩家。存档格式的版本兼容性由统一建模语言的版本号管理。

---

## 八、统一响应函数注册表

交互规则——铁遇到酸会发生什么、肌肉纤维遇到神经信号会发生什么——不再被硬编码在各自引擎内部。它们被统一注册到一个响应函数注册表中。注册表的键是属性向量的哈希值，值是对应的专属引擎响应函数。当两个元体在空间上接近时，系统查询注册表，找到匹配的响应函数，调用专属引擎执行。新规则可以被动态添加。

```rust
struct ResponseRegistry {
    rules: HashMap<u64, ResponseFunction>,
    default_rules: HashMap<InteractionType, ResponseFunction>,
    custom_rules: HashMap<String, ResponseFunction>,
}

type ResponseFunction = fn(&MetaEntity, &MetaEntity, &dyn UnifiedWorld) -> Vec<EntityChanges>;
```

---

## 九、性能预算

| 组件 | 硬件 | 目标 |
|------|------|------|
| CPU | i5-7000 | 主线程渲染 + 输入 |
| 物理核心 | 独占1核 | Rapier确定性模式 |
| 化学核心 | 独占1核 | 低频触发 |
| 生物核心 | 后台线程 | 分钟级批量更新 |
| 调度器 | 剩余核心 | 并行分发独立任务 |
| GPU | RTX 4060 8G | 1080p 可变帧率 |
| RAM | 10GB | 世界状态 + 资产 |

---

## 十、已知缺陷与缓解

| 缺陷 | 缓解 |
|------|------|
| 执行顺序仍有先后 | 版本号机制保证数据一致性，确定性调度器保证执行顺序确定 |
| 属性语义由开发者定义 | 扩展字段允许运行时动态添加新属性 |
| 密集场景并行度下降 | 调度器自动检测并行度，低于阈值降级串行 |

---

## 十一、开发路线

| 阶段 | 时间 | 内容 |
|------|------|------|
| 1 | 第1周 | 定义元体属性向量完整字段，实现统一接口读写函数 |
| 2 | 第2-4周 | 集成Rapier确定性模式，物理引擎对接统一接口 |
| 3 | 第2-3月 | 化学推导模块 + 生物批量更新，对接统一接口 |
| 4 | 第4-6月 | 确定性并行调度器，多线程协同 |
| 5 | 第7-12月 | 统一资产管线、存档系统、响应函数注册表 |

---

**这就是废土创世的最终技术方案。数据统一，接口统一，计算用最狠的专属引擎，调度用确定性并行。**