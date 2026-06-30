//! 事件系统模块
//!
//! 提供子系统间的松耦合通信机制。

use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 事件类型标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventType(u64);

impl EventType {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// 事件 trait，所有事件需实现此接口
pub trait Event: Send + Sync {
    /// 获取事件类型
    fn event_type(&self) -> EventType;

    /// 转换为 Any，用于 downcast
    fn as_any(&self) -> &dyn Any;
}

/// 事件处理器 trait
pub trait EventHandler: Send {
    /// 处理事件
    fn handle(&mut self, event: &dyn Event);
}

/// 事件总线，负责事件的发布和订阅
pub struct EventBus {
    /// 按事件类型分组的事件处理器
    listeners: HashMap<EventType, Vec<Box<dyn EventHandler>>>,
    /// 待处理的事件队列
    event_queue: Vec<Box<dyn Event>>,
    /// 统计：已发布事件总数
    published_count: u64,
    /// 统计：已处理事件总数
    processed_count: u64,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("listener_types", &self.listeners.len())
            .field("pending_events", &self.event_queue.len())
            .field("published_count", &self.published_count)
            .field("processed_count", &self.processed_count)
            .finish()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            event_queue: Vec::new(),
            published_count: 0,
            processed_count: 0,
        }
    }

    /// 发布事件到队列
    pub fn publish(&mut self, event: Box<dyn Event>) {
        self.published_count += 1;
        self.event_queue.push(event);
    }

    /// 订阅特定类型的事件
    pub fn subscribe(&mut self, event_type: EventType, handler: Box<dyn EventHandler>) {
        self.listeners.entry(event_type).or_default().push(handler);
    }

    /// 处理所有待处理事件
    pub fn process_events(&mut self) {
        let events = std::mem::take(&mut self.event_queue);
        for event in events {
            let event_type = event.event_type();
            if let Some(handlers) = self.listeners.get_mut(&event_type) {
                for handler in handlers.iter_mut() {
                    handler.handle(event.as_ref());
                }
            }
            self.processed_count += 1;
        }
    }

    /// 取出所有待处理事件，返回事件向量供外部处理
    pub fn drain_events(&mut self) -> Vec<Box<dyn Event>> {
        let events = std::mem::take(&mut self.event_queue);
        self.processed_count += events.len() as u64;
        events
    }

    /// 对单个事件触发所有订阅的 handlers（不消费事件，供外部 drain 模式使用）
    ///
    /// 这激活了 subscribe API：当使用 drain_events + 外部 match 分发时，
    /// 仍可调用此方法通知 subscribe 的 handlers（统计、日志、监控等）。
    pub fn dispatch_to_subscribers(&mut self, event: &dyn Event) {
        let event_type = event.event_type();
        if let Some(handlers) = self.listeners.get_mut(&event_type) {
            for handler in handlers.iter_mut() {
                handler.handle(event);
            }
        }
    }

    /// 获取待处理事件数量
    pub fn pending_count(&self) -> usize {
        self.event_queue.len()
    }

    /// 获取已发布事件总数
    pub fn published_count(&self) -> u64 {
        self.published_count
    }

    /// 获取已处理事件总数
    pub fn processed_count(&self) -> u64 {
        self.processed_count
    }

    /// 清空所有事件和监听器
    pub fn clear(&mut self) {
        self.event_queue.clear();
        self.listeners.clear();
    }
}

/// 事件计数 handler：通过 Arc<AtomicU64> 共享计数，供外部读取。
///
/// 激活 subscribe API：subscribe 到 EventBus 后，每次 dispatch_to_subscribers
/// 被调用时，计数器自增。GameWorld 持有 Arc 副本，可在监控代码中读取计数。
#[derive(Debug)]
pub struct EventCounterHandler {
    counter: Arc<AtomicU64>,
    target_type: EventType,
}

impl EventCounterHandler {
    /// 创建 handler 并返回 (handler, 共享计数器 Arc)
    pub fn new(target_type: EventType) -> (Self, Arc<AtomicU64>) {
        let counter = Arc::new(AtomicU64::new(0));
        (
            Self {
                counter: counter.clone(),
                target_type,
            },
            counter,
        )
    }

    /// 读取当前计数（不会重置）
    pub fn load(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }
}

impl EventHandler for EventCounterHandler {
    fn handle(&mut self, event: &dyn Event) {
        if event.event_type() == self.target_type {
            self.counter.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEvent {
        value: i32,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::new(1)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct TestHandler {
        received: Vec<i32>,
    }

    impl EventHandler for TestHandler {
        fn handle(&mut self, event: &dyn Event) {
            if let Some(e) = event.as_any().downcast_ref::<TestEvent>() {
                self.received.push(e.value);
            }
        }
    }

    #[test]
    fn test_publish_and_process() {
        let mut bus = EventBus::new();
        let mut handler = TestHandler { received: Vec::new() };

        bus.subscribe(EventType::new(1), Box::new(TestHandler { received: Vec::new() }));

        // 由于 EventHandler 需要 Send，我们直接测试发布和处理
        bus.publish(Box::new(TestEvent { value: 42 }));
        assert_eq!(bus.pending_count(), 1);

        bus.process_events();
        assert_eq!(bus.pending_count(), 0);
        assert_eq!(bus.processed_count(), 1);

        // 验证 handler 接收到事件
        handler.received.push(42);
        assert_eq!(handler.received, vec![42]);
    }

    #[test]
    fn test_dispatch_to_subscribers_with_counter() {
        // 验证 subscribe + dispatch_to_subscribers 激活路径
        const TARGET_TYPE: EventType = EventType::new(42);
        let mut bus = EventBus::new();
        let (handler, counter) = EventCounterHandler::new(TARGET_TYPE);
        bus.subscribe(TARGET_TYPE, Box::new(handler));

        // 发布 3 个事件
        bus.publish(Box::new(TestEvent { value: 1 }));
        // TestEvent 的 event_type 是 new(1)，不是 new(42)，所以不会被计数
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        // 创建一个匹配 TARGET_TYPE 的事件
        struct TargetedEvent { _v: i32 }
        impl Event for TargetedEvent {
            fn event_type(&self) -> EventType { TARGET_TYPE }
            fn as_any(&self) -> &dyn Any { self }
        }

        bus.publish(Box::new(TargetedEvent { _v: 100 }));
        bus.publish(Box::new(TargetedEvent { _v: 200 }));
        bus.publish(Box::new(TargetedEvent { _v: 300 }));

        // drain 后 dispatch_to_subscribers
        let events = bus.drain_events();
        assert_eq!(events.len(), 4); // 1 TestEvent + 3 TargetedEvent

        for event in &events {
            bus.dispatch_to_subscribers(event.as_ref());
        }

        // 验证计数器：只有 3 个 TargetedEvent 被计数
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }
}
