//! 强类型 Handle（借鉴 Fyrox + rend3）
//!
//! 设计：
//! - 每个资源类型 T 有独立的 Handle<T>，编译期类型安全
//! - 内部 = Arc<T> + (index, generation) 元数据
//! - 悬空检测：当资源被释放后，generation +1，旧 Handle 查找返回 None
//! - Arc 引用计数：Handle 内部持有 Arc<T>，自动管理生命周期
//!
//! 对比 v1：v1 直接持有 &Texture/&Buffer，借用冲突频繁；
//! v2 用 Handle<T> 间接引用，借用 checker 不再报错。

use std::sync::Arc;
use std::fmt;

/// Handle 错误类型
#[derive(Debug, Clone)]
pub enum HandleError {
    /// 资源已被释放（generation 不匹配）
    Dangled { index: u32, got: u32, expected: u32 },
    /// 资源未找到（index 越界）
    NotFound { index: u32 },
    /// 类型不匹配
    TypeMismatch,
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dangled { index, got, expected } => write!(f, "handle dangled: index {} generation {} != {}", index, got, expected),
            Self::NotFound { index } => write!(f, "handle not found: index {}", index),
            Self::TypeMismatch => write!(f, "handle type mismatch"),
        }
    }
}
impl std::error::Error for HandleError {}

/// 强类型资源 Handle
///
/// - `T`: 资源类型标记（编译期类型安全）
/// - 内部：Arc<T> 共享所有权 + (index, generation) 元数据
pub struct Handle<T: 'static + Send + Sync + ?Sized> {
    inner: Arc<T>,
    meta: HandleMeta,
    _marker: std::marker::PhantomData<fn() -> T>,
}

/// Handle 元数据
#[derive(Clone, Copy)]
pub(crate) struct HandleMeta {
    pub index: u32,
    pub generation: u32,
}

impl<T: 'static + Send + Sync + ?Sized> Handle<T> {
    /// 从 Arc 构造 Handle（由 Pool 调用）
    pub(crate) fn from_arc(index: u32, generation: u32, arc: Arc<T>) -> Self {
        Self {
            inner: arc,
            meta: HandleMeta { index, generation },
            _marker: Default::default(),
        }
    }

    /// 获取索引
    pub fn index(&self) -> u32 {
        self.meta.index
    }

    /// 获取代数
    pub fn generation(&self) -> u32 {
        self.meta.generation
    }

    /// 引用计数
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// 获取资源引用（通过 Deref coercion: &Arc<T> -> &T）
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// 判断是否相同（index + generation）
    pub fn is_same(&self, other: &Self) -> bool {
        self.meta.index == other.meta.index
            && self.meta.generation == other.meta.generation
    }

    /// 转换为 SlotIndex（用于 Pool 查询）
    pub fn slot(&self) -> super::pool::SlotIndex {
        super::pool::SlotIndex {
            index: self.meta.index,
            generation: self.meta.generation,
        }
    }
}

impl<T: 'static + Send + Sync + ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            meta: self.meta,
            _marker: Default::default(),
        }
    }
}

impl<T: 'static + Send + Sync + ?Sized> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("index", &self.meta.index)
            .field("generation", &self.meta.generation)
            .field("type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T: 'static + Send + Sync + ?Sized> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.is_same(other)
    }
}
impl<T: 'static + Send + Sync + ?Sized> Eq for Handle<T> {}

impl<T: 'static + Send + Sync + ?Sized> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.meta.index.hash(state);
        self.meta.generation.hash(state);
    }
}

/// 弱引用 Handle（不增加引用计数）
pub struct WeakHandle<T: 'static + Send + Sync + ?Sized> {
    inner: std::sync::Weak<T>,
    meta: HandleMeta,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static + Send + Sync + ?Sized> WeakHandle<T> {
    /// 升级为强引用 Handle（失败则资源已释放）
    pub fn upgrade(&self) -> Option<Handle<T>> {
        self.inner.upgrade().map(|arc| Handle {
            inner: arc,
            meta: self.meta,
            _marker: Default::default(),
        })
    }
}

impl<T: 'static + Send + Sync + ?Sized> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            meta: self.meta,
            _marker: Default::default(),
        }
    }
}