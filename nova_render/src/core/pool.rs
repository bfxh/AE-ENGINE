//! Generational Arena Pool（借鉴 Fyrox）
//!
//! 设计：
//! - 固定容量 Vec<PoolEntry<T>>，O(1) 索引访问
//! - 每个槽位带 generation，删除后 +1，悬空检测
//! - 空闲链表：free_head 链表，分配/释放 O(1)
//! - 借鉴 Fyrox 的 Pool<T>，但简化为非线程安全（单线程访问）
//!
//! 对比 v1：v1 用 Vec 直接存资源，删除后 index 失效；
//! v2 用 generation 检测，旧 Handle 自动失效。

use super::handle::{Handle, HandleError};
use std::sync::Arc;

/// 槽位索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotIndex {
    pub index: u32,
    pub generation: u32,
}

/// 池条目
pub(crate) enum PoolEntry<T: 'static + Send + Sync> {
    /// 占用
    Occupied { generation: u32, resource: Arc<T> },
    /// 空闲（generation 已 +1，等待重用）
    Vacant { generation: u32, next_free: Option<u32> },
}

/// Generational Arena Pool
pub struct Pool<T: 'static + Send + Sync> {
    entries: Vec<PoolEntry<T>>,
    free_head: Option<u32>,
    len: u32,
}

impl<T: 'static + Send + Sync> Pool<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_head: None,
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            free_head: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 分配新资源，返回 Handle
    pub fn spawn(&mut self, resource: T) -> Handle<T> {
        self.len += 1;
        let arc = Arc::new(resource);
        match self.free_head {
            Some(index) => {
                // 复用空闲槽位
                let generation = match &self.entries[index as usize] {
                    PoolEntry::Vacant { generation, next_free } => {
                        self.free_head = *next_free;
                        *generation
                    }
                    _ => unreachable!("free_head points to Occupied"),
                };
                let arc_clone = Arc::clone(&arc);
                self.entries[index as usize] = PoolEntry::Occupied {
                    generation,
                    resource: arc_clone,
                };
                Handle::from_arc(index, generation, arc)
            }
            None => {
                // 追加新槽位
                let index = self.entries.len() as u32;
                let generation = 0u32;
                let arc_clone = Arc::clone(&arc);
                self.entries.push(PoolEntry::Occupied {
                    generation,
                    resource: arc_clone,
                });
                Handle::from_arc(index, generation, arc)
            }
        }
    }

    /// 通过 SlotIndex 获取资源引用
    pub fn get(&self, index: SlotIndex) -> Result<&Arc<T>, HandleError> {
        let entry = self.entries
            .get(index.index as usize)
            .ok_or(HandleError::NotFound { index: index.index })?;
        match entry {
            PoolEntry::Occupied { generation, resource } if *generation == index.generation => Ok(resource),
            PoolEntry::Occupied { generation, .. } => Err(HandleError::Dangled {
                index: index.index,
                got: index.generation,
                expected: *generation,
            }),
            PoolEntry::Vacant { generation, .. } => Err(HandleError::Dangled {
                index: index.index,
                got: index.generation,
                expected: *generation,
            }),
        }
    }

    /// 通过 SlotIndex 获取资源可变引用
    pub fn get_mut(&mut self, index: SlotIndex) -> Result<&mut Arc<T>, HandleError> {
        let entry = self.entries
            .get_mut(index.index as usize)
            .ok_or(HandleError::NotFound { index: index.index })?;
        match entry {
            PoolEntry::Occupied { generation, resource } if *generation == index.generation => Ok(resource),
            PoolEntry::Occupied { generation, .. } => Err(HandleError::Dangled {
                index: index.index,
                got: index.generation,
                expected: *generation,
            }),
            PoolEntry::Vacant { generation, .. } => Err(HandleError::Dangled {
                index: index.index,
                got: index.generation,
                expected: *generation,
            }),
        }
    }

    /// 释放资源（generation +1，加入空闲链表）
    pub fn free(&mut self, index: SlotIndex) -> Result<(), HandleError> {
        let entry = self.entries
            .get_mut(index.index as usize)
            .ok_or(HandleError::NotFound { index: index.index })?;
        // 先读取当前 generation 和占用状态，避免在 match 中借用冲突
        let (current_gen, is_occupied) = match entry {
            PoolEntry::Occupied { generation, .. } => (*generation, true),
            PoolEntry::Vacant { generation, .. } => (*generation, false),
        };
        if is_occupied && current_gen == index.generation {
            let new_generation = current_gen + 1;
            *entry = PoolEntry::Vacant {
                generation: new_generation,
                next_free: self.free_head,
            };
            self.free_head = Some(index.index);
            self.len -= 1;
            Ok(())
        } else {
            Err(HandleError::Dangled {
                index: index.index,
                got: index.generation,
                expected: current_gen,
            })
        }
    }

    /// 迭代所有有效资源
    pub fn iter(&self) -> impl Iterator<Item = (SlotIndex, &Arc<T>)> {
        self.entries.iter().enumerate().filter_map(|(i, e)| {
            if let PoolEntry::Occupied { generation, resource } = e {
                Some((SlotIndex {
                    index: i as u32,
                    generation: *generation,
                }, resource))
            } else {
                None
            }
        })
    }
}

impl<T: 'static + Send + Sync> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}