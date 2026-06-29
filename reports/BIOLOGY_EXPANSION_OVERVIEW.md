# 生物学模拟扩展总览

> **更新日期**: 2026-06-28 (Session 38)
> **状态**: 持续扩展中
> **核心理念**: "数都数不清的奇特生物" — 覆盖动物/微生物/植物/科幻全谱系

---

## 1. 已实现模块清单

### 1.1 wasteland_biology（18 模块，~260KB）

| 模块 | 字节数 | 核心内容 | 技术来源 |
|------|--------|----------|----------|
| disease.rs | 20430 | 疾病模型 | 基础病理学 |
| ecosystem.rs | 13449 | 生态系统 | Lotka-Volterra |
| evolution.rs | 10557 | 进化系统 | 达尔文/中性进化 |
| exotic_biology.rs | 27338 | 11类跨物种奇特结构 | 文献综合 |
| genome.rs | 11608 | 基因组 | 分子生物学 |
| hormones.rs | 13056 | 激素系统 | 内分泌学 |
| infection.rs | 6457 | 5变量感染ODE | Wajchman 2011 |
| metabolism.rs | 6108 | 代谢系统 | 生化 |
| microbiome.rs | 34873 | 25物种微生物组 | HMP数据 |
| neural.rs | 9172 | 神经系统 | 基础神经 |
| organisms.rs | 24864 | 生物体 | 综合 |
| organs.rs | 12319 | 基础器官 | 解剖学 |
| organs_extended.rs | 24222 | 31种器官扩展 | 解剖学 |
| regeneration.rs | 7948 | 蝾螈再生 | SHH/FGF8 |
| soft_tissue.rs | 18828 | XPBD切割 | 1-3/2-2 split |
| tissues.rs | 21601 | 28种组织类型 | 组织学 |
| wound_healing.rs | 14362 | 6物种愈合PDE | Javierre 2008 |

### 1.2 wasteland_biomech（7 模块，~50KB）

| 模块 | 核心内容 | 技术来源 |
|------|----------|----------|
| phase_field.rs | 相场法断裂 | Bourdin/Francfort-Marigo |
| prendergast.rs | 机械调控算子 | Prendergast 1997 |
| wolff_remodeling.rs | Wolff定律重塑 | Carter E=3790·ρ³ |
| bmp_osteogenesis.rs | BMP骨生成 | Hill方程 |
| regeneration.rs | 蝾螈芽基再生 | SHH/FGF8反向梯度 |
| prosthetics.rs | 4种义肢 | 外固定器/LCP/髓内钉/骨整合 |
| material_properties.rs | 骨/Ti6Al4V/316L | 材料力学 |

### 1.3 wasteland_character（6 模块，~59KB）

| 模块 | 核心内容 | 技术来源 |
|------|----------|----------|
| skeleton.rs | 骨骼系统 | 解剖学 |
| muscle.rs | Hill肌肉模型 | Zajac 1989/Millard 2012 |
| deformation.rs | 形变系统 | XPBD |
| force_feedback.rs | 力反馈 | 触觉 |
| surface_contact.rs | 表面接触 | 碰撞 |
| surface_stats.rs | 表面统计 | 材质 |

### 1.4 wasteland_chemistry（combustion）

| 模块 | 核心内容 | 技术来源 |
|------|----------|----------|
| combustion.rs | 三组分Arrhenius+6反应+Rothermel | 木材燃烧化学 |

### 1.5 wasteland_compute（fluid）

| 模块 | 核心内容 | 技术来源 |
|------|----------|----------|
| fluid.rs | Stam Stable Fluids 3D+涡量约束+黑体色温 | Stam 1999/Fedkiw 2001 |

### 1.6 nova_render（volumetric_fire）

| 模块 | 核心内容 | 技术来源 |
|------|----------|----------|
| volumetric_fire.rs | Ray marching+Beer-Lambert+黑体RGB | 体积渲染 |

---

## 2. 正在实现的模块（Session 38 第 2-3 批）

### 第 2 批（运行中）

| 子代理 | 目标 crate | 模块 | 核心内容 |
|--------|-----------|------|----------|
| 扩展 biology sci_fic | wasteland_biology | sci_fic_biology.rs | 辐射变异/CRISPR/外星生物/合成生物/共生体/赛博格 |
| 扩展 biomech 老骨骼 | wasteland_biomech | exoskeleton.rs + hydrostatic.rs | 几丁质外骨骼+静水骨骼+蠕动+触手+水母喷射 |

### 第 3 批（运行中）

| 子代理 | 目标 crate | 模块 | 核心内容 |
|--------|-----------|------|----------|
| 扩展 character behavior | wasteland_character | behavior.rs | 觅食/交配/领域/社会等级/群体智能/迁徙/学习 |
| 新建 botany | wasteland_botany (新crate) | 8模块 | 光合作用/解剖/生长/次生代谢/繁殖/生态/根系/物候 |

---

## 3. 规划中的下一批（第 4 批，待启动）

### 3.1 感觉系统 sensory_systems.rs（wasteland_biology）
- 视觉：视网膜/视杆视锥/色觉/暗适应/运动检测
- 听觉：耳蜗/基底膜/频率编码/声源定位
- 嗅觉：嗅觉上皮/嗅球/气味识别
- 味觉：5种基本味觉/味蕾
- 本体感觉：肌梭/高尔基腱器官/前庭
- 痛觉：伤害感受器/痛觉通路
- 电感知/磁感知（部分已在 exotic_biology）

### 3.2 免疫系统详细 immunology.rs（wasteland_biology）
- 固有免疫：补体/巨噬/中性粒/NK细胞
- 适应性免疫：T细胞/B细胞/抗体/记忆
- 细胞因子：白介素/干扰素/趋化因子
- 疫苗响应模型
- 自身免疫
- 过敏反应

### 3.3 发育生物学 development.rs（wasteland_biology）
- 胚胎发生：卵裂/囊胚/原肠胚/神经胚
- 形态发生素：Bicoid/NANOS/SHH/Wnt/FGF
- 同源异形框 Hox 基因
- 干细胞：胚胎/成体/iPS
- 细胞分化谱系

### 3.4 内分泌详细 endocrinology.rs（wasteland_biology）
- HPA轴：下丘脑-垂体-肾上腺
- HPT轴：甲状腺
- HPG轴：性腺
- 生长激素轴
- 泌乳素
- 褪黑素/昼夜节律

### 3.5 神经科学详细 neuroscience.rs（wasteland_biology）
- HH方程：动作电位
- 突触：化学/电，EPSP/IPSP
- 神经递质：谷氨酸/GABA/多巴胺/5-HT/ACh/NE
- 神经回路：反射弧/中枢模式发生器
- 学习记忆：LTP/LTD，Hebbian/STDP

### 3.6 寄生虫学 parasitology.rs（wasteland_biology）
- 生活史：直接/间接（中间宿主）
- 宿主转换
- 免疫逃避策略
- 经典案例：疟原虫/血吸虫/弓形虫/绦虫

### 3.7 古生物学 paleontology.rs（wasteland_biology）
- 化石形成模型
- 灭绝事件（五大灭绝）
- 古DNA降解
- 进化古生物学

### 3.8 真菌学 mycology.rs（wasteland_biology 或新 crate）
- 菌丝网络
- 子实体发育
- 地衣共生（已在 sci_fic_biology 共生体涉及，这里独立深化）
- 真菌毒素
- 发酵

---

## 4. 论文/开源项目映射

### 4.1 已落地论文

| 论文 | 落地模块 | 关键贡献 |
|------|----------|----------|
| Bourdin/Francfort-Marigo 相场法 | biomech/phase_field | (1-φ)²自由能 |
| Prendergast 1997 | biomech/prendergast | 机械调控算子 |
| Wolff 定律 / Carter | biomech/wolff_remodeling | E=3790·ρ³ |
| Zajac 1989 / Millard 2012 | character/muscle | Hill肌肉模型 |
| Stam 1999 Stable Fluids | compute/fluid | 半拉格朗日 |
| Fedkiw 2001 | compute/fluid | 涡量约束 |
| Wajchman 2011 | biology/infection | 凝血3变量ODE |
| Javierre 2008 | biology/wound_healing | 6物种愈合PDE |
| Charnov 边际值定理 | character/behavior | 觅食理论 |
| Reynolds 1987 Boids | character/behavior | 群体智能 |

### 4.2 开源项目参考

| 项目 | 用途 |
|------|------|
| bevy | 渲染架构参考 |
| rend3 | RenderGraph 设计 |
| kajiya | HDR管线 |
| Fyrox | 游戏引擎结构 |
| arxiv-deep-research | 研究流程 |
| langfuse | 追踪架构 |

---

## 5. 编译验证记录

### Session 38 当前状态（截至第 3 批启动）
- ✅ wasteland_biology: 0 errors / 0 warnings（18 模块全通过）
- ✅ wasteland_biomech: 0 errors / 4 warnings（7 模块，non_snake_case 物理符号）
- ✅ wasteland_chemistry: 0 errors / 7 warnings（combustion 物理符号）
- ✅ wasteland_character: 0 errors（6 模块）
- ✅ wasteland_compute: 0 errors / 1 warning（fluid）

### 待验证（子代理完成后）
- ⬜ wasteland_biology + sci_fic_biology（19 模块）
- ⬜ wasteland_biomech + exoskeleton + hydrostatic（9 模块）
- ⬜ wasteland_character + behavior（7 模块）
- ⬜ wasteland_botany（新 crate，8 模块）

---

## 6. 下一步路线图

### 短期（Session 38 余下）
1. 等第 2-3 批 4 个子代理完成，编译验证
2. 启动第 4 批：感觉系统 + 免疫系统 + 发育生物学
3. 启动第 5 批：神经科学 + 内分泌 + 寄生虫学

### 中期（Session 39+）
4. 古生物学 + 真菌学
5. 渲染层对接：皮肤SSS/毛发/植物叶片/火焰（已部分完成）
6. 游戏集成：NPC AI 使用行为学+神经+感觉

### 长期
7. 编辑器可视化这些生物系统
8. 性能优化：SIMD/并行
9. 模块间耦合测试

---

**文档版本**: v1.0
**最后更新**: 2026-06-28
**维护者**: AI 子代理系统
