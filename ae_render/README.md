# Wasteland Render v1

> **Status**: Archived (replaced by `nova_render` v2)
> **Backup**: `D:\AI\storage\CC\2_Old\ae_render_v1_20260628_164448.zip`
> **Version**: 0.1.0

基于 wgpu 24 的跨平台 3A 级实时渲染框架，提供 9-pass HDR 渲染管线。

## Features

### 9-Pass HDR 渲染管线
```
Shadow → Particle Compute → Pass1 场景(jittered) → Pass1.5 体积雾
→ Pass1.6-1.8 SSAO 三pass → Pass1.8.1-1.8.3 SSR 三pass
→ Pass1.9 TAA → Pass2-5 Bloom/Tonemap
```

### 已实现技术栈
- **Shadow Map**: 2048 PCF 软阴影
- **GPU Particle System**: compute shader @workgroup_size(64) + billboard 加法混合
- **Volumetric Fog**: 高度雾 + 距离雾 + god rays（深度重建世界坐标）
- **SSAO**: 3-pass（64 半球核 + 4x4 噪声 + 5x5 双边滤波 + AO 应用）
- **TAA**: Halton jitter + ping-pong 历史帧 + 3x3 邻域 clamping + 速度自适应 blend
- **SSR**: view-space ray march + dpdx/dpdy 重建法线 + Schlick 菲涅尔 + 5x5 双边滤波 + 反射合成
- **Bloom**: 多级高斯模糊
- **Tonemap**: ACES / Reinhard

### WGSL 关键技术点
- `mat4x4` 不允许作为 vertex input/output（用 `[[f32; 4]; 4]` 代替）
- `vec2` 需 8-byte 对齐（用两个 `f32` 代替）
- `Depth32Float` 不支持 filtering（用 `NonFiltering` sampler）
- 循环内采样用 `textureSampleLevel` 避免 non-uniform control flow 限制
- 全屏三角形：3 个顶点覆盖 NDC，无 vertex buffer

## Modules

| 模块 | 功能 |
|------|------|
| `camera` | 相机与视图矩阵 |
| `device` | GPU 设备抽象（RenderContext） |
| `instanced` | 实例化渲染（立方体/点云） |
| `material` | PBR 材质系统 |
| `mesh` | 网格与几何体 |
| `mesh_renderer` | 网格渲染器 |
| `model` | glTF 模型加载 |
| `pipeline` | 渲染管线缓存 |
| `post_process` | 后处理（Bloom/Tonemap） |
| `procedural` | 程序化生成（建筑/NPC） |
| `shadow_map` | 阴影贴图 |
| `skybox` | 天空盒 |
| `ssao` | 屏幕空间环境光遮蔽 |
| `ssr` | 屏幕空间反射 |
| `taa` | 时域抗锯齿 |
| `volumetric_fog` | 体积雾 |
| `water` | 水面渲染 |
| `surface` | Surface 抽象 |
| `texture` | 纹理系统（PNG/DDS/KTX2） |
| `shader` | Shader 管理 |
| `particles` | GPU 粒子系统 |

## Build

```bash
cargo check -p ae_render
cargo run --release -p ae-game --bin world_viewer
```

## License

MIT

## Migration to v2

v1 已归档，新项目使用 `nova_render`（v2）。迁移指南：

| v1 | v2 |
|----|-----|
| `ae_render::Mesh` | `nova_render::assets::Mesh` |
| `ae_render::Texture` | `nova_render::assets::Texture` |
| `ae_render::Camera` | `nova_render::scene::Camera` |
| `ae_render::SsaoRenderer` | `nova_render::post_process::SsaoEffect` |
| `ae_render::SsrRenderer` | `nova_render::post_process::SsrEffect` |
| `ae_render::TaaRenderer` | `nova_render::post_process::TaaEffect` |

v2 提供 `compat-v1` feature 启用兼容层，通过 `nova_render::compat::V1Adapter` 进行资源转换。