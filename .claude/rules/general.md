---
paths:
  - "**"
---

# 通用工程纪律

## Async 契约

- `ActionExecutor::execute` 是 `async fn`。**必须** 用 `.await`
- 错误 **必须** 用 `?` 或 `.map_err()` 传播
- 调用异步 SDK 方法：**必须** 直接用 `.await`

## RwLock 守卫生命周期

- `parking_lot::RwLock*Guard` 是 `!Send`。守卫 **必须** 在任何 `.await` 点之前释放。
- **正确**：`{ let guard = lock.read(); let data = guard.field.clone(); } /* 守卫已释放 */; something().await;`
- **错误**：`let guard = lock.read(); something().await; /* 守卫仍存活 → future !Send */`
- 此规则适用于所有异步代码路径：SessionRouter、ConfigManager、任何持有 `RwLock`/`Mutex` 的异步函数。
- 当需要读 RwLock 且后面有 `.await`：将数据 clone 到局部变量，释放守卫，然后再 `.await`。

## 死代码纪律

- 无用代码立即删除。Git 历史是唯一备份。**禁止** 创建 `.bak`、`.copy`、`.old`、`_backup` 文件。
- 删除模块文件时，同步从 `mod.rs` 中移除 `mod` 声明。
- `// temp`、`// 临时`、`// 待重构`、`// TODO: remove` 标记存活超过一次会话 → 立即处理。
- **禁止** 提交文件名暗示为副本或备份的文件（如 `lib copy.rs`、`old_search.rs`）。

## 变更纪律

- 优先扩展现有抽象。**禁止** 在现有架构能容纳需求时引入新的模块、trait 或层。
- 重构前：确认变更可放入当前的 plugin/pipeline/config 框架。只有在现有抽象被证明无法容纳时才提出新抽象。

## JSON 数值安全

- 从 `serde_json::Value` 读取数值时，**必须** 用 `as_f64()`。前端可能对整数字段发送浮点数，`as_i64()` 遇到浮点数静默返回 0。
- **正确**：`value.as_f64().map(|v| v as i32).unwrap_or(default)`
- **错误**：`value.as_i64().unwrap_or(default)`

## 前后端职责边界

- 前端负责 **数据显示** 与 **用户交互**。**禁止** 在前端代码中实现业务逻辑、直接操作文件系统、启动程序或调用平台 API。
- 后端负责 **数据持久化** 与 **逻辑控制**。所有平台操作（文件读写、程序启动、系统调用）**必须** 通过 IPC 命令委托给后端。
- 前端是"薄"展示层：接收后端结构化数据 → 渲染为 DOM → 收集用户输入 → 通过 IPC 回传后端。
- **正确**：前端调用 `bridge_query` 获取搜索结果并渲染列表。**错误**：前端直接读取文件系统构建候选列表。
- **正确**：前端通过 `bridge_confirm` 委托后端执行程序启动。**错误**：前端直接调用 shell 或进程 API。
- 新增功能时：先确认逻辑属于后端还是前端。纯 UI 交互（主题切换动画、键盘导航、窗口大小调整）可放前端；涉及数据、文件、进程、网络的操作 **必须** 放后端。

## 用户交互

- 需要向用户提问时，**必须** 使用 `AskUserQuestion` 工具。
- 如果用户的需求涉及到了实际的代码的更改，则在更改之前，**必须** 先使用 plan 模式生成一个深度而又详细的计划，返回给用户。由用户确认以后，再完成代码的更改。

## 软件工程

- 在做代码更改时，**必须** 在每个新写的函数前，写精简的一段注释，描述这个函数的功能、输入输出、错误情况等。
- 对于已有函数，如果它的内部存在注释，则需要检查该注释是否正确，**禁止** 将正确的注释删除；如果注释不正确，则需要修正该注释。除非用户要求，注释 **必须** 使用中文书写。

## 冒烟测试

- 任何涉及 `sdk.rs`、`core/`、`plugin_framework/` 或 `commands/` 的改动后，至少验证 `cargo check` 零错误通过。

## AppState 访问规范

- `Arc<AppState>` **必须** 通过 Tauri 的 `app.state::<Arc<AppState>>()` 或 `app_handle.state::<Arc<AppState>>()` 获取
- 在 `commands/` 层通过 `tauri::State<Arc<AppState>>` 注入
- 在回调闭包（如 hotkey callback、focus callback、deep-link handler）中使用 `move` 闭包捕获预先获取的 `Arc<AppState>` clone
- **禁止** 通过全局静态访问 `AppState`

## 文件命名约定

- Vue 组件文件名使用 `PascalCase`（如 `DynamicForm.vue`）
- TypeScript 工具文件使用 `camelCase`（如 `schemaTypes.ts`）
- Store 文件使用 `kebab-case` + `-store` 后缀（如 `config-store.ts`）
- Composable 文件使用 `camelCase` + `use` 前缀（如 `useKeyboard.ts`）


## 日志规范

- **必须** 使用 `tracing` crate（`info!`、`debug!`、`warn!`、`error!`）
- 启动阶段关键步骤使用 `info!`。运行时频繁调用使用 `debug!`
- 错误恢复后使用 `warn!`。不可恢复错误使用 `error!`
- **禁止** 在日志中输出用户敏感信息（密码、完整路径中的用户名等）
