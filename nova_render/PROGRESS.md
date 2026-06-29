# nova_render 大任务进度

> 最后更新: 2026-06-29

## 当前大任务：方向 2->3->4->最后 1（用户指令 "234 最后到1"）

### 方向 2: 推进所有 pass 的 execute() 完整实现 -- DONE

- [x] 扫描所有 pass 的 execute() stub -- 14 个 execute()，1 个 STUB（shadow.rs）
- [x] ShadowMapPass 补 mesh 注册接口（register_mesh, update_instances, set_instance_count + execute() 渲染 mesh）
- [x] ShadowPass 实现 CSM 聚合（持有 Vec<ShadowMapPass>，每级 cascade 独立 VP + 委托渲染）
- [x] 编译验证（111 测试 passed）

### 方向 3: RenderGraph Phase 2 -- DONE

- [x] 资源自动管理：
  - ResourceDesc 扩展（buffer 支持 + texture()/buffer() 构造器 + with_label()）
  - ResourceCache 添加 acquire_buffer/release_buffer
  - NodeContext 添加 request_texture/request_buffer（自动管理，帧末释放回缓存）
  - execute() 集成 ResourceCache（frame_textures/frame_buffers 跟踪 + 帧末释放）
  - resource_hash 扩展（包含 buffer 字段）
- [x] barrier 同步：wgpu 同一 CommandEncoder 内 RenderPass 顺序执行，load/store 隐式处理 barrier（无需显式代码）
- [x] 编译验证（111 测试 passed）

### 方向 4: 扩展生物模拟 / v1 push -- IN PROGRESS

- [ ] 扩展更多生物模板
- [ ] v1 push 到 GitHub 远程

### 最后 1: 实际运行 demo

- [ ] cargo run --example forward_demo
- [ ] cargo run --example composite_demo

## 历史完成

- ✅ visibility_buffer.rs execute() 实现
- ✅ ForwardPass auto_animate_camera 字段 + execute() 自动动画
- ✅ forward_demo.rs 重写为 auto_animate 模式
- ✅ composite_demo.rs 重写为 auto_animate 模式
- ✅ 全量编译验证（111 测试 passed）
- ✅ RenderGraph downcast 能力（node_mut/node_ref + impl_rgn_downcast! 宏）
- ✅ ShadowMapPass mesh 注册接口
- ✅ ShadowPass CSM 聚合实现
- ✅ RenderGraph Phase 2 资源自动管理
