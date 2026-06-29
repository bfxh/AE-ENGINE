//! glTF 模型加载（借鉴 v1 GltfLoader + bevy AssetLoader + Fyrox UUID）
//!
//! 设计：
//! - `Model`：glTF 资源的运行时表示，包含节点层级 + Mesh/Texture Handle + Material
//! - `ModelNode`：节点（local + world transform，children 索引）
//! - `ModelLoader`：glTF 解析器，把 gltf crate 的输出转成 nova 资源
//! - `GltfMaterial`：实现 nova `Material` trait 的 glTF PBR 材质
//!
//! 与 v1 的差异：
//! - 用 nova 的 `Handle<Mesh>` / `Handle<Texture>`（经 `Pool::spawn` 注册）
//! - 用 nova 的 `Material` trait（dyn Material + Arc 共享）
//! - 节点用 `Mat4` 直接存 local/world transform（glam）
//! - 错误统一用 `anyhow::Result`
//! - 资源带 `Uuid`（Fyrox 风格）

use crate::assets::material::Material;
use crate::assets::mesh::{Mesh, MeshData, Vertex};
use crate::assets::texture::{Texture, TextureData};
use crate::core::Handle;
use crate::core::Pool;
use anyhow::{anyhow, Result};
use glam::{Mat4, Quat, Vec3, Vec4};
use gltf::image::Data as ImageData;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// glTF 加载格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// `.gltf` JSON + 外部 buffer/image
    Gltf,
    /// `.glb` 二进制容器
    Glb,
}

/// Alpha 渲染模式（对应 glTF `alphaMode`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlphaMode {
    /// 不透明
    #[default]
    Opaque,
    /// Alpha 测试（1-bit mask）
    Mask,
    /// Alpha 混合
    Blend,
}

impl From<gltf::material::AlphaMode> for AlphaMode {
    fn from(m: gltf::material::AlphaMode) -> Self {
        match m {
            gltf::material::AlphaMode::Opaque => Self::Opaque,
            gltf::material::AlphaMode::Mask => Self::Mask,
            gltf::material::AlphaMode::Blend => Self::Blend,
        }
    }
}

/// glTF 节点（运行时表示）
#[derive(Debug, Clone)]
pub struct ModelNode {
    /// 节点名称（可能为空）
    pub name: String,
    /// 该节点引用的 Mesh 在 `Model::meshes` 中的索引（取首个 primitive）
    pub mesh: Option<usize>,
    /// 该节点对应的 glTF mesh 在 `Model::mesh_primitives` 中的索引（用于多 primitive）
    pub mesh_index: Option<usize>,
    /// 子节点索引列表
    pub children: Vec<usize>,
    /// 局部变换（相对父节点）
    pub local_transform: Mat4,
    /// 世界变换（递归计算后填充）
    pub world_transform: Mat4,
}

impl Default for ModelNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            mesh: None,
            mesh_index: None,
            children: Vec::new(),
            local_transform: Mat4::IDENTITY,
            world_transform: Mat4::IDENTITY,
        }
    }
}

/// glTF 模型（运行时表示）
pub struct Model {
    /// 节点列表（按 glTF node index 顺序）
    pub nodes: Vec<ModelNode>,
    /// 所有 primitive 展开后的 Mesh Handle 列表
    pub meshes: Vec<Handle<Mesh>>,
    /// 每个 glTF mesh 对应的 primitive 在 `meshes` 中的索引列表
    pub mesh_primitives: Vec<Vec<usize>>,
    /// 材质列表（Arc<dyn Material>，glTF material 顺序）
    pub materials: Vec<Arc<dyn Material>>,
    /// 纹理 Handle 列表（glTF image 顺序）
    pub textures: Vec<Handle<Texture>>,
    /// 根节点索引列表（默认场景）
    pub root_nodes: Vec<usize>,
    /// 模型名称（取自 asset.name 或文件名）
    pub name: String,
    /// Fyrox 风格 UUID
    pub uuid: Uuid,
}

impl Model {
    /// 创建空模型
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            nodes: Vec::new(),
            meshes: Vec::new(),
            mesh_primitives: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            root_nodes: Vec::new(),
            name: name.into(),
            uuid: Uuid::new_v4(),
        }
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Mesh 数量
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// 材质数量
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// 纹理数量
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// 递归更新所有节点的 world_transform
    pub fn update_world_transforms(&mut self) {
        let roots: Vec<usize> = self.root_nodes.clone();
        for root in roots {
            Self::update_world_recursive(root, Mat4::IDENTITY, &mut self.nodes);
        }
    }

    fn update_world_recursive(node_idx: usize, parent_world: Mat4, nodes: &mut [ModelNode]) {
        if node_idx >= nodes.len() {
            return;
        }
        let local = nodes[node_idx].local_transform;
        let world = parent_world * local;
        nodes[node_idx].world_transform = world;
        let children = nodes[node_idx].children.clone();
        for child in children {
            Self::update_world_recursive(child, world, nodes);
        }
    }
}

/// glTF PBR 材质（实现 nova `Material` trait）
pub struct GltfMaterial {
    /// 材质名称
    pub name: String,
    /// 基础颜色因子
    pub base_color: [f32; 4],
    /// 基础颜色纹理
    pub base_color_texture: Option<Handle<Texture>>,
    /// 金属度
    pub metallic: f32,
    /// 粗糙度
    pub roughness: f32,
    /// 金属-粗糙度纹理
    pub metallic_roughness_texture: Option<Handle<Texture>>,
    /// 法线纹理
    pub normal_texture: Option<Handle<Texture>>,
    /// 法线强度
    pub normal_scale: f32,
    /// 遮蔽纹理
    pub occlusion_texture: Option<Handle<Texture>>,
    /// 遮蔽强度
    pub occlusion_strength: f32,
    /// 自发光颜色
    pub emissive: [f32; 3],
    /// 自发光纹理
    pub emissive_texture: Option<Handle<Texture>>,
    /// Alpha 模式
    pub alpha_mode: AlphaMode,
    /// Alpha 截断值
    pub alpha_cutoff: f32,
    /// 是否双面
    pub double_sided: bool,
}

impl Default for GltfMaterial {
    fn default() -> Self {
        Self {
            name: String::from("gltf_material"),
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic: 0.0,
            roughness: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

impl GltfMaterial {
    /// 创建默认材质
    pub fn new() -> Self {
        Self::default()
    }
}

impl Material for GltfMaterial {
    fn name(&self) -> &str {
        &self.name
    }

    fn transparent(&self) -> bool {
        matches!(self.alpha_mode, AlphaMode::Blend) || self.base_color[3] < 1.0
    }

    fn double_sided(&self) -> bool {
        self.double_sided
    }
}

/// glTF 加载器
///
/// 用法：
/// ```ignore
/// let mut mesh_pool = Pool::<Mesh>::new();
/// let mut tex_pool = Pool::<Texture>::new();
/// let model = ModelLoader::load_gltf(device, queue, path, &mut mesh_pool, &mut tex_pool)?;
/// ```
pub struct ModelLoader;

impl ModelLoader {
    /// 从 `.gltf` 文件加载
    pub fn load_gltf(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        mesh_pool: &mut Pool<Mesh>,
        texture_pool: &mut Pool<Texture>,
    ) -> Result<Model> {
        Self::load_from_path(device, queue, path, mesh_pool, texture_pool)
    }

    /// 从 `.glb` 文件加载（与 `load_gltf` 内部相同，gltf crate 自动识别）
    pub fn load_glb(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        mesh_pool: &mut Pool<Mesh>,
        texture_pool: &mut Pool<Texture>,
    ) -> Result<Model> {
        Self::load_from_path(device, queue, path, mesh_pool, texture_pool)
    }

    /// 从字节切片加载（自动检测 GLB / 嵌入式 glTF）
    pub fn load_from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        _format: ModelFormat,
        mesh_pool: &mut Pool<Mesh>,
        texture_pool: &mut Pool<Texture>,
    ) -> Result<Model> {
        let (doc, buffers, images) = gltf::import_slice(data)
            .map_err(|e| anyhow!("gltf import_slice error: {}", e))?;
        Self::build_model(device, queue, &doc, &buffers, &images, mesh_pool, texture_pool)
    }

    fn load_from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        mesh_pool: &mut Pool<Mesh>,
        texture_pool: &mut Pool<Texture>,
    ) -> Result<Model> {
        let (doc, buffers, images) = gltf::import(path)
            .map_err(|e| anyhow!("gltf import error (path={}): {}", path.display(), e))?;
        Self::build_model(device, queue, &doc, &buffers, &images, mesh_pool, texture_pool)
    }

    fn build_model(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        doc: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        images: &[ImageData],
        mesh_pool: &mut Pool<Mesh>,
        texture_pool: &mut Pool<Texture>,
    ) -> Result<Model> {
        let model_name = doc
            .as_json()
            .asset
            .generator
            .clone()
            .unwrap_or_else(|| String::from("gltf_model"));

        let mut model = Model::new(model_name);

        Self::load_textures(device, queue, images, texture_pool, &mut model);
        let textures_snapshot = model.textures.clone();
        Self::load_materials(doc, &textures_snapshot, &mut model);
        Self::load_meshes(device, doc, buffers, mesh_pool, &mut model)?;
        Self::load_nodes(doc, &mut model);

        Ok(model)
    }

    fn load_textures(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        images: &[ImageData],
        texture_pool: &mut Pool<Texture>,
        model: &mut Model,
    ) {
        for (i, image) in images.iter().enumerate() {
            // glTF image::Data 的 pixels 通常为 RGBA8
            let format = if image.format == gltf::image::Format::R8G8B8A8 {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            };
            let tex_data = TextureData {
                pixels: image.pixels.clone(),
                width: image.width,
                height: image.height,
                format,
            };
            let label = format!("gltf_texture_{}", i);
            let texture = Texture::from_data(device, queue, &tex_data, Some(&label));
            let handle = texture_pool.spawn(texture);
            model.textures.push(handle);
        }
    }

    fn load_materials(doc: &gltf::Document, textures: &[Handle<Texture>], model: &mut Model) {
        for mat in doc.materials() {
            let pbr = mat.pbr_metallic_roughness();
            let mut material = GltfMaterial::new();
            material.name = mat
                .name()
                .map(String::from)
                .unwrap_or_else(|| String::from("gltf_material"));
            material.base_color = pbr.base_color_factor();
            material.metallic = pbr.metallic_factor();
            material.roughness = pbr.roughness_factor();
            material.emissive = mat.emissive_factor();
            material.alpha_mode = mat.alpha_mode().into();
            material.alpha_cutoff = mat.alpha_cutoff().unwrap_or(0.5);
            material.double_sided = mat.double_sided();

            if let Some(info) = pbr.base_color_texture() {
                let tex_idx = info.texture().source().index();
                if tex_idx < textures.len() {
                    material.base_color_texture = Some(textures[tex_idx].clone());
                }
            }
            if let Some(info) = pbr.metallic_roughness_texture() {
                let tex_idx = info.texture().source().index();
                if tex_idx < textures.len() {
                    material.metallic_roughness_texture = Some(textures[tex_idx].clone());
                }
            }
            if let Some(info) = mat.normal_texture() {
                let tex_idx = info.texture().source().index();
                if tex_idx < textures.len() {
                    material.normal_texture = Some(textures[tex_idx].clone());
                }
                material.normal_scale = info.scale();
            }
            if let Some(info) = mat.occlusion_texture() {
                let tex_idx = info.texture().source().index();
                if tex_idx < textures.len() {
                    material.occlusion_texture = Some(textures[tex_idx].clone());
                }
                material.occlusion_strength = info.strength();
            }
            if let Some(info) = mat.emissive_texture() {
                let tex_idx = info.texture().source().index();
                if tex_idx < textures.len() {
                    material.emissive_texture = Some(textures[tex_idx].clone());
                }
            }

            model.materials.push(Arc::new(material));
        }

        // 兜底：如果没有材质，加一个默认材质
        if model.materials.is_empty() {
            model.materials.push(Arc::new(GltfMaterial::new()));
        }
    }

    fn load_meshes(
        device: &wgpu::Device,
        doc: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        mesh_pool: &mut Pool<Mesh>,
        model: &mut Model,
    ) -> Result<()> {
        for mesh in doc.meshes() {
            let mut prim_indices: Vec<usize> = Vec::new();

            for prim in mesh.primitives() {
                let reader = prim.reader(|buffer| {
                    let idx = buffer.index();
                    buffers.get(idx).map(|b| b.0.as_slice())
                });

                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .map(|p| p.collect())
                    .unwrap_or_default();
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|n| n.collect())
                    .unwrap_or_default();
                let tex_coords0: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|t| t.into_f32().collect())
                    .unwrap_or_default();
                let tangents: Vec<[f32; 4]> = reader
                    .read_tangents()
                    .map(|t| t.collect())
                    .unwrap_or_default();
                let colors0: Vec<[f32; 4]> = reader
                    .read_colors(0)
                    .map(|c| c.into_rgba_f32().collect())
                    .unwrap_or_default();

                let mut vertices = Vec::with_capacity(positions.len());
                for (i, pos) in positions.iter().enumerate() {
                    let v = Vertex {
                        position: *pos,
                        _pad0: 0.0,
                        normal: *normals.get(i).unwrap_or(&[0.0, 1.0, 0.0]),
                        _pad1: 0.0,
                        tangent: *tangents.get(i).unwrap_or(&[1.0, 0.0, 0.0, 1.0]),
                        uv: *tex_coords0.get(i).unwrap_or(&[0.0, 0.0]),
                        _pad2: 0.0,
                        color: *colors0.get(i).unwrap_or(&[1.0, 1.0, 1.0, 1.0]),
                    };
                    vertices.push(v);
                }

                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|idx| idx.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());

                let mesh_data = MeshData { vertices, indices };
                let label = format!("gltf_mesh_{}_prim_{}", mesh.index(), prim.index());
                let gpu_mesh = Mesh::from_data(device, &mesh_data, Some(&label));
                let handle = mesh_pool.spawn(gpu_mesh);
                prim_indices.push(model.meshes.len());
                model.meshes.push(handle);
            }

            model.mesh_primitives.push(prim_indices);
        }

        Ok(())
    }

    fn load_nodes(doc: &gltf::Document, model: &mut Model) {
        for n in doc.nodes() {
            let (t, r, s) = n.transform().decomposed();
            let translation = Vec3::from(t);
            let rotation = Quat::from_vec4(Vec4::from(r));
            let scale = Vec3::from(s);
            let local = Mat4::from_translation(translation)
                * Mat4::from_quat(rotation)
                * Mat4::from_scale(scale);

            // glTF mesh index → model.mesh_primitives 索引
            let mesh_index = n.mesh().map(|m| m.index());
            // 该节点的首个 primitive 在 model.meshes 中的全局索引
            let mesh_handle_idx = mesh_index.and_then(|mi| {
                model
                    .mesh_primitives
                    .get(mi)
                    .and_then(|prims| prims.first().copied())
            });

            let node = ModelNode {
                name: n.name().map(String::from).unwrap_or_default(),
                mesh: mesh_handle_idx,
                mesh_index,
                children: n.children().map(|c| c.index()).collect(),
                local_transform: local,
                world_transform: Mat4::IDENTITY,
            };
            model.nodes.push(node);
        }

        // 默认场景的根节点
        if let Some(scene) = doc.default_scene().or_else(|| doc.scenes().next()) {
            model.root_nodes = scene.nodes().map(|n| n.index()).collect();
        }

        // 递归计算 world_transform
        model.update_world_transforms();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_mode_default_opaque() {
        assert_eq!(AlphaMode::default(), AlphaMode::Opaque);
    }

    #[test]
    fn alpha_mode_from_gltf() {
        assert_eq!(
            AlphaMode::from(gltf::material::AlphaMode::Opaque),
            AlphaMode::Opaque
        );
        assert_eq!(
            AlphaMode::from(gltf::material::AlphaMode::Mask),
            AlphaMode::Mask
        );
        assert_eq!(
            AlphaMode::from(gltf::material::AlphaMode::Blend),
            AlphaMode::Blend
        );
    }

    #[test]
    fn model_new_has_uuid() {
        let m = Model::new("test");
        assert_eq!(m.name, "test");
        assert!(!m.uuid.is_nil());
        assert_eq!(m.node_count(), 0);
        assert_eq!(m.mesh_count(), 0);
    }

    #[test]
    fn model_new_unique_uuid() {
        let a = Model::new("a");
        let b = Model::new("b");
        assert_ne!(a.uuid, b.uuid);
    }

    #[test]
    fn gltf_material_default() {
        let m = GltfMaterial::new();
        assert_eq!(m.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(m.metallic, 0.0);
        assert_eq!(m.roughness, 1.0);
        assert_eq!(m.alpha_mode, AlphaMode::Opaque);
        assert!(!m.double_sided);
    }

    #[test]
    fn gltf_material_transparent_blend() {
        let mut m = GltfMaterial::new();
        m.alpha_mode = AlphaMode::Blend;
        assert!(m.transparent());
    }

    #[test]
    fn gltf_material_transparent_alpha_color() {
        let mut m = GltfMaterial::new();
        m.base_color[3] = 0.5;
        assert!(m.transparent());
    }

    #[test]
    fn gltf_material_opaque_not_transparent() {
        let m = GltfMaterial::new();
        assert!(!m.transparent());
    }

    #[test]
    fn model_node_default_identity() {
        let n = ModelNode::default();
        assert_eq!(n.local_transform, Mat4::IDENTITY);
        assert_eq!(n.world_transform, Mat4::IDENTITY);
        assert!(n.children.is_empty());
        assert!(n.mesh.is_none());
    }

    #[test]
    fn model_update_world_transforms_propagates() {
        let mut m = Model::new("test");
        // root: translate x=2
        let mut root = ModelNode::default();
        root.local_transform = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
        // child: translate y=3
        let mut child = ModelNode::default();
        child.local_transform = Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0));
        m.nodes.push(root);
        m.nodes.push(child);
        m.root_nodes.push(0);
        m.nodes[0].children.push(1);

        m.update_world_transforms();

        // root.world = identity * translate(2,0,0)
        let root_w = m.nodes[0].world_transform;
        assert!((root_w.w_axis.x - 2.0).abs() < 1e-5);
        // child.world = root.world * translate(0,3,0)
        let child_w = m.nodes[1].world_transform;
        assert!((child_w.w_axis.x - 2.0).abs() < 1e-5);
        assert!((child_w.w_axis.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn model_format_variants() {
        let _ = ModelFormat::Gltf;
        let _ = ModelFormat::Glb;
    }
}
