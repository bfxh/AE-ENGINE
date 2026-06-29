# Wasteland Editor 3D 实施进度

> **任务**: 3D编辑器核心模块实施
> **开始日期**: 2026-06-26
> **最后更新**: 2026-06-27 (第十八轮 — Inspector 变换工具增强)
> **目标**: 插件系统 + MCP集成 + 真实3D Viewport + Gizmo交互 + 反射化Inspector

## 当前编译状态

✅ `cargo check -p slime-editor` 通过 (0 errors, 0 warnings)
✅ `cargo build -p slime-editor` 通过 (二进制可执行)
✅ `cargo test -p slime-editor` — 106 passed (30 lib + 30 main + 12 bridge + 34 tools), 0 failed
✅ 3D Viewport 渲染管道已连接 (SceneRenderer + GridRenderer + GizmoRenderer3D)
✅ Gizmo 拖拽交互已实现 (Translate/Scale/Rotate 三轴)
✅ Viewport 纹理动态调整大小 (跟随面板 rect)
✅ MCP HTTP bridge 端口暴露 (bound_addr + 端口文件 + Python 客户端)

## 已完成模块

### 核心架构
| 模块 | 文件 | 状态 |
|------|------|------|
| app.rs | EditorApp 中央状态容器 | ✅ 集成 plugin_registry + mcp_server + mcp_transport_handle + mcp_debug_panel + mcp_http_bridge |
| main.rs | winit + wgpu + egui 主循环 | ✅ 调用 app.init_plugins() + 3D viewport 渲染管道 |
| lib.rs | 库入口 (re-export 全部模块) | ✅ 支持集成测试 |
| Cargo.toml | [[bin]] + [lib] 双 target | ✅ |

### 3D Viewport 渲染管道
| 组件 | 文件 | 状态 |
|------|------|------|
| SceneRenderer | render/scene_renderer.rs | ✅ 渲染 mesh/light/camera 节点 (cube+sphere mesh) |
| GridRenderer | render/grid_renderer.rs | ✅ 渲染坐标网格 (主线+次线+红轴) |
| GizmoRenderer3D | render/gizmo_renderer.rs | ✅ 渲染 Translate/Rotate/Scale gizmo |
| Offscreen Texture | main.rs GpuState | ✅ RENDER_ATTACHMENT + TEXTURE_BINDING, 注册到 egui_wgpu |
| Dynamic Resize | main.rs render_frame() | ✅ 跟随 viewport_rect 动态重建纹理 + 深度缓冲 |
| Depth Buffer | SceneRenderer.depth_texture | ✅ Depth32Float |
| Shaders | shaders/scene.wgsl, grid.wgsl (内联), gizmo.wgsl (内联) | ✅ |
| viewport.rs | panels/viewport.rs | ✅ 显示 GPU 纹理 + 2D fallback + overlay |

### Gizmo 拖拽交互
| 组件 | 文件 | 状态 |
|------|------|------|
| Gizmo drag state | gizmo.rs Gizmo struct | ✅ dragging/active_axis/drag_start_transform/drag_start_rotation/drag_start_t/drag_start_vec |
| begin_drag/end_drag | gizmo.rs | ✅ 捕获起始 transform (translation+scale+rotation) |
| capture_drag_start | gizmo.rs | ✅ 记录起始拾取点 (translate/scale: t沿轴; rotate: 旋转平面上的向量) |
| update_translate | gizmo.rs | ✅ ray-ray 最近点 → 轴上平移量 |
| update_scale | gizmo.rs | ✅ ray-ray 最近点 → 轴向缩放增量 |
| update_rotate | gizmo.rs | ✅ ray-plane 交点 → 有符号旋转角 |
| ray_ray_closest_t | gizmo.rs | ✅ 两条射线的最近点参数 t |
| try_start_gizmo_drag | panels/viewport.rs | ✅ RayPicker::pick_axis_gizmo → begin_drag + capture_drag_start |
| handle_gizmo_drag | panels/viewport.rs | ✅ 每帧更新选中节点 transform |
| Picking | render/picking.rs | ✅ RayPicker + pick_axis_gizmo + pick_aabb/sphere/plane/cylinder |
| 单元测试 | gizmo.rs tests | ✅ 5个测试 (ray_ray_closest_t, translate_drag, rotate_drag, end_drag) |

### MCP 系统
| 文件 | 内容 | 状态 |
|------|------|------|
| mcp/server.rs | McpServer + JSON-RPC 分发 + poll() | ✅ 移除 unsafe (queue_result/queue_error 改为 &mut self) |
| mcp/tools.rs | 15个 MCP 工具 + execute_tool() | ✅ |
| mcp/transport.rs | Stdio/Channel/Memory + MemoryTransportHandle | ✅ |
| mcp/scene_snapshot.rs | 场景快照序列化 | ✅ |
| mcp/bridge.rs | HTTP bridge (POST /mcp, GET /mcp/responses, GET /mcp/status, GET /) | ✅ 标准库 only, 后台线程, bound_addr/mcp_url 访问器 |
| mcp/mod.rs | pub mod bridge; + re-exports | ✅ |
| panels/mcp_debug.rs | MCP调试面板 (状态+URL+端口文件路径显示) | ✅ 新增 Bridge URL 显示 |
| app.rs | EditorApp::new() 写入端口文件到 {temp_dir}/wasteland_editor_mcp_port.txt | ✅ |

### MCP 外部 AI 客户端
| 文件 | 内容 | 状态 |
|------|------|------|
| scripts/mcp_client.py | WastelandEditorClient 类 (15个工具方法 + CLI) | ✅ 标准库 only, 端口自动发现 |
| tests/mcp_bridge.rs | HTTP bridge (6) + transport sharing (2) + 端到端 (4) = 12个测试 | ✅ 全部通过 |

### 插件系统
| 文件 | 内容 | 状态 |
|------|------|------|
| plugin/plugin.rs | EditorPlugin trait (15个生命周期 hook) | ✅ |
| plugin/registry.rs | PluginRegistry (register/finish/update/dispatch) | ✅ |
| plugin/tool.rs | EditorTool trait + ToolContext | ✅ |
| plugin/dock.rs | DockPanel trait | ✅ |
| plugin/builtin.rs | SceneStatsPlugin + McpStatusPlugin + BuiltinToolsPlugin | ✅ 注册4个工具 |
| plugin/tools.rs | TranslateTool/RotateTool/ScaleTool/MeshPlacerTool | ✅ |
| plugin/mod.rs | pub mod tools; + re-exports | ✅ |

### 面板系统
| 文件 | 内容 | 状态 |
|------|------|------|
| panels/mcp_debug.rs | McpDebugPanel (请求编辑器+响应显示+快速操作+状态指示) | ✅ |
| panels/mod.rs | render_all_panels() 包含 mcp_debug_panel | ✅ |
| panels/viewport.rs | 3D viewport (引用 viewport_texture_id/size) | ✅ |
| 其他 40+ panels | 各专业面板 | ✅ |

### 测试
| 文件 | 内容 | 状态 |
|------|------|------|
| tests/mcp_tools.rs | 34个集成测试 (15个工具全覆盖) | ✅ 全部通过 |
| tests/mcp_bridge.rs | 12个测试 (6 HTTP + 2 transport + 4 端到端) | ✅ 全部通过 |

## MCP 工具清单 (15个)

1. get_scene_tree — 场景树 JSON
2. create_node — 创建节点 (empty/mesh/light/camera)
3. delete_node — 删除节点 (含子孙)
4. set_node_property — 设置属性 (name/translation/scale/rotation/path/intensity/fov)
5. get_node_properties — 获取属性
6. transform_node — 设置 transform
7. select_node — 选择节点
8. get_selection — 获取当前选择
9. save_scene — 保存场景
10. load_scene — 加载场景
11. new_scene — 新建空场景
12. validate_scene — 验证场景
13. batch_execute — 批量执行
14. get_editor_state — 获取编辑器状态
15. set_camera_view — 设置相机视角

## 内置工具 (4个, 通过 BuiltinToolsPlugin 注册)

1. TranslateTool (W) — 切换 gizmo 到平移模式
2. RotateTool (E) — 切换 gizmo 到旋转模式
3. ScaleTool (R) — 切换 gizmo 到缩放模式
4. MeshPlacerTool — 点击放置 mesh 到 y=0 地面

## EditorApp 关键字段

```rust
pub struct EditorApp {
    pub scene: Scene,
    pub camera: EditorCamera,
    pub selection: Selection,
    pub command_history: CommandHistory,
    pub gizmo: Gizmo,
    pub engine_bridge: EngineBridge,
    pub plugin_registry: PluginRegistry,       // 插件注册表
    pub mcp_server: McpServer,                 // MCP JSON-RPC 服务
    pub mcp_transport_handle: Option<MemoryTransportHandle>,  // 传输层 handle
    pub mcp_debug_panel: Option<McpDebugPanel>, // MCP 调试面板
    pub mcp_http_bridge: Option<McpHttpBridge>, // HTTP bridge (127.0.0.1:0)
    pub viewport_texture_id: Option<egui::TextureId>,
    pub viewport_texture_size: (u32, u32),
    // ... 46 个 panel 字段
}
```

## 关键架构决策

1. **MCP 传输**: MemoryTransport + MemoryTransportHandle 共享 inbox/outbox
2. **HTTP Bridge**: 127.0.0.1:0 随机端口, POST /mcp 同步等待响应(2s), GET /mcp/responses 批量拉取, GET /mcp/status 计数, GET / HTML 状态页
3. **端口发现**: EditorApp::new() 启动 bridge 后写入 {temp_dir}/wasteland_editor_mcp_port.txt, 外部 AI 客户端读取此文件发现端口
4. **Python 客户端**: scripts/mcp_client.py 仅依赖标准库 (urllib.request), WastelandEditorClient.connect() 自动发现端口, 15个工具方法 + CLI
5. **插件生命周期**: register_all() → new() → init_plugins() → finish_registration()
6. **借用检查器**: 
   - render() 用 std::mem::take 隔离 plugin_registry
   - server.rs poll() 用 scoped borrow 隔离 transport
   - mcp_debug.rs render() 在 egui 闭包前快照 bridge_url 避免 &mut app 冲突
7. **不修改**: 物理化学生物求解器、Scene 数据模型
8. **移除 unsafe**: server.rs 的 queue_result/queue_error 原先用 `&self` + raw pointer cast 修改 outbox (UB), 编译器别名优化导致响应丢失; 改为 `&mut self` 直接 push, 同时 handle_tools_list/handle_tools_call 也改为 `&mut self`
9. **面板状态持久化修复**: world_settings.rs 和 settings.rs 中 `&mut true`/`&mut false` 字面量导致 checkbox 状态不持久; settings.rs 中局部变量每帧重置导致 DragValue/Slider 编辑丢失。修复: 19个新字段加入 SettingsPanel, 6个新字段加入 WorldSettingsPanel, editor_preferences.rs Reset 按钮实现 (恢复默认值, 保留可见性/tab)
10. **移除死代码**: inspector/ 目录 (editors.rs, property.rs, registry.rs, mod.rs) 是未连接的反射系统死代码; panels/inspector.rs 使用内联渲染。已备份到 storage/CC/2_Old/ 并删除, 从 main.rs 和 lib.rs 移除 mod 声明
11. **ViewportPanel 字段连接**: ViewportPanel 的 show_grid/show_stats_overlay/show_labels/view_mode 四个字段原先仅存储不生效。修复: (a) show_grid 在 main.rs 渲染管道中条件渲染网格; (b) show_stats_overlay 控制 draw_viewport_overlay 调用 (轴gizmo+视图模式标签); (c) show_labels 控制 2D fallback 模式下节点名称标签绘制; (d) view_mode 通过新增 EditorCamera::set_orbit_angles() 方法连接到相机定位 (Perspective/Top/Front/Side 四视图), 并在视口左上角添加浮动工具栏 (视图模式选择 + Grid/Stats/Labels 开关); (e) ViewportPanel 通过 std::mem::take 模式持久化到 EditorApp.viewport_panel 字段
12. **相机增强 + WASD 飞行**: EditorCamera 新增 move_speed/invert_y 字段和 fly() 方法; orbit() 尊重 invert_y; 视口添加方向键+Space+Shift 飞行控制 (ArrowUp/Down=前后, ArrowLeft/Right=左右, Space=上, Shift=下), 使用 egui stable_dt 实现帧率无关移动; 避免与 W/E/R gizmo 快捷键冲突
13. **面板设置同步系统**: EditorApp::sync_panel_settings() 每帧调用, 将面板状态同步到实际编辑器组件: (a) SettingsPanel → EditorCamera (camera_speed→move_speed, camera_sensitivity→sensitivity); (b) SettingsPanel → egui ctx (theme_mode→dark/light visuals, ui_scale→pixels_per_point); (c) SettingsPanel → HierarchyPanel (show_node_ids→show_ids); (d) ViewModesPanel ↔ ViewportPanel 双向同步 (visible时控制viewport, 关闭时viewport值回传); (e) ViewModesPanel → EditorCamera (fov/near/far); (f) add_child_with_undo 尊重 auto_select_new_nodes 设置
14. **GridRenderer 动态重建**: 新增 rebuild(device, half_size, step) 方法重建网格几何; GpuState 添加 grid_step 跟踪字段; 渲染循环检测 SettingsPanel.grid_size 变化时自动重建网格, 用户可在设置面板调节网格步长 (0.1~10.0)
15. **Gizmo 网格吸附**: Translate 操作支持网格吸附, 在 handle_gizmo_drag 中读取 SettingsPanel.grid_snapping/snap_distance, 对 new_pos 三轴分别应用 `(v/snap).round()*snap` 公式; 用户可在设置面板开关吸附并调节 snap_distance (0.01~5.0)
16. **Undo 历史上限**: CommandHistory 新增 set_max_undo(max) 方法, 调用时若 undo_stack 超过新上限则从队首弹出多余条目; sync_panel_settings 每帧将 SettingsPanel.undo_history_limit (10~200) 同步到 command_history; 防止长会话中无限增长的历史栈占用内存
17. **2D Gizmo 尺寸可调**: gizmo.render() 接受 gizmo_size 参数, viewport.rs 从 SettingsPanel.gizmo_size (20~200px) 读取并传入, 取代原先硬编码的 60.0; 用户可在设置面板调节 gizmo 视觉尺寸
18. **命令系统单元测试**: commands/mod.rs 新增 test_set_max_undo_trims_excess (验证 set_max_undo(3) 将 5 条历史裁剪到 3 条) 和 test_undo_redo_cycle (验证 execute→undo→redo 完整循环); 使用 Scene::new_empty() 创建带 root 的场景供 add_child(0, "Child") 正常工作 (Scene::default() 的空 nodes 向量会导致 parent_id=0 查找失败)
19. **旋转吸附**: Gizmo::update_rotate 新增 snap_angle: Option<f32> 参数, 启用时将旋转角吸附到最近倍数 `(angle/snap).round()*snap`; SettingsPanel 新增 rotation_snapping(bool)/snap_angle_deg(f32, 1~90°) 字段; viewport.rs Rotate 分支读取设置并传入 snap_angle (度→弧度转换); 新增 test_gizmo_rotate_snap_to_45_degrees 测试验证 50° 旋转吸附到 45°
20. **删除确认对话框**: EditorApp 新增 pending_delete_confirmation: Option<u64> 字段; execute_pending_action 中 DeleteSelected 分支检查 SettingsPanel.confirm_deletes, 启用时设置 pending_confirmation 而非立即删除; 新增 render_delete_confirmation() 渲染居中模态 Window (显示节点名/ID + Delete/Cancel 按钮), 在 render() 中 panels 之后调用; 节点被其他途径删除时自动关闭对话框
21. **字体大小可调**: SettingsPanel.font_size (10~24px) 通过 sync_panel_settings 连接到 egui Style::text_styles; EditorApp 跟踪 last_font_size 检测变化, 当 font_size 变化时按 factor=desired/14.0 缩放所有 TextStyle 的 FontId.size (最小 6.0), 通过 ctx.set_style() 应用; Theme 标签页已有 Font Size 滑块
22. **设置持久化**: 新增 settings.rs 模块, SettingsPanel 添加 #[derive(Serialize, Deserialize)] (跳过 visible/tab UI 状态字段); 配置文件路径跨平台 (Windows: %APPDATA%/wasteland_editor/settings.json, Linux: $XDG_CONFIG_HOME/wasteland_editor/, macOS: ~/Library/Application Support/wasteland_editor/); EditorApp::new() 启动时调用 load_settings() 加载, render() 每 600 帧 (~10s) 自动保存, request_exit() 退出时保存; SettingsPanel General 标签页新增 "Save Now" 和 "Reset to Defaults" 按钮及配置文件路径显示; 2 个单元测试验证序列化往返和缺失文件处理
23. **场景自动保存**: EditorApp 新增 auto_save_timer 字段; render() 每帧累积 egui stable_dt, 当 auto_save_enabled && timer >= auto_save_interval && dirty && scene_path.is_some() 时调用 save_scene() 并重置计时器; 用户在 SettingsPanel General 标签页可开关自动保存并调节间隔 (30~3600s)
24. **最近文件跟踪**: SettingsPanel 新增 recent_files: Vec<String> 字段 (序列化持久化); 新增 add_recent_file() 方法 (去重、插入队首、按 max_recent_files 截断); save_scene_to_path() 和 open_scene_from_path() 成功时自动更新列表; General 标签页显示最近文件列表 (序号+文件名+完整路径), max_recent_files 改为可编辑 DragValue (1~50); 3 个单元测试验证去重排序、截断、空路径忽略
25. **File 菜单 Open Recent 子菜单 + 可点击打开**: 新增 EditorAction::OpenSceneFromPath(String) 变体, execute_pending_action 中调用 open_scene_from_path; menu_bar.rs File 菜单添加 "Open Recent" 子菜单 (克隆 recent_files 列表避免借用冲突, 点击文件名触发 OpenSceneFromPath); SettingsPanel General 标签页最近文件列表改为可点击按钮 (点击触发 OpenSceneFromPath), render_general 签名改为 &mut EditorApp
26. **Hierarchy 内联重命名**: HierarchyPanel 新增 renaming_node: Option<u64>/rename_buffer: String 字段; 右键 "Rename" 设置 renaming_node 并填充 rename_buffer; render_node 中当 renaming_node == Some(node_id) 时渲染 egui::TextEdit (替代 Label), 自动 request_focus; Enter 或失焦+changed 提交 (创建 RenameNodeCommand 经 command_history 执行, 设 dirty), Escape 或单纯失焦取消; 提交时空名/同名不触发命令; 新增 RenameNodeCommand (execute/undo 切换 node.name) + test_rename_node_command_undo_redo 测试
27. **节点子树复制**: Scene 新增 duplicate_subtree(node_id) 方法 — 深度遍历收集子树 id, 分配新 id 并构建 old→new 映射, 克隆每个节点修正 parent/children 引用, 复制根节点 x+2 偏移并加 "(copy)" 后缀, 挂到原节点父节点下作为兄弟; 新增 DuplicateNodeCommand (undo 存储整个克隆子树并从场景移除, redo 恢复); EditorApp::duplicate_node_with_undo() 封装复制+命令+dirty+自动选择; Hierarchy 右键 "Duplicate" 调用该方法; 新增 test_duplicate_subtree_and_undo_redo 测试验证复制/撤销/重做后节点数与子节点数正确
28. **重命名/复制快捷键 + pending_rename 桥接**: ShortcutAction 新增 Duplicate/Rename 变体, Ctrl+D → DuplicateSelected (调用 duplicate_node_with_undo), F2 → RenameSelected; EditorAction 新增对应变体, execute_pending_action 中 DuplicateSelected 直接复制选中节点, RenameSelected 设置 pending_rename: Option<u64>; HierarchyPanel::render 开头消费 pending_rename (take + 查找节点名填 rename_buffer + 设 renaming_node), 实现 winit 快捷键到 egui 内联编辑的跨层桥接; 设置面板 Shortcuts 标签页更新显示 Ctrl+D 和 F2
29. **Inspector undo 支持 (SetTransformCommand + 拖拽批处理)**: InspectorPanel 从 unit struct 改为 #[derive(Default)] 带字段 struct (transform_edit_start/name_edit_start), 存储到 EditorApp.inspector_panel (跨帧保持状态); 新增 SetTransformCommand (完整 transform: translation+rotation+scale, 替代仅 translation 的 TransformCommand); NodeTransform 添加 PartialEq derive; transform 编辑采用"拖拽批处理"模式 — drag start 时记录 pre-edit transform, drag 进行中实时 mutate (即时视觉反馈), drag stop 时提交单个 SetTransformCommand (避免每像素一个 undo 步骤); name 编辑采用 focus-loss 提交模式 — has_focus 时记录 old name, lost_focus 或 Enter 时提交 RenameNodeCommand; Reset Transform 按钮也改用 SetTransformCommand; render_all_panels 改用 std::mem::take 模式; 新增 test_set_transform_command_undo_redo 测试
30. **Gizmo 拖拽 undo 支持**: EditorApp 新增 gizmo_drag_start: Option<(u64, NodeTransform)> 字段; try_start_gizmo_drag 中 begin_drag 后记录 pre-edit transform; 鼠标释放时 end_drag + commit_gizmo_drag() 提交单个 SetTransformCommand (复用 Inspector 的命令); handle_gizmo_drag 中选择丢失时也调用 commit_gizmo_drag 防止状态泄漏; gizmo 拖拽期间仍实时 mutate (即时视觉反馈), 拖拽结束时一次性入栈 undo
31. **Inspector 类型属性 undo 支持 (SetNodeTypeCommand)**: NodeType 和 LightType 添加 PartialEq derive; 新增 SetNodeTypeCommand (execute/undo 切换整个 NodeType, 覆盖 Light color/intensity 和 Camera fov/near/far); InspectorPanel 新增 node_type_edit_start: Option<(u64, NodeType)> 和 color_idle_frames: u32 字段; render_type_properties 改为 &mut self; 滑块和 DragValue (intensity/fov/near/far) 采用 drag_stopped 提交模式 (drag start 记录 pre-edit NodeType, drag 中实时 mutate, drag stop 提交单个命令); 颜色拾取器采用空闲帧提交模式 (changed 时记录+实时 mutate+重置空闲计数, 10 帧无变化后提交 — 因 color_edit_button 无 drag_stopped 事件); render() 开头检查选中节点是否变化, 若变化则 flush 待提交的类型编辑; 新增 test_set_node_type_command_undo_redo_light 和 test_set_node_type_command_undo_redo_camera 测试
32. **Copy/Paste (Ctrl+C/Ctrl+V)**: EditorApp 新增 clipboard: Option<Vec<SceneNode>> 字段 (内部剪贴板, 非系统剪贴板, 存储整个子树的扁平 DFS 节点列表); Scene 新增 collect_subtree_nodes(node_id) 方法 (DFS 递归收集节点, root 优先) 和 paste_subtree(source_nodes, parent_id) 方法 (分配新 ID, 构建 old→new 映射, 修正 parent/children 引用, root x+2 偏移 + "(copy)" 后缀, 挂到指定父节点下); EditorApp 新增 copy_selected() (收集选中节点子树到剪贴板) 和 paste_from_clipboard() (粘贴到选中节点下, 复用 DuplicateNodeCommand 支持 undo/redo); EditorAction 新增 CopySelected/Paste 变体; ShortcutAction 新增 Copy/Paste, Ctrl+C/Ctrl+V 映射; Edit 菜单新增 Copy/Paste/Duplicate/Rename 项 (带快捷键提示和 enabled 状态); 设置面板 Shortcuts 帮助更新; 新增 4 个 scene 测试 (collect_subtree DFS 顺序, paste 创建新 ID 和修正引用, paste 无效父节点, paste 空剪贴板)
33. **拖放重父节点 (Drag-and-Drop Reparenting)**: Scene 新增 reparent_node(node_id, new_parent_id) 方法 — 完整循环检测 (拒绝: 节点不存在/根节点/自指/目标是后代/同父 no-op), 返回 Some(old_parent_id) 或 None; 新增 ReparentNodeCommand (execute 调用 reparent_node 到 new_parent, undo 调用 reparent_node 到 old_parent, 幂等设计因 reparent_node 对同父返回 None); HierarchyPanel 新增 drag_source/drop_target 两个字段 (每帧 render 开头重置 drop_target); 节点 Label 从 Sense::click() 改为 Sense::click_and_drag() (保留点击选择/双击聚焦, 增加拖拽能力); render_node 中 response.dragged() 时记录 drag_source (根节点不可拖), response.hovered() 时用 collect_subtree_nodes 检测后代关系并设置 drop_target; 渲染树之后检测 primary_released, 若 drag_source+drop_target 均有值则调用 commit_reparent (创建 ReparentNodeCommand 经 command_history 执行, 设 dirty, 自动选中移动后的节点); 视觉反馈: drop_target 绿色填充+绿色边框, drag_source 橙色边框 (除 drop_target 外); egui 0.31 的 rect_stroke 需第 4 参数 StrokeKind::Middle; 新增 3 个 commands 测试 (reparent undo/redo, 循环防护, 同父 no-op)
34. **Inspector 变换工具增强**: InspectorPanel 新增 transform_clipboard: Option<NodeTransform> 字段 (Inspector 内 Transform 剪贴板, 非系统剪贴板); Inspector 头部从单个 "Reset Transform" 按钮扩展为工具组 (right-to-left 布局): Paste T (仅当 transform_clipboard 有值时 enabled, 粘贴整个 transform 并提交 SetTransformCommand), Copy T (复制当前节点 transform 到 transform_clipboard), Snap (读取 SettingsPanel.grid_snapping/snap_distance, 对 translation 三轴分别应用 (v/snap).round()*snap 公式, 复用 gizmo 网格吸附逻辑但作为一次性操作), Reset 下拉菜单 (egui menu_button 实现, 4 项: Translation 仅重置 translation 到 ZERO 保留 rotation/scale, Rotation 仅重置 rotation 到 IDENTITY 保留 translation/scale, Scale 仅重置 scale 到 ONE 保留 translation/rotation, All 重置全部到默认值); 新增 ResetKind 枚举和 reset_partial 关联函数 (构造新 NodeTransform 并提交 SetTransformCommand, 仅当 new != old 时提交避免空命令); app.settings_panel 是 Option<SettingsPanel> 需 as_ref() 访问字段; 所有操作均通过 SetTransformCommand 支持 undo/redo


## 测试结果

### 库单元测试 (30 passed)
```
running 30 tests
test commands::tests::test_set_max_undo_trims_excess ... ok
test commands::tests::test_undo_redo_cycle ... ok
test commands::tests::test_rename_node_command_undo_redo ... ok
test commands::tests::test_set_transform_command_undo_redo ... ok
test commands::tests::test_duplicate_subtree_and_undo_redo ... ok
test commands::tests::test_set_node_type_command_undo_redo_light ... ok
test commands::tests::test_set_node_type_command_undo_redo_camera ... ok
test commands::tests::test_reparent_node_command_undo_redo ... ok
test commands::tests::test_reparent_node_command_cycle_prevention ... ok
test commands::tests::test_reparent_node_command_noop_same_parent ... ok
test scene::tests::test_collect_subtree_nodes_dfs_order ... ok
test scene::tests::test_paste_subtree_creates_new_ids_and_fixes_references ... ok
test scene::tests::test_paste_subtree_invalid_parent_returns_none ... ok
test scene::tests::test_paste_subtree_empty_clipboard_returns_none ... ok
test gizmo::tests::test_gizmo_end_drag_clears_state ... ok
test gizmo::tests::test_ray_ray_closest_t_perpendicular ... ok
test gizmo::tests::test_gizmo_translate_drag ... ok
test gizmo::tests::test_ray_ray_closest_t_parallel ... ok
test gizmo::tests::test_gizmo_rotate_drag ... ok
test gizmo::tests::test_gizmo_rotate_snap_to_45_degrees ... ok
test render::picking::tests::test_ray_aabb_miss ... ok
test render::picking::tests::test_ray_cylinder_hit ... ok
test render::picking::tests::test_ray_aabb_hit ... ok
test render::picking::tests::test_ray_plane_hit ... ok
test render::picking::tests::test_ray_sphere_hit ... ok
test settings::tests::test_load_missing_file_returns_none ... ok
test settings::tests::test_settings_roundtrip ... ok
test settings_panel::tests::test_add_recent_file_dedup_and_order ... ok
test settings_panel::tests::test_add_recent_file_ignores_empty ... ok
test settings_panel::tests::test_add_recent_file_trims_to_max ... ok

test result: ok. 30 passed; 0 failed
```

### 集成测试 (34 passed)
```
running 34 tests
test test_batch_execute_missing_commands_param ... ok
test test_batch_execute_multiple_creates ... ok
test test_batch_execute_stops_on_error ... ok
test test_create_node_camera_type ... ok
test test_create_node_defaults ... ok
test test_create_node_empty_type ... ok
test test_create_node_invalid_parent ... ok
test test_create_node_light_type ... ok
test test_create_node_mesh_type ... ok
test test_delete_node_clears_selection ... ok
test test_delete_node_missing_id_param ... ok
test test_delete_node_root_blocked ... ok
test test_delete_node_success ... ok
test test_get_editor_state ... ok
test test_get_node_properties_not_found ... ok
test test_get_node_properties_success ... ok
test test_get_scene_tree_empty_scene ... ok
test test_get_scene_tree_with_nodes ... ok
test test_load_scene_missing_file ... ok
test test_new_scene_resets ... ok
test test_save_and_load_scene_roundtrip ... ok
test test_save_scene_missing_path ... ok
test test_select_and_get_selection ... ok
test test_select_node_not_found ... ok
test test_set_camera_view_acknowledged ... ok
test test_set_node_property_missing_node ... ok
test test_set_node_property_name ... ok
test test_set_node_property_translation ... ok
test test_set_node_property_unknown_property ... ok
test test_transform_node_not_found ... ok
test test_transform_node_position ... ok
test test_unknown_tool_returns_error ... ok
test test_validate_scene_detects_issues ... ok
test test_validate_scene_fresh_is_valid ... ok

test result: ok. 34 passed; 0 failed
```

### HTTP Bridge + 端到端测试 (12 passed)
```
running 12 tests
--- HTTP transport (6) ---
test test_bridge_404_for_unknown_path ... ok
test test_bridge_index_returns_html ... ok
test test_bridge_mcp_url_accessor ... ok
test test_bridge_responses_endpoint_returns_array ... ok
test test_bridge_starts_with_valid_bound_addr ... ok
test test_bridge_status_endpoint ... ok
--- Transport sharing (2) ---
test test_transport_sharing ... ok
test test_server_poll_consumes_and_responds ... ok
--- 端到端 MCP pipeline (4) ---
test test_end_to_end_get_scene_tree ... ok
test test_end_to_end_create_node ... ok
test test_end_to_end_initialize_handshake ... ok
test test_end_to_end_multiple_requests_in_one_poll ... ok

test result: ok. 12 passed; 0 failed
```

## 现有 Scene API（不可改）

```rust
pub struct Scene { pub name: String, pub nodes: Vec<SceneNode>, pub next_id: u64 }
pub struct SceneNode { pub id: u64, pub name: String, pub parent: Option<u64>, pub children: Vec<u64>, pub transform: NodeTransform, pub node_type: NodeType }
pub struct NodeTransform { pub translation: Vec3, pub rotation: Quat, pub scale: Vec3 }
pub enum NodeType { Empty, Mesh { path: String }, Light { light_type: LightType, color: Vec3, intensity: f32 }, Camera { fov: f32, near: f32, far: f32 } }
```
