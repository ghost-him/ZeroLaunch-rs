---
description: 数据流规范：插件生命周期、IPC 关键类型对照、动作分发机制、Executor/TargetType/action_id 映射
condition: ".*"
scope: "tool:read(src-tauri/src/commands/**), tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**), tool:read(src-tauri/src/plugin_framework/**), tool:edit(src-tauri/src/plugin_framework/**), tool:write(src-tauri/src/plugin_framework/**), tool:read(src-tauri/src/core/**), tool:edit(src-tauri/src/core/**), tool:write(src-tauri/src/core/**), tool:read(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**), tool:read(src-ui/bridge/**), tool:edit(src-ui/bridge/**), tool:write(src-ui/bridge/**)"
interruptMode: never
---

# 数据流规范

> 以下数据流均基于实际代码验证（`src-tauri/src/`）。当文档与代码不一致时，以代码为准。

## 插件生命周期总览

从插件加载到用户交互再到后端执行的完整闭环。

```
┌─────────────────────────────────────────────────────────────────────┐
│  阶段一：启动加载 (lib.rs → init_plugin_system)                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. HostApi::register() — 为每个子系统创建 PluginHandle               │
│  2. 注册三层：                                                        │
│     ├─ ConfigManager   ← 配置管理（schema、持久化）                    │
│     ├─ SessionRouter   ← 运行时调度（executor、engine、booster、plugin）│
│     └─ CandidatePipeline ← 候选采集（data_source、keyword_optimizer）  │
│  3. load_from_storage() — 从磁盘恢复用户配置                           │
│  4. candidate_pipeline.collect() — 全量采集候选人 → 缓存               │
│  5. SearchPipeline 构建 — 默认引擎 + boosters + top_k=10              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  阶段二：前端展示 (搜索栏唤醒 → 用户输入 → 结果渲染)                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  用户打开启动器                                                       │
│    │                                                                  │
│    ├─ 前端调用 bridge_wake()                                          │
│    │   → 后端捕获系统参数快照（剪贴板、选中文本、活动窗口句柄）           │
│    │                                                                  │
│    ├─ 用户输入文字，前端逐键调用 bridge_query(rawQuery)                 │
│    │   → SessionRouter.route_query() 路由分发：                        │
│    │       ├─ 命中触发器关键词（如 "="）→ Plugin 模式                   │
│    │       │   → 返回 CustomPanel（插件自定义面板数据）                 │
│    │       └─ 未命中 → Search 模式                                    │
│    │           → SearchPipeline 打分排序                               │
│    │           → 映射为 ListItem[]（含 id、title、icon、actions[]）    │
│    │                                                                  │
│    └─ 前端收到 BridgeQueryResponse：                                  │
│        ├─ mode="search" → 渲染结果列表（图标、标题、副标题、快捷键提示） │
│        └─ mode="plugin_panel" → 渲染插件自定义面板（如计算器）          │
│                                                                      │
│  每个 ListItem 携带 actions[] 数组，例如：                             │
│    { id: "execute",       label: "打开",             shortcut: "" }       │
│    { id: "execute_admin", label: "以管理员身份运行", shortcut: "Ctrl+Enter" }│
│    { id: "open_folder",   label: "打开所在文件夹",   shortcut: "" }       │
│                                                                      │
│  前端根据 shortcut_key 绑定键盘快捷键，根据 is_default 标记默认动作     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  阶段三：用户确认 → 后端执行 (bridge_confirm)                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  用户选中结果并确认（回车 / 点击 / 快捷键）                              │
│    │                                                                  │
│    ├─ 前端构造 ConfirmPayload：                                       │
│    │   { candidateId, actionId, queryText, userArgs? }               │
│    │                                                                  │
│    └─ 前端调用 bridge_confirm(payload)                                │
│        → SessionRouter.route_confirm()：                              │
│            │                                                          │
│            ├─ Plugin 模式：                                           │
│            │   plugin_service.execute_action(plugin_id, action_id, payload) │
│            │                                                                  │
│            └─ Search 模式：                                                   │
│                1. 从缓存按 candidate_id 查找 SearchCandidate                  │
│                2. 构建 ExecutionContext { target, display_name, user_args,     │
│                    parameter_snapshot }                                       │
│                3. search_pipeline.record() ← 学习反馈（历史记录、查询亲和度）   │
│                4. executor_registry.resolve(ctx, action_id)                    │
│                   → 按 (TargetType, action_id) 查找对应的 ActionExecutor       │
│                5. executor.execute(ctx, action_id).await                       │
│                   → action_id 决定实际行为（见下方「动作分发机制」）              │
│                6. 若 ActivationFailed → resolve_fallback → 重试执行             │
│                                                                                │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  阶段四：会话重置 (bridge_reset)                                      │
├─────────────────────────────────────────────────────────────────────┤
│  用户关闭启动器                                                       │
│    → 前端调用 bridge_reset()                                         │
│    → SessionRouter.reset_session()                                   │
│        current_mode = None，parameter_snapshot = 空                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 关键类型对照

| 层级 | Rust 类型 | 前端类型 | 说明 |
|------|-----------|----------|------|
| 查询响应 | `BridgeQueryResponse` | `BridgeQueryResponse` | mode + results[] 或 panel 数据 |
| 搜索结果 | `BridgeSearchResult` (ListItem) | `ListItem` | id, title, subtitle, icon(base64), score, actions[], targetType |
| 确认载荷 | `ConfirmPayload` (JSON Value) | `ConfirmPayload` | candidateId, actionId, queryText, userArgs? |
| 执行动作 | `ResultAction` | `ResultAction` | id, label, icon, isDefault, shortcutKey |
| 执行目标 | `ExecutionTarget` 枚举 | — | Path / App / File / Url / Command |

### 图标解析路径

```
SearchCandidate.icon (存储的是路径字符串)
  → bridge_query() 中调用 core_handle.get_icon_or_default(icon_path)
    → L1 缓存命中 → 直接返回 base64
    → L1 未命中 → 从文件系统读取 → 缩放为 32×32 RGBA → 转 base64 → 写入 L1 缓存
  → 嵌入 BridgeSearchResult.icon 字段
  → 前端直接用 base64 data URL 渲染 <img src="data:image/png;base64,...">
```

## 动作分发机制

确认执行时，`action_id` 是决定行为的核心标识。整个分发链路：

```
前端 ConfirmPayload.actionId
  → bridge_confirm()
  → SessionRouter.route_confirm(trace_id, action_id, payload)
      → executor_registry.resolve(&exec_ctx, action_id)
          → 按 (exec_ctx.target.type → TargetType, action_id) 查表
          → 返回 Arc<dyn ActionExecutor>
      → executor.execute(&exec_ctx, action_id).await
          → executor 内部 match action_id 分发到具体方法
```

### Executor 与 TargetType / action_id 映射表

| Executor | TargetType | 支持的 action_id | 行为 |
|----------|------------|------------------|------|
| `PathExecutor` | `Path` | `execute` | 普通启动 `.exe` / `.lnk`，调用 `shell_open()` |
| | | `execute_admin` | 管理员权限启动，调用 `shell_execute_elevation()` |
| | | `open_folder` | 打开文件所在文件夹，调用 `shell_open_folder()` |
| `AppExecutor` | `App` | `execute` | 启动 UWP 应用（通过 AppUserModelId） |
| `FileExecutor` | `File` | `execute` | 用默认关联程序打开文件 |
| | | `open_folder` | 打开文件所在文件夹 |
| `UrlExecutor` | `Url` | `execute` | 在默认浏览器打开 URL |
| `CommandExecutor` | `Command` | `execute` | 执行 Shell 命令（支持参数注入） |
| `WindowActivateExecutor` | `Path`, `App` | `activate_window` | 尝试激活已有窗口，失败回退到 `execute` |

### 普通启动 vs 管理员启动的完整链路

```
用户对某个 .exe 结果按 Ctrl+Enter
  → 前端读取该 result.actions[] 中 shortcut_key="Ctrl+Enter" 的 action
    → 找到 { id: "execute_admin", label: "以管理员身份运行", shortcutKey: "Ctrl+Enter" }
  → 前端调用 bridge_confirm({ candidateId: 42, actionId: "execute_admin", ... })

  → 后端 route_confirm():
      1. 从缓存查找 candidate_id=42 → ExecutionTarget::Path("C:\\...\\app.exe")
      2. executor_registry.resolve(ctx, "execute_admin")
         → 查表键 = (TargetType::Path, "execute_admin")
         → 命中 → 返回 Arc<PathExecutor>
      3. path_executor.execute(ctx, "execute_admin")
         → match action_id { "execute_admin" => self.execute_elevation(path) }
         → PluginHandle::shell_execute_elevation("C:\\...\\app.exe")
         → Windows ShellExecuteW + "runas" 谓词 → UAC 弹窗 → 管理员启动

对比普通启动（直接回车）：
  → actionId="execute" → match action_id { "execute" => self.execute_normal(path) }
  → PluginHandle::shell_open(OpenTarget::File(path))
  → Windows ShellExecuteW 默认谓词 → 普通权限启动
```

### 窗口激活的 Fallback 机制

```
用户对某程序按 Shift+Enter（activate_window）
  → WindowActivateExecutor.execute(ctx, "activate_window")
    → 尝试 FindWindow + SetForegroundWindow
    → 成功 → Ok(())
    → 失败 → Err(ExecutionError::ActivationFailed { fallback_action: "execute" })

  → SessionRouter.route_confirm() 捕获 ActivationFailed：
      → executor_registry.resolve_fallback(ctx, "execute")
        → 按 TargetType 查找该类型的默认 executor（如 PathExecutor）
      → fallback_executor.execute(ctx, "execute")
        → 普通启动目标程序（作为窗口激活失败后的降级策略）
```

### 核心设计原则

- **action_id 是行为标识**：同一 executor 通过 match action_id 分发到不同方法，对外暴露为 `supported_actions()` 列表
- **TargetType 是路由键**：每个 executor 声明支持的 `TargetType`，ExecutorRegistry 按 `(TargetType, action_id)` 复合键查表
- **shortcut_key 是前端绑定依据**：executor 声明动作时携带 `shortcut_key`（如 `"Ctrl+Enter"`），前端据此绑定键盘快捷键，用户按键时自动选择对应的 action_id
- **Fallback 是容错机制**：executor 可通过 `ActivationFailed` 错误声明降级策略，由 SessionRouter 统一处理回退
