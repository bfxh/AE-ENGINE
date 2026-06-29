//! Extract 阶段（借鉴 bevy）
//!
//! 设计：
//! - 从 MainWorld 提取渲染所需数据到 RenderWorld
//! - 数据通过 ExtractContext 中转，避免直接借用 MainWorld
//! - Extract 在主线程执行，Prepare/Queue/Render 在渲染线程

use super::world::{MainWorld, RenderWorld};

/// Extract 上下文
pub struct ExtractContext<'a> {
    pub main_world: &'a MainWorld,
    pub render_world: &'a mut RenderWorld,
}

/// Extract 阶段 trait
pub trait ExtractStage: Send + Sync {
    /// 从 MainWorld 提取数据到 RenderWorld
    fn extract(&self, ctx: &mut ExtractContext);
}

/// 默认 Extract Pipeline
pub struct ExtractPipeline {
    stages: Vec<Box<dyn ExtractStage>>,
}

impl ExtractPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: Box<dyn ExtractStage>) {
        self.stages.push(stage);
    }

    pub fn run(&self, main_world: &MainWorld, render_world: &mut RenderWorld) {
        for stage in &self.stages {
            // 注意：&mut T 在结构体字面量中不会自动重借用，需显式 &mut *render_world
            let mut ctx = ExtractContext {
                main_world,
                render_world: &mut *render_world,
            };
            stage.extract(&mut ctx);
        }
    }
}

impl Default for ExtractPipeline {
    fn default() -> Self {
        Self::new()
    }
}