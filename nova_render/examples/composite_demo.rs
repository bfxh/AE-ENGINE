//! Composite Demo - SkyboxPass + ForwardPass in series, verify LoadOp::Load承接 effect
//!
//! Render flow:
//!   1. SkyboxPass first (LoadOp::Clear clear screen -> procedural sky + atmospheric scattering -> Store)
//!   2. ForwardPass second (LoadOp::Load承接 sky background -> draw cube -> Store)
//!
//! Use RenderGraph::add_edge(skybox -> forward) to guarantee topological order.
//! ForwardPass color attachment uses LoadOp::Load to preserve SkyboxPass output sky,
//! achieving composite render of sky + geometry.
//!
//! Expected: on procedural sky background, a cube with 6 different colored faces hovers, camera rotates around Y axis.
//!
//! Run:
//! ```bash
//! cargo run --example composite_demo --target-dir d:\rj\ae_project\target2
//! ```

use std::cell::Cell;

use nova_render::application::NovaApp;
use nova_render::passes::forward::{
    ForwardPass, ForwardVertex, LightUniform, MeshInstanceData,
};
use nova_render::passes::skybox::SkyboxPass;
use nova_render::render_graph::{Edge, NodeId, RenderGraph, ResourceHandle, ResourceUsage};

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

/// sentinel NodeId, pass not yet added to graph
const UNINIT: NodeId = NodeId(u64::MAX);

fn main() {
    env_logger::try_init().ok();

    let graph = RenderGraph::new();
    let skybox_id = Cell::new(UNINIT);
    let forward_id = Cell::new(UNINIT);

    NovaApp::builder()
        .title("Nova Render - Composite Demo (Skybox + Forward)")
        .size(1280, 720)
        .with_render_graph(graph)
        .on_render(move |app| {
            let sid = skybox_id.get();

            if sid == UNINIT {
                // first frame: get all needed refs, create two passes, finally add_node + add_edge
                let device = app.device();
                let queue = app.queue();
                let color_format = app.surface_format();
                let depth_format = wgpu::TextureFormat::Depth32Float;

                // 1. create SkyboxPass (procedural sky, execute internally self-drives camera/sun)
                let skybox = SkyboxPass::new(device, color_format, depth_format);

                // 2. create ForwardPass (承接 SkyboxPass color output)
                let mut forward = ForwardPass::new(device, color_format, depth_format, 64);
                // enable auto animate camera: execute() rotates around Y axis based on ctx.time (0.5 rad/s)
                forward.auto_animate_camera = true;

                let (verts, idxs) = unit_cube();
                forward.register_mesh(device, &verts, &idxs);

                let light = LightUniform::default_day();
                forward.update_light(queue, &light);

                let instance = MeshInstanceData::from_position_scale(
                    [0.0, 0.0, 0.0],
                    1.0,
                    [1.0, 1.0, 1.0, 1.0],
                );
                forward.update_instances(queue, &[instance]);
                forward.set_instance_count(1);

                // 3. after device/queue all used, add_node + add_edge (avoid borrow conflict)
                let sid_new = app.graph_mut().add_node(Box::new(skybox));
                let fid_new = app.graph_mut().add_node(Box::new(forward));
                app.graph_mut().add_edge(Edge {
                    from: sid_new,
                    to: fid_new,
                    resource: ResourceHandle(0),
                    usage: ResourceUsage::Write,
                });
                skybox_id.set(sid_new);
                forward_id.set(fid_new);

                log::info!(
                    "Composite: SkyboxPass(id={:?}) + ForwardPass(id={:?}) initialized with edge, auto_animate_camera = true",
                    sid_new,
                    fid_new
                );
            }
            // subsequent frames: SkyboxPass and ForwardPass::execute() auto drive (no node_mut borrow needed)
        })
        .build()
        .run();
}
