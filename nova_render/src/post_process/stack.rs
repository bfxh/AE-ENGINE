//! EffectStack + 乒乓纹理（借鉴 bevy ViewTarget）

use wgpu::TextureView;

/// Effect Slot（后处理阶段）
pub struct EffectSlot {
    pub name: String,
    pub execute: Box<dyn Fn(&wgpu::Device, &wgpu::Queue, &TextureView, &TextureView) + Send + Sync>,
}

impl EffectSlot {
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&wgpu::Device, &wgpu::Queue, &TextureView, &TextureView) + Send + Sync + 'static,
    {
        Self { name: name.into(), execute: Box::new(f) }
    }
}

/// EffectStack
pub struct EffectStack {
    pub slots: Vec<EffectSlot>,
    pub ping_pong: [Option<TextureView>; 2],
    pub current: usize,
}

impl EffectStack {
    pub fn new() -> Self {
        Self { slots: Vec::new(), ping_pong: [None, None], current: 0 }
    }

    pub fn add(&mut self, slot: EffectSlot) {
        self.slots.push(slot);
    }

    pub fn run(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for slot in &self.slots {
            let (src, dst) = if self.current == 0 {
                (self.ping_pong[0].as_ref().unwrap(), self.ping_pong[1].as_ref().unwrap())
            } else {
                (self.ping_pong[1].as_ref().unwrap(), self.ping_pong[0].as_ref().unwrap())
            };
            (slot.execute)(device, queue, src, dst);
            self.current = 1 - self.current;
        }
    }
}

impl Default for EffectStack {
    fn default() -> Self { Self::new() }
}