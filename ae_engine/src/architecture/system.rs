//! 系统调度器模块
//!
//! 提供统一的系统注册、依赖管理和调度执行机制。

use std::collections::HashMap;

use super::context::WorldContext;

/// 系统唯一标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(u64);

impl SystemId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for SystemId {
    fn default() -> Self {
        Self::new()
    }
}

/// 系统优先级，数值越小优先级越高
pub type Priority = i32;

/// 系统层级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemLayer {
    /// 数据层：资源加载、持久化
    Data,
    /// 模拟层：物理、化学、生物等模拟
    Simulation,
    /// 游戏逻辑层：实体管理、交互
    GameLogic,
    /// 渲染层：视觉呈现
    Rendering,
}

/// 系统 trait，所有子系统需实现此接口
pub trait System: Send {
    /// 系统名称
    fn name(&self) -> &str;

    /// 系统所属层级
    fn layer(&self) -> SystemLayer;

    /// 系统优先级（同层内按优先级排序，数值越小越先执行）
    fn priority(&self) -> Priority {
        0
    }

    /// 依赖的其他系统 ID
    fn dependencies(&self) -> Vec<SystemId> {
        Vec::new()
    }

    /// 更新系统状态
    fn update(&mut self, dt: f32, ctx: &WorldContext);
}

/// 系统调度器，负责管理所有子系统的注册和执行
pub struct SystemScheduler {
    systems: HashMap<SystemId, Box<dyn System>>,
    execution_order: Vec<SystemId>,
    next_id: u64,
}

impl std::fmt::Debug for SystemScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemScheduler")
            .field("system_count", &self.systems.len())
            .field("execution_order_len", &self.execution_order.len())
            .finish()
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self { systems: HashMap::new(), execution_order: Vec::new(), next_id: 0 }
    }

    /// 注册一个新系统，返回其 ID
    pub fn register(&mut self, system: Box<dyn System>) -> SystemId {
        let id = SystemId(self.next_id);
        self.next_id += 1;
        self.systems.insert(id, system);
        self.rebuild_execution_order();
        id
    }

    /// 注销系统
    pub fn unregister(&mut self, id: SystemId) -> Option<Box<dyn System>> {
        let result = self.systems.remove(&id);
        if result.is_some() {
            self.rebuild_execution_order();
        }
        result
    }

    /// 获取系统数量
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// 按层级和优先级重建执行顺序
    fn rebuild_execution_order(&mut self) {
        let mut entries: Vec<(SystemId, SystemLayer, Priority)> =
            self.systems.iter().map(|(&id, sys)| (id, sys.layer(), sys.priority())).collect();

        // 按层级排序，同层内按优先级排序
        entries.sort_by(|a, b| {
            let layer_a = layer_order(a.1);
            let layer_b = layer_order(b.1);
            layer_a
                .cmp(&layer_b)
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.0.raw().cmp(&b.0.raw()))
        });

        self.execution_order = entries.into_iter().map(|(id, _, _)| id).collect();
    }

    /// 按执行顺序更新所有系统
    pub fn update(&mut self, dt: f32, ctx: &WorldContext) {
        let order = self.execution_order.clone();
        for &id in &order {
            if let Some(system) = self.systems.get_mut(&id) {
                system.update(dt, ctx);
            }
        }
    }

    /// 仅更新指定层级的系统
    pub fn update_layer(&mut self, dt: f32, ctx: &WorldContext, layer: SystemLayer) {
        let order = self.execution_order.clone();
        for &id in &order {
            if let Some(system) = self.systems.get(&id) {
                if system.layer() == layer {
                    if let Some(system) = self.systems.get_mut(&id) {
                        system.update(dt, ctx);
                    }
                }
            }
        }
    }

    /// 获取所有系统名称
    pub fn system_names(&self) -> Vec<&str> {
        self.execution_order
            .iter()
            .filter_map(|id| self.systems.get(id).map(|s| s.name()))
            .collect()
    }
}

/// 层级排序值
fn layer_order(layer: SystemLayer) -> u8 {
    match layer {
        SystemLayer::Data => 0,
        SystemLayer::Simulation => 1,
        SystemLayer::GameLogic => 2,
        SystemLayer::Rendering => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSystem {
        name: String,
        layer: SystemLayer,
        update_count: u32,
    }

    impl System for TestSystem {
        fn name(&self) -> &str {
            &self.name
        }
        fn layer(&self) -> SystemLayer {
            self.layer
        }
        fn update(&mut self, _dt: f32, _ctx: &WorldContext) {
            self.update_count += 1;
        }
    }

    #[test]
    fn test_register_and_update() {
        let mut scheduler = SystemScheduler::new();
        let ctx = WorldContext::new();

        let _id = scheduler.register(Box::new(TestSystem {
            name: "test".into(),
            layer: SystemLayer::Simulation,
            update_count: 0,
        }));

        assert_eq!(scheduler.len(), 1);
        scheduler.update(0.016, &ctx);
    }

    #[test]
    fn test_layer_ordering() {
        let mut scheduler = SystemScheduler::new();
        let _ctx = WorldContext::new();

        scheduler.register(Box::new(TestSystem {
            name: "render".into(),
            layer: SystemLayer::Rendering,
            update_count: 0,
        }));
        scheduler.register(Box::new(TestSystem {
            name: "data".into(),
            layer: SystemLayer::Data,
            update_count: 0,
        }));
        scheduler.register(Box::new(TestSystem {
            name: "sim".into(),
            layer: SystemLayer::Simulation,
            update_count: 0,
        }));

        let names = scheduler.system_names();
        assert_eq!(names, vec!["data", "sim", "render"]);
    }
}
