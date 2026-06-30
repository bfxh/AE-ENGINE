//! glTF 模型加载

use crate::material::{PbrMaterial, PbrMaterialParams};
use crate::mesh::{Mesh, MeshBuilder, Vertex};
use crate::texture::Texture;
use gltf::image::Data as ImageData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// glTF 加载错误
#[derive(Debug)]
pub enum GltfLoadError {
    Io(std::io::Error),
    Gltf(String),
    Texture(String),
    UnsupportedFeature(&'static str),
}

impl std::fmt::Display for GltfLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Gltf(e) => write!(f, "gltf error: {e}"),
            Self::Texture(e) => write!(f, "texture error: {e}"),
            Self::UnsupportedFeature(s) => write!(f, "unsupported: {s}"),
        }
    }
}

impl std::error::Error for GltfLoadError {}

/// 已加载的 glTF 场景节点
#[derive(Debug, Clone)]
pub struct ModelNode {
    pub name: Option<String>,
    pub mesh_index: Option<usize>,
    pub children: Vec<usize>,
    pub transform: Transform,
}

/// 变换
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4], // quat
    pub scale: [f32; 3],
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn matrix(&self) -> [[f32; 4]; 4] {
        let [tx, ty, tz] = self.translation;
        let [qx, qy, qz, qw] = self.rotation;
        let [sx, sy, sz] = self.scale;

        let r00 = 1.0 - 2.0 * (qy * qy + qz * qz);
        let r01 = 2.0 * (qx * qy - qz * qw);
        let r02 = 2.0 * (qx * qz + qy * qw);
        let r10 = 2.0 * (qx * qy + qz * qw);
        let r11 = 1.0 - 2.0 * (qx * qx + qz * qz);
        let r12 = 2.0 * (qy * qz - qx * qw);
        let r20 = 2.0 * (qx * qz - qy * qw);
        let r21 = 2.0 * (qy * qz + qx * qw);
        let r22 = 1.0 - 2.0 * (qx * qx + qy * qy);

        [
            [r00 * sx, r10 * sx, r20 * sx, 0.0],
            [r01 * sy, r11 * sy, r21 * sy, 0.0],
            [r02 * sz, r12 * sz, r22 * sz, 0.0],
            [tx, ty, tz, 1.0],
        ]
    }
}

/// glTF 原语（一个 draw call）
pub struct GltfPrimitive {
    pub mesh: Arc<Mesh>,
    pub material: Arc<PbrMaterial>,
    pub mode: PrimitiveMode,
}

/// 图元模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveMode {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    Triangles,
    TriangleStrip,
    TriangleFan,
}

impl From<gltf::mesh::Mode> for PrimitiveMode {
    fn from(m: gltf::mesh::Mode) -> Self {
        match m {
            gltf::mesh::Mode::Points => Self::Points,
            gltf::mesh::Mode::Lines => Self::Lines,
            gltf::mesh::Mode::LineLoop => Self::LineLoop,
            gltf::mesh::Mode::LineStrip => Self::LineStrip,
            gltf::mesh::Mode::Triangles => Self::Triangles,
            gltf::mesh::Mode::TriangleStrip => Self::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => Self::TriangleFan,
        }
    }
}

/// 已加载的 glTF 模型
pub struct GltfModel {
    pub primitives: Vec<GltfPrimitive>,
    pub nodes: Vec<ModelNode>,
    pub scenes: Vec<Vec<usize>>,
    pub textures: Vec<Arc<Texture>>,
    pub materials: Vec<Arc<PbrMaterial>>,
}

/// glTF 加载器
#[allow(dead_code)]
pub struct GltfLoader {
    base_path: PathBuf,
}

impl GltfLoader {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// 从文件加载 glTF/GLB
    pub fn load(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<GltfModel, GltfLoadError> {
        let (doc, buffers, images) =
            gltf::import(path).map_err(|e| GltfLoadError::Gltf(e.to_string()))?;

        // 加载纹理
        let textures = self.load_textures(device, queue, &doc, &images)?;
        // 加载材质
        let materials = self.load_materials(&doc, &textures);
        // 加载网格
        let primitives = self.load_meshes(device, &doc, &buffers, &materials)?;
        // 加载节点
        let (nodes, scenes) = self.load_nodes(&doc);

        Ok(GltfModel { primitives, nodes, scenes, textures, materials })
    }

    fn load_textures(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        doc: &gltf::Document,
        images: &[ImageData],
    ) -> Result<Vec<Arc<Texture>>, GltfLoadError> {
        let mut textures = Vec::new();
        for image in images {
            let format = crate::texture::TextureFormat::Rgba8UnormSrgb;
            let tex = Texture::from_pixels(
                device,
                queue,
                &image.pixels,
                format,
                image.width,
                image.height,
                Some("gltf texture"),
            );
            textures.push(Arc::new(tex));
        }
        let _ = doc;
        Ok(textures)
    }

    fn load_materials(
        &self,
        doc: &gltf::Document,
        textures: &[Arc<Texture>],
    ) -> Vec<Arc<PbrMaterial>> {
        doc.materials()
            .map(|mat| {
                let pbr = mat.pbr_metallic_roughness();
                let mut params = PbrMaterialParams::new();
                params.base_color = pbr.base_color_factor();
                params.metallic_roughness[0] = pbr.metallic_factor();
                params.metallic_roughness[1] = pbr.roughness_factor();

                let emissive = mat.emissive_factor();
                params.emissive = [emissive[0], emissive[1], emissive[2], 0.0];
                params.metallic_roughness[2] = 1.0;

                let mut material = PbrMaterial::new(params);

                if let Some(info) = pbr.base_color_texture() {
                    let tex_idx = info.texture().source().index();
                    if tex_idx < textures.len() {
                        material = material.with_base_color_texture(textures[tex_idx].clone());
                    }
                }

                if let Some(info) = mat.normal_texture() {
                    let tex_idx = info.texture().source().index();
                    if tex_idx < textures.len() {
                        material = material.with_normal_texture(textures[tex_idx].clone());
                    }
                    material.params.normal_scale = info.scale();
                }

                if let Some(info) = pbr.metallic_roughness_texture() {
                    let tex_idx = info.texture().source().index();
                    if tex_idx < textures.len() {
                        material =
                            material.with_metallic_roughness_texture(textures[tex_idx].clone());
                    }
                }

                if let Some(info) = mat.occlusion_texture() {
                    let tex_idx = info.texture().source().index();
                    if tex_idx < textures.len() {
                        material = material.with_occlusion_texture(textures[tex_idx].clone());
                    }
                    material.params.occlusion_strength = info.strength();
                }

                if let Some(info) = mat.emissive_texture() {
                    let tex_idx = info.texture().source().index();
                    if tex_idx < textures.len() {
                        material = material.with_emissive_texture(textures[tex_idx].clone());
                    }
                }

                match mat.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => {},
                    gltf::material::AlphaMode::Mask => {
                        material = material.alpha_mask(true);
                        material.params.metallic_roughness[3] = mat.alpha_cutoff().unwrap_or(0.5);
                    },
                    gltf::material::AlphaMode::Blend => {
                        material = material.alpha_blend(true);
                    },
                }

                if mat.double_sided() {
                    material = material.two_sided(true);
                }

                Arc::new(material)
            })
            .collect()
    }

    fn load_meshes(
        &self,
        device: &wgpu::Device,
        doc: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        materials: &[Arc<PbrMaterial>],
    ) -> Result<Vec<GltfPrimitive>, GltfLoadError> {
        let mut primitives = Vec::new();

        for mesh in doc.meshes() {
            for prim in mesh.primitives() {
                let reader = prim.reader(|buffer| {
                    let idx = buffer.index();
                    buffers.get(idx).map(|b| b.0.as_slice())
                });

                let mut builder = MeshBuilder::new();

                let positions: Vec<[f32; 3]> =
                    reader.read_positions().map(|p| p.collect()).unwrap_or_default();
                let normals: Vec<[f32; 3]> =
                    reader.read_normals().map(|n| n.collect()).unwrap_or_default();
                let tex_coords0: Vec<[f32; 2]> =
                    reader.read_tex_coords(0).map(|t| t.into_f32().collect()).unwrap_or_default();
                let tangents: Vec<[f32; 4]> =
                    reader.read_tangents().map(|t| t.collect()).unwrap_or_default();
                let colors0: Vec<[f32; 4]> =
                    reader.read_colors(0).map(|c| c.into_rgba_f32().collect()).unwrap_or_default();

                for (i, pos) in positions.iter().enumerate() {
                    let v = Vertex {
                        position: *pos,
                        normal: *normals.get(i).unwrap_or(&[0.0, 1.0, 0.0]),
                        tangent: *tangents.get(i).unwrap_or(&[1.0, 0.0, 0.0, 1.0]),
                        uv: *tex_coords0.get(i).unwrap_or(&[0.0, 0.0]),
                        color: *colors0.get(i).unwrap_or(&[1.0, 1.0, 1.0, 1.0]),
                    };
                    builder.push_vertex(v);
                }

                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|idx| idx.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());

                for chunk in indices.chunks(3) {
                    if chunk.len() == 3 {
                        builder.push_triangle(chunk[0], chunk[1], chunk[2]);
                    }
                }

                let (verts, idx) = builder.into_parts();
                let mesh = Arc::new(Mesh::from_data(device, &verts, &idx, Some("gltf mesh")));
                let material = prim
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned())
                    .unwrap_or_else(|| Arc::new(PbrMaterial::new(PbrMaterialParams::new())));

                primitives.push(GltfPrimitive { mesh, material, mode: prim.mode().into() });
            }
        }

        Ok(primitives)
    }

    fn load_nodes(&self, doc: &gltf::Document) -> (Vec<ModelNode>, Vec<Vec<usize>>) {
        let nodes: Vec<ModelNode> = doc
            .nodes()
            .map(|n| {
                let (t, r, s) = n.transform().decomposed();
                ModelNode {
                    name: n.name().map(String::from),
                    mesh_index: n.mesh().map(|m| m.index()),
                    children: n.children().map(|c| c.index()).collect(),
                    transform: Transform { translation: t, rotation: r, scale: s },
                }
            })
            .collect();

        let scenes: Vec<Vec<usize>> =
            doc.scenes().map(|s| s.nodes().map(|n| n.index()).collect()).collect();

        (nodes, scenes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_identity_matrix() {
        let t = Transform::identity();
        let m = t.matrix();
        // 单位矩阵
        assert!((m[0][0] - 1.0).abs() < 1e-5);
        assert!((m[1][1] - 1.0).abs() < 1e-5);
        assert!((m[2][2] - 1.0).abs() < 1e-5);
        assert!((m[3][3] - 1.0).abs() < 1e-5);
        assert!((m[3][0]).abs() < 1e-5);
        assert!((m[3][1]).abs() < 1e-5);
        assert!((m[3][2]).abs() < 1e-5);
    }

    #[test]
    fn transform_translation() {
        let t = Transform {
            translation: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        };
        let m = t.matrix();
        assert!((m[3][0] - 1.0).abs() < 1e-5);
        assert!((m[3][1] - 2.0).abs() < 1e-5);
        assert!((m[3][2] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn transform_scale() {
        let t = Transform {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [2.0, 3.0, 4.0],
        };
        let m = t.matrix();
        assert!((m[0][0] - 2.0).abs() < 1e-5);
        assert!((m[1][1] - 3.0).abs() < 1e-5);
        assert!((m[2][2] - 4.0).abs() < 1e-5);
    }

    #[test]
    fn transform_rotation_90_y() {
        // 绕 Y 轴旋转 90 度
        let t = Transform {
            translation: [0.0; 3],
            rotation: [0.0, std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2], // sin(45), cos(45)
            scale: [1.0; 3],
        };
        let m = t.matrix();
        // 绕 Y 轴 90 度：x -> -z, z -> x
        // 行主序：m[0] = x 基向量变换后 = [0, 0, -1]
        assert!((m[0][0]).abs() < 1e-5);
        assert!((m[0][2] + 1.0).abs() < 1e-5);
        assert!((m[2][0] - 1.0).abs() < 1e-5);
        assert!((m[2][2]).abs() < 1e-5);
    }

    #[test]
    fn primitive_mode_from_gltf() {
        assert_eq!(PrimitiveMode::from(gltf::mesh::Mode::Triangles), PrimitiveMode::Triangles);
        assert_eq!(PrimitiveMode::from(gltf::mesh::Mode::Points), PrimitiveMode::Points);
        assert_eq!(PrimitiveMode::from(gltf::mesh::Mode::Lines), PrimitiveMode::Lines);
    }

    #[test]
    fn gltf_load_error_display() {
        let e = GltfLoadError::Gltf("test".into());
        assert!(format!("{e}").contains("gltf error"));
        let e = GltfLoadError::UnsupportedFeature("morph targets");
        assert!(format!("{e}").contains("morph targets"));
    }
}
