# ae_project 规则 — 废土创世游戏引擎

> **版本**: v4.1
> **更新**: 2026-06-07
> **设计文档**: [游戏核心系统技术文档 v7.2](file:///E:/开发/我的/游戏核心系统技术文档_v7.0.md)
> **演示文档**: [FEATURE_DEMO.md](file:///d:/rj/ae_project/FEATURE_DEMO.md)

---

## 一、项目定义

| 属性 | 值 |
|------|-----|
| 项目名 | ae_project / 废土创世 |
| 类型 | 规则驱动物理沙盒游戏引擎 |
| 语言 | Rust (核心 Crate) + C (数学/SIMD) + C++ (Godot 集成) + Go (场服务) + Java (宏观模拟) + Python (资产管线) |
| 目标引擎 | Godot 4.6+ (Jolt Physics 默认) |
| 目标硬件 | i5-7000 / RTX 4060 8G / 10GB RAM / 1080p @ 60fps |
| 许可证 | MIT |

---

## 二、开发前必读

| 阶段 | 规范 |
|------|------|
| 资格检查 | [qualify.md](file:///d:/rj/.trae/specs/pipeline/qualify.md) |
| 技术路线 | [roadmap.md](file:///d:/rj/.trae/specs/pipeline/roadmap.md) |
| 项目技术规格 | [spec.md](file:///d:/rj/ae_project/spec.md) |

### 按需加载规范

| 场景 | 加载 |
|------|------|
| Rust 代码 | [rust.md](file:///d:/rj/.trae/specs/lang/rust.md) |
| **AI 3D 管线** | **[ai_3d_pipeline.md](file:///d:/rj/.trae/specs/domain/ai_3d_pipeline.md)** |
| 3D/渲染 | [3d.md](file:///d:/rj/.trae/specs/domain/3d.md) |
| 建模管线 | [modeling.md](file:///d:/rj/.trae/specs/domain/modeling.md) |
| 游戏开发 | [game.md](file:///d:/rj/.trae/specs/domain/game.md) / [game_advanced.md](file:///d:/rj/.trae/specs/domain/game_advanced.md) |
| 模拟/仿真 | [sim.md](file:///d:/rj/.trae/specs/domain/sim.md) |
| 物理引擎 | [physics.md](file:///d:/rj/.trae/specs/domain/physics.md) |
| 化学引擎 | [chemistry.md](file:///d:/rj/.trae/specs/domain/chemistry.md) |
| 生物引擎 | [biology.md](file:///d:/rj/.trae/specs/domain/biology.md) |
| 热力学 | [thermo.md](file:///d:/rj/.trae/specs/domain/thermo.md) |
| 流体力学 | [fluid.md](file:///d:/rj/.trae/specs/domain/fluid.md) |
| 声学 | [acoustics.md](file:///d:/rj/.trae/specs/domain/acoustics.md) |
| 光学 | [optics.md](file:///d:/rj/.trae/specs/domain/optics.md) |
| 地质学 | [geology.md](file:///d:/rj/.trae/specs/domain/geology.md) |
| 气象学 | [weather.md](file:///d:/rj/.trae/specs/domain/weather.md) |
| 水文学 | [hydrology.md](file:///d:/rj/.trae/specs/domain/hydrology.md) |
| 生态学 | [ecology.md](file:///d:/rj/.trae/specs/domain/ecology.md) |
| 工厂/自动化 | [factory.md](file:///d:/rj/.trae/specs/domain/factory.md) |
| 公理引擎 | [axiom.md](file:///d:/rj/.trae/specs/domain/axiom.md) |
| 信息传播 | [info.md](file:///d:/rj/.trae/specs/domain/info.md) |
| 音频/音效 | [audio.md](file:///d:/rj/.trae/specs/domain/audio.md) |
| 构建 | [create.md](file:///d:/rj/.trae/specs/pipeline/create.md) |
| 部署 | [deploy.md](file:///d:/rj/.trae/specs/pipeline/deploy.md) |
| 运行 | [run.md](file:///d:/rj/.trae/specs/pipeline/run.md) |
| 测试 | [test.md](file:///d:/rj/.trae/specs/pipeline/test.md) |
| 质量 | [quality.md](file:///d:/rj/.trae/specs/workflow/quality.md) |
| 安全 | [security.md](file:///d:/rj/.trae/specs/pipeline/security.md) |
| Git | [collab.md](file:///d:/rj/.trae/specs/workflow/collab.md) |
| API 设计 | [api.md](file:///d:/rj/.trae/specs/domain/api.md) |
| AI Agent | [agent.md](file:///d:/rj/.trae/specs/ai/agent.md) |
| LLM 调用 | [llm.md](file:///d:/rj/.trae/specs/ai/llm.md) |

---

## 三、安全铁律（BLOCK — 违反即不可合并）

### 3.1 代码安全
- **S-001**: 禁止 `unsafe` 块，除非在 FFI 边界且附完整 SAFETY 注释说明不变量
- **S-002**: 禁止裸 `eval()` / `exec()` / `system()` 调用
- **S-003**: 禁止 `except: pass`（静默吞噬异常）
- **S-004**: 禁止硬编码绝对路径或密钥/Token
- **S-005**: 生产路径禁止 `unwrap()` 和 `expect()`，使用 `?` 或 `Result` 传播
- **S-006**: 所有网络服务必须有认证（不允许匿名访问）
- **S-007**: 存档文件必须 HMAC 签名验证完整性

### 3.2 数据安全
- **S-101**: 密钥/Token 不暴露到代码或日志
- **S-102**: 用户输入必须先验证再使用（CWE-20）
- **S-103**: 定点数运算不可溢出（CWE-190）
- **S-104**: GDExtension FFI 边界必须校验所有参数
- **S-105**: 敏感数据不写入日志或调试输出

### 3.3 操作安全
- **S-201**: 永远不 commit（除非用户明确要求）
- **S-202**: 修改受保护文件前先备份到 `storage/CC/2_Old/`
- **S-203**: 删除前先列出清单 → 报告 → 等用户确认

---

## 四、架构约束（BLOCK — 违反即不可合并）

### 4.1 模块边界
- **A-001**: 每个子 crate 独立职责，禁止循环依赖
- **A-002**: 核心层（ae_physics/chemistry/biology）不依赖应用层（gdextension）
- **A-003**: 所有跨模块通信通过 `ae_eventbus` 事件总线
- **A-004**: 外部依赖接口必须定义 trait，先写假实现再写真实实现

### 4.2 数据流
- **A-101**: 实体状态统一存储在 ECS 组件数组中（`ae_engine::ecs`）
- **A-102**: 跨系统状态变更通过 `ae_engine::arbitration` 仲裁器合并
- **A-103**: 所有状态变更为不可变事件序列（事件溯源模式）
- **A-104**: 物理计算使用 64 位定点数保证确定性（`ae_physics::fixed_point`）

### 4.3 模块依赖图
```
ae_physics ──┐
ae_chemistry ─┤
ae_biology ───┼── ae_metaentity ── ae_engine ── gdextension
ae_field ─────┤         │
ae_particle ──┤         ├── ae_eventbus
ae_emergence ─┘         ├── ae_crafting
                               └── ae_modding
ae_timeslice ── (独立，被 engine 引用)
```

---

## 五、性能约束（BLOCK）

| 约束 | 目标值 | 测量方式 |
|------|--------|---------|
| **P-001**: 帧率（Release） | ≥ 30fps，目标 60fps | Tracy profiler |
| **P-002**: 物理步长 | 固定 1/60s | 帧时间戳差 |
| **P-003**: 单帧物理计算 | < 8ms（RTX 4060） | Tracy GPU 区间 |
| **P-004**: 内存占用 | < 8GB（总计） | 进程内存监控 |
| **P-005**: 启动时间 | < 5s（冷启动） | 启动计时器 |
| **P-006**: 存档加载 | < 3s（10MB 存档） | 加载计时器 |
| **P-007**: 着色器编译 | < 2s（Shader Baker 预编译后） | Godot 分析器 |

---

## 六、代码规范（HIGH）

### 6.1 Rust 代码风格
- **C-001**: 使用 `rustfmt` 自动格式化（配置见 `rustfmt.toml`）
- **C-002**: 使用 `clippy` 静态检查（配置见 `clippy.toml`），`cargo clippy -- -D warnings`
- **C-003**: 命名：模块 `snake_case`，类型 `PascalCase`，函数 `snake_case`，常量 `UPPER_SNAKE_CASE`
- **C-004**: 行宽 100 字符，4 空格缩进
- **C-005**: 不加注释（除非明确要求或 SAFETY 注释）
- **C-006**: 优先编辑现有文件，不新建

### 6.2 代码质量
- **C-101**: 新代码必须有单元测试（`#[cfg(test)]`）
- **C-102**: 关键模块交互有集成测试
- **C-103**: 覆盖率 ≥ 70%
- **C-104**: 避免不必要的 `clone()`，优先使用引用
- **C-105**: `&str` 优先于 `String` 作为参数
- **C-106**: 禁止过度工程 — 不添加未被要求的功能、重构或"改进"
- **C-107**: 不为一次性操作创建 helper/util/abstraction
- **C-108**: 不添加错误处理/fallback/验证用于不可能发生的场景

### 6.3 错误处理
- **C-201**: 使用 `thiserror` 定义错误类型
- **C-202**: 使用 `?` 操作符传播错误
- **C-203**: 对外暴露具体错误类型，内部可用 `anyhow::Result`
- **C-204**: 错误消息包含"为什么出错"和"如何修复"

---

## 七、AI 治理规则（HIGH）

### 7.1 AI 代码生成
- **AI-001**: 所有 AI 生成代码必须附带 `GENERATED_BY` 和 `REVIEWED_BY` 注释头
- **AI-002**: CI 拒绝无审核签名的 AI 生成代码
- **AI-003**: 禁止生成含 `eval()`、`os.system`、SQL 拼接等模式的代码（白名单策略）
- **AI-004**: 生成代码必须通过 `cargo clippy -- -D warnings` 和 `cargo test`

### 7.2 AI 交互协议
- **AI-101**: `?` = 草稿，只回"已阅"
- **AI-102**: `!` = 讨论基准，可延伸
- **AI-103**: `!!` = 执行指令，立刻行动
- **AI-104**: 沙盒区 / 生效区 双空间模型
- **AI-105**: 不信任内在状态，先确认指令再执行

### 7.3 AI 护栏
- **AI-201**: 重复检测：过滤与公开仓库高度相似的代码
- **AI-202**: 每次会话启动自动运行 `session_init.py` 记录追踪
- **AI-203**: 敏感操作白名单审批（`unsafe`、文件系统写入、网络调用）
- **AI-204**: 不降级原则：需求不降级，不知道的基于现有知识虚构推演，标注 `[推断]`/`[推演]`/`[假设]`

---

## 八、构建与测试

### 8.1 构建命令
```bash
cargo build                    # Debug 构建
cargo build --release          # Release 构建
cargo build -p gdextension     # 仅构建 GDExtension
cargo check                    # 快速检查（不生成二进制）
cargo clippy -- -D warnings    # 静态分析
cargo doc --no-deps --open     # 生成文档
```

### 8.2 测试命令
```bash
cargo test                     # 全部测试
cargo test -p ae_physics  # 单模块测试
cargo test -- --nocapture      # 显示输出
cargo test -- --test-threads=1 # 单线程测试
python tests/run_all_tests.py  # 集成测试脚本
```

### 8.3 运行 Godot
```bash
# 确保 gdextension DLL 在 godot_project/bin/ 目录
# 打开 Godot 4.6，导入 godot_project/project.godot
```

---

## 九、物理引擎规范

### 9.1 后端选择
| 场景 | 后端 |
|------|------|
| 默认（生产） | Jolt Physics 3.0+（Godot 4.6 默认） |
| 确定性需求 | Rapier 0.22（通过 PhysicsTrait 切换） |
| GPU 加速破坏 | IPC（增量势能接触） |
| 流体/软体 | XPBD（扩展位置动力学） |

### 9.2 坐标系统
- 右手坐标系：X 右，Y 上，Z 前
- 长度单位：米
- 质量单位：千克
- 时间单位：秒
- 定点数：64 位，32 位整数部分 + 32 位小数部分

### 9.3 碰撞检测层级
| 优先级 | 对象 | 检测方式 |
|--------|------|---------|
| 最高 | 玩家 | 连续碰撞检测（CCD） |
| 高 | Boss / 关键 NPC | 连续碰撞检测 |
| 中 | 普通 NPC / 建筑 | 离散碰撞检测 |
| 低 | 碎片 / 粒子 | 简化球体检测 |

---

## 十、化学引擎规范

### 10.1 反应计算
- 基于吉布斯自由能变化（ΔG）判断反应可行性
- ΔG < 0：自发反应
- ΔG = 0：平衡状态
- ΔG > 0：不可行（需外部能量）

### 10.2 反应类型
- 酸碱中和、氧化还原、沉淀、络合、水解、燃烧、聚合、分解、酯化、置换、加成、消除、重排、光化学、电化学

### 10.3 SMARTS 匹配
- 基于官能团索引的化学模式匹配
- 验证反应条件和热力学可行性

---

## 十一、生物引擎规范

### 11.1 基因组系统
- 20 层基因架构：敏捷 5 层 + 力量 5 层 + 智力 5 层 + 体质 5 层
- 支持遗传（父母各贡献 50%）、突变（随机 1-3 层）、编辑（CRISPR 精度）

### 11.2 生态系统
- 物种间关系：捕食、竞争、共生、寄生
- 种群动态：出生率、死亡率、迁移率
- 环境承载力：资源限制、空间限制

---

## 十二、模组系统规范

### 12.1 沙盒
- WASM 沙盒（默认）：编译为 WASM 的 Rust/C/C++ 模组
- Lua 沙盒（备选）：简单脚本模组
- 沙盒 CPU 限制：单模组 < 10ms/帧
- 沙盒内存限制：单模组 < 50MB

### 12.2 依赖管理
- mod.toml 声明依赖
- 拓扑排序确保加载顺序
- 冲突检测：同名物品/方块/实体自动标记

---
## 十三、底层开发规范（BLOCK — 底层模块必读）

### 13.1 定点数运算标准
| 规则 | 内容 |
|------|------|
| **FP-001** | 所有物理计算使用 `FixedPoint`（64 位，32 位整数 + 32 位小数），禁止 `f32`/`f64` 直接参与碰撞 |
| **FP-002** | 定点数加/减/乘/除必须通过 `FixedPoint` impl，禁止裸整数运算 |
| **FP-003** | 定点数转换边界：角度 [-π, π] 用 `FixedPoint::from_f32`，距离 [0, 10000] 米，速度 [0, 1000] m/s |
| **FP-004** | 定点数不可溢出（CWE-190），`checked_add`/`checked_mul` 优先于直接运算 |
| **FP-005** | 定点数三角函数用 CORDIC 或查表实现，精度 1e-6 |

### 13.2 MPM（物质点法）规范
| 规则 | 内容 |
|------|------|
| **MPM-001** | 网格分辨率：静态 64³，动态最大 128³ |
| **MPM-002** | 粒子间距：最小 0.05m，默认 0.1m |
| **MPM-003** | 每网格最大粒子数：256（超出则降采样） |
| **MPM-004** | 时间步长：CFL 条件约束，max Δt = 0.001s |
| **MPM-005** | 本构模型：弹塑性（Neo-Hookean + von Mises 屈服准则） |
| **MPM-006** | 隐式积分（XPBD）用于静态/准静态，显式用于动态 |

### 13.3 碰撞检测规范
| 规则 | 内容 |
|------|------|
| **COL-001** | 宽阶段：空间哈希（cell_size = 最大物体包围盒对角线） |
| **COL-002** | 窄阶段：GJK/EPA（凸体），BVH（三角网格） |
| **COL-003** | 碰撞层级：CCD（玩家/Boss）→ 离散（NPC）→ 球体简化（碎片/粒子） |
| **COL-004** | 碰撞响应：velocity-level 冲量，位置修正最大 5% 穿透深度 |
| **COL-005** | 休眠：速度 < 0.01 m/s 且持续 60 帧进入休眠 |

### 13.4 破坏系统规范
| 规则 | 内容 |
|------|------|
| **DEST-001** | 混合网格-体素：完整物体用三角形网格，受损区域激活稀疏八叉树体素 |
| **DEST-002** | 破坏阶段：表面损伤 → 局部变形 → 构件断裂 → 完全破坏 |
| **DEST-003** | 应力传播：沿结构场约束图，最大深度 10 层 |
| **DEST-004** | 八叉树深度：最大 8 层（最小体素 ≈ 0.01m³） |
| **DEST-005** | 碎片生成：Voronoi 分割，碎片数 8-64 随冲击力动态调整 |

### 13.5 性能层级规范
| 规则 | 内容 |
|------|------|
| **PERF-001** | 物理更新频率：Full（每帧）、Half（隔帧）、Macro（每 30 帧） |
| **PERF-002** | 化学更新频率：Full（每帧）、Half（每 5 帧）、Macro（每 100 帧） |
| **PERF-003** | 生物更新频率：Full（每 10 帧）、Macro（每 600 帧） |
| **PERF-004** | 距离衰减：< 50m Full，50-200m Half，> 200m Macro |
| **PERF-005** | 实体数量阈值：> 1000 生物 → Macro，> 100 化学实体 → Half |

### 13.6 确定性计算规范
| 规则 | 内容 |
|------|------|
| **DET-001** | 所有模拟必须确定性：相同输入 + 相同种子 → 相同输出 |
| **DET-002** | 随机数使用确定性 PRNG（PCG/Xoshiro），种子从存档派生 |
| **DET-003** | 禁止使用系统时间、线程 ID、硬件熵源作为模拟输入 |
| **DET-004** | 浮点运算仅用于渲染和显示，不参与物理状态更新 |
| **DET-005** | 跨平台一致性：所有平台使用相同定点数实现，禁止平台特定优化 |

### 13.7 底层内存管理
| 规则 | 内容 |
|------|------|
| **MEM-001** | 热路径禁止堆分配（`Vec::push` 在循环内），优先预分配 `Vec::with_capacity` |
| **MEM-002** | 物理数据布局：SoA（结构体数组），SIMD 友好 |
| **MEM-003** | 八叉树/空间哈希用 `slotmap` 而非 `HashMap`（避免哈希开销） |
| **MEM-004** | 粒子数据 < 16 bytes 时优先值传递，> 16 bytes 时优先引用 |
| **MEM-005** | 大数组（> 1MB）用 `Box<[T]>` 而非 `Vec<T>` 避免过度分配 |

### 13.8 跨系统仲裁规范
| 规则 | 内容 |
|------|------|
| **ARB-001** | 同一帧内多系统对同一实体的修改通过 `ArbitrationSystem` 合并 |
| **ARB-002** | 冲突解决优先级：物理（破坏）> 化学（反应）> 生物（状态变化） |
| **ARB-003** | 仲裁结果写入 ECS 组件，触发 `ae_eventbus` 事件 |
| **ARB-004** | 仲裁必须是确定性的：同帧同输入 → 同合并结果 |

### 13.9 底层编译约束
| 规则 | 内容 |
|------|------|
| **BLD-001** | Release 构建：`opt-level = 3`，`lto = true`，`codegen-units = 1` |
| **BLD-002** | 目标 CPU：`target-cpu = native`（x86-64-v3，SSE4.2 + AVX2） |
| **BLD-003** | 禁止 `panic = "abort"`（物理模拟 panic 需可恢复堆栈） |
| **BLD-004** | Debug 构建保留 `overflow-checks = true`，Release 关闭 |
| **BLD-005** | 底层模块禁止依赖 `std::collections::HashMap`（无序），使用 `BTreeMap` 或 `indexmap` |

### 13.10 底层测试规范
| 规则 | 内容 |
|------|------|
| **TST-001** | 每个物理函数必须有 `#[cfg(test)]` 单元测试 |
| **TST-002** | 定点数运算必须测试边界值（0, MIN, MAX, 溢出） |
| **TST-003** | 碰撞检测必须测试：穿透、边缘接触、重叠、高速穿透 |
| **TST-004** | 确定性测试：相同种子运行 100 帧，断言状态完全一致 |
| **TST-005** | 性能回归测试：基准测试（criterion），回归阈值 5% |

---
## 十四、版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v1.0 | 2026-05 | 初始版本 |
| v2.0 | 2026-06 | 添加安全铁律、架构约束、物理/化学/生物规范 |
| v3.0 | 2026-06-05 | 添加 AI 治理规则、代码质量规范、模组/构建规范 |
| v4.0 | 2026-06-05 | 添加底层开发规范（定点数、MPM、碰撞、破坏、性能层级、确定性、内存、仲裁、编译、测试） |
| v4.1 | 2026-06-07 | 添加 11 个新领域规范索引（热力学/流体/声学/光学/地质/气象/水文/生态/工厂/公理/信息） |