//! Forward Demo - verify ForwardPass::execute() end-to-end rendering (auto animate camera version)
//!
//! Launch winit window -> first frame create ForwardPass + register cube mesh +
//! set auto_animate_camera = true -> each frame execute() auto updates camera based on ctx.time -> present
//!
//! Expected: window shows a cube with 6 different colored faces, camera slowly rotates around Y axis.
//!
//! Run:
//! ```bash
//! cargo run --example forward_demo --target-dir d:\rj\wasteland_project\target2
//! ```

use std::cell::Cell;

use nova_render::application::NovaApp;
use nova_render::passes::forward::{
    ForwardPass, ForwardVertex, LightUniform, MeshInstanceData,
};
use nova_render::render_graph::{NodeId, RenderGraph};

/// Generate unit cube (edge length 2, centered at origin): 24 vertices + 36 indices
fn unit_cube() -> (Vec<ForwardVertex>, Vec<u32>) {
    let faces: [([f32; 3], [[f32; 3]; 4], [f32; 4]); 6] = [
        ([1.0, 0.0, 0.0], [[1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]], [1.0, 0.2, 0.2, 1.0]),
        ([-1.0, 0.0, 0.0], [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, 1.0, -1.0], [-1.0, -1.0, -1.0]], [0.2, 1.0, 0.2, 1.0]),
        ([0.0, 1.0, 0.0], [[-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0]], [0.2, 0.2, 1.0, 1.0]),
        ([0.0, -1.0, 0.0], [[-1.0, -1.0, 1.0], [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0]], [1.0, 1.0, 0.2, 1.0]),
        ([0.0, 0.0, 1.0], [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]], [0.8, 0.2, 0.8, 1.0]),
        ([0.0, 0.0, -1.0], [[1.0, -1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0]], [0.2, 0.8, 0.8, 1.0]),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (face_idx, (normal, pts, color)) in faces.iter().enumerate() {
        let base = (face_idx * 4) as u32;
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for (i, pt) in pts.iter().enumerate() {
            vertices.push(ForwardVertex {
                position: *pt,
                _pad1: 0.0,
                normal: *normal,
                _pad2: 0.0,
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: uvs[i],
                _pad3: [0.0; 2],
                color: *color,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

/// sentinel NodeId, ForwardPass not yet added to graph
const UNINIT: NodeId = NodeId(u64::MAX);

fn main() {
    env_logger::try_init().ok();

    let graph = RenderGraph::new();
    let forward_id = Cell::new(UNINIT);

    NovaApp::builder()
        .title("Nova Render - Forward Demo (Auto Animate Camera)")
        .size(1280, 720)
        .with_render_graph(graph)
        .on_render(move |app| {
            let id = forward_id.get();
            if id == UNINIT {
                // first frame: create and configure ForwardPass, enable auto animate camera
                let device = app.device();
                let queue = app.queue();
                let color_format = app.surface_format();
                let depth_format = wgpu::TextureFormat::Depth32Float;

                let mut pass = ForwardPass::new(device, color_format, depth_format, 64);
                // enable auto animate camera: execute() rotates around Y axis based on ctx.time (0.5 rad/s)
                pass.auto_animate_camera = true;

                let (verts, idxs) = unit_cube();
                pass.register_mesh(device, &verts, &idxs);

                let light = LightUniform::default_day();
                pass.update_light(queue, &light);

                let instance = MeshInstanceData::from_position_scale(
                    [0.0, 0.0, 0.0],
                    1.0,
                    [1.0, 1.0, 1.0, 1.0],
                );
                pass.update_instances(queue, &[instance]);
                pass.set_instance_count(1);

                log::info!("ForwardPass initialized: 1 cube mesh, auto_animate_camera = true");
                let nid = app.graph_mut().add_node(Box::new(pass));
                forward_id.set(nid);
            }
            // subsequent frames: ForwardPass::execute() auto animates camera (no node_mut borrow needed)
        })
        .build()
        .run();
}
