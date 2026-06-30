# Nova Render v2

> **Version**: 0.1.0
> **Status**: Active development

高可维护性渲染框架，基于 wgpu 24。

## 设计理念

借鉴 4 个开源渲染引擎的最佳实践：

| 引擎 | 借鉴点 |
|------|--------|
| **bevy** | 双 World 分离（MainWorld↔RenderWorld）、四阶段管线（Extract/Prepare/Queue/Render）、PipelineCache、乒乓纹理 |
| **rend3** | Handle+Arc 资源管理、RenderGraph、Megabuffer、Material Trait ABI、profiling crate |
| **kajiya** | 后处理栈顺序、RenderGraph（handle+temporal+imageops）、FidelityFX Shadow Denoiser |
| **Fyrox** | Pool+强类型Handle、Visit trait IR、Prefab 继承、UUID 资源、GraphicsServer trait |

## 模块架构

```
nova_render/
├── core/           # Handle + Pool + World 分离 + Extract
├── backend/        # GraphicsServer trait + wgpu 实现 + PipelineCache
├── render_graph/   # DAG + temporal + imageops
├── assets/         # Mesh + Megabuffer + Texture + Material trait + Shader
├── scene/          # Scene Graph + Node + Camera + Light + Prefab
├── renderer/       # PhaseItem + RenderPhase + Culling + Batch
├── passes/         # Shadow / Forward / Skybox / Water / Particles
├── post_process/   # EffectStack + Bloom / Tonemap / TAA / SSAO / SSR / ...
├── gi/             # DDGI + SSGI + RT
├── serialize/      # Visit trait + Prefab
├── compat/         # v1_adapter + engine_bridge
└── profiling/      # 性能追踪
```

## Features

- `default`: 基础渲染框架
- `ktx2`: KTX2/BasisU 纹理压缩
- `profile`: 性能追踪（profiling crate）
- `compat-v1`: v1 ae_render 兼容层
- `engine-bridge`: ae_engine GameWorld 对接

## Build

```bash
# 默认编译
cargo check -p nova_render

# 启用所有 feature
cargo check -p nova_render --features compat-v1,engine-bridge
```

## License

MIT