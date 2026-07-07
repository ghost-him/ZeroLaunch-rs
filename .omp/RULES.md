# 工程纪律（粘性规则 — 始终生效）

## Async 契约

- `ActionExecutor::execute` 是 `async fn`。MUST `.await`
- 错误 MUST 用 `?` 或 `.map_err()` 传播
- 调用异步 SDK 方法：MUST 直接用 `.await`

## 同步锁守卫生命周期（跨 `.await`）

本规则只约束**同步锁**，`tokio::sync::*` 异步锁豁免（见末尾）。

- **同步锁**：`parking_lot::Mutex/RwLock`、`std::sync::Mutex/RwLock`，以及内部使用它们的容器（如 `DashMap` 的 `Ref`/`RefMut`）。
- 这些守卫通常 `!Send`（parking_lot/std），跨 `.await` 会让 future 变 `!Send` → `tokio::spawn` 编译失败或运行时 panic；即便容器手动 `impl Send`（如 `DashMap::Ref`），跨 `.await` 长期持有阻塞式锁会阻塞同锁/同分片的其它任务，且可能死锁。
- 因此同步锁守卫 MUST 在任何 `.await` 点之前释放。
- **正确**（块作用域）：`let data = { let guard = lock.read(); guard.field.clone() }; /* 守卫已释放 */; something().await;`
- **正确**（闭包作用域，推荐用于 `DashMap` 等）：`let data = map.get(&k).map(|r| r.value().clone()); something().await;` —— 守卫仅在闭包内存活，返回即释放，无需手动 `drop`。
- **错误**：`let guard = lock.read(); something().await; /* 守卫仍存活 → future !Send */`
- 此规则适用于所有异步代码路径：SessionRouter、ConfigManager、任何持有同步 `RwLock`/`Mutex`/`DashMap` 的异步函数。
- 当需要读锁且后面有 `.await`：把数据 clone 到局部变量（或 owned `Arc`），让守卫在 `.await` 前释放，再 `.await`。
- **`tokio::sync::*` 异步锁豁免**：`tokio::sync::Mutex/RwLock` 等的守卫是 `Send`，且 `read()`/`write()` 本身是 `.await`，**设计上允许**跨 `.await` 持有，不受本规则约束。但仍应缩短临界区；tokio 官方建议：短临界区优先用同步锁，仅当必须跨 `.await` 持有锁时才用 `tokio::sync` 锁。

## 死代码纪律

- 无用代码立即删除。Git 历史是唯一备份。NEVER 创建 `.bak`、`.copy`、`.old`、`_backup` 文件。
- 删除模块文件时，同步从 `mod.rs` 中移除 `mod` 声明。
- `// temp`、`// 临时`、`// 待重构`、`// TODO: remove` 标记存活超过一次会话 → 立即处理。
- NEVER 提交文件名暗示为副本或备份的文件（如 `lib copy.rs`、`old_search.rs`）。

## 变更纪律

- 优先扩展现有抽象。NEVER 在现有架构能容纳需求时引入新的模块、trait 或层。
- 重构前：确认变更可放入当前的 plugin/pipeline/config 框架。只有在现有抽象被证明无法容纳时才提出新抽象。

## JSON 数值安全

- 从 `serde_json::Value` 读取数值时，MUST 用 `as_f64()`。前端可能对整数字段发送浮点数，`as_i64()` 遇到浮点数静默返回 0。
- **正确**：`value.as_f64().map(|v| v as i32).unwrap_or(default)`
- **错误**：`value.as_i64().unwrap_or(default)`

## 前后端职责边界

- 前端负责 **数据显示** 与 **用户交互**。NEVER 在前端代码中实现业务逻辑、直接操作文件系统、启动程序或调用平台 API。
- 后端负责 **数据持久化** 与 **逻辑控制**。所有平台操作（文件读写、程序启动、系统调用）MUST 通过 IPC 命令委托给后端。
- 前端是"薄"展示层：接收后端结构化数据 → 渲染为 DOM → 收集用户输入 → 通过 IPC 回传后端。
- **正确**：前端调用 `bridge_query` 获取搜索结果并渲染列表。**错误**：前端直接读取文件系统构建候选列表。
- **正确**：前端通过 `bridge_confirm` 委托后端执行程序启动。**错误**：前端直接调用 shell 或进程 API。
- 新增功能时：先确认逻辑属于后端还是前端。纯 UI 交互（主题切换动画、键盘导航、窗口大小调整）可放前端；涉及数据、文件、进程、网络的操作 MUST 放后端。

## 用户交互

- 需要向用户提问时，MUST 使用 `ask` 工具。
- 如果用户的需求涉及到了实际的代码的更改，则在更改之前，MUST 先使用 plan 模式生成一个深度而又详细的计划，返回给用户。由用户确认以后，再完成代码的更改。

## 软件工程

- 在做代码更改时，MUST 在每个新写的函数前，写精简的一段注释，描述这个函数的功能、输入输出、错误情况等。
- 对于已有函数，如果它的内部存在注释，则需要检查该注释是否正确，NEVER 将正确的注释删除；如果注释不正确，则需要修正该注释。除非用户要求，注释 MUST 使用中文书写。

## 冒烟测试

- 任何涉及 `sdk.rs`、`core/`、`plugin_framework/` 或 `commands/` 的改动后，至少验证 `cargo check` 零错误通过。

## AppState 访问规范

- `Arc<AppState>` MUST 通过 Tauri 的 `app.state::<Arc<AppState>>()` 或 `app_handle.state::<Arc<AppState>>()` 获取
- 在 `commands/` 层通过 `tauri::State<Arc<AppState>>` 注入
- 在回调闭包（如 hotkey callback、focus callback、deep-link handler）中使用 `move` 闭包捕获预先获取的 `Arc<AppState>` clone
- NEVER 通过全局静态访问 `AppState`

## 日志规范

- MUST 使用 `tracing` crate（`info!`、`debug!`、`warn!`、`error!`）
- 启动阶段关键步骤使用 `info!`。运行时频繁调用使用 `debug!`
- 错误恢复后使用 `warn!`。不可恢复错误使用 `error!`
- NEVER 在日志中输出用户敏感信息（密码、完整路径中的用户名等）
