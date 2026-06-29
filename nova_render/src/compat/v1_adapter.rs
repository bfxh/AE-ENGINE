//! V1 Adapter
//!
//! 将 v1 wasteland_render 的资源类型转换为 nova_render 类型

use crate::assets::{Mesh, MeshData, Vertex};

/// V1 适配器
pub struct V1Adapter;

impl V1Adapter {
    /// 从 v1 Vertex 转换为 nova Vertex
    pub fn convert_vertex(
        position: [f32; 3],
        normal: [f32; 3],
        tangent: [f32; 4],
        uv: [f32; 2],
        color: [f32; 4],
    ) -> Vertex {
        Vertex {
            position,
            _pad0: 0.0,
            normal,
            _pad1: 0.0,
            tangent,
            uv,
            _pad2: 0.0,
            color,
        }
    }

    /// 从 v1 mesh 数据转换为 nova MeshData
    pub fn convert_mesh(
        vertices: impl IntoIterator<Item = ([f32; 3], [f32; 3], [f32; 4], [f32; 2], [f32; 4])>,
        indices: Vec<u32>,
    ) -> MeshData {
        let vertices: Vec<Vertex> = vertices
            .into_iter()
            .map(|(p, n, t, uv, c)| Self::convert_vertex(p, n, t, uv, c))
            .collect();
        MeshData { vertices, indices }
    }

    /// 创建 GPU Mesh
    pub fn create_mesh(device: &wgpu::Device, data: &MeshData, label: Option<&str>) -> Mesh {
        Mesh::from_data(device, data, label)
    }
}

/// V1 Mesh 转换器
pub struct V1MeshConverter;

/// V1 Texture 转换器
pub struct V1TextureConverter;