//! RenderGraph 资源表 — handle → GPU 资源映射
//!
//! 在 RenderGraph::execute 期间，pass 通过 NodeContext.resources 访问
//! 其他 pass 输出的 TextureView / Buffer，无需直接相互依赖。
//!
//! 设计借鉴：
//! - bevy RenderGraph 的 SlotType 资源传递
//! - Granite 的 ResourcePool 句柄表
//!
//! 用法：
//! ```ignore
//! let mut table = ResourceTable::new();
//! table.insert_texture(handle_a, view_a);
//! // pass execute 内：
//! let view = ctx.resources.get_texture(handle_a).unwrap();
//! ```

use super::handle::ResourceHandle;
use hashbrown::HashMap;

/// 资源表：handle → TextureView / Buffer
pub struct ResourceTable {
    textures: HashMap<ResourceHandle, wgpu::TextureView>,
    buffers: HashMap<ResourceHandle, wgpu::Buffer>,
}

impl ResourceTable {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    /// 注册一个 TextureView（pass 输出时调用）
    pub fn insert_texture(&mut self, handle: ResourceHandle, view: wgpu::TextureView) {
        self.textures.insert(handle, view);
    }

    /// 注册一个 Buffer（pass 输出时调用）
    pub fn insert_buffer(&mut self, handle: ResourceHandle, buffer: wgpu::Buffer) {
        self.buffers.insert(handle, buffer);
    }

    /// 查询 TextureView（pass 输入时调用）
    pub fn get_texture(&self, handle: ResourceHandle) -> Option<&wgpu::TextureView> {
        self.textures.get(&handle)
    }

    /// 查询 Buffer
    pub fn get_buffer(&self, handle: ResourceHandle) -> Option<&wgpu::Buffer> {
        self.buffers.get(&handle)
    }

    /// 已注册的 texture 数
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// 已注册的 buffer 数
    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }

    /// 清空（每帧开始前调用）
    pub fn clear(&mut self) {
        self.textures.clear();
        self.buffers.clear();
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_empty_by_default() {
        let t = ResourceTable::new();
        assert_eq!(t.texture_count(), 0);
        assert_eq!(t.buffer_count(), 0);
        assert!(t.get_texture(ResourceHandle::new(0)).is_none());
    }

    #[test]
    fn clear_empties_table() {
        let mut t = ResourceTable::new();
        // 不能不插入 TextureView 就测，所以只测 clear 行为
        t.clear();
        assert_eq!(t.texture_count(), 0);
    }
}
