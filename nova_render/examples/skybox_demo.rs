//! Skybox Demo — 验证 RenderGraph 端到端管线
//!
//! 启动 winit 窗口 → wgpu 初始化 → RenderGraph 调度 SkyboxPass → present
//!
//! 预期效果：窗口显示程序化大气散射天空盒，相机绕 Y 轴缓慢旋转。
//!
//! 运行：
//! ```bash
//! cargo run --example skybox_demo --features compat-v1 --target-dir d:\rj\ae_project\target2
//! ```

use nova_render::application::NovaApp;
use nova_render::passes::SkyboxPass;
use nova_render::render_graph::RenderGraph;

fn main() {
    env_logger::try_init().ok();

    let graph = RenderGraph::new();

    NovaApp::builder()
        .title("Nova Render — Skybox Demo")
        .size(1280, 720)
        .with_render_graph(graph)
        .on_render(|app| {
            // 首帧：创建 SkyboxPass 并加入 RenderGraph
            // （需等待 wgpu device + surface 就绪后才能创建 pipeline）
            if app.graph().node_count() == 0 {
                let device = app.device();
                let color_format = app.surface_format();
                // Depth32Float：通用深度格式，支持 depth_compare
                let depth_format = wgpu::TextureFormat::Depth32Float;
                let pass = SkyboxPass::new(device, color_format, depth_format);
                app.graph_mut().add_node(Box::new(pass));
            }
        })
        .build()
        .run();
}
