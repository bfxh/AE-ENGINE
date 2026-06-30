//! 程序化生成系统
//!
//! 突破性程序化建模管线：
//! - `geometry`：基础几何操作（挤出/lathe/sweep/bevel）
//! - `building`：参数化建筑部件（窗框/人字屋顶/阳台/钢筋）+ 建筑语义识别
//! - `skeleton`：骨骼层级 + 蒙皮权重（突破 AnimatedCharacter 扁平无层级限制）
//! - `npc`：12 部位 NPC 程序化生成 + 步行动画 + 形态模板系统
//! - `morph`：母巢子实体 8 种异形生物（虫族/追猎者/碎脊者/锈骑士/蜂群/臃肿者/窃听者/编织者）
//! - `damage`：四级损伤系统 + 生理地图 + 血液流动 + 骨骼破坏
//! - `action`：动作合成引擎 + 原子动作库 + 分层混合器 + 意图解析器
//! - `texture`：PBR 贴图程序化生成（Value/Worley noise + 物理参数映射）
//!
//! 设计目标：超越手工建模的真实感 + 参数化灵活性 + 零外部依赖

pub mod action;
pub mod building;
pub mod damage;
pub mod geometry;
pub mod morph;
pub mod npc;
pub mod skeleton;
pub mod texture;

pub use action::{ActionIntent, ActionType, MotionState, AtomicAction, ActionLibrary, ActionSynthesizer};
pub use building::{
    BalconyParams, BuildingGenerator, BuildingParams, BuildingSemantics, BuildingType,
    DecayState, FunctionZone, GableRoofParams, LoadBearingGraph, RebarParams, StructuralElement,
    StructuralType, WindowParams, ZoneType,
};
pub use damage::{
    BleedingSource, BodyRegion, BodyRegionId, DamageEvent, DamageLevel, DamageType,
    ForeignBody, OrganState, OrganType, PhysiologicalMap,
};
pub use geometry::{bevel_edges, cylinder, lathe_profile, sweep_along_path, CylinderParams};
pub use morph::{
    all_templates, bloated_template, build_skeleton, create_locomotion_animation,
    crusher_template, hunter_template, listener_template, rust_knight_template, stalker_template,
    swarm_template, weaver_template, MorphGenerator,
};
pub use npc::{
    BiologicalMaterial, BodyPlan, HumanoidSkeleton, MorphMutation, MorphParams, MorphTemplate,
    NpcBodyGenerator, NpcBodyParams, SizeClass,
};
pub use skeleton::{Bone, BoneId, JointTransform, Skeleton, SkinWeights};
pub use texture::{NoiseParams, NoiseType, PbrTextureSet, TextureGenerator};
