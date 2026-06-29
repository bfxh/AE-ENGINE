//! Mesh 资源（借鉴 rend3 Megabuffer）
//!
//! 设计：
//! - MeshData：CPU 侧顶点/索引数据
//! - Mesh：GPU 侧 Buffer（vertex + index）
//! - Megabuffer：多个小 Mesh 合并到大 Buffer，减少 binding 切换

use bytemuck::{Pod, Zeroable};
use crate::core::Handle;

/// 顶点属性（WGSL 对齐：vec2 需 8-byte，vec3 12-byte，vec4 16-byte）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub _pad0: f32,
    pub normal: [f32; 3],
    pub _pad1: f32,
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub _pad2: f32,
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(pos: [f32; 3]) -> Self {
        Self {
            position: pos,
            _pad0: 0.0,
            normal: [0.0, 1.0, 0.0],
            _pad1: 0.0,
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            _pad2: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
    pub fn with_normal(mut self, n: [f32; 3]) -> Self { self.normal = n; self }
    pub fn with_uv(mut self, uv: [f32; 2]) -> Self { self.uv = uv; self }
    pub fn with_color(mut self, c: [f32; 4]) -> Self { self.color = c; self }
    pub fn with_tangent(mut self, t: [f32; 4]) -> Self { self.tangent = t; self }
}

/// CPU 侧 Mesh 数据
#[derive(Debug, Clone)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
    pub fn empty() -> Self { Self { vertices: Vec::new(), indices: Vec::new() } }
}

/// GPU 侧 Mesh
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
}

/// Mesh Handle
pub type MeshHandle = Handle<Mesh>;

impl Mesh {
    pub fn from_data(device: &wgpu::Device, data: &MeshData, label: Option<&str>) -> Self {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            vertex_buffer,
            index_buffer,
            num_vertices: data.vertices.len() as u32,
            num_indices: data.indices.len() as u32,
        }
    }
}