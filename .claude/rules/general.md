# 通用工程纪律

## Async 契约

- `ActionExecutor::execute` 是 `async fn`。**必须** 用 `.await`。**禁止** 使用 `tauri::async_runtime::block_on`、`tokio::runtime::Handle::block_on` 或任何在异步运行时上的同步阻塞。
- 错误用 `?` 或 `.map_err()` 传播。**禁止** 用 `tokio::spawn` fire-and-forget 并静默吞掉错误。
- 调用异步 SDK 方法：直接用 `.await`。**禁止** 包在 `block_on` 或 `spawn_blocking` 中。

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

- 从 `serde_json::Value` 读取数值时，**必须** 用 `as_f64()`。**禁止** 用 `as_i64()`。前端可能对整数字段发送浮点数，`as_i64()` 遇到浮点数静默返回 0。
- **正确**：`value.as_f64().map(|v| v as i32).unwrap_or(default)`
- **错误**：`value.as_i64().unwrap_or(default)`

## 用户交互

- 需要向用户提问时，**必须** 使用 `AskUserQuestion` 工具。**禁止** 自行假设或猜测用户意图。
- 在改动公共 API、模块结构或配置 schema 之前先确认。

## 冒烟测试

- 任何涉及 `sdk/`、`core/`、`plugin_system/` 或 `commands/` 的改动后，至少验证 `cargo check` 零错误通过。
