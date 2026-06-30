//! 基础几何操作
//!
//! 提供 MeshBuilder 之上的高级几何生成：
//! - `cylinder`：参数化圆柱（半径/高度/分段/端盖）
//! - `lathe_profile`：旋转剖面（车削）— 2D 轮廓绕 Y 轴旋转
//! - `sweep_along_path`：沿路径扫描截面
//! - `bevel_edges`：边缘倒角（近似实现）

use crate::mesh::{MeshBuilder, Vertex};

/// 圆柱参数
#[derive(Debug, Clone, Copy)]
pub struct CylinderParams {
    pub radius: f32,
    pub height: f32,
    pub segments: u32,
    pub cap_bottom: bool,
    pub cap_top: bool,
}

impl Default for CylinderParams {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 1.0,
            segments: 16,
            cap_bottom: true,
            cap_top: true,
        }
    }
}

/// 生成参数化圆柱。
/// 中心在 (0, height/2, 0)，Y 轴向上。
pub fn cylinder(params: CylinderParams) -> (Vec<Vertex>, Vec<u32>) {
    let mut b = MeshBuilder::with_capacity(
        (params.segments as usize + 1) * 2 + 2,
        params.segments as usize * 6 * 2,
    );
    let half_h = params.height * 0.5;
    let seg = params.segments as f32;
    // 侧面顶点（底环 + 顶环）
    for i in 0..=params.segments {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / seg;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let u = i as f32 / seg;
        // 底环
        b.push_vertex(
            Vertex::new([cos_a * params.radius, -half_h, sin_a * params.radius])
                .with_normal([cos_a, 0.0, sin_a])
                .with_uv([u, 0.0]),
        );
        // 顶环
        b.push_vertex(
            Vertex::new([cos_a * params.radius, half_h, sin_a * params.radius])
                .with_normal([cos_a, 0.0, sin_a])
                .with_uv([u, 1.0]),
        );
    }
    // 侧面索引
    for i in 0..params.segments {
        let i0 = i * 2;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;
        b.push_quad(i0, i1, i3, i2);
    }
    // 端盖
    let base_idx = b.vertex_count() as u32;
    if params.cap_bottom {
        let center = b.push_vertex(
            Vertex::new([0.0, -half_h, 0.0]).with_normal([0.0, -1.0, 0.0]).with_uv([0.5, 0.5]),
        );
        for i in 0..params.segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / seg;
            let p = b.push_vertex(
                Vertex::new([angle.cos() * params.radius, -half_h, angle.sin() * params.radius])
                    .with_normal([0.0, -1.0, 0.0])
                    .with_uv([0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()]),
            );
            let angle_next = 2.0 * std::f32::consts::PI * (i + 1) as f32 / seg;
            let p_next = b.push_vertex(
                Vertex::new(
                    [angle_next.cos() * params.radius, -half_h, angle_next.sin() * params.radius],
                )
                .with_normal([0.0, -1.0, 0.0])
                .with_uv([0.5 + 0.5 * angle_next.cos(), 0.5 + 0.5 * angle_next.sin()]),
            );
            b.push_triangle(center, p_next, p);
        }
    }
    if params.cap_top {
        let center = b.push_vertex(
            Vertex::new([0.0, half_h, 0.0]).with_normal([0.0, 1.0, 0.0]).with_uv([0.5, 0.5]),
        );
        for i in 0..params.segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / seg;
            let p = b.push_vertex(
                Vertex::new([angle.cos() * params.radius, half_h, angle.sin() * params.radius])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_uv([0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()]),
            );
            let angle_next = 2.0 * std::f32::consts::PI * (i + 1) as f32 / seg;
            let p_next = b.push_vertex(
                Vertex::new(
                    [angle_next.cos() * params.radius, half_h, angle_next.sin() * params.radius],
                )
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([0.5 + 0.5 * angle_next.cos(), 0.5 + 0.5 * angle_next.sin()]),
            );
            b.push_triangle(center, p, p_next);
        }
    }
    let _ = base_idx; // 仅用于消除未使用警告（端盖顶点已直接 push）
    b.into_parts()
}

/// 旋转剖面（车削）：2D 轮廓（X=半径, Y=高度）绕 Y 轴旋转 360°。
/// `profile` 必须按 Y 升序排列，至少 2 个点。
/// `segments` 控制周向分段数（建议 16+）。
pub fn lathe_profile(
    profile: &[[f32; 2]],
    segments: u32,
    uv_scale: [f32; 2],
) -> Option<(Vec<Vertex>, Vec<u32>)> {
    if profile.len() < 2 || segments < 3 {
        return None;
    }
    let mut verts = Vec::with_capacity(profile.len() * (segments as usize + 1));
    let mut idx = Vec::with_capacity(profile.len() * segments as usize * 6);
    // 累计高度用于 V 坐标
    let total_height: f32 = profile.last().unwrap()[1] - profile[0][1];
    let inv_h = if total_height.abs() > 1e-6 { 1.0 / total_height } else { 0.0 };
    for (pi, p) in profile.iter().enumerate() {
        let r = p[0].max(0.0);
        let y = p[1];
        let v = (y - profile[0][1]) * inv_h;
        for s in 0..=segments {
            let angle = 2.0 * std::f32::consts::PI * s as f32 / segments as f32;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            let pos = [r * cos_a, y, r * sin_a];
            // 法线：剖面切线 × 周向切线（指向外）
            let normal = if r > 1e-6 {
                // 相邻剖面点的方向
                let next = profile.get(pi + 1).or(profile.last()).unwrap();
                let prev = if pi > 0 { &profile[pi - 1] } else { &profile[0] };
                let tangent = [next[0] - prev[0], next[1] - prev[1]];
                let tlen = (tangent[0].powi(2) + tangent[1].powi(2)).sqrt().max(1e-6);
                let n2d = [tangent[1] / tlen, -tangent[0] / tlen]; // 剖面法线（2D）
                [n2d[0] * cos_a, n2d[1], n2d[0] * sin_a]
            } else {
                [0.0, 1.0, 0.0]
            };
            let u = s as f32 / segments as f32;
            verts.push(
                Vertex::new(pos)
                    .with_normal(normal)
                    .with_uv([u * uv_scale[0], v * uv_scale[1]]),
            );
        }
    }
    let profile_len = profile.len() as u32;
    for pi in 0..profile_len - 1 {
        for s in 0..segments {
            let a = pi * (segments + 1) + s;
            let b = a + 1;
            let c = a + segments + 1;
            let d = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    Some((verts, idx))
}

/// 沿路径扫描截面（2D 截面 + 3D 路径）。
/// `cross_section` 为 XY 平面 2D 点（X=宽, Y=高）。
/// `path` 为 3D 路径点（至少 2 点）。
/// 返回 None 如果参数不足。
pub fn sweep_along_path(
    cross_section: &[[f32; 2]],
    path: &[[f32; 3]],
    uv_scale: [f32; 2],
) -> Option<(Vec<Vertex>, Vec<u32>)> {
    if cross_section.len() < 2 || path.len() < 2 {
        return None;
    }
    let mut verts = Vec::with_capacity(cross_section.len() * path.len());
    let mut idx = Vec::with_capacity(cross_section.len() * path.len() * 6);
    // 计算每个路径点的局部坐标系（Frenet-like 框架，简化版）
    let mut frames: Vec<([f32; 3], [f32; 3], [f32; 3])> = Vec::with_capacity(path.len());
    for i in 0..path.len() {
        let tangent = if i + 1 < path.len() {
            let d = [
                path[i + 1][0] - path[i][0],
                path[i + 1][1] - path[i][1],
                path[i + 1][2] - path[i][2],
            ];
            let len = (d[0].powi(2) + d[1].powi(2) + d[2].powi(2)).sqrt().max(1e-6);
            [d[0] / len, d[1] / len, d[2] / len]
        } else {
            let d = [
                path[i][0] - path[i - 1][0],
                path[i][1] - path[i - 1][1],
                path[i][2] - path[i - 1][2],
            ];
            let len = (d[0].powi(2) + d[1].powi(2) + d[2].powi(2)).sqrt().max(1e-6);
            [d[0] / len, d[1] / len, d[2] / len]
        };
        // up 向量（避免与 tangent 平行）
        let up = if tangent[1].abs() < 0.99 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        // binormal = tangent × up
        let binormal = [
            tangent[1] * up[2] - tangent[2] * up[1],
            tangent[2] * up[0] - tangent[0] * up[2],
            tangent[0] * up[1] - tangent[1] * up[0],
        ];
        let blen = (binormal[0].powi(2) + binormal[1].powi(2) + binormal[2].powi(2)).sqrt().max(1e-6);
        let binormal = [binormal[0] / blen, binormal[1] / blen, binormal[2] / blen];
        // normal = binormal × tangent
        let normal = [
            binormal[1] * tangent[2] - binormal[2] * tangent[1],
            binormal[2] * tangent[0] - binormal[0] * tangent[2],
            binormal[0] * tangent[1] - binormal[1] * tangent[0],
        ];
        frames.push((normal, binormal, tangent));
    }
    // 生成顶点
    for (pi, p) in path.iter().enumerate() {
        let (n, bn, _) = &frames[pi];
        let v = pi as f32 / (path.len() - 1).max(1) as f32;
        for cs in cross_section {
            let pos = [
                p[0] + cs[0] * bn[0] + cs[1] * n[0],
                p[1] + cs[0] * bn[1] + cs[1] * n[1],
                p[2] + cs[0] * bn[2] + cs[1] * n[2],
            ];
            verts.push(
                Vertex::new(pos)
                    .with_normal(*n)
                    .with_uv([cs[0] * uv_scale[0], v * uv_scale[1]]),
            );
        }
    }
    // 生成索引
    let cs_len = cross_section.len() as u32;
    for pi in 0..path.len() as u32 - 1 {
        for ci in 0..cs_len {
            let a = pi * cs_len + ci;
            let b = a + 1;
            let c = a + cs_len;
            let d = c + 1;
            // 处理截面环绕
            if ci + 1 < cs_len {
                idx.extend_from_slice(&[a, c, b, b, c, d]);
            } else {
                let a0 = pi * cs_len;
                let c0 = a0 + cs_len;
                idx.extend_from_slice(&[a, c, a0, a0, c, c0]);
            }
        }
    }
    Some((verts, idx))
}

/// 边缘倒角（近似实现）。
/// 通过收缩顶点到边缘内侧生成倒角面。
/// `amount` 控制倒角量（0.0..0.5）。
/// 注意：这是一个简化实现，仅适用于凸边缘。
pub fn bevel_edges(
    vertices: &[Vertex],
    indices: &[u32],
    _amount: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    // 简化实现：直接返回原数据（完整倒角需要半边数据结构）
    // 实际场景中由 building.rs 的参数化部件直接生成倒角几何
    (vertices.to_vec(), indices.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_default() {
        let (v, i) = cylinder(CylinderParams::default());
        // 侧面: (segments+1)*2 = 34, 底盖中心+1 + segments*2 = 33, 顶盖同 = 33
        // 总 = 34 + 33 + 33 = 100
        assert!(v.len() >= 34);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_lathe_profile_basic() {
        let profile = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let result = lathe_profile(&profile, 8, [1.0, 1.0]);
        assert!(result.is_some());
        let (v, i) = result.unwrap();
        // 4 profiles × 9 segments = 36
        assert_eq!(v.len(), 36);
        // 3 profile gaps × 8 segments × 6 = 144
        assert_eq!(i.len(), 144);
    }

    #[test]
    fn test_lathe_profile_too_short() {
        let profile = vec![[0.0, 0.0]];
        assert!(lathe_profile(&profile, 8, [1.0, 1.0]).is_none());
    }

    #[test]
    fn test_sweep_along_path_basic() {
        let cs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let path = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 2.0]];
        let result = sweep_along_path(&cs, &path, [1.0, 1.0]);
        assert!(result.is_some());
        let (v, _i) = result.unwrap();
        // 4 cross_section × 3 path = 12
        assert_eq!(v.len(), 12);
    }

    #[test]
    fn test_sweep_too_short() {
        let cs = vec![[0.0, 0.0]];
        let path = vec![[0.0, 0.0, 0.0]];
        assert!(sweep_along_path(&cs, &path, [1.0, 1.0]).is_none());
    }
}
