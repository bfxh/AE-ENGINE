# Wasteland 项目 AI 自治规范 v2.0

> 适用AI: Claude/GPT/DeepSeek/Kimi/所有AI助手
> 生效范围: wasteland_project 全生命周期
> 执行模式: 自动化检查 + 人工复审

---

## 0. 核心理念

**AI是工具，不是主人。** 所有行为必须：
- 可审计：每一步变更可追溯
- 可回滚：任何操作都可以撤销
- 可验证：每个输出都能被独立验证
- 可复现：相同输入产生相同输出

---

## 1. 安全铁律 (BLOCK级 - 违反即停止)

### 1.1 提交禁止
- ❌ 禁止 git commit（除非用户明确说"提交"）
- ❌ 禁止 git push（除非用户明确说"推送"）
- ❌ 禁止 git push --force / --force-with-lease（任何情况下）
- ❌ 禁止修改 .git/config

### 1.2 删除保护
- ❌ 禁止删除含 .exe/.dll/.msi/.sys 的目录
- ❌ 禁止删除 D:\rj\ 下的任何目录
- ❌ 禁止永久删除（禁止绕过回收站）
- ✅ 删除前必须：列清单 → 备份到 CC/2_Old/ → 等用户确认
- ✅ 只允许 Remove-Item（进回收站）

### 1.3 敏感信息
- ❌ 禁止暴露 API Key/Token/Password 到代码或日志
- ❌ 禁止 commit .env / credentials.json / secrets
- ✅ 敏感文件使用 .gitignore 排除

### 1.4 代码安全
- ❌ 禁止裸 eval() / exec()
- ❌ 禁止 except: pass（静默吞噬异常）
- ❌ 禁止硬编码绝对路径
- ❌ 禁止 subprocess(shell=True) 带用户输入

---

## 2. 开发规范 (HIGH级)

### 2.1 代码风格
- 不加注释（除非明确要求）
- 优先编辑现有文件，不新建
- 遵循项目既有代码风格
- 函数单一职责，不超过 100 行

### 2.2 命名规范
| 语言 | 变量 | 函数 | 类/结构体 | 文件名 |
|------|------|------|-----------|--------|
| Rust | snake_case | snake_case | PascalCase | snake_case.rs |
| Python | snake_case | snake_case | PascalCase | snake_case.py |
| GDScript | snake_case | snake_case | PascalCase | snake_case.gd |

### 2.3 资源管理
- 3D模型：CC0许可优先, GLB格式, 三角面<30K
- 纹理：2K分辨率为主, PNG/WebP格式, PBR材质
- 音效：WAV 48kHz/24bit, 压缩为 OGG Vorbis

### 2.4 性能目标
- 最低帧率：50fps（复杂场景）
- 目标帧率：60fps
- 内存预算：<4GB（RTX 4060 8G VRAM）
- 加载时间：<30秒（初始场景）

### 2.5 平台优先
- 主开发平台：Linux (WSL2 Ubuntu)
- 目标平台：Windows 10/11 x64
- 路径：os.path.join / Path（不用硬编码）
- 行尾：LF（不用CRLF）

---

## 3. 测试规范 (HIGH级)

### 3.1 修改后必做
1. cargo check --workspace（Rust代码修改后）
2. cargo test --workspace（功能修改后）
3. 至少3个不同场景测试
4. 边界条件测试
5. 回滚验证

### 3.2 测试覆盖率要求
- 核心引擎 crate：>80%
- GDExtension 桥接：>60%
- 工具脚本：>50%
- Godot场景脚本：>40%（smoke test）

---

## 4. 知识管理 (MEDIUM级)

### 4.1 发现即记录
- 新项目/新技术 → 知识库
- 错误+根因+修复 → ERROR-LESSONS.md
- 重要决策 → 项目 memory

### 4.2 文档更新
- 修改代码 → 同步更新关联文档
- 新增模块 → 更新索引
- 优先编辑现有文档，不新建

---

## 5. 漏洞管理 (HIGH级)

### 5.1 扫描频率
- 每次代码修改后：comprehensive_scanner.py
- 每次依赖更新：cargo audit
- 每周：全量扫描 + 依赖审计

### 5.2 漏洞处理
| 严重度 | 响应时间 | 处理 |
|--------|----------|------|
| CRITICAL | 立即 | 停止开发, 修复后继续 |
| HIGH | 当天 | 当前迭代修复 |
| MEDIUM | 本周 | 记录, 排期修复 |
| LOW | 本月 | 记录, 择机修复 |

### 5.3 扫描工具
- Rust: cargo audit, comprehensive_scanner.py
- Python: bandit, pip-audit, safety
- GDScript: gdscript_scanner.py
- 全部: comprehensive_scanner.py 统一入口

---

## 6. 工作流程

### 6.1 开发前
1. 确认任务类型→判定 LEVEL
2. LEVEL 3: 列出操作清单→等用户确认
3. 检查是否存在关联内存/上下文

### 6.2 开发中
1. 语义搜索理解代码
2. 列出修改清单
3. 备份到 CC/2_Old/
4. 执行修改
5. cargo check + cargo test

### 6.3 开发后
1. comprehensive_scanner.py
2. 至少3个场景测试
3. 记录到知识库
4. 报告完成状态

---

## 7. 交互协议

### 7.1 意图签名
- `?` = 草稿，只回"已阅"
- `!` = 讨论基准，可延伸
- `!!` = 执行指令，立刻行动

### 7.2 确认机制
- 不确定意图时：追问，不假设
- 每次输入视为独立事件
- 只有明确"确认"/"执行"/"提交"/"就用这个"才行动

### 7.3 汇报格式
```
[COMPLETED] 任务描述
Files: 修改的文件列表
Tests: 测试结果 (passed/failed)
Issues: 发现的问题
Next: 下一步建议
```

---

## 8. 违规处理

| 次数 | 处理 |
|------|------|
| 3次 | 警告通知 |
| 5次 | 强制暂停操作 |
| 10次 | 限制操作权限 |

---

## 9. 规范更新

- 发现新问题→立即追加规则
- 规则冲突→以更严格的为准
- 每季度审查一次完整性

---

**版本**: v2.0
**生效日期**: 2026-06-08
**下次审查**: 2026-09-08