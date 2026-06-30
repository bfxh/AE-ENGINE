//! 网格数据：顶点、索引、GPU 缓冲

use bytemuck::{Pod, Zeroable};
use wgpu::{Buffer, Device, util::DeviceExt};

/// 标准顶点：位置 + 法线 + 切线 + UV + 颜色
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x4,
        3 => Float32x2,
        4 => Float32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Vertex::ATTRIBS,
    };

    pub fn new(pos: [f32; 3]) -> Self {
        Self {
            position: pos,
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_normal(mut self, n: [f32; 3]) -> Self {
        self.normal = n;
        self
    }
    pub fn with_uv(mut self, uv: [f32; 2]) -> Self {
        self.uv = uv;
        self
    }
    pub fn with_color(mut self, c: [f32; 4]) -> Self {
        self.color = c;
        self
    }
    pub fn with_tangent(mut self, t: [f32; 4]) -> Self {
        self.tangent = t;
        self
    }
}

/// 已上传到 GPU 的网格
pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
}

impl Mesh {
    pub fn from_data(
        device: &Device,
        vertices: &[Vertex],
        indices: &[u32],
        label: Option<&str>,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: label.map(|s| format!("{s} indices")).as_deref(),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            num_vertices: vertices.len() as u32,
        }
    }
}

/// CPU 端网格构建器
pub struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self { vertices: Vec::new(), indices: Vec::new() }
    }

    pub fn with_capacity(v: usize, i: usize) -> Self {
        Self { vertices: Vec::with_capacity(v), indices: Vec::with_capacity(i) }
    }

    pub fn push_vertex(&mut self, v: Vertex) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(v);
        idx
    }

    pub fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    pub fn push_quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.indices.extend_from_slice(&[a, b, c, a, c, d]);
    }

    /// 推入一个带法线的四边形面（4 顶点 + 2 三角形）。
    /// 顶点顺序：v0→v1→v2→v3 形成四边形（按右手法则确定法线方向）。
    pub fn push_quad_face(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
        normal: [f32; 3],
        uv_scale: [f32; 2],
    ) {
        let i0 = self.push_vertex(Vertex::new(v0).with_normal(normal).with_uv([0.0, 0.0]));
        let i1 = self.push_vertex(Vertex::new(v1).with_normal(normal).with_uv([uv_scale[0], 0.0]));
        let i2 = self.push_vertex(Vertex::new(v2).with_normal(normal).with_uv(uv_scale));
        let i3 = self.push_vertex(Vertex::new(v3).with_normal(normal).with_uv([0.0, uv_scale[1]]));
        self.push_quad(i0, i1, i2, i3);
    }

    /// 推入一个参数化盒子（中心 + 三轴尺寸）。
    /// 比 cube() 更灵活：支持任意位置和尺寸，UV 按 0..1 映射到各面。
    pub fn push_box(&mut self, center: [f32; 3], extent: [f32; 3]) {
        let [cx, cy, cz] = center;
        let [ex, ey, ez] = extent;
        let hx = ex * 0.5;
        let hy = ey * 0.5;
        let hz = ez * 0.5;
        // 6 个面，每面 4 顶点 + 2 三角形
        // +X
        self.push_quad_face(
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz + hz],
            [cx + hx, cy - hy, cz + hz],
            [1.0, 0.0, 0.0],
            [ex, ez],
        );
        // -X
        self.push_quad_face(
            [cx - hx, cy - hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz - hz],
            [cx - hx, cy - hy, cz - hz],
            [-1.0, 0.0, 0.0],
            [ex, ez],
        );
        // +Y
        self.push_quad_face(
            [cx - hx, cy + hy, cz - hz],
            [cx - hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz - hz],
            [0.0, 1.0, 0.0],
            [ex, ez],
        );
        // -Y
        self.push_quad_face(
            [cx - hx, cy - hy, cz + hz],
            [cx - hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz + hz],
            [0.0, -1.0, 0.0],
            [ex, ez],
        );
        // +Z
        self.push_quad_face(
            [cx + hx, cy - hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
            [cx - hx, cy - hy, cz + hz],
            [0.0, 0.0, 1.0],
            [ex, ey],
        );
        // -Z
        self.push_quad_face(
            [cx - hx, cy - hy, cz - hz],
            [cx - hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
            [cx + hx, cy - hy, cz - hz],
            [0.0, 0.0, -1.0],
            [ex, ey],
        );
    }

    /// 挤出 2D 多边形（XY 平面）沿 Z 轴生成 3D 几何。
    /// `polygon` 为顺时针或逆时针的 2D 点序列（至少 3 点）。
    /// 返回是否成功（点数不足或挤出长度为零则失败）。
    pub fn extrude_polygon(
        &mut self,
        polygon: &[[f32; 2]],
        extrude_z: f32,
        uv_scale: [f32; 2],
    ) -> bool {
        if polygon.len() < 3 || extrude_z.abs() < 1e-6 {
            return false;
        }
        let n = polygon.len() as u32;
        // 顶面 + 底面顶点（带共享 UV）
        // 底面 (z=0)：法线 -Z
        let bottom_start = self.vertices.len() as u32;
        for p in polygon {
            self.vertices.push(
                Vertex::new([p[0], p[1], 0.0])
                    .with_normal([0.0, 0.0, -1.0])
                    .with_uv([p[0] * uv_scale[0], p[1] * uv_scale[1]]),
            );
        }
        // 顶面 (z=extrude_z)：法线 +Z
        let top_start = bottom_start + n;
        for p in polygon {
            self.vertices.push(
                Vertex::new([p[0], p[1], extrude_z])
                    .with_normal([0.0, 0.0, 1.0])
                    .with_uv([p[0] * uv_scale[0], p[1] * uv_scale[1]]),
            );
        }
        // 底面（按 polygon 顺序逆时针 → 三角形为 (i, i+1, i+2) reverse）
        // 简化：扇形三角化，法线已设
        for i in 1..n - 1 {
            self.indices.extend_from_slice(&[bottom_start, bottom_start + i, bottom_start + i + 1]);
        }
        // 顶面
        for i in 1..n - 1 {
            self.indices.extend_from_slice(&[top_start, top_start + i + 1, top_start + i]);
        }
        // 侧面：每条边形成四边形（bottom_i, bottom_i+1, top_i+1, top_i）
        for i in 0..n {
            let i_next = (i + 1) % n;
            let b0 = bottom_start + i;
            let b1 = bottom_start + i_next;
            let t1 = top_start + i_next;
            let t0 = top_start + i;
            // 计算法线（边方向 × Z）
            let edge = [
                polygon[i_next as usize][0] - polygon[i as usize][0],
                polygon[i_next as usize][1] - polygon[i as usize][1],
                0.0,
            ];
            let normal = [edge[1], -edge[0], 0.0];
            let len = (normal[0].powi(2) + normal[1].powi(2)).sqrt();
            let normal = if len > 1e-6 {
                [normal[0] / len, normal[1] / len, 0.0]
            } else {
                [0.0, 1.0, 0.0]
            };
            // 覆盖已有顶点法线（简化处理：直接修改）
            // 为了保持 API 简单，创建新顶点用于侧面（避免法线冲突）
            let s0 = self.push_vertex(
                Vertex::new([polygon[i as usize][0], polygon[i as usize][1], 0.0])
                    .with_normal(normal)
                    .with_uv([0.0, 0.0]),
            );
            let s1 = self.push_vertex(
                Vertex::new([polygon[i_next as usize][0], polygon[i_next as usize][1], 0.0])
                    .with_normal(normal)
                    .with_uv([1.0, 0.0]),
            );
            let s2 = self.push_vertex(
                Vertex::new([polygon[i_next as usize][0], polygon[i_next as usize][1], extrude_z])
                    .with_normal(normal)
                    .with_uv([1.0, extrude_z * uv_scale[1]]),
            );
            let s3 = self.push_vertex(
                Vertex::new([polygon[i as usize][0], polygon[i as usize][1], extrude_z])
                    .with_normal(normal)
                    .with_uv([0.0, extrude_z * uv_scale[1]]),
            );
            self.push_quad(s0, s1, s2, s3);
        }
        true
    }

    /// 合并另一个 builder 的顶点和索引（偏移索引）。
    pub fn append(&mut self, other: &MeshBuilder) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        for &i in &other.indices {
            self.indices.push(i + offset);
        }
    }

    /// 对所有顶点应用变换（位置 + 法线旋转）。
    /// `translation` + `rotation`(Quat) + `scale`(uniform)。
    pub fn transform(&mut self, translation: [f32; 3], rotation: [f32; 4], scale: f32) {
        let quat = glam::Quat::from_vec4(glam::Vec4::from(rotation));
        let t = glam::Vec3::from(translation);
        for v in &mut self.vertices {
            let p = glam::Vec3::from(v.position) * scale;
            let p = quat * p + t;
            v.position = p.into();
            let n = quat * glam::Vec3::from(v.normal);
            v.normal = n.into();
            // tangent.xyz 也旋转
            let tan = glam::Vec3::from([v.tangent[0], v.tangent[1], v.tangent[2]]);
            let tan = quat * tan;
            v.tangent[0] = tan.x;
            v.tangent[1] = tan.y;
            v.tangent[2] = tan.z;
        }
    }

    /// 重新计算所有三角形的法线（覆盖每顶点法线，平滑模式）。
    /// 用于无明确法线的几何（如导入后修复）。
    pub fn recompute_smooth_normals(&mut self) {
        let mut normals = vec![[0.0f32; 3]; self.vertices.len()];
        for tri in self.indices.chunks_exact(3) {
            let a = self.vertices[tri[0] as usize].position;
            let b = self.vertices[tri[1] as usize].position;
            let c = self.vertices[tri[2] as usize].position;
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let len = (n[0].powi(2) + n[1].powi(2) + n[2].powi(2)).sqrt().max(1e-12);
            let n = [n[0] / len, n[1] / len, n[2] / len];
            for &i in tri {
                normals[i as usize][0] += n[0];
                normals[i as usize][1] += n[1];
                normals[i as usize][2] += n[2];
            }
        }
        for (v, n) in self.vertices.iter_mut().zip(normals.iter()) {
            let len = (n[0].powi(2) + n[1].powi(2) + n[2].powi(2)).sqrt().max(1e-12);
            v.normal = [n[0] / len, n[1] / len, n[2] / len];
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn build(self, device: &Device, label: Option<&str>) -> Mesh {
        Mesh::from_data(device, &self.vertices, &self.indices, label)
    }

    pub fn into_parts(self) -> (Vec<Vertex>, Vec<u32>) {
        (self.vertices, self.indices)
    }

    /// 借用视图（用于 procedural 模块读取已构建的几何）
    pub fn parts(&self) -> (&[Vertex], &[u32]) {
        (&self.vertices, &self.indices)
    }

    /// 可变借用顶点（用于 procedural 模块修改顶点颜色等）
    pub fn vertices_mut(&mut self) -> &mut Vec<Vertex> {
        &mut self.vertices
    }

    /// 可变借用索引
    pub fn indices_mut(&mut self) -> &mut Vec<u32> {
        &mut self.indices
    }

    /// 生成立方体网格
    pub fn cube() -> (Vec<Vertex>, Vec<u32>) {
        let mut b = Self::with_capacity(24, 36);
        let s = 0.5f32;
        let faces = [
            // +X
            ([s, -s, -s], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([s, s, -s], [1.0, 0.0, 0.0], [1.0, 0.0]),
            ([s, s, s], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([s, -s, s], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // -X
            ([-s, -s, s], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([-s, s, s], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([-s, s, -s], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            ([-s, -s, -s], [-1.0, 0.0, 0.0], [0.0, 1.0]),
            // +Y
            ([-s, s, -s], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([-s, s, s], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([s, s, s], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ([s, s, -s], [0.0, 1.0, 0.0], [0.0, 1.0]),
            // -Y
            ([-s, -s, s], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([s, -s, s], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([s, -s, -s], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([-s, -s, -s], [0.0, -1.0, 0.0], [0.0, 1.0]),
            // +Z
            ([s, -s, s], [0.0, 0.0, 1.0], [0.0, 0.0]),
            ([s, s, s], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([-s, s, s], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([-s, -s, s], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // -Z
            ([-s, -s, -s], [0.0, 0.0, -1.0], [0.0, 0.0]),
            ([-s, s, -s], [0.0, 0.0, -1.0], [1.0, 0.0]),
            ([s, s, -s], [0.0, 0.0, -1.0], [1.0, 1.0]),
            ([s, -s, -s], [0.0, 0.0, -1.0], [0.0, 1.0]),
        ];
        for (pos, normal, uv) in faces.iter() {
            b.push_vertex(Vertex::new(*pos).with_normal(*normal).with_uv(*uv));
        }
        for i in 0..6u32 {
            let base = i * 4;
            b.push_quad(base, base + 1, base + 2, base + 3);
        }
        b.into_parts()
    }

    /// 生成 UV 球网格
    pub fn sphere(segments: u32, rings: u32) -> (Vec<Vertex>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for r in 0..=rings {
            let theta = std::f32::consts::PI * r as f32 / rings as f32;
            let sin_t = theta.sin();
            let cos_t = theta.cos();
            for s in 0..=segments {
                let phi = 2.0 * std::f32::consts::PI * s as f32 / segments as f32;
                let sin_p = phi.sin();
                let cos_p = phi.cos();
                let pos = [sin_t * cos_p, cos_t, sin_t * sin_p];
                verts.push(
                    Vertex::new(pos)
                        .with_normal(pos)
                        .with_uv([s as f32 / segments as f32, r as f32 / rings as f32]),
                );
            }
        }
        for r in 0..rings {
            for s in 0..segments {
                let a = r * (segments + 1) + s;
                let b = a + segments + 1;
                idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
        (verts, idx)
    }

    /// 生成平面网格（XZ 平面，size x size 分段）
    pub fn plane(size: f32, segments: u32) -> (Vec<Vertex>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        let half = size * 0.5;
        let step = size / segments as f32;
        for z in 0..=segments {
            for x in 0..=segments {
                let px = -half + x as f32 * step;
                let pz = -half + z as f32 * step;
                verts.push(
                    Vertex::new([px, 0.0, pz])
                        .with_uv([x as f32 / segments as f32, z as f32 / segments as f32]),
                );
            }
        }
        for z in 0..segments {
            for x in 0..segments {
                let a = z * (segments + 1) + x;
                let b = a + segments + 1;
                idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
        (verts, idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_size_aligned() {
        // 3+3+4+2+4 = 16 floats = 64 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 64);
    }

    #[test]
    fn cube_mesh_has_correct_counts() {
        let (v, i) = MeshBuilder::cube();
        assert_eq!(v.len(), 24);
        assert_eq!(i.len(), 36);
    }

    #[test]
    fn sphere_mesh_has_correct_counts() {
        let (v, i) = MeshBuilder::sphere(8, 4);
        assert_eq!(v.len(), (8 + 1) * (4 + 1));
        assert_eq!(i.len(), 8 * 4 * 6);
    }

    #[test]
    fn plane_mesh_has_correct_counts() {
        let (v, i) = MeshBuilder::plane(10.0, 4);
        assert_eq!(v.len(), (4 + 1) * (4 + 1));
        assert_eq!(i.len(), 4 * 4 * 6);
    }

    #[test]
    fn builder_push_quad_adds_six_indices() {
        let mut b = MeshBuilder::new();
        let a = b.push_vertex(Vertex::new([0.0; 3]));
        let c = b.push_vertex(Vertex::new([0.0; 3]));
        let d = b.push_vertex(Vertex::new([0.0; 3]));
        let e = b.push_vertex(Vertex::new([0.0; 3]));
        b.push_quad(a, c, d, e);
        let (_, idx) = b.into_parts();
        assert_eq!(idx, vec![a, c, d, a, d, e]);
    }
}
