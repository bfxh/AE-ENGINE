//! 双 World 分离（借鉴 bevy）
//!
//! 设计：
//! - `MainWorld`: 逻辑侧，存储游戏数据、组件、资源（CPU 侧）
//! - `RenderWorld`: 渲染侧，存储 GPU 资源、Extract 后的镜像数据
//! - 每帧 Extract 阶段：MainWorld → RenderWorld 数据同步
//!
//! 对比 v1：v1 直接在 GameWorld 中调用 render，逻辑与渲染耦合；
//! v2 双 World 分离，逻辑可独立测试，渲染线程可独立运行。

use hashbrown::HashMap;
use std::any::{Any, TypeId};

/// World ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldId(usize);

/// World：资源 + 组件容器
///
/// 借鉴 bevy 的 World，但简化为类型擦除的 Resource 存储
pub struct World {
    id: WorldId,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl World {
    pub fn new(id: WorldId) -> Self {
        Self {
            id,
            resources: HashMap::new(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    /// 插入资源
    pub fn insert_resource<R: 'static + Send + Sync>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// 获取资源引用
    pub fn get_resource<R: 'static + Send + Sync>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .and_then(|r| r.downcast_ref::<R>())
    }

    /// 获取资源可变引用
    pub fn get_resource_mut<R: 'static + Send + Sync>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|r| r.downcast_mut::<R>())
    }

    /// 移除资源
    pub fn remove_resource<R: 'static + Send + Sync>(&mut self) -> Option<R> {
        self.resources
            .remove(&TypeId::of::<R>())
            .and_then(|r| r.downcast::<R>().ok())
            .map(|b| *b)
    }
}

/// 主世界（逻辑侧）
pub struct MainWorld(pub World);

impl MainWorld {
    pub fn new() -> Self {
        Self(World::new(WorldId(0)))
    }
}

impl Default for MainWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for MainWorld {
    type Target = World;
    fn deref(&self) -> &World {
        &self.0
    }
}

impl std::ops::DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.0
    }
}

/// 渲染世界（GPU 侧）
pub struct RenderWorld(pub World);

impl RenderWorld {
    pub fn new() -> Self {
        Self(World::new(WorldId(1)))
    }
}

impl Default for RenderWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for RenderWorld {
    type Target = World;
    fn deref(&self) -> &World {
        &self.0
    }
}

impl std::ops::DerefMut for RenderWorld {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.0
    }
}